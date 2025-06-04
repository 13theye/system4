// src/particle/particle_system.rs
//
//
// The Particle System of System 3

use nannou::prelude::*;
use nannou::rand::{rngs::ThreadRng, Rng};

use crate::{forces::ForceFields, particle::Particle, view::Mask};

pub struct ParticleSystem {
    // Particles
    pub particles: Vec<Particle>,
    particle_limit: usize,

    // forces
    pub forces: ForceFields,

    // mask
    pub mask: Mask,

    // Origin and bounds
    origin: Point2,
    bounds_size: Vec2,
    pub bounds_rect: Rect,
    default_particle_size: f32,
    default_particle_color: Rgba,

    // DPI scale
    dpi_scale: f32,
}

impl ParticleSystem {
    pub fn new(
        origin: Point2,
        width: f32,
        height: f32,
        default_particle_size: f32,
        default_particle_color: Rgba,
        particle_limit: u32,
        dpi_scale: f32,
    ) -> Self {
        let bounds_size = Vec2::new(width, height);
        let bounds_rect = Rect::from_x_y_w_h(origin.x, origin.y, width, height);
        let grid_cols = 96;
        let grid_rows = 54;

        let mask = Mask::full_screen();

        Self {
            particles: Vec::new(),
            forces: ForceFields::new(origin, bounds_size, grid_cols, grid_rows),
            origin,
            mask,
            bounds_size,
            bounds_rect,
            default_particle_size,
            default_particle_color,
            particle_limit: particle_limit as usize,
            dpi_scale,
        }
    }

    pub fn update(&mut self, show_forces: bool) {
        self.forces.update(show_forces);

        let mut write_inx = 0;
        for read_inx in 0..self.particles.len() {
            let particle = &mut self.particles[read_inx];
            self.forces.apply(particle);
            particle.update();

            if particle.is_offscreen(self.bounds_rect) {
                particle.kill();
            }
            if !particle.is_dead() {
                if write_inx != read_inx {
                    self.particles[write_inx] = self.particles[read_inx];
                }
                write_inx += 1;
            }
        }
        self.particles.truncate(write_inx);
        if self.particles.len() > self.particle_limit {
            self.cull_excess_particles();
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

    fn cull_excess_particles(&mut self) {
        for i in 0..self.particles.len() - self.particle_limit {
            self.particles[i].set_age_per_tick(10.0);
        }
    }

    pub fn draw(&self, draw: &Draw) {
        for particle in self.particles.iter() {
            if self.mask.contains(particle.position()) {
                particle.draw(draw, self.dpi_scale);
            }
        }
    }

    pub fn draw_forces(&self, draw: &Draw, scale_x: f32, scale_y: f32) {
        self.draw_origin(draw, scale_x, scale_y);
        self.forces.wind_field.draw(draw, scale_x, scale_y);
        for circle in self.forces.wind_circles.values() {
            circle.draw_center(draw, scale_x, scale_y);
        }
    }

    pub fn draw_origin(&self, draw: &Draw, scale_x: f32, scale_y: f32) {
        draw.ellipse()
            .xy(self.origin * vec2(scale_x, scale_y))
            .w_h(10.0 * scale_x, 10.0 * scale_y)
            .color(rgba(1.0, 0.0, 1.0, 0.2));
    }
}
