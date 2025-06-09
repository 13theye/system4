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
    utils::IdGenerator,
    view::{Mask, Voice},
};

pub struct ParticleSystem {
    // Particles
    pub particles: Vec<Particle>,
    pub particle_limit: usize,
    pub spawn_rate: f32,
    pub feedback: HashMap<Voice, f32>,

    // forces
    pub forces: ForceFields,

    // masks and emitters
    pub masks: HashMap<Voice, Mask>,
    pub emitters: Vec<Emitter>,

    // Origin and bounds
    origin: Point2,
    bounds_size: Vec2,
    pub bounds_rect: Rect,
    default_particle_size: f32,
    default_particle_color: Rgb,

    // OSC params
    pub alpha_limits: HashMap<Voice, f32>, // scale the alpha of the particles
    pub color_limits: HashMap<Voice, Rgb>,
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
            feedback: HashMap::new(),
            forces: ForceFields::new(origin, bounds_size, grid_cols, grid_rows),
            spawn_rate: 20.0,
            masks,
            emitters: Vec::new(),
            bounds_size,
            bounds_rect,
            default_particle_size,
            default_particle_color,

            alpha_limits: HashMap::new(),
            color_limits: HashMap::new(),
            particle_num_factor: 1.0,
            trail: 0.0,

            dpi_scale,
        }
    }

    /********************* Make drone ********************************** */

    // Create a drone with a mask and emitters. Return the mask's rect
    pub fn make_drone_with(
        &mut self,
        id_generator: &mut IdGenerator,
        voice: Voice,
        circle: WindCircle,
        alpha: i32,
        num_particles: i32,
        trail: i32,
    ) -> Rect {
        if self.masks.contains_key(&voice) {
            self.masks.remove(&voice);
        }

        let mask = Mask::make_drone(voice);

        let emitter_left_origin = vec2(mask.rect.left() - 20.0, mask.origin.y);
        let emitter_right_origin = vec2(mask.rect.right() + 20.0, mask.origin.y);

        let spawn_rate_factor = (num_particles as f32) / 100.0;

        // Create particle emitters
        let emitter_left = Emitter::new(
            id_generator.generate(),
            voice,
            emitter_left_origin,
            mask.rect.top_left(),
            mask.rect.bottom_left(),
            EmitDirection::East,
            self.spawn_rate,
            spawn_rate_factor,
        );

        let emitter_right = Emitter::new(
            id_generator.generate(),
            voice,
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
        let alpha_limit = (alpha as f32) / 100.0;
        self.alpha_limits.insert(voice, alpha_limit);
        self.color_limits.insert(voice, self.default_particle_color);

        self.particle_num_factor = spawn_rate_factor;
        //self.num_particles = 1.0;
        self.trail = (trail as f32) / 100.0;

        // Add the wind circle to the forces
        self.forces.add_wind_circle(circle);

        // Add the mask
        let mask_rect = mask.rect;
        self.masks.insert(voice, mask);

        // Return the mask's rect
        mask_rect
    }

    /********************* Update methods ********************************** */

    pub fn update(&mut self, rng: &mut ThreadRng, show_forces: bool) {
        // Cull excess particles
        let limit = (self.particle_limit as f32 * self.particle_num_factor) as usize;
        if self.particles.len() > limit {
            self.cull_excess_particles(limit);
        }

        self.handle_spawning(rng);

        self.forces.update(show_forces);

        let mut write_inx = 0;
        for read_inx in 0..self.particles.len() {
            let particle = &mut self.particles[read_inx];
            self.forces.apply(particle);

            let parent_voice = particle.parent_voice;
            let Some(color_limit) = self.color_limits.get(&parent_voice) else {
                return;
            };

            let Some(alpha_limit) = self.alpha_limits.get(&parent_voice) else {
                return;
            };

            particle.update(rgba_from(*color_limit, *alpha_limit));
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
                let parent_voice = emitter.parent_voice;

                let color_limit = self
                    .color_limits
                    .get(&parent_voice)
                    .copied()
                    .unwrap_or(self.default_particle_color);

                let alpha_limit = self.alpha_limits.get(&parent_voice).copied().unwrap_or(1.0);

                let particles = emitter.emit(
                    10.0,
                    self.default_particle_size,
                    rgba_from(color_limit, alpha_limit),
                    rng,
                );
                self.particles.extend(particles);
            }
        }
    }

    /********************* Particle methods ********************************** */

    fn cull_excess_particles(&mut self, limit: usize) {
        let num_particles = self.particles.len();
        for i in 0..(num_particles - limit).clamp(0, num_particles) {
            self.particles[i].set_age_per_tick(200.0);
        }
    }

    /********************* Accessor/Helper methods ********************************** */

    pub fn change_bounds_size_to(&mut self, width: f32, height: f32) {
        self.bounds_size = Vec2::new(width, height);
        self.bounds_rect = self.make_bounds_rect();
    }

    pub fn kill_voice(&mut self, voice: Voice) {
        self.emitters
            .retain(|emitter| emitter.parent_voice != voice);
        self.forces.remove_wind_by_voice(voice);
    }

    pub fn set_alpha_limit(&mut self, voice: Voice, alpha: f32) {
        self.alpha_limits.insert(voice, alpha);
    }

    pub fn set_feedback(&mut self, voice: Voice, feedback: f32) {
        self.feedback.insert(voice, feedback);
    }

    pub fn set_gravity(&mut self, voice: Voice, gravity: f32) {
        self.forces.set_circle_center_bias_by_voice(voice, gravity);
    }

    pub fn set_is_spawning(&mut self, voice: Voice, is_spawning: bool) {
        self.emitters.iter_mut().for_each(|emitter| {
            if emitter.parent_voice == voice {
                emitter.is_spawning = is_spawning;
            }
        });
    }

    pub fn set_strength(&mut self, voice: Voice, strength: f32) {
        self.forces.set_circle_strength_by_voice(voice, strength);
    }

    pub fn set_num_particles(&mut self, voice: Voice, num_particles: f32) {
        self.emitters.iter_mut().for_each(|emitter| {
            if emitter.parent_voice == voice {
                emitter.spawn_rate_factor = num_particles;
            }
        });
    }

    pub fn set_radius_inner(&mut self, voice: Voice, val: f32) {
        let Some(mask) = self.masks.get(&voice) else {
            println!("Can't set inner radius:No mask found for voice: {}", voice);
            return;
        };

        // The maximum outer radius is half the largest side of the mask
        let max_radius = (mask.size.x.max(mask.size.y) + 50.0) / 2.0;
        let radius = max_radius * val;

        self.forces.set_circle_outer_radius_by_voice(voice, radius);
    }

    pub fn set_radius_outer(&mut self, voice: Voice, val: f32) {
        self.forces.set_circle_outer_radius_by_voice(voice, val);
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
            let Some(mask) = self.masks.get(&particle.parent_voice) else {
                continue;
            };

            let feedback = self.feedback.get(&particle.parent_voice).unwrap_or(&0.0);

            if mask.contains(particle.position()) {
                particle.draw(draw, *feedback, self.dpi_scale);
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
