// src/particle/particle_system.rs
//
//
// The Particle System of System 3

use nannou::prelude::*;
use nannou::rand::{rngs::ThreadRng, Rng};

use crate::{
    forces::{ForceFields, WindCircle},
    particle::Particle,
    view::Mask,
};

pub struct ParticleSystem {
    // Particles
    pub particles: Vec<Particle>,
    pub particle_limit: usize,
    pub is_spawning: bool,
    pub spawn_speed: f32,

    // forces
    pub forces: ForceFields,

    // mask
    pub mask: Mask,

    // Origin and bounds
    origin: Point2,
    bounds_size: Vec2,
    pub bounds_rect: Rect,
    default_particle_size: f32,
    default_particle_color: Rgb,

    // OSC params
    pub alpha: f32,         // scale the alpha of the particles
    pub num_particles: i32, // normalized proportion of particle_limit
    pub shake: f32,         // scale the shake of the particles
    pub trail: f32,         // scale the trail of the particles

    // DPI scale
    dpi_scale: f32,
}

impl ParticleSystem {
    pub fn new(
        origin: Point2,
        width: f32,
        height: f32,
        default_particle_size: f32,
        default_particle_color: Rgb,
        particle_limit: u32,
        dpi_scale: f32,
    ) -> Self {
        let bounds_size = Vec2::new(width, height);
        let bounds_rect = Rect::from_x_y_w_h(origin.x, origin.y, width, height);
        let grid_cols = 3840;
        let grid_rows = 2160;

        let mask = Mask::make_drone_1(1);

        Self {
            origin,
            particles: Vec::new(),
            particle_limit: particle_limit as usize,
            forces: ForceFields::new(origin, bounds_size, grid_cols, grid_rows),
            is_spawning: false,
            spawn_speed: 10.0,
            mask,
            bounds_size,
            bounds_rect,
            default_particle_size,
            default_particle_color,

            alpha: 0.0,
            num_particles: 1000,
            shake: 0.0,
            trail: 0.0,

            dpi_scale,
        }
    }

    /********************* Make drone ********************************** */
    pub fn make_drone_with(
        &mut self,
        circle: WindCircle,
        alpha: f32,
        num_particles: i32,
        shake: f32,
        trail: f32,
    ) {
        self.alpha = alpha;
        self.num_particles = num_particles;
        self.shake = shake;
        self.trail = trail;
        self.is_spawning = true;
        self.forces.add_wind_circle(circle);
    }

    /********************* Update methods ********************************** */

    pub fn update(&mut self, rng: &mut ThreadRng, show_forces: bool) {
        if self.is_spawning {
            self.spawn(rng);
        }

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

    pub fn spawn(&mut self, rng: &mut ThreadRng) {
        let rect = self.bounds_rect;
        let spawn_speed = self.spawn_speed;

        // Spawn particles from the left
        for _ in 0..20 {
            let y = rng.gen_range(-1080.0..1080.0);
            /*
            if y > -50.0 && y < 50.0 {
                continue;
            }
             */
            let position: Vec2 = if y > 0.0 {
                vec2(rect.left() - 10.0, y)
            } else {
                vec2(rect.right() + 10.0, y)
            };
            let velocity = if y > 0.0 {
                vec2(spawn_speed, 0.0)
            } else {
                vec2(-spawn_speed, 0.0)
            };

            self.add_particle_with_velocity(position, velocity);
        }

        // Spawn particles from the right
        for _ in 0..35 {
            let y = rng.gen_range(-1080.0..1080.0);
            /*
            if y > -50.0 && y < 50.0 {
                continue;
            }
            */
            let position: Vec2 = if y > 0.0 {
                vec2(rect.right() + 10.0, y)
            } else {
                vec2(rect.left() - 10.0, y)
            };
            let velocity = if y > 0.0 {
                vec2(-spawn_speed, 0.0)
            } else {
                vec2(spawn_speed, 0.0)
            };

            self.add_particle_with_velocity(position, velocity);
        }

        /*
        // Spawn particles from the top
        for _ in 0..35 {
            let x = rng.gen_range(-1920.0..1920.0);
            /*
            if x > -50.0 && x < 50.0 {
                continue;
            }
             */
            let position: Vec2 = if x > 0.0 {
                vec2(x, rect.top() + 10.0)
            } else {
                vec2(x, rect.bottom() - 10.0)
            };
            let velocity = if x > 0.0 {
                vec2(0.0, -spawn_speed)
            } else {
                vec2(0.0, spawn_speed)
            };

            self.add_particle_with_velocity(position, velocity);
        }


        // Spawn particles from the bottom
        for _ in 0..35 {
            let x = rng.gen_range(-1920.0..1920.0);
            /*
            if x > -50.0 && x < 50.0 {
                continue;
            }
            */
            let position: Vec2 = if x > 0.0 {
                vec2(x, rect.bottom() - 10.0)
            } else {
                vec2(x, rect.top() + 10.0)
            };
            let velocity = if x > 0.0 {
                vec2(0.0, spawn_speed)
            } else {
                vec2(0.0, -spawn_speed)
            };

            self.add_particle_with_velocity(position, velocity);
        }
        */
    }

    /********************* Particle methods ********************************** */

    pub fn add_particle(&mut self, position: Vec2) {
        self.particles.push(Particle::new(
            position,
            self.default_particle_size,
            rgba_from(self.default_particle_color, self.alpha),
        ));
    }

    pub fn add_particle_with_velocity(&mut self, position: Vec2, velocity: Vec2) {
        let acceleration = vec2(0.0, 0.0);

        self.particles.push(Particle::new_with_motion(
            position,
            self.default_particle_size,
            rgba_from(self.default_particle_color, self.alpha),
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
            rgba_from(self.default_particle_color, self.alpha),
            acceleration,
            velocity,
        ));
    }

    fn cull_excess_particles(&mut self) {
        for i in 0..self.particles.len() - self.particle_limit {
            self.particles[i].set_age_per_tick(10.0);
        }
    }

    /********************* Accessor/Helper methods ********************************** */

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

    /********************* Draw methods ********************************** */

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

// helper to make a Rgba from an Rgb and alpha value
fn rgba_from(rgb: Rgb, alpha: f32) -> Rgba {
    rgba(rgb.red, rgb.green, rgb.blue, alpha)
}
