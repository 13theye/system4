/// particle_system_new.rs
///
/// A new particle system that uses the new Particles struct-of-arrays
///
use std::collections::HashMap;

use nannou::prelude::*;
use nannou::rand::{rngs::ThreadRng, seq::SliceRandom};
use rayon::prelude::*;
use serde::de::value::UsizeDeserializer;

use crate::{
    forces::{ForceFields, WindCircle},
    particle::{EmitDirection, Emitter, LinearEmitter, Particles, PointEmitter},
    utils::IdGenerator,
    view::{Mask, Voice},
};

pub struct ParticleSystemNew {
    // Particles
    pub particles: Particles,

    // forces
    pub forces: ForceFields,

    // masks and emitters
    pub masks: HashMap<Voice, Mask>,
    pub emitters: Vec<Box<dyn Emitter>>,

    // Origin and bounds
    origin: Point2,
    bounds_size: Vec2,
    pub bounds_rect: Rect,

    // Global Params
    pub per_voice_particle_limit: usize,
    pub feedback_settings: HashMap<Voice, f32>,
    pub global_max_spawn_rate: f32,

    // Default particle params
    default_particle_size: f32,
    default_particle_color: Rgb,

    // Params set via OSC
    pub alpha_limits: HashMap<Voice, f32>, // scale the alpha of the particles
    pub color_limits: HashMap<Voice, Rgb>,
    pub particle_limits: HashMap<Voice, usize>,
    pub particle_num_factors: HashMap<Voice, f32>, // normalized proportion of particle_limit
    pub trail: f32,

    // DPI scale
    dpi_scale: f32, // scale the trail of the particles
}

impl ParticleSystemNew {
    pub fn new(
        origin: Point2,
        width: f32,
        height: f32,
        default_particle_size: f32,
        default_particle_color: Rgb,
        default_particle_limit: u32,
        dpi_scale: f32,
    ) -> Self {
        let bounds_size = Vec2::new(width, height);
        let bounds_rect = Rect::from_x_y_w_h(origin.x, origin.y, width, height);
        let grid_cols = (width / 30.0) as usize;
        let grid_rows = (height / 30.0) as usize;

        // pre-populate the first mask
        let masks = HashMap::new();

        Self {
            particles: Particles::new(),
            forces: ForceFields::new(origin, bounds_size, grid_cols, grid_rows),

            masks,
            emitters: Vec::new(),

            origin,
            bounds_size,
            bounds_rect,

            per_voice_particle_limit: default_particle_limit as usize,
            feedback_settings: HashMap::new(),
            global_max_spawn_rate: 40.0,

            default_particle_size,
            default_particle_color,

            alpha_limits: HashMap::new(),
            color_limits: HashMap::new(),
            particle_limits: HashMap::new(),
            particle_num_factors: HashMap::new(),
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

        let max_particle_percentage = (num_particles as f32) / 100.0;
        let spawn_rate_factor = 1.0;

        // Create particle emitters
        let emitter_left = LinearEmitter::new(
            id_generator.generate(),
            voice,
            emitter_left_origin,
            mask.rect.top_left(),
            mask.rect.bottom_left(),
            EmitDirection::East,
            self.global_max_spawn_rate,
            spawn_rate_factor,
        );

        let emitter_right = LinearEmitter::new(
            id_generator.generate(),
            voice,
            emitter_right_origin,
            mask.rect.top_right(),
            mask.rect.bottom_right(),
            EmitDirection::West,
            self.global_max_spawn_rate,
            spawn_rate_factor,
        );

        let emitter_center = PointEmitter::new(
            id_generator.generate(),
            voice,
            mask.origin,
            self.global_max_spawn_rate,
            spawn_rate_factor,
        );

        // Add the emitters
        //self.emitters.push(Box::new(emitter_left)); //emitter_left);
        //self.emitters.push(Box::new(emitter_right));
        self.emitters.push(Box::new(emitter_center));

        // Set the particle system params
        let alpha_limit = (alpha as f32) / 100.0;
        self.alpha_limits.insert(voice, alpha_limit);
        self.color_limits.insert(voice, self.default_particle_color);

        self.particle_num_factors
            .insert(voice, max_particle_percentage);
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
        //self.handle_particle_emission(rng);
        //self.cull_excess_particles();

        self.forces.update(show_forces);

        /*
        for (voice, particles) in self.particles.iter_mut() {
            let mut write_inx = 0;
            for read_inx in 0..particles.len() {
                let particle = &mut particles[read_inx];s
                self.forces.apply_forces_to_particle(particle);

                let Some(color_limit) = self.color_limits.get(voice) else {
                    return live_positions;
                };

                let Some(alpha_limit) = self.alpha_limits.get(voice) else {
                    return live_positions;
                };

                particle.update(*color_limit, *alpha_limit);
                if particle.is_out_of_bounds(self.bounds_rect) {
                    particle.kill();
                }

                if !particle.is_dead() {
                    if particle.is_alive() {
                        live_positions.push(particle.position());
                    }
                    if write_inx != read_inx {
                        particles[write_inx] = particles[read_inx];
                    }
                    write_inx += 1;
                }
            }
            particles.truncate(write_inx);
        }
         */
    }
}
