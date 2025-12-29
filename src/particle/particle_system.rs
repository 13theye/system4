//! src/particle/particle_system.rs
//!
//!
//! The Particle System of System 4
use std::collections::HashMap;

use nannou::noise::{NoiseFn, Perlin, Seedable};
use nannou::prelude::*;
use nnpipe::renderers::{ParticleRenderer, SegmentRenderer};
use rand::{rngs::ThreadRng, seq::SliceRandom, Rng};
use rayon::prelude::*;
use std::time::Instant;

use crate::{
    forces::{field::ForceFields, wind_circle::WindCircle},
    groups::{Voice, VoiceId},
    particle::{to_segment_gpu, ParticleCore, ParticleFeedback},
    utils::tween,
};

use super::constants::*;

pub struct ParticleSystem {
    // Split particle storage for cache locality
    // Core: hot data for physics updates (~100 bytes per particle)
    pub particle_cores: HashMap<VoiceId, Vec<ParticleCore>>,
    // Feedback: cold data for trail rendering (~3.4KB per particle)
    pub particle_feedback: HashMap<VoiceId, Vec<ParticleFeedback>>,

    // forces
    pub force_fields: ForceFields,

    // timestamp for framerate-independent physics updates
    pub last_update: Instant,

    // Global params
    pub params: ParticleSystemParams,
    // Mass variation parameters
    pub mass_var_params: MassVarianceParams,

    // Perlin noise generator
    pub perlin_gen: Perlin,
}

/// Global parameters for the ParticleSystem
pub struct ParticleSystemParams {
    // Center-origin of the system
    pub origin: Point2,
    // Size of the system in pixels
    pub bounds_size: Vec2,
    // Global limit on particle count
    pub overall_particle_limit: usize,
    // Default color at spawn time
    pub default_particle_color: Rgb,
    // Particle size in pixels
    pub default_particle_size: f32,
    // Number of particles that can be spawned into the system per frame
    pub global_max_spawn_rate: f32,
}

/// Parameters defining per-frame mass variance for particles
pub struct MassVarianceParams {
    pub enabled: bool,
    // percentage of base mass to vary (e.g., 0.1 = 10%)
    pub amount: f32,
}

impl ParticleSystem {
    pub fn new(
        origin: Point2,
        width: f32,
        height: f32,
        default_particle_size: f32,
        default_particle_color: Rgb,
        overall_particle_limit: u32,
    ) -> Self {
        let bounds_size = Vec2::new(width, height);

        let params = ParticleSystemParams {
            origin,
            bounds_size: vec2(width, height),
            overall_particle_limit: overall_particle_limit as usize,
            default_particle_color,
            default_particle_size,
            global_max_spawn_rate: PARTICLE_MAX_SPAWN_RATE,
        };

        // Initialize mass variance parameters
        let mass_var_params = MassVarianceParams {
            enabled: true,
            amount: 0.05,
        };

        Self {
            particle_cores: HashMap::new(),
            particle_feedback: HashMap::new(),
            force_fields: ForceFields::new(origin, bounds_size),
            params,
            mass_var_params,

            // Init Perlin noise generator
            perlin_gen: Perlin::new(),

            last_update: Instant::now(),
        }
    }

    /********************* Update methods ********************************** */

    /// ZERO-COPY UPDATE: Updates particles and writes directly to GPU staging memory
    /// Returns (particle_count, segment_count) of particles written to GPU buffers
    #[allow(clippy::too_many_arguments)]
    pub fn update_zero_copy(
        &mut self,
        voices: &mut HashMap<VoiceId, Voice>,
        rng: &mut ThreadRng,
        queue: &nannou::wgpu::Queue,
        particle_renderer: &ParticleRenderer,
        segment_renderer: &SegmentRenderer,
        now: Instant,
        perlin_seed: u32,
    ) -> (usize, usize) {
        let frame_start = Instant::now();

        // Calculate time delta
        let dt = (now - self.last_update).as_secs_f32();

        // Calculate the bounds rect
        let bounds_rect = self.bounds_rect();

        // Update perlin nouse with app time as seed
        self.perlin_gen = self.perlin_gen.set_seed(perlin_seed);

        // how many frames have passed with 60fps target
        let framerate_factor = (dt / 0.0167).min(1.5);

        self.handle_particle_emission(voices, rng);
        self.cull_excess_particles(voices);

        let physics_start = Instant::now();

        // Pre-compute "vibration" position offset factors for all voices
        let vibration_values: HashMap<VoiceId, f32> = voices
            .values()
            .map(|voice| (voice.id, voice.params.vibration))
            .collect();

        // Init empty particle counters
        let mut total_particle_count = 0;
        let mut total_segment_count = 0;
        let mut voice_particle_counts: HashMap<VoiceId, usize> = HashMap::new();

        // Init Hashmap to Store computed offsets from physics loop to reuse in GPU write
        let mut computed_offsets_map: HashMap<VoiceId, Vec<Vec2>> = HashMap::new();

        for (voice_id, cores) in self.particle_cores.iter_mut() {
            let voice = voices.get(voice_id);
            let color_limit = voice.map(|v| v.params.color_limit);
            let alpha_limit = voice.map(|v| v.params.alpha_limit);

            if color_limit.is_none() || alpha_limit.is_none() {
                continue;
            }

            let (color_limit, alpha_limit) = (color_limit.unwrap(), alpha_limit.unwrap());

            // Interpolate color (same for all particles)
            let color = tween::interpolate_color(
                color_limit,
                rgb(PARTICLE_HIGH_R, PARTICLE_HIGH_G, PARTICLE_HIGH_B),
                FADE_DURATION,
                RAMP_UP_PERCENT,
                DWELL_PERCENT,
                RAMP_CURVE_EXPONENT,
                FADE_CURVE_EXPONENT,
                now,
                self.last_update,
            );

            // Pre-compute mass variance using bulk RNG fill for better performance
            let mut mass_variations = vec![0.0; cores.len()];
            if self.mass_var_params.enabled && self.mass_var_params.amount > 0.0 {
                rng.fill(&mut mass_variations[..]);
                // Scale from [0.0, 1.0) to [-mass_variation_amount, mass_variation_amount]
                for v in &mut mass_variations {
                    *v = (*v * 2.0 - 1.0) * self.mass_var_params.amount;
                }
            }

            // Pre-compute "vibration" position offset factors
            let vibration = vibration_values.get(voice_id).copied().unwrap_or(0.0);
            let mut offset_factors = vec![0.0; cores.len()];
            if vibration > 0.0 {
                rng.fill(&mut offset_factors[..]);
                // Scale from [0.0, 1.0) to [-vibration, vibration]
                for v in &mut offset_factors {
                    *v = (*v * 2.0 - 1.0) * vibration;
                }
            }

            // Retrieve a mutable reference to the feedback array for this voice
            let feedback_array = self.particle_feedback.get_mut(voice_id).unwrap();

            // Pre-allocate storage for computed offsets (to save for GPU write)
            let mut computed_offsets = vec![vec2(0.0, 0.0); cores.len()];

            // Collect all WindCircles for all voices
            let circles_vec: Vec<&WindCircle> = voices
                .values()
                .flat_map(|v| v.wind_circles.values())
                .collect();

            // Physics update for each particle
            // HOT LOOP in parallel execution
            cores
                .par_iter_mut()
                .zip(feedback_array.par_iter_mut())
                .zip(computed_offsets.par_iter_mut())
                .enumerate()
                .for_each(|(index, ((core, feedback), computed_offset))| {
                    let mass_variation_factor = mass_variations[index];
                    let noise_factor = self
                        .perlin_gen
                        .get([core.position.x as f64, core.position.y as f64]);

                    // Stage force applications
                    self.force_fields.apply_forces_to_particle(
                        core,
                        &circles_vec,
                        mass_variation_factor,
                        noise_factor,
                    );

                    // Apply forces and color changes to particle core
                    core.update(color, alpha_limit, framerate_factor);

                    // Calculate and apply offset
                    let offset = if vibration > 0.0 && core.velocity.length_squared() > 0.0 {
                        let normal = vec2(-core.velocity.y, core.velocity.x).normalize_or_zero();
                        normal * PARTICLE_MAX_POSITION_OFFSET * offset_factors[index]
                    } else {
                        vec2(0.0, 0.0)
                    };

                    // Store the computed offset for GPU write
                    *computed_offset = offset;

                    // Apply the offset (adding zero is fast, no need to check)
                    let offset_position = core.position + offset;
                    feedback.record(offset_position, color);

                    if core.is_out_of_bounds(bounds_rect) {
                        core.kill();
                    }
                });

            // Save computed offsets for GPU write (must use SAME offsets)
            computed_offsets_map.insert(*voice_id, computed_offsets);

            // Count alive particles
            let alive_count = cores
                .iter()
                .filter(|c| c.is_alive && c.is_activated)
                .count();
            voice_particle_counts.insert(*voice_id, alive_count);
            total_particle_count += alive_count;
            total_segment_count += alive_count;

            // Cull dead particles
            let mut write_index = 0;
            for read_index in 0..cores.len() {
                if cores[read_index].is_alive {
                    if write_index != read_index {
                        cores[write_index] = cores[read_index];
                        feedback_array[write_index] = feedback_array[read_index].clone();
                    }
                    write_index += 1;
                }
            }
            cores.truncate(write_index);
            feedback_array.truncate(write_index);
        }

        let physics_time = physics_start.elapsed();
        println!(
            "[Physics] Parallel update + culling: {:.3}ms ({} particles)",
            physics_time.as_secs_f64() * 1000.0,
            total_particle_count
        );

        let gpu_write_start = Instant::now();

        // Second pass: ZERO-COPY write directly to GPU staging memory
        // Use the SAME computed offsets from physics loop for consistency
        let particles_written = self.write_particles_zero_copy(
            queue,
            particle_renderer,
            &computed_offsets_map,
            total_particle_count,
        );
        let segments_written = self.write_segments_zero_copy(
            queue,
            segment_renderer,
            voices,
            &computed_offsets_map,
            total_segment_count,
        );

        let gpu_write_time = gpu_write_start.elapsed();
        let frame_time = frame_start.elapsed();

        println!(
            "[GPU Write] Total GPU write time: {:.3}ms",
            gpu_write_time.as_secs_f64() * 1000.0
        );
        println!(
            "[FRAME] Total update_zero_copy: {:.3}ms (Physics: {:.1}%, GPU: {:.1}%)\n",
            frame_time.as_secs_f64() * 1000.0,
            (physics_time.as_secs_f64() / frame_time.as_secs_f64()) * 100.0,
            (gpu_write_time.as_secs_f64() / frame_time.as_secs_f64()) * 100.0
        );

        // Record the last update time as the last item of business
        self.last_update = now;

        (particles_written, segments_written)
    }

    /// Write particles directly to GPU staging memory (zero-copy)
    fn write_particles_zero_copy(
        &self,
        queue: &nannou::wgpu::Queue,
        renderer: &ParticleRenderer,
        computed_offsets_map: &HashMap<VoiceId, Vec<Vec2>>,
        total_count: usize,
    ) -> usize {
        if total_count == 0 {
            return 0;
        }

        renderer.write_particles_direct(queue, total_count, |gpu_particles| {
            let mut write_idx = 0;

            for (voice_id, cores) in self.particle_cores.iter() {
                let computed_offsets = computed_offsets_map.get(voice_id);

                for (index, core) in cores.iter().enumerate() {
                    if core.is_alive && core.is_activated {
                        // Use pre-computed offset from physics loop (no recalculation!)
                        let offset = if let Some(computed_offsets) = computed_offsets {
                            computed_offsets[index]
                        } else {
                            vec2(0.0, 0.0)
                        };

                        gpu_particles[write_idx] = core.to_gpu(offset);
                        write_idx += 1;
                    }
                }
            }
        })
    }

    /// Write segments directly to GPU staging memory (zero-copy)
    fn write_segments_zero_copy(
        &self,
        queue: &nannou::wgpu::Queue,
        renderer: &SegmentRenderer,
        voices: &HashMap<VoiceId, Voice>,
        computed_offsets_map: &HashMap<VoiceId, Vec<Vec2>>,
        total_count: usize,
    ) -> usize {
        if total_count == 0 {
            return (0, 0).1;
        }

        let start_total = Instant::now();

        // Phase 1: Collect work items (sequential - just bookkeeping)
        let start_collect = Instant::now();
        let mut work_items = Vec::with_capacity(total_count);

        for (voice_id, cores) in self.particle_cores.iter() {
            let voice = voices.get(voice_id);
            let segment_length = voice.map(|v| v.params.segment_length).unwrap_or(0.0);
            let segment_line_width = voice.map(|v| v.params.segment_line_width).unwrap_or(1.0);

            for (index, core) in cores.iter().enumerate() {
                if core.is_alive && core.is_activated {
                    work_items.push((*voice_id, index, segment_length, segment_line_width));
                }
            }
        }
        let collect_time = start_collect.elapsed();

        let (_, segment_count) =
            renderer.write_segments_direct(queue, total_count, |gpu_segments| {
                // Phase 2: Parallel generation of SegmentGpu structs
                let start_assembly = Instant::now();

                let segments: Vec<_> = work_items
                    .par_iter()
                    .map(|(voice_id, particle_idx, segment_length, segment_line_width)| {
                        let computed_offsets = computed_offsets_map.get(voice_id).unwrap();
                        let feedback_array = self.particle_feedback.get(voice_id).unwrap();
                        let cores = self.particle_cores.get(voice_id).unwrap();

                        to_segment_gpu(
                            &cores[*particle_idx],
                            &feedback_array[*particle_idx],
                            computed_offsets[*particle_idx],
                            *segment_length,
                            *segment_line_width,
                        )
                    })
                    .collect();

                let assembly_time = start_assembly.elapsed();

                // Phase 3: Sequential copy into GPU buffer
                let start_copy = Instant::now();
                gpu_segments[..segments.len()].copy_from_slice(&segments);
                let copy_time = start_copy.elapsed();

                println!(
                    "  [Segments] Collect: {:.3}ms, Parallel assembly: {:.3}ms, Copy: {:.3}ms ({} segments)",
                    collect_time.as_secs_f64() * 1000.0,
                    assembly_time.as_secs_f64() * 1000.0,
                    copy_time.as_secs_f64() * 1000.0,
                    segments.len()
                );
            });

        let total_time = start_total.elapsed();
        println!(
            "[Segments] Total write_segments_zero_copy: {:.3}ms",
            total_time.as_secs_f64() * 1000.0
        );

        segment_count
    }

    pub fn handle_particle_emission(
        &mut self,
        voices: &HashMap<VoiceId, Voice>,
        rng: &mut ThreadRng,
    ) {
        for voice in voices.values() {
            let mut emitters: Vec<_> = voice.emitters.iter().collect();
            emitters.shuffle(rng);

            for emitter in emitters.iter() {
                if emitter.is_enabled() {
                    let parent_voice = emitter.parent_voice();
                    let core_vec = self.particle_cores.entry(parent_voice).or_default();
                    let feedback_vec = self.particle_feedback.entry(parent_voice).or_default();
                    let current_count = core_vec.len();

                    // Calculate emission scaling based on how close we are to the limit
                    let voice_limit = voices
                        .get(&parent_voice)
                        .map(|v| v.params.volume * v.params.particle_limit as f32)
                        .unwrap_or(self.params.overall_particle_limit as f32);
                    let emission_scaling =
                        Self::linear_emission_scaling(voice_limit, current_count);

                    // Skip emission entirely if scaling is near zero
                    if emission_scaling < 0.001 {
                        continue;
                    }

                    let color_limit = voices
                        .get(&parent_voice)
                        .map(|v| v.params.color_limit)
                        .unwrap_or(self.params.default_particle_color);

                    let mut new_particles = emitter.emit(
                        emission_scaling,
                        10.0,
                        self.params.default_particle_size,
                        rgba_from(color_limit, 0.0),
                        rng,
                    );
                    let new_particles_count = new_particles.len();
                    let mut new_feedback = vec![ParticleFeedback::new(); new_particles_count];

                    core_vec.append(&mut new_particles);
                    feedback_vec.append(&mut new_feedback);
                }
            }
        }
    }

    /********************* Particle methods ********************************** */

    /// Calculate emission rate scaling factor based on how close we are to the particle limit
    /// Returns a value from 0.0 to 1.0 where:
    /// - 1.0 when far from the limit (aggressive emission)
    /// - 0.0 when at or over the limit (no emission)
    /// - Smooth curve in between to avoid jerky transitions
    #[allow(dead_code)]
    fn calculate_emission_scaling(limit: f32, current_count: usize) -> f32 {
        if limit < 0.001 {
            return 0.0;
        }

        let ratio = current_count as f32 / limit;

        // If we're over the limit, stop emitting
        if ratio >= 1.0 {
            return 0.0;
        }

        // Use a smooth quadratic curve that starts aggressive (1.0) and
        // gradually reduces as we approach the limit
        // At 80% of limit, we're at 4% emission rate
        // At 90% of limit, we're at 1% emission rate
        let remaining_capacity = 1.0 - ratio;
        remaining_capacity.powf(1.2)
    }

    /// Linearly scale emission rate based on how close we are to the particle limit
    #[allow(dead_code)]
    fn linear_emission_scaling(limit: f32, current_count: usize) -> f32 {
        let ratio = current_count as f32 / limit;
        1.0 - ratio
    }

    /// Sets the n oldest particles to fade out, where n is the number of particles above the limit
    fn cull_excess_particles(&mut self, voices: &HashMap<VoiceId, Voice>) {
        for (voice_id, cores) in self.particle_cores.iter_mut() {
            let limit = voices
                .get(voice_id)
                .map(|v| v.params.volume * v.params.particle_limit as f32)
                .unwrap_or(self.params.overall_particle_limit as f32);

            let mut active_particles = 0;
            for core in cores.iter_mut().rev() {
                if core.remaining_life_span > core.fade_out_duration() {
                    active_particles += 1;
                    if active_particles > limit as usize {
                        // assumes that the oldest particles are at the beginning
                        core.set_to_fade_out();
                    }
                }
            }
        }
    }

    /********************* Mass Variation methods ********************************** */

    /// Enable or disable mass variation for all particles
    pub fn set_mass_variation_enabled(&mut self, enabled: bool) {
        self.mass_var_params.enabled = enabled;
    }

    /// Set the amount of mass variation (as percentage of base mass)
    /// e.g., 0.1 means particles can vary by ±10% of their base mass
    pub fn set_mass_variation_amount(&mut self, amount: f32) {
        self.mass_var_params.amount = amount.max(0.0); // Ensure non-negative
    }

    /// Get current mass variation settings
    pub fn get_mass_variation_enabled(&self) -> bool {
        self.mass_var_params.enabled
    }

    pub fn get_mass_variation_amount(&self) -> f32 {
        self.mass_var_params.amount
    }

    /********************* Accessor/Helper methods ********************************** */

    pub fn change_bounds_size_to(&mut self, width: f32, height: f32) {
        self.params.bounds_size = Vec2::new(width, height);
    }

    pub fn get_particle_count(&self) -> usize {
        self.particle_cores.values().map(|cores| cores.len()).sum()
    }

    fn bounds_rect(&self) -> Rect {
        Rect::from_x_y_w_h(
            self.params.origin.x,
            self.params.origin.y,
            self.params.bounds_size.x,
            self.params.bounds_size.y,
        )
    }

    /********************* Draw methods ********************************** */

    /// Draw the forces and emitters
    pub fn draw_forces(
        &self,
        voices: &HashMap<VoiceId, Voice>,
        draw: &Draw,
        scale_x: f32,
        scale_y: f32,
    ) {
        let circles_vec: Vec<&WindCircle> = voices
            .values()
            .flat_map(|v| v.wind_circles.values())
            .collect();

        self.draw_origin(draw, scale_x, scale_y);

        self.force_fields
            .wind_field
            .draw(&circles_vec, draw, scale_x, scale_y, self.perlin_gen);
        self.draw_emitters(voices, draw, scale_x, scale_y);

        for voice in voices.values() {
            for circle in voice.wind_circles.values() {
                circle.draw_center(draw, scale_x, scale_y);
                circle.draw(draw, scale_x, scale_y);
            }
        }
    }

    /// Draw the origin
    pub fn draw_origin(&self, draw: &Draw, scale_x: f32, scale_y: f32) {
        draw.ellipse()
            .xy(self.params.origin * vec2(scale_x, scale_y))
            .w_h(10.0 * scale_x, 10.0 * scale_y)
            .color(rgba(1.0, 0.0, 1.0, 0.2));
    }

    /// Draw the emitters
    pub fn draw_emitters(
        &self,
        voices: &HashMap<VoiceId, Voice>,
        draw: &Draw,
        scale_x: f32,
        scale_y: f32,
    ) {
        let mut emitters = Vec::new();
        for voice in voices.values() {
            emitters.extend(&voice.emitters);
        }
        for emitter in emitters.iter() {
            emitter.draw(draw, scale_x, scale_y);
        }
    }
}

/// Helper to make a Rgba from an Rgb and alpha value
fn rgba_from(rgb: Rgb, alpha: f32) -> Rgba {
    rgba(rgb.red, rgb.green, rgb.blue, alpha)
}
