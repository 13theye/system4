// system4-core/src/physics/particles/particles.rs
// Particle data structures for physics simulation

use crate::constants::{
    FADE_IN_DURATION, FADE_OUT_DURATION, FEEDBACK_POSITIONS, PARTICLE_LIFE_SPAN, PARTICLE_MASS,
};
use crate::physics::{Rgb, Rgba};
use glam::Vec2;

/// Core particle data for physics updates (~100 bytes)
/// This is the "hot" data that gets accessed every frame during physics calculations
#[derive(Clone, Copy, Debug)]
pub struct ParticleCore {
    pub position: Vec2,
    pub velocity: Vec2,
    pub acceleration: Vec2,
    pub age: f32,
    pub remaining_life_span: f32,
    pub age_per_tick: f32,
    pub is_alive: bool,
    pub is_activated: bool,
    pub size: f32,
    pub mass: f32,
    pub rgba: Rgba,
}

/// Feedback/trail data for rendering (~3.4KB)
/// This is the "cold" data that only gets accessed during segment generation
#[derive(Clone, Debug)]
pub struct ParticleFeedback {
    pub positions: [Option<Vec2>; FEEDBACK_POSITIONS],
    pub colors: [Option<Rgb>; FEEDBACK_POSITIONS],
    pub current_index: usize,
}

impl Default for ParticleFeedback {
    fn default() -> Self {
        Self {
            positions: [None; FEEDBACK_POSITIONS],
            colors: [None; FEEDBACK_POSITIONS],
            current_index: 0,
        }
    }
}

impl ParticleFeedback {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&mut self, position: Vec2, color: Rgb) {
        self.positions[self.current_index] = Some(position);
        self.colors[self.current_index] = Some(color);
        self.current_index = (self.current_index + 1) % FEEDBACK_POSITIONS;
    }
}

impl ParticleCore {
    pub fn new(position: Vec2, size: f32, color: Rgba) -> Self {
        Self {
            position,
            velocity: Vec2::ZERO,
            acceleration: Vec2::ZERO,
            age: 0.0,
            remaining_life_span: PARTICLE_LIFE_SPAN,
            age_per_tick: 1.0,
            is_alive: true,
            is_activated: false,
            size,
            mass: PARTICLE_MASS,
            rgba: color,
        }
    }

    pub fn with_velocity(mut self, velocity: Vec2) -> Self {
        self.velocity = velocity;
        self
    }

    /// Update the particle based on forces and age
    /// This is the hot path - keep it tight and cache-friendly
    #[inline]
    pub fn update(&mut self, color_limit: Rgb, alpha_limit: f32) {
        // Apply velocity and reset acceleration
        self.velocity += self.acceleration;
        self.position += self.velocity;
        self.acceleration = Vec2::ZERO;

        // Update color if needed
        if self.rgba.color != color_limit {
            self.rgba.color = color_limit;
        }

        // Calculate fade-in factor based on natural age
        let fade_in_factor = if self.age < FADE_IN_DURATION {
            self.age / FADE_IN_DURATION
        } else {
            1.0
        };

        // Calculate the maximum alpha this particle has reached so far
        let max_alpha_reached = alpha_limit * fade_in_factor.powi(3);

        // Calculate life-based alpha fade-out using remaining life span
        let end_of_life_alpha = if self.remaining_life_span <= FADE_OUT_DURATION {
            self.remaining_life_span / FADE_OUT_DURATION
        } else {
            1.0
        };

        self.rgba.alpha = max_alpha_reached * end_of_life_alpha;

        // Increment natural age and decrement remaining life span
        // Only increment age if the particle is activated
        if self.is_activated {
            self.age += self.age_per_tick;
            self.remaining_life_span -= self.age_per_tick;
        }

        if self.remaining_life_span <= 0.0 {
            self.kill();
        }
    }

    /// Check if particle is out of bounds (takes rectangle bounds as min/max points)
    #[inline]
    pub fn is_out_of_bounds(&self, min: Vec2, max: Vec2, buffer: f32) -> bool {
        self.position.x < min.x - buffer
            || self.position.x > max.x + buffer
            || self.position.y < min.y - buffer
            || self.position.y > max.y + buffer
    }

    #[inline]
    pub fn kill(&mut self) {
        self.remaining_life_span = 0.0;
        self.is_alive = false;
    }

    #[inline]
    pub fn set_to_fade_out(&mut self) {
        self.remaining_life_span = FADE_OUT_DURATION;
    }

    #[inline]
    pub fn activate(&mut self) {
        self.is_activated = true;
    }

    pub fn fade_out_duration(&self) -> f32 {
        FADE_OUT_DURATION
    }
}
