// src/particle/particles.rs
//
// Particle struct for the Particle System

use crate::view::Voice;
use nannou::prelude::*;

const PARTICLE_MASS: f32 = 11.0;

#[derive(Clone, Copy)]
pub struct Particle {
    pub parent_emitter: usize, // the emitter that spawned this particle
    pub parent_voice: Voice,   // the voice that this particle belongs to
    position: Point2,
    feedback_positions: [Option<Point2>; 10],
    pub velocity: Vec2,
    pub acceleration: Vec2,
    pub life_span: f32,

    age_per_tick: f32,
    pub is_alive: bool,
    pub size: f32,
    pub mass: f32,
    pub color: Rgba,
}

impl Particle {
    pub fn new(
        parent_id: usize,
        parent_voice: Voice,
        position: Point2,
        size: f32,
        color: Rgba,
    ) -> Self {
        Self {
            parent_emitter: parent_id,
            parent_voice,
            acceleration: vec2(0.0, 0.0),
            velocity: vec2(0.0, 0.0),
            position,
            feedback_positions: [None; 10],
            life_span: 1000.0,
            age_per_tick: 1.0,
            is_alive: true,
            size,
            color,
            mass: PARTICLE_MASS,
        }
    }

    pub fn new_with_motion(
        parent_id: usize,
        parent_voice: Voice,
        position: Point2,
        size: f32,
        color: Rgba,
        acceleration: Vec2,
        velocity: Vec2,
    ) -> Self {
        Self {
            parent_emitter: parent_id,
            parent_voice,
            acceleration,
            velocity,
            position,
            feedback_positions: [None; 10],
            life_span: 1000.0,
            age_per_tick: 1.0,
            is_alive: true,
            size,
            color,
            mass: PARTICLE_MASS,
        }
    }

    pub fn update(&mut self) {
        // Add the current position to the feedback positions
        self.record_feedback_position();

        self.velocity += self.acceleration;
        self.position += self.velocity;

        // Reset acceleration for next frame (forces will be reapplied)
        self.acceleration = vec2(0.0, 0.0);

        self.life_span -= self.age_per_tick;

        if self.life_span < 200.0 {
            self.color.alpha = self.life_span / 200.0;
        } else {
            self.color.alpha = 1.0;
        }
    }

    pub fn position(&self) -> Point2 {
        self.position
    }

    fn record_feedback_position(&mut self) {
        for i in 0..9 {
            self.feedback_positions[i] = self.feedback_positions[i + 1];
        }
        self.feedback_positions[0] = Some(self.position);
    }

    pub fn draw(&self, draw: &Draw, feedback: f32, dpi_scale: f32) {
        let scaled_position = self.position / dpi_scale;
        let scaled_size = self.size / dpi_scale;

        if feedback < 0.05 {
            draw.line()
                .xy(scaled_position)
                .start(scaled_position + vec2(scaled_size / 2.0, 0.0))
                .end(scaled_position + vec2(0.0, scaled_size / 2.0))
                .stroke_weight(scaled_size)
                .color(self.color);
        } else {
            for i in 0..(feedback * 10.0).round() as usize {
                if let Some(position) = self.feedback_positions[i] {
                    let scaled_position = position / dpi_scale;
                    draw.line()
                        .xy(scaled_position)
                        .start(scaled_position + vec2(scaled_size / 2.0, 0.0))
                        .end(scaled_position + vec2(0.0, scaled_size / 2.0))
                        .stroke_weight(scaled_size)
                        .color(self.color);
                }
            }
        }
    }

    pub fn is_out_of_bounds(&self, bounds_rect: Rect) -> bool {
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

    pub fn set_age_per_tick(&mut self, age_per_tick: f32) {
        self.age_per_tick = age_per_tick;
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
