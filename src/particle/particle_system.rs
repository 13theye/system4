//! src/particle/particle_system.rs
//!
//!
//! The Particle System of System 4
use std::collections::HashMap;

use nannou::noise::{NoiseFn, Perlin, Seedable};
use nannou::prelude::*;
use rand::{rngs::ThreadRng, seq::SliceRandom, Rng};
use rayon::prelude::*;
use std::time::{Duration, Instant};

use crate::{
    forces::{force_field::ForceFields, wind::CircleFormation},
    groups::{Voice, VoiceId},
    particle::{to_segment_gpu, ParticleCore, ParticleFeedback},
};

use super::constants::*;

/// Describes whether the Drones' forces are applied separately or combined
/// When `separate`, essentially works as two particle systems.
pub enum ParticleSystemMode {
    Separate,
    Combined,
}

impl ParticleSystemMode {
    /// Convenience function to check the state of the ParticleSystemMode
    pub fn is_combined(&self) -> bool {
        matches!(self, ParticleSystemMode::Combined)
    }

    /// Convenience function to toggle the ParticleSystemMode
    pub fn toggle(&mut self) {
        match self {
            ParticleSystemMode::Separate => *self = ParticleSystemMode::Combined,
            ParticleSystemMode::Combined => *self = ParticleSystemMode::Separate,
        }
    }
}

pub struct ParticleSystem {
    // Split particle storage for cache locality
    // Core: hot data for physics updates (~100 bytes per particle)
    pub particle_cores: HashMap<VoiceId, Vec<ParticleCore>>,
    // Feedback: cold data for trail rendering (~3.4KB per particle)
    // Boxed for performance improvement when culling particles
    pub particle_feedback: HashMap<VoiceId, Vec<Box<ParticleFeedback>>>,

    // Render State Helper
    pub frame_state: ParticleSystemFrameState,

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

    // Voice mode
    pub mode: ParticleSystemMode,
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

/// Update helper struct to track per-frame state
pub struct ParticleSystemFrameState {
    pub frame_start: Instant,
    pub physics_time: Duration,
    pub alive_particle_counts: HashMap<VoiceId, usize>,
}

impl ParticleSystemFrameState {
    pub fn init() -> Self {
        Self {
            frame_start: Instant::now(),
            physics_time: Duration::ZERO,
            alive_particle_counts: HashMap::new(),
        }
    }
}

impl ParticleSystem {
    pub fn init(
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
            frame_state: ParticleSystemFrameState::init(),

            force_fields: ForceFields::new(origin, bounds_size),
            params,
            mass_var_params,

            // Init Perlin noise generator
            perlin_gen: Perlin::new(),

            // Starting mode is separate
            mode: ParticleSystemMode::Separate,

            last_update: Instant::now(),
        }
    }

    pub fn size(&self) -> Vec2 {
        self.params.bounds_size
    }

    /********************* Update methods ********************************** */

    /// ZERO-COPY UPDATE: Updates particles only -- no longer writes
    /// Mutates particle_counts, segment_counts of particles written to GPU buffers
    pub fn update(
        &mut self,
        voices: &mut HashMap<VoiceId, Voice>,
        rng: &mut ThreadRng,
        now: Instant,
        perlin_seed: u32,
        engine_debug: bool,
    ) {
        self.frame_state.frame_start = Instant::now();

        // Calculate time delta for the physics simulation.
        // This is measured from the end of the previous update().
        let dt = (now - self.last_update).as_secs_f32();

        // Calculate the bounds rect of the sim space
        let bounds_rect = self.bounds_rect();

        // Update perlin noise with app time as seed
        self.perlin_gen = self.perlin_gen.set_seed(perlin_seed);

        // The percentage of framerate with 60fps target
        let framerate_factor = (dt / 0.0167).min(1.5);

        // Emit particles according to spawn rates
        self.handle_particle_emission(voices, rng);

        // Set the oldest particles to fade out, considering the particle limit
        let cull_start = Instant::now();
        self.cull_oldest_excess_particles(voices);
        let cull_time = cull_start.elapsed();

        // Init empty particle counters
        let mut total_alive_count = 0;

        // Determine ParticleSystem mode
        let should_combine = self.mode.is_combined();

        // Init physics timer
        let physics_start = Instant::now();

        // Update the particle cores by iterating over all Drone voices
        for (voice_id, cores) in self.particle_cores.iter_mut() {
            // Check that drone exists
            let Some(drone) = voices.get(voice_id).and_then(|v| v.as_drone()) else {
                // Ensure tht a drone voice that doesn't exist has no particles, then skip
                cores.clear();
                continue;
            };

            // Retrieve rgba parameter state for this Drone
            let color_limit = drone.params.color_limit;
            let alpha_limit = drone.params.alpha_limit;
            let color = color_limit;

            // Pre-compute mass variance using bulk RNG fill for better performance
            // Values are indexed by the same value as cores' position in its vector
            // Must be done before the parallelization step later
            let mut mass_variations = vec![0.0; cores.len()];
            if self.mass_var_params.enabled && self.mass_var_params.amount > 0.0 {
                rng.fill(&mut mass_variations[..]);
                // Scale from [0.0, 1.0) to [-mass_variation_amount, mass_variation_amount]
                for v in &mut mass_variations {
                    *v = (*v * 2.0 - 1.0) * self.mass_var_params.amount;
                }
            }

            // Pre-compute "vibration" position offset factors using bulk RNG fill.
            // Values are indexed by the same value as cores' position in its vector
            // Must be done before the parallelization step later.
            let vibration = drone.params.vibration;
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

            // Collect WindCircleFormations. If combined mode, collect all formations from all drones.
            // Otherwise, collect only the formations from this voice
            let formations: Vec<&dyn CircleFormation> = if should_combine {
                voices
                    .values()
                    .filter_map(|v| v.as_drone())
                    .flat_map(|d| d.wind_circle_formations.values())
                    .map(|f| f.as_ref())
                    .collect()
            } else {
                drone
                    .wind_circle_formations
                    .values()
                    .map(|f| f.as_ref())
                    .collect()
            };

            // Physics update for each particle
            // HOT LOOP in parallel execution
            cores
                .par_iter_mut()
                .zip(feedback_array.par_iter_mut())
                .enumerate()
                .for_each(|(index, (core, feedback))| {
                    // Get the mass variation from the pre-computed array
                    let mass_variation_factor = mass_variations[index];

                    // Look up noise factor based on pre-seeded Perlin noise at particle position
                    let noise_factor = self
                        .perlin_gen
                        .get([core.position.x as f64, core.position.y as f64]);

                    // Apply forces to the particle core.
                    // These changes are "staged" as velocity/acceleration parameter updates.
                    self.force_fields.apply_unified_forces_to_particle(
                        core,
                        &formations,
                        mass_variation_factor,
                        noise_factor,
                    );

                    // Apply forces to the particle position.
                    // Apply particle-level color change.
                    core.update(color, alpha_limit, framerate_factor);

                    // Calculate the "vibration" position offset
                    let position_offset = if vibration > 0.0 && core.velocity.length_squared() > 0.0
                    {
                        let normal = vec2(-core.velocity.y, core.velocity.x).normalize_or_zero();
                        normal * PARTICLE_MAX_POSITION_OFFSET * offset_factors[index]
                    } else {
                        vec2(0.0, 0.0)
                    };

                    // Apply the offset (adding zero is fast, no need to check)
                    // This is stored in a separate "offset_position" so that vibrations
                    // don't accumulate from frame to frame.
                    core.offset_position = core.position + position_offset;

                    // However, we do save the offset_position in the particle's feedback positions
                    // so that the trails make sense, because the offset_position is the position
                    // that is displayed.
                    feedback.record(core.offset_position, color);

                    // Mark a particle for immediate deletion if it's offscreen and past the
                    // buffer zone.
                    if core.is_out_of_bounds(bounds_rect) {
                        core.mark_for_deletion();
                    }
                });

            // Count particles that are both alive and activated after the update.
            // These are the ones that will be drawn.
            let count_start = Instant::now();
            let alive_count = cores
                .iter()
                .filter(|c| c.is_alive && c.is_activated)
                .count();

            // Total_alive_count is the running total number of alive particles for all voices.
            // Used mostly for benchmarking.
            total_alive_count += alive_count;

            // Store the value so that it can be read in the view function elsewhere.
            self.frame_state
                .alive_particle_counts
                .insert(*voice_id, alive_count);
            let count_time = count_start.elapsed();

            // Delete dead particles
            let dead_particle_delete_start = Instant::now();
            let mut write_index = 0;

            // Fast pointer swap to remove dead particles and consolidate the vector
            // This technique saves a lot of time especially for feedback.
            // Moves the pointer instead of the data.
            for read_index in 0..cores.len() {
                if cores[read_index].is_alive {
                    if write_index != read_index {
                        cores.swap(write_index, read_index);
                        feedback_array.swap(write_index, read_index);
                    }
                    write_index += 1;
                }
            }

            // Truncate the vectors since the particles beyond the alive_count have already
            // been shifted forward.
            cores.truncate(write_index);
            feedback_array.truncate(write_index);

            let dead_particle_delete_time = dead_particle_delete_start.elapsed();

            if engine_debug {
                println!(
                    "[Physics] Count time : {:.3}ms",
                    count_time.as_secs_f64() * 1000.0
                );
                println!(
                    "[Physics] Dead particle delete time : {:.3}ms",
                    dead_particle_delete_time.as_secs_f64() * 1000.0
                );
            }
        }

        self.frame_state.physics_time = physics_start.elapsed();
        if engine_debug {
            println!(
                "[Physics] Particle end of life marking time : {:.3}ms",
                cull_time.as_secs_f64() * 1000.0
            );

            println!(
                "[Physics] Parallel update : {:.3}ms ({} alive particles)",
                self.frame_state.physics_time.as_secs_f64() * 1000.0,
                total_alive_count
            );
        }

        // Cleanup pass: Remove voices in Clearing state that have no particles left
        // (culling has already removed dead particles, so we just check if the vec is empty)
        let voices_to_remove: Vec<VoiceId> = voices
            .iter()
            .filter_map(|(voice_id, voice)| {
                if let Some(drone) = voice.as_drone() {
                    if drone.state == crate::groups::DroneState::Clearing {
                        let is_empty = self
                            .particle_cores
                            .get(voice_id)
                            .map(|cores| cores.is_empty())
                            .unwrap_or(true);
                        if is_empty {
                            return Some(*voice_id);
                        }
                    }
                }
                None
            })
            .collect();

        for voice_id in voices_to_remove {
            println!("Removing voice {}", voice_id);
            voices.remove(&voice_id);
            self.particle_cores.remove(&voice_id);
            self.particle_feedback.remove(&voice_id);
        }

        // Record the last update time as the last item of business
        self.last_update = now;
    }

    /// Write particles for a specific voice directly to GPU staging memory (zero-copy)
    /// Assemble GPU particle data for a voice (CPU work only, no GPU write)
    /// This can be called in parallel for different voices using Rayon
    /// Internally sequential for speed
    pub fn assemble_particles_for_voice(
        &self,
        voice_id: VoiceId,
        engine_debug: bool,
    ) -> Vec<nnpipe::renderers::ParticleGpu> {
        let cores = match self.particle_cores.get(&voice_id) {
            Some(cores) => cores,
            None => return Vec::new(),
        };

        let start_assembly = Instant::now();

        // Assemble the ParticleGpu from ParticleCores.
        // This could have been parallelized but at particle counts of about 30,000
        // it's not worth the overhead.
        let gpu_particles: Vec<nnpipe::ParticleGpu> = cores
            .iter()
            .filter(|core| core.is_alive && core.is_activated)
            .map(|core| core.to_gpu())
            .collect();

        let assembly_time = start_assembly.elapsed();
        if engine_debug {
            println!(
                "  [PARALLEL Assembly Voice {}] CPU assembly: {:.3}ms ({} particles)",
                voice_id.to_i32(),
                assembly_time.as_secs_f64() * 1000.0,
                gpu_particles.len()
            );
        }

        gpu_particles
    }

    /// Write segments for a specific voice directly to GPU staging memory (zero-copy)
    /// Assemble GPU segment data for a voice (CPU work only, no GPU write)
    /// This can be called in parallel for different voices using Rayon
    /// Internally parallel for speed
    pub fn assemble_segments_for_voice(
        &self,
        voice_id: VoiceId,
        segment_length: f32,
        segment_line_width: f32,
        engine_debug: bool,
    ) -> Vec<nnpipe::renderers::SegmentGpu> {
        // Get the particle cores and feedback array
        let cores = match self.particle_cores.get(&voice_id) {
            Some(cores) => cores,
            None => return Vec::new(),
        };

        let feedback_array = match self.particle_feedback.get(&voice_id) {
            Some(fb) => fb,
            None => return Vec::new(),
        };

        let start_collect = Instant::now();

        // Pre-allocate Vec of work items. We know it won't be larger than the
        // total number of particles in the system.
        let mut work_items = Vec::with_capacity(
            *self
                .frame_state
                .alive_particle_counts
                .get(&voice_id)
                .unwrap_or(&cores.len()),
        );

        // Collect activated & alive items.
        for (index, core) in cores.iter().enumerate() {
            if core.is_alive && core.is_activated {
                work_items.push(index);
            }
        }

        if engine_debug {
            let collect_time = start_collect.elapsed();
            println!(
                "  [PARALLEL Segments Voice {}] Collect: {:.3}ms",
                voice_id.to_i32(),
                collect_time.as_secs_f64() * 1000.0
            );
        }

        // Assemble SegmentGpu instances. This is a parallel for performance as the
        // number of segments is much larger than the nubmer of particles.
        let start_assembly = Instant::now();
        let segments: Vec<nnpipe::renderers::SegmentGpu> = work_items
            .par_iter()
            .map(|&particle_idx| {
                to_segment_gpu(
                    &cores[particle_idx],
                    &feedback_array[particle_idx],
                    segment_length,
                    segment_line_width,
                )
            })
            .collect();

        let assembly_time = start_assembly.elapsed();
        if engine_debug {
            println!(
                "  [PARALLEL Segments Voice {}] CPU assembly: {:.3}ms ({} segments)",
                voice_id.to_i32(),
                assembly_time.as_secs_f64() * 1000.0,
                segments.len()
            );
        }

        segments
    }

    /// For all active Drones, emit particles from active emitters.
    /// The emission is scaled based on how close we are to the particle limit.
    /// The particle limit is the max particles allowed for a voice times the
    /// "Volume" parameter of the voice.
    pub fn handle_particle_emission(
        &mut self,
        voices: &HashMap<VoiceId, Voice>,
        rng: &mut ThreadRng,
    ) {
        for (voice_id, voice) in voices {
            // Only handle Drone voices
            let Some(drone) = voice.as_drone() else {
                continue;
            };

            // Only emit if drone is DroneState::Active
            if !drone.is_active() {
                continue;
            }

            // Collect the emitters of this Drone and shuffle the order.
            let mut emitters: Vec<_> = drone.emitters.iter().collect();
            emitters.shuffle(rng);

            for emitter in emitters.iter() {
                // Only emit if emitter is enabled
                if !emitter.is_enabled() {
                    continue;
                }

                let core_vec = self.particle_cores.entry(*voice_id).or_default();
                let feedback_vec = self.particle_feedback.entry(*voice_id).or_default();
                let current_count = core_vec.len();

                // Calculate emission scaling based on how close we are to the particle limit
                let emission_scaling = Self::linear_emission_scaling(
                    drone.params.particle_limit as f32 * drone.params.volume,
                    current_count,
                );

                // Skip emission entirely if scaling is near zero (account for float error)
                if emission_scaling < 0.001 {
                    continue;
                }

                // Emit new particles
                let mut new_particles = emitter.emit(
                    emission_scaling,
                    10.0,
                    self.params.default_particle_size,
                    rgba_from(drone.params.color_limit, 0.0),
                    rng,
                );

                // Extend the feedback vector by the number of new particles
                let new_particles_count = new_particles.len();
                let mut new_feedback = vec![Box::new(ParticleFeedback::new()); new_particles_count];

                // Append to the particle core and feedback vectors.
                core_vec.append(&mut new_particles);
                feedback_vec.append(&mut new_feedback);
            }
        }
    }

    /********************* Particle methods ********************************** */

    /// Calculate emission rate scaling factor based on how close we are to the particle limit
    /// Returns a value from 0.0 to 1.0 where:
    /// - 1.0 when far from the limit (aggressive emission)
    /// - 0.0 when at or over the limit (no emission)
    /// - Smooth curve in between to avoid jerky transitions
    /// - Not currently used (linear scaling used instead)
    #[allow(dead_code)]
    fn curved_emission_scaling(limit: f32, current_count: usize) -> f32 {
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
    fn linear_emission_scaling(limit: f32, current_count: usize) -> f32 {
        let ratio = current_count as f32 / limit;
        1.0 - ratio
    }

    /// Sets the n oldest particles to fade out, where n is the number of particles above the limit.
    /// This should happen before calculating physics.
    fn cull_oldest_excess_particles(&mut self, voices: &HashMap<VoiceId, Voice>) {
        for (voice_id, cores) in self.particle_cores.iter_mut() {
            // Calculate particle limit
            let limit = voices
                .get(voice_id)
                .and_then(|v| v.as_drone())
                .map(|d| d.params.volume * d.params.particle_limit as f32)
                .unwrap_or(0.0);

            let mut active_particles = 0;

            // iterate over all cores
            for core in cores.iter_mut().rev() {
                if core.remaining_life_span > core.fade_out_duration() {
                    active_particles += 1;
                    if active_particles > limit as usize {
                        // assumes that the oldest particles are at the beginning of the vec,
                        // which is why we iterate in reverse so that oldest particles are last.
                        // We fade out the particles instead of immediately deleting them.
                        core.set_to_fade_out();
                    }
                }
            }
        }
    }

    /// Mark all particles of a voice to fade out (used when clearing a voice)
    pub fn fade_out_all_particles(&mut self, voice_id: VoiceId) {
        if let Some(cores) = self.particle_cores.get_mut(&voice_id) {
            for core in cores.iter_mut() {
                // Immediately kill non-activated particles
                if !core.is_activated {
                    core.mark_for_deletion();
                }

                // Set all other particles to fade out
                if core.remaining_life_span >= core.fade_out_duration() {
                    core.set_to_fade_out();
                }
            }
        }
    }

    /********************* Mass Variation methods ********************************** */

    /// Enable or disable mass variation for all particles
    pub fn set_mass_variation(&mut self, enabled: bool) {
        self.mass_var_params.enabled = enabled;
    }

    /// Set the amount of mass variation (as percentage of base mass)
    /// e.g., 0.1 means particles can vary by ±10% of their base mass
    pub fn set_mass_variation_amount(&mut self, amount: f32) {
        self.mass_var_params.amount = amount.max(0.0); // Ensure non-negative
    }

    /// Get current mass variation settings
    pub fn is_mass_variation_enabled(&self) -> bool {
        self.mass_var_params.enabled
    }

    pub fn get_mass_variation_amount(&self) -> f32 {
        self.mass_var_params.amount
    }

    /********************* Accessor/Helper methods ********************************** */

    pub fn change_bounds_size_to(&mut self, width: f32, height: f32) {
        self.params.bounds_size = Vec2::new(width, height);
    }

    pub fn get_total_particle_count(&self) -> usize {
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
    /// - Requires scale_x and scale_y because this is meant for the performer window,
    ///   which is a scaled version of the audience window.
    pub fn draw_forces(
        &self,
        voices: &HashMap<VoiceId, Voice>,
        draw: &Draw,
        scale_x: f32,
        scale_y: f32,
    ) {
        // Collect all WindCircles across all voices
        let formations: Vec<&dyn CircleFormation> = voices
            .values()
            .filter_map(|v| v.as_drone())
            .flat_map(|d| d.wind_circle_formations.values())
            .map(|f| f.as_ref())
            .collect();

        // Draw the origin of the ParticleSystem
        self.draw_origin(draw, scale_x, scale_y);

        // Draw representative vectors of the WindField
        self.force_fields.wind_field.draw(
            &formations,
            draw,
            scale_x,
            scale_y,
            self.perlin_gen,
            self.mode.is_combined(),
        );

        // Draw all voices' emitters
        self.draw_emitters(voices, draw, scale_x, scale_y);

        // Draw the circles themselves
        formations.iter().for_each(|circle| {
            circle.draw_center(draw, scale_x, scale_y);
            circle.draw(draw, scale_x, scale_y);
        });
    }

    /// Draw the origin
    pub fn draw_origin(&self, draw: &Draw, scale_x: f32, scale_y: f32) {
        draw.ellipse()
            .xy(self.params.origin * vec2(scale_x, scale_y))
            .w_h(10.0 * scale_x, 10.0 * scale_y)
            .color(rgba(1.0, 0.0, 1.0, 0.2));
    }

    /// Draw all voices' emitters
    pub fn draw_emitters(
        &self,
        voices: &HashMap<VoiceId, Voice>,
        draw: &Draw,
        scale_x: f32,
        scale_y: f32,
    ) {
        let mut emitters = Vec::new();
        for drone in voices.values().filter_map(|v| v.as_drone()) {
            emitters.extend(&drone.emitters);
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
