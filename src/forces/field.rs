/// src/force/field.rs
///
/// Force field for field-based forces
use nannou::prelude::*;
use std::collections::BTreeMap;

use crate::{
    forces::wind::{WindCircle, WindCircleParams, WindField},
    particle::Particle,
    voice::Voice,
};

/// The ForceField tracks the forces that are acting on the particles.
/// It provides a coordinate space to align forces to screen locations.
pub struct ForceFields {
    // Force fields
    pub wind_field: WindField,

    // Force objects
    pub wind_circles: Vec<WindCircle>,

    // Origin in the World Coordinate Space
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
            wind_circles: Vec::new(),
            origin,
            bounds_size,
            grid_cols,
            grid_rows,
            cell_size,
        }
    }

    /// Update all circles in this ForceField with per-circle angle variations
    pub fn update(&mut self, show_forces: bool, rng: &mut nannou::rand::rngs::ThreadRng) {
        // Update the wind circle meta-force
        for circle in self.wind_circles.iter_mut() {
            circle.update(&mut self.wind_field, show_forces);
        }

        // Collect per-circle angle variations
        let circle_angle_variations: std::collections::HashMap<usize, f32> = self
            .wind_circles
            .iter()
            .map(|circle| (circle.id, circle.params().noise))
            .collect();

        // Update each cell with per-circle angle variations
        self.wind_field
            .par_force_update_all(rng, &circle_angle_variations);
    }

    /// Update all Winds in this ForceField
    pub fn force_update_all(&mut self) {
        // Use a dummy RNG and empty variations for compatibility
        let mut dummy_rng = nannou::rand::thread_rng();
        let empty_variations = std::collections::HashMap::new();
        self.wind_field
            .par_force_update_all(&mut dummy_rng, &empty_variations);
    }

    /// Apply all applicable forces to a particle with mass variation factor
    pub fn apply_forces_to_particle(&self, particle: &mut Particle, mass_variation_factor: f32) {
        // Apply wind with mass variation
        self.wind_field.apply(particle, mass_variation_factor);
    }

    /// Recalculate all applicable forces in this ForceField
    pub fn recalculate_once(&mut self) {
        // Use the regular force_update_all for recalculation without variation
        self.force_update_all();
    }

    /// Add a WindCircle to this ForceField (replaces existing circle for same voice)
    pub fn add_wind_circle(&mut self, circle: WindCircle) {
        println!("Added wind circle for {:?}", circle.parent_voice);
        // Remove existing circle for this voice if it exists
        self.wind_circles
            .retain(|c| c.parent_voice != circle.parent_voice);
        // Add the new circle
        self.wind_circles.push(circle);
    }

    /// Returns a BTreeMap of all WindCircleParams by Voice
    pub fn get_wind_circle_params_all(&self) -> BTreeMap<Voice, WindCircleParams> {
        self.wind_circles
            .iter()
            .map(|circle| (circle.parent_voice, circle.params().clone()))
            .collect()
    }

    /// Returns WindCircleParams for a given Voice (if it exists)
    pub fn get_wind_circle_params(&self, voice: Voice) -> Option<&WindCircleParams> {
        self.wind_circles
            .iter()
            .find(|circle| circle.parent_voice == voice)
            .map(|circle| circle.params())
    }

    /// Returns a mutable ref to WindCircleParams for a given Voice (if it exists)
    pub fn get_wind_circle_params_mut(&mut self, voice: Voice) -> Option<&mut WindCircleParams> {
        self.wind_circles
            .iter_mut()
            .find(|circle| circle.parent_voice == voice)
            .map(|circle| circle.params_mut())
    }

    /// Returns a mutable reference to WindCircle for a given Voice (if it exists)
    pub fn get_wind_circle_mut(&mut self, voice: Voice) -> Option<&mut WindCircle> {
        self.wind_circles
            .iter_mut()
            .find(|circle| circle.parent_voice == voice)
    }

    /// Returns true if a WindCircle exists for the given Voice
    pub fn has_circle_for_voice(&self, voice: &Voice) -> bool {
        self.wind_circles
            .iter()
            .any(|circle| circle.parent_voice == *voice)
    }

    /******************* OSC command compatibility methods ********************* */

    /// Remove WindCircle for a given Voice
    pub fn remove_wind(&mut self, voice: &Voice) {
        if let Some(index) = self
            .wind_circles
            .iter()
            .position(|circle| circle.parent_voice == *voice)
        {
            let mut circle = self.wind_circles.remove(index);
            circle.remove_from_field(&mut self.wind_field, true);
            println!("Removed wind circle for {:?}", voice);
        } else {
            println!("Wind circle not found for {:?}", voice);
        }
    }

    /// Get the center vias of a WindCircle for a given Voice
    pub fn get_center_bias(&mut self, voice: &Voice) -> f32 {
        self.get_wind_circle_params(*voice)
            .map(|params| params.center_bias)
            .unwrap_or(0.0)
    }

    /// Set the center bias of WindCircle for a given Voice
    pub fn set_center_bias(&mut self, voice: &Voice, bias: f32) {
        if let Some(circle) = self.get_wind_circle_mut(*voice) {
            circle.params_mut().set_center_bias(bias);
        } else {
            println!("Wind circle not found for {:?}", voice);
        }
    }

    /// Set the strength of WindCircle for a given Voice
    pub fn set_strength(&mut self, voice: &Voice, strength: f32) {
        if let Some(circle) = self.get_wind_circle_mut(*voice) {
            circle.params_mut().set_strength(strength);
        } else {
            println!("Wind circle not found for {:?}", voice);
        }
    }

    /// Set the outer radius of WindCircle for a given Voice
    pub fn set_outer_radius(&mut self, voice: &Voice, radius: f32) {
        if let Some(circle) = self.get_wind_circle_mut(*voice) {
            circle.params_mut().set_outer_radius(radius);
        } else {
            println!("Wind circle not found for {:?}", voice);
        }
    }

    /// Set the inner radius of WindCircle for a given Voice
    pub fn set_inner_radius(&mut self, voice: &Voice, val: f32) {
        if let Some(circle) = self.get_wind_circle_mut(*voice) {
            circle.params_mut().set_inner_radius(val);
        } else {
            println!("Wind circle not found for {:?}", voice);
        }
    }

    /// Set the outer and inner radius of a WindCircle by Voice
    pub fn set_circle_dims(&mut self, voice: &Voice, radius: f32, width: f32) {
        if let Some(circle) = self.get_wind_circle_mut(*voice) {
            circle.params_mut().set_outer_radius(radius);
            circle.params_mut().set_inner_radius(radius - width);
        } else {
            println!("Wind circle not found for {:?}", voice);
        }
    }

    /// Get the angle variation of a WindCircle for a given Voice
    pub fn get_angle_variation(&self, voice: &Voice) -> f32 {
        self.get_wind_circle_params(*voice)
            .map(|params| params.noise)
            .unwrap_or(0.0)
    }

    /// Set the angle variation of WindCircle for a given Voice
    pub fn set_noise(&mut self, voice: &Voice, noise: f32) {
        if let Some(circle) = self.get_wind_circle_mut(*voice) {
            circle.params_mut().set_noise(noise);
        } else {
            println!("Wind circle not found for {:?}", voice);
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct CellIdx {
    pub x: usize,
    pub y: usize,
}
