/// src/particle/particle_system.rs
///
///
/// The Particle System of System 4
use std::collections::HashMap;

use nannou::prelude::*;
use nnpipe::renderers::{ParticleGpu, SegmentGpu};
use rand::{rngs::ThreadRng, seq::SliceRandom, Rng};
use rayon::prelude::*;

use crate::{
    forces::ForceFields,
    groups::{Voice, VoiceId},
    model::{GpuParticleBuffer, GpuSegmentBuffer},
    particle::{Particle, ParticleCore, ParticleFeedback, to_segment_gpu},
    utils::tween,
};

pub const EMPTY_GPU_PARTICLE_BUFFER: GpuParticleBuffer = Vec::new();
pub const EMPTY_GPU_SEGMENT_BUFFER: GpuSegmentBuffer = Vec::new();
const MAX_POSITION_OFFSET: f32 = 10.0; // Maximum screen distance for position offset in pixels
const MAX_SPAWN_RATE: f32 = 80.0;

const HIGH_R: f32 = 1.0;
const HIGH_G: f32 = 1.0;
const HIGH_B: f32 = 0.0;

// Animation timing constants
const FADE_DURATION: f32 = 1.0;
const RAMP_UP_PERCENT: f32 = 0.1;
const DWELL_PERCENT: f32 = 0.3;
const RAMP_CURVE_EXPONENT: f32 = 3.0;
const FADE_CURVE_EXPONENT: f32 = 1.5;

pub struct ParticleSystem {
    // Split particle storage for cache locality
    // Core: hot data for physics updates (~100 bytes per particle)
    pub particle_cores: HashMap<VoiceId, Vec<ParticleCore>>,
    // Feedback: cold data for trail rendering (~3.4KB per particle)
    pub particle_feedback: HashMap<VoiceId, Vec<ParticleFeedback>>,

    // Legacy monolithic storage (will be removed)
    pub particles: HashMap<VoiceId, Vec<Particle>>,

    // experimental
    pub last_event_time: f32,

    // forces
    pub forces: ForceFields,

    // Global params
    pub default_particle_limit: usize,
    pub global_max_spawn_rate: f32,

    // Origin and bounds
    origin: Point2,
    bounds_size: Vec2,
    pub bounds_rect: Rect,
    pub default_particle_color: Rgb,
    default_particle_size: f32,

    // DPI scale
    dpi_scale: f32,

    // Mass variation parameters
    pub mass_variation_enabled: bool,
    pub mass_variation_amount: f32, // percentage of base mass to vary (e.g., 0.1 = 10%)
}

impl ParticleSystem {
    pub fn new(
        origin: Point2,
        width: f32,
        height: f32,
        default_particle_size: f32,
        default_particle_color: Rgb,
        default_particle_limit: u32,
        dpi_scale: f32,
    ) -> Self {
        let bounds_size = Vec2::new(width, height);
        let bounds_rect = Rect::from_x_y_w_h(origin.x, origin.y, width, height);
        let grid_cols = (width / 4.0) as usize;
        let grid_rows = (height / 4.0) as usize;

        Self {
            origin,
            particle_cores: HashMap::new(),
            particle_feedback: HashMap::new(),
            particles: HashMap::new(), // Legacy, will be removed
            forces: ForceFields::new(origin, bounds_size, grid_cols, grid_rows),
            global_max_spawn_rate: MAX_SPAWN_RATE,
            bounds_size,
            bounds_rect,
            default_particle_size,
            default_particle_color,
            default_particle_limit: default_particle_limit as usize,

            dpi_scale,

            // Initialize mass variation parameters
            mass_variation_enabled: true,
            mass_variation_amount: 0.05, // 5% variation by default

            last_event_time: 0.0,
        }
    }

    /********************* Update methods ********************************** */

    /// Emit particles, update forces, update particles, and cull particles - returns simple particles and accompanying trails
    /// OPTIMIZED: Split storage for cache locality - physics updates only touch ~100B per particle
    pub fn update(
        &mut self,
        voices: &mut HashMap<VoiceId, Voice>,
        rng: &mut ThreadRng,
        gpu_particle_buffer: &mut GpuParticleBuffer,
        gpu_segment_buffers: &mut HashMap<VoiceId, GpuSegmentBuffer>,
        event: bool,
        time: f32,
    ) {
        let start_total = std::time::Instant::now();

        let start = std::time::Instant::now();
        self.handle_particle_emission(voices, rng);
        self.cull_excess_particles(voices);
        let emit_time = start.elapsed();

        let start = std::time::Instant::now();
        self.forces.update(voices, rng);
        let forces_time = start.elapsed();

        // Reuse existing buffer to avoid allocations
        for (_, s_gpu) in gpu_segment_buffers.iter_mut() {
            s_gpu.clear();
        }
        gpu_particle_buffer.clear();

        // Pre-compute position offset factors for all voices to avoid borrow conflicts
        let vibration_values: HashMap<VoiceId, f32> = voices
            .values()
            .map(|voice| (voice.id, voice.params.vibration))
            .collect();

        // NEW: Iterate over split storage for optimal cache locality
        for (voice_id, cores) in self.particle_cores.iter_mut() {
            let voice = voices.get(voice_id);
            let color_limit = voice.map(|v| v.params.color_limit);
            let alpha_limit = voice.map(|v| v.params.alpha_limit);
            let segment_length = voice.map(|v| v.params.segment_length);
            let segment_line_width = voice.map(|v| v.params.segment_line_width);

            // Skip if no limits for this voice
            if color_limit.is_none()
                || alpha_limit.is_none()
                || segment_length.is_none()
                || segment_line_width.is_none()
            {
                continue;
            }

            let (color_limit, alpha_limit, segment_length, segment_line_width) = (
                color_limit.unwrap(),
                alpha_limit.unwrap(),
                segment_length.unwrap(),
                segment_line_width.unwrap(),
            );

            if event {
                self.last_event_time = time;
            }

            let color = tween::interpolate_color(
                color_limit,
                rgb(HIGH_R, HIGH_G, HIGH_B),
                FADE_DURATION,
                RAMP_UP_PERCENT,
                DWELL_PERCENT,
                RAMP_CURVE_EXPONENT,
                FADE_CURVE_EXPONENT,
                time,
                self.last_event_time,
            );

            // Ensure buffer exists for this voice
            let sgpu_buf = gpu_segment_buffers
                .entry(*voice_id)
                .or_insert_with(|| EMPTY_GPU_SEGMENT_BUFFER);

            // Pre-compute mass variation factors for all particles in this voice
            let mass_variations: Vec<f32> = if self.mass_variation_enabled
                && self.mass_variation_amount > 0.0
            {
                cores
                    .iter()
                    .map(|_| {
                        rng.random_range(-self.mass_variation_amount..=self.mass_variation_amount)
                    })
                    .collect()
            } else {
                vec![0.0; cores.len()]
            };

            // Pre-compute position offset random signs for all particles in this voice
            let vibration = vibration_values.get(voice_id).copied().unwrap_or(0.0);
            let position_offsets: Vec<f32> = if vibration > 0.0 {
                cores
                    .iter()
                    .map(|_| rng.random_range(-vibration..vibration))
                    .collect()
            } else {
                vec![0.0; cores.len()]
            };

            // Get mutable reference to feedback array for this voice
            let feedback_array = self.particle_feedback.get_mut(voice_id).unwrap();

            let start = std::time::Instant::now();
            // OPTIMIZATION: Physics update only touches ParticleCore (~100B per particle)
            // This is the hot path - Rayon threads have excellent cache locality
            // We zip cores and feedback together to allow parallel mutation of both
            cores.par_iter_mut()
                .zip(feedback_array.par_iter_mut())
                .enumerate()
                .for_each(|(index, (core, feedback))| {
                    let mass_variation_factor = mass_variations[index];
                    self.forces.apply_forces_to_particle(core, mass_variation_factor);

                    // Calculate position offset perpendicular to velocity BEFORE updating particle
                    let offset = if vibration > 0.0 && core.velocity.length_squared() > 0.0 {
                        let normal = vec2(-core.velocity.y, core.velocity.x).normalize_or_zero();
                        normal * MAX_POSITION_OFFSET * position_offsets[index]
                    } else {
                        vec2(0.0, 0.0)
                    };

                    // Record feedback position (this writes to separate cold data structure)
                    let offset_position = if offset.length_squared() > 0.0 {
                        core.position + offset
                    } else {
                        core.position
                    };
                    feedback.record(offset_position, color);

                    // Update core particle (hot data only)
                    core.update(color, alpha_limit);

                    if core.is_out_of_bounds(self.bounds_rect) {
                        core.kill();
                    }
                });
            let physics_time = start.elapsed();

            let start = std::time::Instant::now();
            // Generate GPU data: Now we read both core (hot) and feedback (cold)
            // This happens sequentially after physics update, so cache impact is minimal
            let (gpu_particle_group, gpu_segment_group): (Vec<ParticleGpu>, Vec<SegmentGpu>) =
                cores
                    .par_iter()
                    .enumerate()
                    .filter_map(|(index, core)| {
                        if core.is_alive && core.is_activated {
                            // Calculate offset again for GPU conversion
                            let offset = if vibration > 0.0 && core.velocity.length_squared() > 0.0 {
                                let normal = vec2(-core.velocity.y, core.velocity.x).normalize_or_zero();
                                normal * MAX_POSITION_OFFSET * position_offsets[index]
                            } else {
                                vec2(0.0, 0.0)
                            };

                            Some((
                                core.to_gpu(offset),
                                to_segment_gpu(core, &feedback_array[index], offset, segment_length, segment_line_width),
                            ))
                        } else {
                            None
                        }
                    })
                    .unzip();
            let gpu_gen_time = start.elapsed();

            // Append to buffers
            gpu_particle_buffer.extend(gpu_particle_group);
            sgpu_buf.extend(gpu_segment_group);

            // Cull dead particles from both arrays (must maintain index sync!)
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

            if cores.len() > 100 {
                println!("Voice {:?}: {} particles | Physics: {:.2}ms | GPU gen: {:.2}ms",
                    voice_id, cores.len(), physics_time.as_secs_f32() * 1000.0, gpu_gen_time.as_secs_f32() * 1000.0);
            }
        }

        let total_time = start_total.elapsed();
        if self.get_particle_count() > 100 {
            println!("TOTAL UPDATE: {:.2}ms | Emit: {:.2}ms | Forces: {:.2}ms",
                total_time.as_secs_f32() * 1000.0, emit_time.as_secs_f32() * 1000.0, forces_time.as_secs_f32() * 1000.0);
        }
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
                        .unwrap_or(self.default_particle_limit as f32);
                    let emission_scaling =
                        Self::linear_emission_scaling(voice_limit, current_count);

                    // Skip emission entirely if scaling is near zero
                    if emission_scaling < 0.001 {
                        continue;
                    }

                    let color_limit = voices
                        .get(&parent_voice)
                        .map(|v| v.params.color_limit)
                        .unwrap_or(self.default_particle_color);

                    let new_particles = emitter.emit(
                        emission_scaling,
                        10.0,
                        self.default_particle_size,
                        rgba_from(color_limit, 0.0),
                        rng,
                    );

                    // Convert from legacy Particle to split storage
                    for particle in new_particles {
                        core_vec.push(ParticleCore::new(
                            particle.position(),
                            self.default_particle_size,
                            rgba_from(color_limit, 0.0),
                        ).with_velocity(particle.velocity));
                        feedback_vec.push(ParticleFeedback::new());
                    }
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
                .unwrap_or(self.default_particle_limit as f32);

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

    /// Older version of cull_excess_particles
    fn _cull_excess_particles(&mut self, voices: &HashMap<VoiceId, Voice>) {
        for (voice_id, particles) in self.particles.iter_mut() {
            let limit = voices
                .get(voice_id)
                .map(|v| v.params.volume * v.params.particle_limit as f32)
                .unwrap_or(self.default_particle_limit as f32);

            // Partition particles into active and fading groups
            let active_particles: Vec<_> = particles
                .iter_mut()
                .filter(|p| p.remaining_life_span > p.fade_out_duration())
                .collect();

            let num_active_particles = active_particles.len() as f32;

            if num_active_particles > limit {
                let excess_active = num_active_particles - limit;

                // kill oldest particles first - take() takes the first n elements
                for particle in active_particles.into_iter().take(excess_active as usize) {
                    particle.set_to_fade_out();
                }
            }
        }
    }

    /********************* Mass Variation methods ********************************** */

    /// Enable or disable mass variation for all particles
    pub fn set_mass_variation_enabled(&mut self, enabled: bool) {
        self.mass_variation_enabled = enabled;
    }

    /// Set the amount of mass variation (as percentage of base mass)
    /// e.g., 0.1 means particles can vary by ±10% of their base mass
    pub fn set_mass_variation_amount(&mut self, amount: f32) {
        self.mass_variation_amount = amount.max(0.0); // Ensure non-negative
    }

    /// Get current mass variation settings
    pub fn get_mass_variation_enabled(&self) -> bool {
        self.mass_variation_enabled
    }

    pub fn get_mass_variation_amount(&self) -> f32 {
        self.mass_variation_amount
    }

    /********************* Accessor/Helper methods ********************************** */

    pub fn change_bounds_size_to(&mut self, width: f32, height: f32) {
        self.bounds_size = Vec2::new(width, height);
        self.bounds_rect = self.make_bounds_rect();
    }

    pub fn get_particle_count(&self) -> usize {
        self.particle_cores
            .values()
            .map(|cores| cores.len())
            .sum()
    }

    fn make_bounds_rect(&self) -> Rect {
        Rect::from_x_y_w_h(
            self.origin.x,
            self.origin.y,
            self.bounds_size.x,
            self.bounds_size.y,
        )
    }

    /********************* Draw methods ********************************** */

    /// In this draw mode, particles are only drawn if they are within the bounds of the mask
    /// associated with the emitter that spawned them.
    pub fn draw(&self, voices: &HashMap<VoiceId, Voice>, mask: &Rect, draw: &Draw) {
        for (voice_id, particles) in self.particles.iter() {
            for particle in particles.iter() {
                let feedback = voices
                    .get(voice_id)
                    .map(|v| &v.params.feedback)
                    .unwrap_or(&0.0);

                if mask.contains(particle.position()) {
                    particle.draw(draw, *feedback, self.dpi_scale);
                }
            }
        }
    }

    /// Draw the forces and emitters
    pub fn draw_forces(
        &self,
        voices: &HashMap<VoiceId, Voice>,
        draw: &Draw,
        scale_x: f32,
        scale_y: f32,
    ) {
        self.draw_origin(draw, scale_x, scale_y);
        self.forces.wind_field.draw(draw, scale_x, scale_y, 27);
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
            .xy(self.origin * vec2(scale_x, scale_y))
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
