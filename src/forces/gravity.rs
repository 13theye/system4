// src/force/forces.rs
//
//
// Defining forces for the particle system

use nannou::prelude::*;

use crate::particle::Particle;

#[derive(Clone, Default)]
pub struct Gravity {
    origin: Vec2,
    mass: f32,
}

impl Gravity {
    pub fn new(origin: Vec2, mass: f32) -> Self {
        Self { origin, mass }
    }

    pub fn apply(&self, particle: &mut Particle) {
        let direction = self.origin - particle.position();
        let distance = direction.length();

        // Swallow particles that are too close
        if distance < 100.0 {
            particle.kill();
        }
        let force_magnitude = self.mass * particle.mass / distance.powi(2);
        let force_direction = direction.normalize();
        let force = force_direction * force_magnitude;
        particle.acceleration += force;
    }
}
