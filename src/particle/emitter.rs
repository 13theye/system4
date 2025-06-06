// src/particle/emitter.rs
//
// The thing that spits out particles

use crate::particle::Particle;
use nannou::prelude::*;
use nannou::rand::{rngs::ThreadRng, Rng};

pub struct Emitter {
    pub id: usize, // player number that this Emitter belongs to
    pub origin: Vec2,
    pub start: Vec2,
    pub end: Vec2,
    pub direction: EmitDirection,
    pub max_spawn_rate: f32,
}

pub enum EmitDirection {
    North,
    South,
    East,
    West,
}

impl Emitter {
    pub fn new(
        id: usize,
        origin: Vec2,
        start: Vec2,
        end: Vec2,
        direction: EmitDirection,
        max_spawn_rate: f32,
    ) -> Self {
        Self {
            id,
            origin,
            start,
            end,
            direction,
            max_spawn_rate,
        }
    }

    // Generate a Vec of particles based on the emitter's parameters
    pub fn emit(
        &self,
        rate_factor: f32,
        speed: f32,
        size: f32,
        color: Rgba,
        rng: &mut ThreadRng,
    ) -> Vec<Particle> {
        let mut particles = Vec::new();

        let rate = (self.max_spawn_rate * rate_factor) as usize;

        let velocity = match self.direction {
            EmitDirection::North => vec2(0.0, speed),
            EmitDirection::South => vec2(0.0, -speed),
            EmitDirection::West => vec2(-speed, 0.0),
            EmitDirection::East => vec2(speed, 0.0),
        };
        let length = self.end.distance(self.start) - 4.0;
        let gen_range = -length / 2.0..length / 2.0;

        for _ in 0..rate {
            let var_pos = rng.gen_range(gen_range.clone());

            let position = match self.direction {
                EmitDirection::North => vec2(var_pos + self.origin.x, self.origin.y),
                EmitDirection::South => vec2(var_pos + self.origin.x, self.origin.y),
                EmitDirection::East => vec2(self.origin.x, var_pos + self.origin.y),
                EmitDirection::West => vec2(self.origin.x, var_pos + self.origin.y),
            };

            particles.push(Particle::new_with_motion(
                self.id,
                position,
                size,
                color,
                vec2(0.0, 0.0),
                velocity,
            ));
        }

        particles
    }

    pub fn draw(&self, draw: &Draw, scale_x: f32, scale_y: f32) {
        draw.line()
            .start(self.start * vec2(scale_x, scale_y))
            .end(self.end * vec2(scale_x, scale_y))
            .color(rgba(1.0, 0.0, 0.0, 0.2))
            .stroke_weight(4.0);
    }
}
