//! src/force/field.rs
//!
//! Force field system supporting both CPU and GPU force computation
//!
//! The `ForceFields` struct manages a hybrid force field that can compute forces
//! either on the CPU (using `WindField`) or GPU (using `GpuForceField`). In Phase 2,
//! GPU computes forces but CPU applies them during physics integration.
//!
//! # Architecture
//!
//! - CPU mode: `WindField` computes and stores forces in `winds_combined`
//! - GPU mode: `GpuForceField` computes forces, `read_back_gpu()` transfers to `gpu_force_cache`
//! - Physics: `apply_forces_to_particle()` routes to CPU or GPU path based on `use_gpu` flag
//!
//! # Coordinate Systems
//!
//! - World space: Center origin (0,0), +X right, +Y up
//! - Grid space: Top-left origin (0,0), +X right, +Y down
//! - Transformations handled by `position_to_grid_idx()` and GPU adapter
//!
//! # Example
//!
//! ```ignore
//! let mut forces = ForceFields::new(pt2(0.0, 0.0), vec2(800.0, 600.0), 64, 48);
//! forces.init_gpu(device, true)?; // Enable GPU mode
//! forces.update_gpu(voices, queue, encoder)?;
//! forces.read_back_gpu(device, queue); // Blocking transfer to CPU
//! forces.apply_forces_to_particle(&mut particle, 0.0); // Uses GPU forces
//! ```
use nannou::prelude::*;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use crate::{
    forces::WindField,
    groups::{Voice, VoiceId},
    particle::ParticleCore,
};

/// Inertial resistance coefficient for wind force application
/// Higher values = particles with momentum resist changes more strongly
/// Matches Wind::apply() coefficient
const INERTIA_COEFFICIENT: f32 = 0.1;

/// Create a unique hash from voice_id and circle_id for noise parameter indexing
fn hash_voice_circle(voice_id: VoiceId, circle_id: usize) -> u64 {
    let mut hasher = DefaultHasher::new();
    (voice_id, circle_id).hash(&mut hasher);
    hasher.finish()
}

/// The ForceField tracks the forces that are acting on the particles.
/// It provides a coordinate space to align forces to screen locations.
pub struct ForceFields {
    // CPU force field
    pub wind_field: WindField,

    // Flag to enable GPU force field computation
    pub use_gpu: bool,

    // Origin in the World Coordinate Space
    // Kept for future use
    #[allow(dead_code)]
    origin: Vec2,
    #[allow(dead_code)]
    bounds_size: Vec2,
    #[allow(dead_code)]
    grid_cols: usize,
    #[allow(dead_code)]
    grid_rows: usize,
    #[allow(dead_code)]
    cell_size: Vec2,
}

impl ForceFields {
    pub fn new(origin: Vec2, bounds_size: Vec2, grid_cols: usize, grid_rows: usize) -> Self {
        let cell_size = Vec2::new(
            bounds_size.x / grid_cols as f32,
            bounds_size.y / grid_rows as f32,
        );

        Self {
            wind_field: WindField::new(origin, bounds_size, grid_cols, grid_rows),
            use_gpu: false,
            origin,
            bounds_size,
            grid_cols,
            grid_rows,
            cell_size,
        }
    }

    /// Enable or disable GPU force field computation
    pub fn set_use_gpu(&mut self, use_gpu: bool) {
        self.use_gpu = use_gpu;
    }

    /// Update ForceField with all Voices' WindCircles with per-circle angle variations
    ///
    /// This method updates only CPU force field.
    pub fn update(
        &mut self,
        voices: &mut HashMap<VoiceId, Voice>,
        rng: &mut rand::rngs::ThreadRng,
    ) {
        let mut circle_noise_values: HashMap<u64, f32> = HashMap::new();

        // Update each circle and collect per-circle noise values using hash keys
        for voice in voices.values_mut() {
            // Update the wind circle meta-force
            for circle in voice.wind_circles.values_mut() {
                circle.update(&mut self.wind_field);
                let hash_key = hash_voice_circle(voice.id, circle.id);
                circle_noise_values.insert(hash_key, circle.params().noise);
            }
        }

        // Update CPU wind field
        self.wind_field
            .par_update_all_combined_cells(rng, &circle_noise_values);
    }

    /// Update all Winds in this ForceField
    pub fn force_update_all(&mut self) {
        // Use a dummy RNG and empty variations for compatibility
        let mut dummy_rng = rand::rng();
        let empty_variations = HashMap::new();
        self.wind_field
            .par_update_all_combined_cells(&mut dummy_rng, &empty_variations);
    }

    /// Apply all applicable forces to a particle with mass variation factor
    /// OPTIMIZED: Now works with ParticleCore for better cache locality
    ///
    /// If GPU force field is enabled, reads from cached GPU forces.
    /// Otherwise, uses CPU wind field.
    pub fn apply_forces_to_particle(
        &self,
        particle: &mut ParticleCore,
        mass_variation_factor: f32,
    ) {
        if self.use_gpu {
            // If using GPU, ForceFields use a different pathway.
            // DO NOTHING
        } else {
            // Use CPU wind field
            self.wind_field.apply(particle, mass_variation_factor);
        }
    }

    /// Recalculate all applicable forces in this ForceField
    pub fn recalculate_once(&mut self) {
        // Use the regular force_update_all for recalculation without variation
        self.force_update_all();
    }
}

#[derive(Debug, PartialEq)]
pub struct CellIdx {
    pub x: usize,
    pub y: usize,
}
