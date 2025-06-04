// src/particle/particles.rs
//
// Particle struct for the Particle System

use nannou::prelude::*;

const PARTICLE_MASS: f32 = 5.0;

#[derive(Clone, Copy, Default)]
pub struct Particle {
    position: Point2,
    pub velocity: Vec2,
    pub acceleration: Vec2,
    pub life_span: f32,
    pub is_alive: bool,
    pub size: f32,
    pub mass: f32,
    pub color: Rgba,
}

impl Particle {
    pub fn new(position: Point2, size: f32, color: Rgba) -> Self {
        Self {
            acceleration: vec2(0.0, 0.0),
            velocity: vec2(0.0, 0.0),
            position,
            life_span: 1000.0,
            is_alive: true,
            size,
            color,
            mass: PARTICLE_MASS,
        }
    }

    pub fn new_with_motion(
        position: Point2,
        size: f32,
        color: Rgba,
        acceleration: Vec2,
        velocity: Vec2,
    ) -> Self {
        Self {
            acceleration,
            velocity,
            position,
            life_span: 1000.0,
            is_alive: true,
            size,
            color,
            mass: PARTICLE_MASS,
        }
    }

    pub fn update(&mut self) {
        self.velocity += self.acceleration;
        self.position += self.velocity;

        // Reset acceleration for next frame (forces will be reapplied)
        self.acceleration = vec2(0.0, 0.0);

        self.life_span -= 1.0;
        self.color.alpha = 1.0;
    }

    pub fn position(&self) -> Point2 {
        self.position
    }

    pub fn draw(&self, draw: &Draw) {
        draw.line()
            .xy(self.position)
            .start(self.position + vec2(self.size / 2.0, 0.0))
            .end(self.position + vec2(0.0, self.size / 2.0))
            .stroke_weight(self.size)
            .color(self.color);
    }

    pub fn is_offscreen(&self, bounds_rect: Rect) -> bool {
        let buffer = 1000.0;
        self.position.x < bounds_rect.left() - buffer
            || self.position.x > bounds_rect.right() + buffer
            || self.position.y < bounds_rect.bottom() - buffer
            || self.position.y > bounds_rect.top() + buffer
    }

    pub fn kill(&mut self) {
        self.life_span = 0.0;
        self.is_alive = false;
    }

    pub fn is_dead(&self) -> bool {
        if self.life_span <= 0.0 || !self.is_alive {
            return true;
        }
        false
    }

    pub fn is_alive(&self) -> bool {
        self.is_alive
    }

    /********************* Accessors *********************/
    pub fn set_position(&mut self, position: Point2) {
        self.position = position;
    }

    pub fn set_velocity(&mut self, velocity: Vec2) {
        self.velocity = velocity;
    }

    pub fn set_acceleration(&mut self, acceleration: Vec2) {
        self.acceleration = acceleration;
    }

    pub fn set_life_span(&mut self, life_span: f32) {
        self.life_span = life_span;
    }

    pub fn set_size(&mut self, size: f32) {
        self.size = size;
    }

    pub fn set_color(&mut self, color: Rgba) {
        self.color = color;
    }
}
