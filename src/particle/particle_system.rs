/// src/particle/particle_system.rs
///
///
/// The Particle System of System 4
use std::collections::HashMap;

use nannou::prelude::*;
use nannou::rand::{rngs::ThreadRng, seq::SliceRandom};
use nnpipe::renderers::{ParticleGpu, SegmentGpu};
use rayon::prelude::*;

use crate::{
    forces::ForceFields,
    groups::{Voice, VoiceId},
    model::GpuBuffers,
    particle::Particle,
};

pub const EMPTY_GPU_BUFFER: GpuBuffers = (Vec::new(), Vec::new());
const MAX_POSITION_OFFSET: f32 = 10.0; // Maximum screen distance for position offset in pixels
const MAX_SPAWN_RATE: f32 = 80.0;

pub struct ParticleSystem {
    // Particles
    pub particles: HashMap<VoiceId, Vec<Particle>>,

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
            particles: HashMap::new(),
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
        }
    }

    /********************* Update methods ********************************** */

    /// Emit particles, update forces, update particles, and cull particles - returns simple particles and accompanying trails
    pub fn update(
        &mut self,
        voices: &mut HashMap<VoiceId, Voice>,
        rng: &mut ThreadRng,
        gpu_buffers: &mut HashMap<VoiceId, GpuBuffers>,
    ) {
        self.handle_particle_emission(voices, rng);
        self.cull_excess_particles(voices);

        self.forces.update(voices, rng);

        // Reuse existing buffer to avoid allocations
        for (_, (p_gpu, s_gpu)) in gpu_buffers.iter_mut() {
            p_gpu.clear();
            s_gpu.clear();
        }

        // Pre-compute position offset factors for all voices to avoid borrow conflicts
        let vibration_values: HashMap<VoiceId, f32> = voices
            .values()
            .map(|voice| (voice.id, voice.params.vibration))
            .collect();

        for (voice_id, particles) in self.particles.iter_mut() {
            let color_limit = voices.get(voice_id).map(|v| v.params.color_limit);
            let alpha_limit = voices.get(voice_id).map(|v| v.params.alpha_limit);

            // Skip if no limits for this voice
            if color_limit.is_none() || alpha_limit.is_none() {
                continue;
            }

            let (color_limit, alpha_limit) = (color_limit.unwrap(), alpha_limit.unwrap());

            // Ensure buffer exists for this voice
            let (pgpu_buf, sgpu_buf) = gpu_buffers
                .entry(*voice_id)
                .or_insert_with(|| EMPTY_GPU_BUFFER);

            // Pre-compute mass variation factors for all particles in this voice
            let mass_variations: Vec<f32> =
                if self.mass_variation_enabled && self.mass_variation_amount > 0.0 {
                    use nannou::rand::Rng;
                    particles
                        .iter()
                        .map(|_| {
                            rng.gen_range(-self.mass_variation_amount..=self.mass_variation_amount)
                        })
                        .collect()
                } else {
                    vec![0.0; particles.len()]
                };

            // Pre-compute position offset random signs for all particles in this voice
            let vibration = vibration_values.get(voice_id).copied().unwrap_or(0.0);
            let position_offsets: Vec<f32> = if vibration > 0.0 {
                use nannou::rand::Rng;
                particles
                    .iter()
                    .map(|_| rng.gen_range(-vibration..vibration))
                    .collect()
            } else {
                vec![0.0; particles.len()]
            };

            // Parallel update, collect simple particle data and segments
            let (gpu_particle_group, gpu_segment_group): (Vec<ParticleGpu>, Vec<SegmentGpu>) =
                particles
                    .par_iter_mut()
                    .enumerate()
                    .filter_map(|(index, particle)| {
                        let mass_variation_factor = mass_variations[index];
                        self.forces
                            .apply_forces_to_particle(particle, mass_variation_factor);

                        // Calculate position offset perpendicular to velocity BEFORE updating particle
                        // This ensures we use the velocity from this frame for the offset calculation
                        let offset = if vibration > 0.0 && particle.velocity.length_squared() > 0.0
                        {
                            let normal =
                                vec2(-particle.velocity.y, particle.velocity.x).normalize_or_zero();
                            normal * MAX_POSITION_OFFSET * position_offsets[index]
                        } else {
                            vec2(0.0, 0.0)
                        };

                        // Update particle with the calculated offset for feedback recording
                        particle.update(color_limit, alpha_limit, offset);

                        if particle.is_out_of_bounds(self.bounds_rect) {
                            particle.kill();
                        }

                        if particle.is_alive() && particle.is_activated() {
                            Some((
                                particle.to_gpu_with_offset(offset),
                                particle.to_segment_gpu(),
                            ))
                        } else {
                            None
                        }
                    })
                    .unzip();

            // Append to buffers
            pgpu_buf.extend(gpu_particle_group);
            sgpu_buf.extend(gpu_segment_group);

            // Cull dead particles
            particles.retain(|particle| particle.is_alive());
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
                    let particle_vec = self.particles.entry(parent_voice).or_default();
                    let current_count = particle_vec.len();

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

                    // Add particles to the voice's particle vector
                    particle_vec.extend(new_particles);
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

    fn cull_excess_particles(&mut self, voices: &HashMap<VoiceId, Voice>) {
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

                for particle in active_particles
                    .into_iter()
                    //.rev() // kill oldest particles first
                    .take(excess_active as usize)
                {
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
        self.particles
            .values()
            .map(|particles| particles.len())
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
