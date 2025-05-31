// src/particle/particle_system.rs
//
//
// The Particle System of System 3

use nannou::prelude::*;
use nannou::rand::{rngs::ThreadRng, Rng};

use crate::{forces::ForceFields, particle::Particle};

pub struct ParticleSystem {
    pub particles: Vec<Particle>,
    pub forces: ForceFields,

    // Origin and bounds
    origin: Point2,
    bounds_size: Vec2,
    pub bounds_rect: Rect,
    default_particle_size: f32,
    default_particle_color: Rgba,
}

impl ParticleSystem {
    pub fn new(
        origin: Point2,
        width: f32,
        height: f32,
        default_particle_size: f32,
        default_particle_color: Rgba,
    ) -> Self {
        let bounds_size = Vec2::new(width, height);
        let bounds_rect = Rect::from_x_y_w_h(origin.x, origin.y, width, height);
        let grid_cols = 96;
        let grid_rows = 54;

        Self {
            particles: Vec::new(),
            forces: ForceFields::new(origin, bounds_size, grid_cols, grid_rows),
            origin,
            bounds_size,
            bounds_rect,
            default_particle_size,
            default_particle_color,
        }
    }

    pub fn update(&mut self, show_forces: bool) {
        self.forces.update(show_forces);

        for i in (0..self.particles.len()).rev() {
            self.forces.apply(&mut self.particles[i]);

            self.particles[i].update();
            if self.particles[i].is_offscreen(self.bounds_rect) {
                self.particles[i].kill();
            }
            if self.particles[i].is_dead() {
                self.particles.remove(i);
            }
        }
    }

    pub fn change_bounds_size_to(&mut self, width: f32, height: f32) {
        self.bounds_size = Vec2::new(width, height);
        self.bounds_rect = self.make_bounds_rect();
    }

    fn make_bounds_rect(&self) -> Rect {
        Rect::from_x_y_w_h(
            self.origin.x,
            self.origin.y,
            self.bounds_size.x,
            self.bounds_size.y,
        )
    }

    pub fn add_particle(&mut self, position: Vec2) {
        self.particles.push(Particle::new(
            position,
            self.default_particle_size,
            self.default_particle_color,
        ));
    }

    pub fn add_particle_with_velocity(&mut self, position: Vec2, velocity: Vec2) {
        let acceleration = vec2(0.0, 0.0);
        self.particles.push(Particle::new_with_motion(
            position,
            self.default_particle_size,
            self.default_particle_color,
            acceleration,
            velocity,
        ));
    }

    pub fn add_particle_with_random_motion(&mut self, rng: &mut ThreadRng) {
        let lo = -2.0;
        let hi = 2.0;
        let acceleration = vec2(rng.gen_range(lo..hi), rng.gen_range(lo..hi));
        let velocity = vec2(rng.gen_range(lo..hi), rng.gen_range(lo..hi));
        self.particles.push(Particle::new_with_motion(
            vec2(
                rng.gen_range(self.bounds_rect.left()..self.bounds_rect.right()),
                rng.gen_range(self.bounds_rect.bottom()..self.bounds_rect.top()),
            ),
            self.default_particle_size,
            self.default_particle_color,
            acceleration,
            velocity,
        ));
    }

    pub fn draw(&self, draw: &Draw) {
        for particle in self.particles.iter() {
            particle.draw(draw);
        }
    }

    pub fn draw_forces(&self, draw: &Draw, scale_x: f32, scale_y: f32) {
        self.draw_origin(draw);
        self.forces.wind_field.draw(draw, scale_x, scale_y);
    }

    pub fn draw_origin(&self, draw: &Draw) {
        draw.ellipse()
            .xy(self.origin)
            .w_h(10.0, 10.0)
            .color(rgba(1.0, 0.0, 1.0, 0.2));
    }
}
