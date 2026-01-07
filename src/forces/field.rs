//! src/force2/field.rs
//!
//! Force field for field-based forces
use nannou::prelude::*;

use crate::{
    forces::wind::{wind_field, WindCircle, WindField},
    groups::VoiceId,
    particle::ParticleCore,
};

/// The ForceField tracks the forces that are acting on the particles.
/// It provides a coordinate space to align forces to screen locations.
pub struct ForceFields {
    // Force fields
    pub wind_field: WindField,

    // Origin in the World Coordinate Space
    // Kept for future use
    #[allow(dead_code)]
    origin: Vec2,
    #[allow(dead_code)]
    bounds_size: Vec2,
}

impl ForceFields {
    pub fn new(origin: Vec2, bounds_size: Vec2) -> Self {
        Self {
            wind_field: WindField::new(origin, bounds_size),
            origin,
            bounds_size,
        }
    }

    /// Apply all applicable forces to a particle with mass variation factor
    /// OPTIMIZED: Now works with ParticleCore for better cache locality
    pub fn apply_unified_forces_to_particle(
        &self,
        particle: &mut ParticleCore,
        circles: &[&WindCircle],
        mass_variation_factor: f32,
        noise: f64,
    ) {
        // Apply wind with mass variation
        wind_field::apply_combined_winds_to_particle(
            particle,
            circles,
            mass_variation_factor,
            noise,
        );
    }

    /// This version considers the ParticleSystemState to determine if forces
    /// should be combined or applied separately by voice.
    pub fn apply_forces_to_particle(
        &self,
        particle: &mut ParticleCore,
        circles: &[&WindCircle],
        mass_variation_factor: f32,
        noise: f64,
        current_voice: &VoiceId,
        should_combine: bool,
    ) {
        if should_combine {
            wind_field::apply_combined_winds_to_particle(
                particle,
                circles,
                mass_variation_factor,
                noise,
            );
        } else {
            wind_field::apply_voice_wind_to_particle(
                particle,
                circles,
                mass_variation_factor,
                noise,
                current_voice,
            );
        }
    }
}
