// system4-core/src/physics/forces/force_fields.rs
// Force field management and application

use super::WindField;
use crate::physics::particles::ParticleCore;
use glam::Vec2;
use std::collections::HashMap;

/// ForceFields manages and applies forces to particles
pub struct ForceFields {
    pub wind_field: WindField,
    origin: Vec2,
    bounds_size: Vec2,
    grid_cols: usize,
    grid_rows: usize,
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
            origin,
            bounds_size,
            grid_cols,
            grid_rows,
            cell_size,
        }
    }

    /// Update force field with wind circles using a callback pattern
    /// This allows any collection type without generic complexity
    pub fn update_with_circles<F>(&mut self, mut update_fn: F, rng: &mut rand::rngs::ThreadRng)
    where
        F: FnMut(&mut WindField) -> HashMap<u64, f32>,
    {
        // Let the caller update circles and collect noise values
        let circle_noise_values = update_fn(&mut self.wind_field);

        // Update wind field cells with angle variations
        self.wind_field
            .par_force_update_all(rng, &circle_noise_values);
    }

    /// Update all winds without variations
    pub fn force_update_all(&mut self) {
        let mut dummy_rng = rand::rng();
        let empty_variations = HashMap::new();
        self.wind_field
            .par_force_update_all(&mut dummy_rng, &empty_variations);
    }

    /// Apply all forces to a particle
    #[inline]
    pub fn apply_forces_to_particle(&self, particle: &mut ParticleCore, mass_variation_factor: f32) {
        self.wind_field.apply(particle, mass_variation_factor);
    }

    /// Recalculate all forces
    pub fn recalculate_once(&mut self) {
        self.force_update_all();
    }

    // Accessors
    pub fn origin(&self) -> Vec2 {
        self.origin
    }

    pub fn bounds_size(&self) -> Vec2 {
        self.bounds_size
    }

    pub fn grid_cols(&self) -> usize {
        self.grid_cols
    }

    pub fn grid_rows(&self) -> usize {
        self.grid_rows
    }

    pub fn cell_size(&self) -> Vec2 {
        self.cell_size
    }
}
