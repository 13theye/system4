/// src/particle/particle_system.rs
///
///
/// The Particle System of System 4
use std::collections::HashMap;
use std::time::Instant;

use nannou::prelude::*;
use nnpipe::renderers::{ParticleRenderer, SegmentRenderer};
use rand::{rngs::ThreadRng, seq::SliceRandom, Rng};
use rayon::prelude::*;

use crate::{
    forces::ForceFields,
    groups::{Voice, VoiceId},
    particle::{to_segment_gpu, GpuParticleBridge, ParticleCore, ParticleFeedback},
    utils::tween,
};

use super::constants::*;

pub struct ParticleSystem {
    // Split particle storage for cache locality
    // Core: hot data for physics updates (~100 bytes per particle)
    pub particle_cores: HashMap<VoiceId, Vec<ParticleCore>>,
    // Feedback: cold data for trail rendering (~3.4KB per particle)
    pub particle_feedback: HashMap<VoiceId, Vec<ParticleFeedback>>,

    // experimental
    pub last_update: Instant,

    // forces
    pub forces: ForceFields,

    // GPU particle bridge (Phase 3)
    pub gpu_particle_bridge: Option<GpuParticleBridge>,

    // Flag to use GPU physics (Phase 3)
    pub use_gpu_physics: bool,

    // Global params
    pub default_particle_limit: usize,
    pub global_max_spawn_rate: f32,

    // Origin and bounds
    origin: Point2,
    bounds_size: Vec2,
    pub bounds_rect: Rect,
    pub default_particle_color: Rgb,
    default_particle_size: f32,

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
    ) -> Self {
        let bounds_size = Vec2::new(width, height);
        let bounds_rect = Rect::from_x_y_w_h(origin.x, origin.y, width, height);
        let grid_cols = (width / 3.0) as usize;
        let grid_rows = (height / 3.0) as usize;

        Self {
            origin,
            particle_cores: HashMap::new(),
            particle_feedback: HashMap::new(),
            forces: ForceFields::new(origin, bounds_size, grid_cols, grid_rows),
            gpu_particle_bridge: None,
            use_gpu_physics: false,
            global_max_spawn_rate: PARTICLE_MAX_SPAWN_RATE,
            bounds_size,
            bounds_rect,
            default_particle_size,
            default_particle_color,
            default_particle_limit: default_particle_limit as usize,

            // Initialize mass variation parameters
            mass_variation_enabled: true,
            mass_variation_amount: 0.05, // 5% variation by default

            last_update: Instant::now(),
        }
    }

    /// Enable or disable GPU force field computation
    pub fn set_use_gpu_forces(&mut self, use_gpu: bool) {
        self.forces.set_use_gpu(use_gpu);
    }

    /// Check if GPU force field is enabled
    pub fn is_using_gpu_forces(&self) -> bool {
        self.forces.use_gpu
    }

    /// Initialize GPU physics (Phase 3)
    ///
    /// Creates a GPU particle bridge for full GPU physics simulation.
    /// This eliminates CPU physics and the blocking read_back operation.
    ///
    /// # Arguments
    ///
    /// * `device` - WebGPU device
    /// * `max_particles` - Maximum number of particles
    /// * `enable` - Whether to enable GPU physics immediately
    ///
    /// # Returns
    ///
    /// Ok if successful, Err with error message otherwise
    pub fn init_gpu_physics(
        &mut self,
        device: &wgpu::Device,
        max_particles: usize,
        enable: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use nnpipe::compute::GpuParticleConfig;

        let grid_cols = (self.bounds_size.x / 3.0) as u32;
        let grid_rows = (self.bounds_size.y / 3.0) as u32;

        let config = GpuParticleConfig {
            max_particles,
            grid_width: grid_cols,
            grid_height: grid_rows,
            bounds: [
                -self.bounds_size.x / 2.0,
                -self.bounds_size.y / 2.0,
                self.bounds_size.x / 2.0,
                self.bounds_size.y / 2.0,
            ],
            damping: 0.99,
            max_force: 100.0,
            noise_scale: 0.01,
            trail_capacity: 0,        // Not used in Phase 3
            inertia_coefficient: 0.1, // Match CPU momentum-based inertial resistance
        };

        self.gpu_particle_bridge = Some(GpuParticleBridge::new(device, config)?);
        self.use_gpu_physics = enable;

        Ok(())
    }

    /// Enable or disable GPU physics
    pub fn set_use_gpu_physics(&mut self, use_gpu: bool) {
        if self.gpu_particle_bridge.is_some() {
            self.use_gpu_physics = use_gpu;
        }
    }

    /// Check if GPU physics is enabled
    pub fn is_using_gpu_physics(&self) -> bool {
        self.use_gpu_physics && self.gpu_particle_bridge.is_some()
    }

    /********************* Update methods ********************************** */

    /// GPU RENDER POPULATION UPDATE: Full GPU physics and rendering (Phase 5 - Clean GPU Spawning)
    ///
    /// This is the complete GPU path with:
    /// - GPU-side particle spawning (no CPU tracking)
    /// - GPU physics simulation
    /// - GPU render buffer population
    /// - Only emission requests run on CPU
    ///
    /// NO CPU TRACKING, NO SYNC, NO READBACK!
    ///
    /// Returns encoder with encoded GPU commands (caller must submit).
    /// The alive count can be read from the GPU bridge's alive_count_buffer if needed.
    #[allow(clippy::too_many_arguments)]
    pub fn update_gpu_render_populate(
        &mut self,
        voices: &mut HashMap<VoiceId, Voice>,
        rng: &mut ThreadRng,
        _device: &wgpu::Device, // No longer needed - no readback!
        queue: &nannou::wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        now: Instant,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dt = (now - self.last_update).as_secs_f32();

        // Calculate framerate_factor for frame-rate independent physics
        // This converts dt (in seconds) to "frame units" where 1.0 = one 60fps frame
        let framerate_factor = (dt / 0.0167).min(1.5);

        // 1. Emitters produce spawn requests (constant rate, no capacity checks)
        let spawn_requests = self.handle_particle_emission_gpu(voices, rng);

        // Get GPU particle bridge
        if let Some(ref mut gpu_bridge) = self.gpu_particle_bridge {
            // 2. Upload spawn requests to GPU
            if !spawn_requests.is_empty() {
                let upload_start = Instant::now();
                let uploaded = gpu_bridge.upload_spawn_requests(queue, &spawn_requests);
                println!(
                    "  Upload spawn requests: {} requests ({:?})",
                    uploaded,
                    upload_start.elapsed()
                );
            }

            // 3. Update voice limits for culling
            let limits_start = Instant::now();
            gpu_bridge.update_voice_limits(queue, voices);
            println!("  Update voice limits: {:?}", limits_start.elapsed());

            // 4. Clear culling counters
            gpu_bridge.clear_cull_counts(queue);

            // 5. Update GPU force field
            let forces_start = Instant::now();
            gpu_bridge.update_forces(queue, encoder, voices)?;
            println!("  Force update: {:?}", forces_start.elapsed());

            // 6. GPU spawns particles (finds slots, drops overflow)
            if !spawn_requests.is_empty() {
                let spawn_start = Instant::now();
                gpu_bridge.encode_spawn(encoder);
                println!("  Spawn encode: {:?}", spawn_start.elapsed());
            }

            // 7. GPU culls excess particles per voice
            let cull_start = Instant::now();
            gpu_bridge.encode_cull(encoder);
            println!("  Cull encode: {:?}", cull_start.elapsed());

            // 8. GPU physics
            let physics_start = Instant::now();
            gpu_bridge.encode_physics_update(queue, encoder, framerate_factor);
            println!(
                "  Physics encode: {:?} (dt={:.4}s, framerate_factor={:.2})",
                physics_start.elapsed(),
                dt,
                framerate_factor
            );

            // 9. GPU render populate
            let render_start = Instant::now();
            gpu_bridge.encode_render_populate(encoder);
            println!("  Render populate encode: {:?}", render_start.elapsed());
        } else {
            return Err("GPU particle bridge not initialized".into());
        }

        // NO CPU TRACKING, NO SYNC, NO READBACK!
        // The GPU manages all particle lifecycle.

        self.last_update = now;
        Ok(())
    }

    /// ZERO-COPY UPDATE: Updates particles and writes directly to GPU staging memory
    /// Returns (particle_count, segment_count) written to GPU buffers
    #[allow(clippy::too_many_arguments)]
    pub fn update_zero_copy(
        &mut self,
        voices: &mut HashMap<VoiceId, Voice>,
        rng: &mut ThreadRng,
        queue: &nannou::wgpu::Queue,
        particle_renderer: &ParticleRenderer,
        segment_renderer: &SegmentRenderer,
        now: Instant,
    ) -> (usize, usize) {
        let dt = (now - self.last_update).as_secs_f32();

        // how many frames have passed with 60fps target
        let framerate_factor = (dt / 0.0167).min(3.0);

        self.handle_particle_emission(voices, rng);
        self.cull_excess_particles(voices);

        // Update forces (always update CPU for debugging/comparison)
        self.forces.update(voices, rng);

        // Pre-compute position offset factors for all voices
        let vibration_values: HashMap<VoiceId, f32> = voices
            .values()
            .map(|voice| (voice.id, voice.params.vibration))
            .collect();

        // First pass: Physics updates and count alive particles
        let mut total_particle_count = 0;
        let mut total_segment_count = 0;
        let mut voice_particle_counts: HashMap<VoiceId, usize> = HashMap::new();

        // Store computed offsets from physics loop to reuse in GPU write
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

            // Pre-compute variations using bulk RNG fill for better performance
            let mut mass_variations = vec![0.0; cores.len()];
            if self.mass_variation_enabled && self.mass_variation_amount > 0.0 {
                rng.fill(&mut mass_variations[..]);
                // Scale from [0.0, 1.0) to [-mass_variation_amount, mass_variation_amount]
                for v in &mut mass_variations {
                    *v = (*v * 2.0 - 1.0) * self.mass_variation_amount;
                }
            }

            let vibration = vibration_values.get(voice_id).copied().unwrap_or(0.0);
            let mut offset_factors = vec![0.0; cores.len()];
            if vibration > 0.0 {
                rng.fill(&mut offset_factors[..]);
                // Scale from [0.0, 1.0) to [-vibration, vibration]
                for v in &mut offset_factors {
                    *v = (*v * 2.0 - 1.0) * vibration;
                }
            }

            let feedback_array = self.particle_feedback.get_mut(voice_id).unwrap();

            // Pre-allocate storage for computed offsets (to save for GPU write)
            let mut computed_offsets = vec![vec2(0.0, 0.0); cores.len()];

            // Physics update for each particle
            cores
                .par_iter_mut()
                .zip(feedback_array.par_iter_mut())
                .zip(computed_offsets.par_iter_mut())
                .enumerate()
                .for_each(|(index, ((core, feedback), computed_offset))| {
                    let mass_variation_factor = mass_variations[index];

                    // Stage force applications
                    self.forces
                        .apply_forces_to_particle(core, mass_variation_factor);

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

                    if core.is_out_of_bounds(self.bounds_rect) {
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

        let (_, segment_count) =
            renderer.write_segments_direct(queue, total_count, |gpu_segments| {
                let mut write_idx = 0;

                for (voice_id, cores) in self.particle_cores.iter() {
                    let voice = voices.get(voice_id);
                    let segment_length = voice.map(|v| v.params.segment_length).unwrap_or(0.0);
                    let segment_line_width =
                        voice.map(|v| v.params.segment_line_width).unwrap_or(1.0);
                    let computed_offsets = computed_offsets_map.get(voice_id).unwrap();
                    let feedback_array = self.particle_feedback.get(voice_id).unwrap();

                    for (index, core) in cores.iter().enumerate() {
                        if core.is_alive && core.is_activated {
                            // Use pre-computed offset from physics loop (no recalculation!)
                            let offset = computed_offsets[index];

                            gpu_segments[write_idx] = to_segment_gpu(
                                core,
                                &feedback_array[index],
                                offset,
                                segment_length,
                                segment_line_width,
                            );
                            write_idx += 1;
                        }
                    }
                }
            });

        segment_count
    }

    /// GPU-only emission: Produce spawn requests respecting volume and limits
    ///
    /// This is the new GPU-side emission that eliminates CPU tracking.
    /// Emitters produce spawn requests at rates scaled by voice volume and particle_limit.
    /// The GPU handles slot finding and drops overflow requests (backpressure).
    ///
    /// Volume scaling: emission rate is multiplied by voice volume (0.0-1.0)
    /// Particle limit: per-voice caps are enforced via spawn rate limiting
    pub fn handle_particle_emission_gpu(
        &mut self,
        voices: &HashMap<VoiceId, Voice>,
        rng: &mut ThreadRng,
    ) -> Vec<nnpipe::compute::ParticleSpawnRequest> {
        let mut spawn_requests = Vec::new();

        for voice in voices.values() {
            // Respect voice volume for emission scaling
            let volume = voice.params.volume;
            let particle_limit = voice.params.particle_limit;

            // Skip if volume is near zero
            if volume < 0.001 {
                continue;
            }

            // Calculate effective spawn rate considering both volume and particle_limit
            // particle_limit acts as a maximum capacity per voice
            // We scale emission to approach but not exceed this limit
            let target_particles = (particle_limit as f32 * volume).round() as usize;

            // Simple budget: emit at scaled rate, let GPU handle overflow
            // The spawn rate is naturally limited by emitter.max_spawn_rate * volume
            for emitter in voice.emitters.iter().filter(|e| e.is_enabled()) {
                let color_limit = voice.params.color_limit;

                // Scale spawn rate by volume (emitters at low volume produce fewer particles)
                // Note: particle_limit primarily affects the total capacity, not individual emission
                // The GPU will naturally limit to max_particles across all voices
                let particles = emitter.emit(
                    volume, // Scale by volume (0.0-1.0)
                    10.0,   // Default speed
                    self.default_particle_size,
                    rgba_from(color_limit, 0.0),
                    rng,
                );

                // Convert ParticleCores to spawn requests
                // Respect target_particles as a soft limit per voice
                let mut voice_request_count = 0;
                let voice_id = voice.id.to_i32();
                for particle in particles {
                    if voice_request_count >= target_particles {
                        break; // Reached per-voice limit
                    }
                    spawn_requests.push(particle.to_spawn_request(voice_id));
                    voice_request_count += 1;
                }
            }
        }

        if !spawn_requests.is_empty() {
            // Debug: Check first spawn request colors
            if let Some(first) = spawn_requests.first() {
                println!(
                    "  GPU Emission: {} spawn requests generated (first color: [{:.2}, {:.2}, {:.2}, {:.2}], pos: [{:.1}, {:.1}])",
                    spawn_requests.len(),
                    first.color[0], first.color[1], first.color[2], first.color[3],
                    first.position[0], first.position[1]
                );
            }
        }

        spawn_requests
    }

    /// CPU emission with capacity tracking (legacy method for CPU physics)
    ///
    /// DEPRECATED: Use handle_particle_emission_gpu() for GPU physics.
    pub fn handle_particle_emission(
        &mut self,
        voices: &HashMap<VoiceId, Voice>,
        rng: &mut ThreadRng,
    ) {
        let mut total_emitted = 0;
        let mut total_emitters = 0;
        let mut enabled_emitters = 0;

        for voice in voices.values() {
            let mut emitters: Vec<_> = voice.emitters.iter().collect();
            emitters.shuffle(rng);
            total_emitters += emitters.len();

            for emitter in emitters.iter() {
                if emitter.is_enabled() {
                    enabled_emitters += 1;
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

                    println!(
                        "  Voice {:?}: current={}, limit={}, scaling={}",
                        parent_voice, current_count, voice_limit, emission_scaling
                    );

                    // Skip emission entirely if scaling is near zero
                    if emission_scaling < 0.001 {
                        println!("    Skipping emission (scaling too low)");
                        continue;
                    }

                    let color_limit = voices
                        .get(&parent_voice)
                        .map(|v| v.params.color_limit)
                        .unwrap_or(self.default_particle_color);

                    let mut new_particles = emitter.emit(
                        emission_scaling,
                        10.0,
                        self.default_particle_size,
                        rgba_from(color_limit, 0.0),
                        rng,
                    );
                    let new_particles_count = new_particles.len();
                    total_emitted += new_particles_count;
                    let mut new_feedback = vec![ParticleFeedback::new(); new_particles_count];

                    core_vec.append(&mut new_particles);
                    feedback_vec.append(&mut new_feedback);
                }
            }
        }

        println!(
            "Emission: {} total emitters, {} enabled, {} particles emitted",
            total_emitters, enabled_emitters, total_emitted
        );
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
        self.particle_cores.values().map(|cores| cores.len()).sum()
    }

    /// Get reference to GPU render vertex buffer (Phase 4)
    ///
    /// This buffer contains GPU-populated render vertices ready for rendering.
    /// Returns None if GPU physics is not initialized.
    pub fn gpu_render_vertex_buffer(&self) -> Option<&wgpu::Buffer> {
        self.gpu_particle_bridge
            .as_ref()
            .map(|bridge| bridge.render_vertex_buffer())
    }

    /// Get reference to GPU alive count buffer (Phase 4)
    ///
    /// This buffer contains the GPU-computed alive particle count.
    /// Returns None if GPU physics is not initialized.
    pub fn gpu_alive_count_buffer(&self) -> Option<&wgpu::Buffer> {
        self.gpu_particle_bridge
            .as_ref()
            .map(|bridge| bridge.alive_count_buffer())
    }

    /// Read back GPU-computed alive count (Phase 4)
    ///
    /// This is a blocking operation. Use sparingly for debugging or UI.
    /// Returns None if GPU physics is not initialized.
    pub fn read_gpu_alive_count(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> Option<u32> {
        self.gpu_particle_bridge
            .as_ref()
            .map(|bridge| bridge.read_back_gpu_alive_count(device, queue))
    }

    /// Swap GPU particle buffers (Phase 4)
    ///
    /// Call this after submitting GPU work to prepare for the next frame.
    pub fn end_gpu_frame(&mut self) {
        if let Some(ref mut bridge) = self.gpu_particle_bridge {
            bridge.end_frame();
        }
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
