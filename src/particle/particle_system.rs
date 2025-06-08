// src/particle/particle_system.rs
//
//
// The Particle System of System 3

use std::collections::HashMap;

use nannou::prelude::*;
use nannou::rand::rngs::ThreadRng;

use crate::{
    forces::{ForceFields, WindCircle},
    particle::{EmitDirection, Emitter, Particle},
    view::Mask,
};

pub struct ParticleSystem {
    // Particles
    pub particles: Vec<Particle>,
    pub particle_limit: usize,
    pub spawn_rate: f32,

    // forces
    pub forces: ForceFields,

    // masks and emitters
    pub masks: HashMap<usize, Mask>,
    pub emitters: Vec<Emitter>,

    // Origin and bounds
    origin: Point2,
    bounds_size: Vec2,
    pub bounds_rect: Rect,
    default_particle_size: f32,
    default_particle_color: Rgb,

    // OSC params
    pub alpha: f32,               // scale the alpha of the particles
    pub particle_num_factor: f32, // normalized proportion of particle_limit
    pub trail: f32,               // scale the trail of the particles

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
        let grid_cols = (width / 30.0) as usize;
        let grid_rows = (height / 30.0) as usize;

        // pre-populate the first mask
        let masks = HashMap::new();

        Self {
            origin,
            particles: Vec::new(),
            particle_limit: particle_limit as usize,
            forces: ForceFields::new(origin, bounds_size, grid_cols, grid_rows),
            spawn_rate: 20.0,
            masks,
            emitters: Vec::new(),
            bounds_size,
            bounds_rect,
            default_particle_size,
            default_particle_color,

            alpha: 0.0,
            particle_num_factor: 1.0,
            trail: 0.0,

            dpi_scale,
        }
    }

    /********************* Make drone ********************************** */

    // Create a drone with a mask and emitters. Return the mask's rect
    pub fn make_drone_with(
        &mut self,
        id: usize,
        circle: WindCircle,
        alpha: i32,
        num_particles: i32,
        trail: i32,
    ) -> Rect {
        if self.masks.contains_key(&id) {
            self.masks.remove(&id);
        }

        let mask = Mask::make_drone(id);

        let emitter_left_origin = vec2(mask.rect.left() - 20.0, mask.origin.y);
        let emitter_right_origin = vec2(mask.rect.right() + 20.0, mask.origin.y);

        let spawn_rate_factor = (num_particles as f32) / 100.0;

        // Create particle emitters
        let emitter_left = Emitter::new(
            circle.id,
            emitter_left_origin,
            mask.rect.top_left(),
            mask.rect.bottom_left(),
            EmitDirection::East,
            self.spawn_rate,
            spawn_rate_factor,
        );

        let emitter_right = Emitter::new(
            circle.id,
            emitter_right_origin,
            mask.rect.top_right(),
            mask.rect.bottom_right(),
            EmitDirection::West,
            self.spawn_rate,
            spawn_rate_factor,
        );

        // Add the emitters
        self.emitters.push(emitter_left);
        self.emitters.push(emitter_right);

        // Set the particle system params
        self.alpha = (alpha as f32) / 100.0;
        self.particle_num_factor = spawn_rate_factor;
        //self.num_particles = 1.0;
        self.trail = (trail as f32) / 100.0;

        // Add the wind circle to the forces
        self.forces.add_wind_circle(circle);

        // Add the mask
        let mask_rect = mask.rect;
        self.masks.insert(id, mask);

        // Return the mask's rect
        mask_rect
    }

    /********************* Update methods ********************************** */

    pub fn update(&mut self, rng: &mut ThreadRng, show_forces: bool) {
        // Cull excess particles
        let limit = (self.particle_limit as f32 * self.particle_num_factor) as usize;
        if self.particles.len() > limit {
            println!("Particles: {}", self.particles.len());
            println!("Max Particle limit: {}", self.particle_limit);
            println!("Particle num factor: {}", self.particle_num_factor);
            println!("Current particle limit: {}", limit);
            self.cull_excess_particles(limit);
        }

        self.handle_spawning(rng);

        self.forces.update(show_forces);

        let mut write_inx = 0;
        for read_inx in 0..self.particles.len() {
            let particle = &mut self.particles[read_inx];
            self.forces.apply(particle);
            particle.update();
            if particle.is_out_of_bounds(self.bounds_rect) {
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
    }

    pub fn handle_spawning(&mut self, rng: &mut ThreadRng) {
        for emitter in self.emitters.iter() {
            if emitter.is_spawning {
                let particles = emitter.emit(
                    10.0,
                    self.default_particle_size,
                    rgba_from(self.default_particle_color, self.alpha),
                    rng,
                );
                self.particles.extend(particles);
            }
        }
    }
    /*
    pub fn spawn_old(&mut self, rng: &mut ThreadRng) {
        let rect = self.bounds_rect;
        let spawn_speed = self.spawn_rate;

        let spawn_rate_sides = (20.0 * self.num_particles) as usize;

        // Spawn particles from the left
        for _ in 0..spawn_rate_sides {
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
        for _ in 0..spawn_rate_sides {
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
     */

    /********************* Particle methods ********************************** */
    /*
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
     */

    fn cull_excess_particles(&mut self, limit: usize) {
        let num_particles = self.particles.len();
        for i in 0..(num_particles - limit).clamp(0, num_particles) {
            self.particles[i].set_age_per_tick(100.0);
        }
    }

    /********************* Accessor/Helper methods ********************************** */

    pub fn change_bounds_size_to(&mut self, width: f32, height: f32) {
        self.bounds_size = Vec2::new(width, height);
        self.bounds_rect = self.make_bounds_rect();
    }

    pub fn set_alpha(&mut self, alpha: f32) {
        self.alpha = alpha;
    }

    pub fn set_circle_volume(&mut self, id: i32, alpha: f32) {
        let Some(params) = self.forces.get_circle_params(id as usize) else {
            return;
        };

        let radius = params.radius;
        let new_width = radius * alpha + 100.0;

        self.forces.set_circle_dims(id as usize, radius, new_width);
    }

    pub fn set_gravity(&mut self, id: usize, gravity: f32) {
        self.forces.update_wind_circle_center_bias(id, gravity);
    }

    pub fn set_is_spawning(&mut self, id: usize, is_spawning: bool) {
        self.emitters.iter_mut().for_each(|emitter| {
            if emitter.id == id {
                emitter.is_spawning = is_spawning;
            }
        });
    }

    pub fn set_strength(&mut self, id: usize, strength: f32) {
        self.forces.update_wind_circle_strength(id, strength);
    }

    pub fn set_num_particles(&mut self, id: i32, num_particles: f32) {
        self.particle_num_factor = num_particles;
        self.emitters.iter_mut().for_each(|emitter| {
            if emitter.id == id as usize {
                emitter.spawn_rate_factor = num_particles;
            }
        });
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

    // In this draw mode, particles are only drawn if they are within the bounds of the mask
    // associated with the emitter that spawned them.
    pub fn draw(&self, draw: &Draw) {
        for particle in self.particles.iter() {
            let Some(mask) = self.masks.get(&particle.parent_id) else {
                continue;
            };

            if mask.contains(particle.position()) {
                particle.draw(draw, self.dpi_scale);
            }
        }
    }

    pub fn draw_forces(&self, draw: &Draw, scale_x: f32, scale_y: f32) {
        self.draw_origin(draw, scale_x, scale_y);
        self.forces.wind_field.draw(draw, scale_x, scale_y);
        self.draw_emitters(draw, scale_x, scale_y);
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

    pub fn draw_emitters(&self, draw: &Draw, scale_x: f32, scale_y: f32) {
        for emitter in self.emitters.iter() {
            emitter.draw(draw, scale_x, scale_y);
        }
    }
}

// helper to make a Rgba from an Rgb and alpha value
fn rgba_from(rgb: Rgb, alpha: f32) -> Rgba {
    rgba(rgb.red, rgb.green, rgb.blue, alpha)
}
