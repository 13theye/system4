/// src/force/field.rs
///
/// Force field for field-based forces
use nannou::prelude::*;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::{
    forces::wind::WindField,
    groups::{Voice, VoiceId},
    particle::Particle,
};

/// Create a unique hash from voice_id and circle_id for noise parameter indexing
fn hash_voice_circle(voice_id: VoiceId, circle_id: usize) -> u64 {
    let mut hasher = DefaultHasher::new();
    (voice_id, circle_id).hash(&mut hasher);
    hasher.finish()
}

/// The ForceField tracks the forces that are acting on the particles.
/// It provides a coordinate space to align forces to screen locations.
pub struct ForceFields {
    // Force fields
    pub wind_field: WindField,

    // Force objects
    //pub wind_circles: Vec<WindCircle>,

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
            //wind_circles: Vec::new(),
            origin,
            bounds_size,
            grid_cols,
            grid_rows,
            cell_size,
        }
    }

    /// Update ForceField with all Voices' WindCircles with per-circle angle variations
    pub fn update(
        &mut self,
        voices: &mut HashMap<VoiceId, Voice>,
        rng: &mut nannou::rand::rngs::ThreadRng,
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

        // Update each cell and use per-circle angle variations
        self.wind_field
            .par_force_update_all(rng, &circle_noise_values);
    }

    /// Update all Winds in this ForceField
    pub fn force_update_all(&mut self) {
        // Use a dummy RNG and empty variations for compatibility
        let mut dummy_rng = nannou::rand::thread_rng();
        let empty_variations = HashMap::new();
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

    /*
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
    pub fn get_wind_circle_params_all(&self) -> BTreeMap<VoiceId, WindCircleParams> {
        self.wind_circles
            .iter()
            .map(|circle| (circle.parent_voice, circle.params().clone()))
            .collect()
    }

    /// Returns WindCircleParams for a given Voice (if it exists)
    pub fn get_wind_circle_params(&self, voice: VoiceId) -> Option<&WindCircleParams> {
        self.wind_circles
            .iter()
            .find(|circle| circle.parent_voice == voice)
            .map(|circle| circle.params())
    }

    /// Returns a mutable ref to WindCircleParams for a given Voice (if it exists)
    pub fn get_wind_circle_params_mut(&mut self, voice: VoiceId) -> Option<&mut WindCircleParams> {
        self.wind_circles
            .iter_mut()
            .find(|circle| circle.parent_voice == voice)
            .map(|circle| circle.params_mut())
    }

    /// Returns a mutable reference to WindCircle for a given Voice (if it exists)
    pub fn get_wind_circle_mut(&mut self, voice: VoiceId) -> Option<&mut WindCircle> {
        self.wind_circles
            .iter_mut()
            .find(|circle| circle.parent_voice == voice)
    }

    /// Returns true if a WindCircle exists for the given Voice
    pub fn has_circle_for_voice(&self, voice: &VoiceId) -> bool {
        self.wind_circles
            .iter()
            .any(|circle| circle.parent_voice == *voice)
    }
     */
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct CellIdx {
    pub x: usize,
    pub y: usize,
}
