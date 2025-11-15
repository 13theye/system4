// system4-core/src/physics/particles/particle_system.rs
// Core particle system - physics simulation without rendering

use crate::physics::{
    forces::ForceFields,
    particles::{Emitter, ParticleCore, ParticleFeedback},
    Rgb, VoiceId,
};
use glam::Vec2;
use rand::{rngs::ThreadRng, seq::SliceRandom, Rng};
use rayon::prelude::*;
use std::collections::HashMap;

const MAX_SPAWN_RATE: f32 = 80.0;
const BOUNDS_BUFFER: f32 = 1500.0;

/// Voice parameters needed for particle system operations
/// This trait allows the core to work with app-level Voice types
pub trait VoiceParams {
    fn particle_limit(&self) -> usize;
    fn volume(&self) -> f32;
    fn color_limit(&self) -> Rgb;
    fn alpha_limit(&self) -> f32;
    fn vibration(&self) -> f32;
    fn emitters(&self) -> &[Box<dyn Emitter>];
}

/// Core particle system - manages particle physics without rendering
pub struct ParticleSystemCore {
    // Split particle storage for cache locality
    pub particle_cores: HashMap<VoiceId, Vec<ParticleCore>>,
    pub particle_feedback: HashMap<VoiceId, Vec<ParticleFeedback>>,

    // Forces
    pub forces: ForceFields,

    // Global params
    pub default_particle_limit: usize,
    pub global_max_spawn_rate: f32,

    // Bounds (stored as min/max for core compatibility)
    origin: Vec2,
    bounds_min: Vec2,
    bounds_max: Vec2,
    pub default_particle_color: Rgb,
    default_particle_size: f32,

    // Mass variation parameters
    pub mass_variation_enabled: bool,
    pub mass_variation_amount: f32,
}

impl ParticleSystemCore {
    pub fn new(
        origin: Vec2,
        width: f32,
        height: f32,
        default_particle_size: f32,
        default_particle_color: Rgb,
        default_particle_limit: usize,
        grid_cols: usize,
        grid_rows: usize,
    ) -> Self {
        let bounds_size = Vec2::new(width, height);
        let half_size = bounds_size * 0.5;

        Self {
            particle_cores: HashMap::new(),
            particle_feedback: HashMap::new(),
            forces: ForceFields::new(origin, bounds_size, grid_cols, grid_rows),
            global_max_spawn_rate: MAX_SPAWN_RATE,
            origin,
            bounds_min: origin - half_size,
            bounds_max: origin + half_size,
            default_particle_size,
            default_particle_color,
            default_particle_limit,
            mass_variation_enabled: true,
            mass_variation_amount: 0.05, // 5% variation by default
        }
    }

    /// Update particle physics for a specific voice
    /// This is called per-voice from the app layer
    pub fn update_voice_particles(
        &mut self,
        voice_id: VoiceId,
        color_limit: Rgb,
        alpha_limit: f32,
        vibration: f32,
        rng: &mut ThreadRng,
    ) {
        let Some(cores) = self.particle_cores.get_mut(&voice_id) else {
            return;
        };

        // Pre-compute variations
        let mass_variations: Vec<f32> = if self.mass_variation_enabled && self.mass_variation_amount > 0.0 {
            cores.iter().map(|_| rng.random_range(-self.mass_variation_amount..=self.mass_variation_amount)).collect()
        } else {
            vec![0.0; cores.len()]
        };

        let position_offsets: Vec<f32> = if vibration > 0.0 {
            cores.iter().map(|_| rng.random_range(-vibration..vibration)).collect()
        } else {
            vec![0.0; cores.len()]
        };

        let feedback_array = self.particle_feedback.get_mut(&voice_id).unwrap();

        // OPTIMIZATION: Physics update only touches ParticleCore (~100B per particle)
        cores
            .par_iter_mut()
            .zip(feedback_array.par_iter_mut())
            .enumerate()
            .for_each(|(index, (core, feedback))| {
                let mass_variation_factor = mass_variations[index];

                // Apply forces
                self.forces.apply_forces_to_particle(core, mass_variation_factor);

                // Update core particle
                core.update(color_limit, alpha_limit);

                // Calculate position offset for vibration
                let offset = if vibration > 0.0 && core.velocity.length_squared() > 0.0 {
                    let normal = Vec2::new(-core.velocity.y, core.velocity.x).normalize_or_zero();
                    normal * 10.0 * position_offsets[index]
                } else {
                    Vec2::ZERO
                };

                // Record feedback position
                let offset_position = if offset.length_squared() > 0.0 {
                    core.position + offset
                } else {
                    core.position
                };
                feedback.record(offset_position, color_limit);

                // Check bounds
                if core.is_out_of_bounds(self.bounds_min, self.bounds_max, BOUNDS_BUFFER) {
                    core.kill();
                }
            });

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

    pub fn handle_particle_emission<V: VoiceParams>(
        &mut self,
        voices: &HashMap<VoiceId, V>,
        rng: &mut ThreadRng,
    ) {
        for (voice_id, voice) in voices.iter() {
            let emitters: Vec<_> = voice.emitters().iter().collect();
            let mut shuffled_emitters = emitters;
            shuffled_emitters.shuffle(rng);

            for emitter in shuffled_emitters.iter() {
                if emitter.is_enabled() {
                    let core_vec = self.particle_cores.entry(*voice_id).or_default();
                    let feedback_vec = self.particle_feedback.entry(*voice_id).or_default();
                    let current_count = core_vec.len();

                    let voice_limit = voice.volume() * voice.particle_limit() as f32;
                    let emission_scaling = Self::linear_emission_scaling(voice_limit, current_count);

                    if emission_scaling < 0.001 {
                        continue;
                    }

                    let color_limit = voice.color_limit();
                    let new_particles = emitter.emit(
                        emission_scaling,
                        10.0,
                        self.default_particle_size,
                        crate::physics::Rgba::new(color_limit.r, color_limit.g, color_limit.b, 0.0),
                        rng,
                    );

                    for particle_core in new_particles {
                        core_vec.push(particle_core);
                        feedback_vec.push(ParticleFeedback::new());
                    }
                }
            }
        }
    }

    fn linear_emission_scaling(limit: f32, current_count: usize) -> f32 {
        let ratio = current_count as f32 / limit;
        (1.0 - ratio).max(0.0)
    }

    fn cull_excess_particles<V: VoiceParams>(&mut self, voices: &HashMap<VoiceId, V>) {
        for (voice_id, cores) in self.particle_cores.iter_mut() {
            let limit = voices
                .get(voice_id)
                .map(|v| v.volume() * v.particle_limit() as f32)
                .unwrap_or(self.default_particle_limit as f32);

            let mut active_particles = 0;
            for core in cores.iter_mut().rev() {
                if core.remaining_life_span > core.fade_out_duration() {
                    active_particles += 1;
                    if active_particles > limit as usize {
                        core.set_to_fade_out();
                    }
                }
            }
        }
    }

    /********************* Accessors ********************************** */

    pub fn set_mass_variation_enabled(&mut self, enabled: bool) {
        self.mass_variation_enabled = enabled;
    }

    pub fn set_mass_variation_amount(&mut self, amount: f32) {
        self.mass_variation_amount = amount.max(0.0);
    }

    pub fn get_particle_count(&self) -> usize {
        self.particle_cores.values().map(|cores| cores.len()).sum()
    }

    pub fn change_bounds_size_to(&mut self, width: f32, height: f32) {
        let bounds_size = Vec2::new(width, height);
        let half_size = bounds_size * 0.5;
        self.bounds_min = self.origin - half_size;
        self.bounds_max = self.origin + half_size;
    }

    pub fn origin(&self) -> Vec2 {
        self.origin
    }

    pub fn bounds_min(&self) -> Vec2 {
        self.bounds_min
    }

    pub fn bounds_max(&self) -> Vec2 {
        self.bounds_max
    }
}
