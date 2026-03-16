//! src/forces/wind.rs
//!
//! Rewrite of wind system

use nannou::prelude::*;

use crate::particle::ParticleCore;

// Re-export main Wind types
pub mod circle_formation;
pub mod double_circle;
pub mod wind_circle;
pub mod wind_field;

pub use circle_formation::CircleFormation;
pub use wind_circle::WindCircle;
pub use wind_field::WindField;

/// Original Wind struct has been simplified as a simple Vec2 encoding both strengh and direction.
#[derive(Debug, Default, Clone)]
pub struct Wind {
    pub velocity: Vec2,
}

impl Wind {
    /// Create a default wind, equivalent to Vec2::ZERO
    pub fn zero() -> Self {
        Self::default()
    }

    /// Create a new wind with the given vector
    pub fn new(velocity: Vec2) -> Self {
        Self { velocity }
    }

    /// Create a new Wind with a direction and speed
    pub fn new_with(direction: Vec2, speed: f32) -> Self {
        Self {
            velocity: direction * speed,
        }
    }

    /// Return the strength of the wind as a scalar
    pub fn strength(&self) -> f32 {
        self.velocity.length()
    }

    /// Return the direction component of the wind as a unit vector
    pub fn direction(&self) -> Vec2 {
        self.velocity.try_normalize().unwrap_or(Vec2::ZERO)
    }

    /// Apply the Wind to a ParticleCore with mass variation factor
    #[inline]
    pub fn apply(&self, particle: &mut ParticleCore, mass_variation_factor: f32) {
        // Activate the particle
        particle.activate();

        // Calculate the difference between wind's target velocity and particle's current velocity
        let diff = self.velocity - particle.velocity;

        // Calculate effective mass with variation factor
        let effective_mass = particle.mass * (1.0 + mass_variation_factor);

        // Calculate inertial resistance based on current momentum using effective mass
        let current_speed = particle.velocity.length();
        let momentum_magnitude = effective_mass * current_speed;

        // Inertial resistance: particles with higher momentum resist changes more
        let inertia_coefficient = 0.1; // Adjust this to control resistance strength
        let inertia_factor = 1.0 / (1.0 + momentum_magnitude * inertia_coefficient);

        // Apply the force with inertial resistance using effective mass
        let force = diff * inertia_factor;
        particle.acceleration += force / effective_mass;
    }
}
