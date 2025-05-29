// src/particle/particles.rs
//
// Particle struct for the Particle System

use nannou::prelude::*;

#[derive(Default)]
pub struct Particle {
    pub position: Point2,
    pub velocity: Vec2,
    pub acceleration: Vec2,
    pub life_span: f32,
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
            life_span: 2000.0,
            size,
            color,
            mass: 22.0,
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
            life_span: 2000.0,
            size,
            color,
            mass: 11.0,
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

    pub fn draw(&self, draw: &Draw) {
        draw.ellipse()
            .xy(self.position)
            .w_h(self.size, self.size)
            .color(self.color);
    }

    pub fn is_offscreen(&self, bounds_rect: Rect) -> bool {
        let buffer = 100.0;
        self.position.x < bounds_rect.left() - buffer
            || self.position.x > bounds_rect.right() + buffer
            || self.position.y < bounds_rect.bottom() - buffer
            || self.position.y > bounds_rect.top() + buffer
    }

    pub fn kill(&mut self) {
        self.life_span = 0.0;
    }

    pub fn is_dead(&self) -> bool {
        if self.life_span <= 0.0 {
            return true;
        }
        false
    }
}
