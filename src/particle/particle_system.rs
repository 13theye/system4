// src/particle/particle_system.rs
//
//
// The Particle System of System 3

use nannou::{noise::Turbulence, prelude::*};

#[derive(Default)]
pub struct Particle {
    pub position: Point2,
    pub velocity: Vec2,
    pub acceleration: Vec2,
    pub life_span: f32,
    pub size: f32,
    pub color: Rgba,
}

impl Particle {
    pub fn new(position: Point2, size: f32, color: Rgba) -> Self {
        Self {
            acceleration: vec2(0.0, 0.0),
            velocity: vec2(0.0, 0.0),
            position,
            life_span: 255.0,
            size,
            color,
        }
    }

    fn update(&mut self) {
        self.velocity += self.acceleration;
        self.position += self.velocity;
        self.life_span -= 2.0;
        self.color.alpha = 1.0 / 255.0;
    }

    fn draw(&self, draw: &Draw) {
        draw.ellipse()
            .xy(self.position)
            .w_h(self.size, self.size)
            .color(self.color)
            .stroke(self.color)
            .stroke_weight(2.0);
    }

    fn is_dead(&self) -> bool {
        if self.life_span <= 0.0 {
            return true;
        }
        false
    }
}

pub struct ParticleSystem {
    pub particles: Vec<Particle>,
    origin: Point2,
    default_size: f32,
    default_color: Rgba,
}

impl ParticleSystem {
    pub fn new(origin: Point2, default_size: f32, default_color: Rgba) -> Self {
        Self {
            particles: Vec::new(),
            origin,
            default_size,
            default_color,
        }
    }

    pub fn add_particle(&mut self) {
        self.particles.push(Particle::new(
            self.origin,
            self.default_size,
            self.default_color,
        ));
    }

    pub fn update(&mut self) {
        for i in (0..self.particles.len()).rev() {
            self.particles[i].update();
            if self.particles[i].is_dead() {
                self.particles.remove(i);
            }
        }
    }

    pub fn draw(&self, draw: &Draw) {
        for particle in self.particles.iter() {
            particle.draw(draw);
        }
    }
}
