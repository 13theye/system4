/// src/particle/particle_system.rs
///
///
/// The Particle System of System 4
use std::collections::HashMap;

use nannou::prelude::*;
use nannou::rand::{rngs::ThreadRng, seq::SliceRandom};
use nnpipe::renderers::{ParticleGpu, SegmentGpu};
use rayon::prelude::*;

use crate::{
    forces::{ForceFields, WindCircle},
    particle::{EmitDirection, Emitter, LinearEmitter, Particle, PointEmitter},
    utils::IdGenerator,
    view::{Mask, Voice},
};

pub struct ParticleSystem {
    // Particles
    pub particles: HashMap<Voice, Vec<Particle>>,

    // forces
    pub forces: ForceFields,

    // masks and emitters
    pub masks: HashMap<Voice, Mask>,
    pub emitters: Vec<Box<dyn Emitter>>,

    // Global params
    pub default_particle_limit: usize,
    pub particle_limits: HashMap<Voice, usize>,
    pub feedback: HashMap<Voice, f32>,
    pub global_max_spawn_rate: f32,

    // Origin and bounds
    origin: Point2,
    bounds_size: Vec2,
    pub bounds_rect: Rect,
    default_particle_size: f32,
    default_particle_color: Rgb,

    // OSC params
    pub alpha_limits: HashMap<Voice, f32>, // scale the alpha of the particles
    pub color_limits: HashMap<Voice, Rgb>,
    pub particle_num_factors: HashMap<Voice, f32>, // normalized proportion of particle_limit
    pub trail: f32,                                // scale the trail of the particles

    // DPI scale
    dpi_scale: f32,

    // Reusable GPU particle buffer to avoid allocations
    gpu_particle_buffer: Vec<ParticleGpu>,
    gpu_segment_buffer: Vec<SegmentGpu>,
}

impl ParticleSystem {
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
        let grid_cols = (width / 4.0) as usize;
        let grid_rows = (height / 4.0) as usize;

        // pre-populate the first mask
        let masks = HashMap::new();

        Self {
            origin,
            particles: HashMap::new(),
            particle_limits: HashMap::new(),
            feedback: HashMap::new(),
            forces: ForceFields::new(origin, bounds_size, grid_cols, grid_rows),
            global_max_spawn_rate: 40.0,
            masks,
            emitters: Vec::new(),
            bounds_size,
            bounds_rect,
            default_particle_size,
            default_particle_color,
            default_particle_limit: default_particle_limit as usize,

            alpha_limits: HashMap::new(),
            color_limits: HashMap::new(),
            particle_num_factors: HashMap::new(),
            trail: 0.0,

            dpi_scale,
            gpu_particle_buffer: Vec::new(),
            gpu_segment_buffer: Vec::new(),
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

    /// Emit particles, update forces, update particles, and cull particles - returns simple particles and accompanying trails
    pub fn update(
        &mut self,
        rng: &mut ThreadRng,
        show_forces: bool,
    ) -> (&[ParticleGpu], &[SegmentGpu]) {
        self.handle_particle_emission(rng);
        self.cull_excess_particles();

        self.forces.update(show_forces);

        // Reuse existing buffer to avoid allocations
        self.gpu_particle_buffer.clear();
        self.gpu_segment_buffer.clear();

        for (voice, particles) in self.particles.iter_mut() {
            let color_limit = self.color_limits.get(voice).copied();
            let alpha_limit = self.alpha_limits.get(voice).copied();

            // Skip if no limits for this voice
            if color_limit.is_none() || alpha_limit.is_none() {
                continue;
            }

            let (color_limit, alpha_limit) = (color_limit.unwrap(), alpha_limit.unwrap());

            // Parallel update, collect simple particle data and segments
            let (gpu_particle_group, gpu_segment_group): (Vec<ParticleGpu>, Vec<SegmentGpu>) =
                particles
                    .par_iter_mut()
                    .map(|particle| {
                        self.forces.apply_forces_to_particle(particle);

                        particle.update(color_limit, alpha_limit);

                        if particle.is_out_of_bounds(self.bounds_rect) {
                            particle.kill();
                        }

                        (particle.to_gpu(), particle.to_segment_gpu())
                    })
                    .unzip();

            self.gpu_particle_buffer.extend(gpu_particle_group);
            self.gpu_segment_buffer.extend(gpu_segment_group);

            // Cull dead particles
            particles.retain(|particle| particle.is_alive());
        }

        (&self.gpu_particle_buffer, &self.gpu_segment_buffer)
    }

    pub fn handle_particle_emission(&mut self, rng: &mut ThreadRng) {
        let mut indices: Vec<usize> = (0..self.emitters.len()).collect();
        indices.shuffle(rng);

        let mut parent_voice: Voice;
        let mut particle_limit: usize;
        let mut color_limit: Rgb;

        for i in indices {
            let emitter = &self.emitters[i];
            if emitter.is_spawning() {
                parent_voice = emitter.parent_voice();
                let particle_vec = self.particles.entry(parent_voice).or_default();
                particle_limit = self
                    .particle_limits
                    .get(&parent_voice)
                    .copied()
                    .unwrap_or(self.default_particle_limit);

                // Don't add new particles if limit is reached
                if particle_vec.len() > particle_limit {
                    return;
                }

                color_limit = self
                    .color_limits
                    .get(&parent_voice)
                    .copied()
                    .unwrap_or(self.default_particle_color);

                particle_vec.extend(emitter.emit(
                    10.0,
                    self.default_particle_size,
                    rgba_from(color_limit, 0.0),
                    rng,
                ));
            }
        }
    }

    /********************* Particle methods ********************************** */

    fn cull_excess_particles(&mut self) {
        for (voice, particles) in self.particles.iter_mut() {
            let limit = self
                .particle_limits
                .get(voice)
                .unwrap_or(&self.default_particle_limit);

            // Partition particles into active and fading groups
            let active_particles: Vec<_> = particles
                .iter_mut()
                .filter(|p| p.remaining_life_span > p.fade_out_duration())
                .collect();

            let num_active_particles = active_particles.len();

            if num_active_particles > *limit {
                let excess_active = num_active_particles - limit;

                for particle in active_particles
                    .into_iter()
                    .rev() // kill oldest particles first
                    .take(excess_active)
                {
                    particle.set_to_fade_out();
                }
            }
        }
    }

    /********************* Accessor/Helper methods ********************************** */

    pub fn change_bounds_size_to(&mut self, width: f32, height: f32) {
        self.bounds_size = Vec2::new(width, height);
        self.bounds_rect = self.make_bounds_rect();
    }

    pub fn get_particle_count(&self) -> usize {
        self.particles
            .values()
            .map(|particles| particles.len())
            .sum()
    }

    pub fn kill_voice(&mut self, voice: &Voice) {
        self.emitters
            .retain(|emitter| emitter.parent_voice() != *voice);
        self.forces.remove_wind_by_voice(voice);
    }

    pub fn set_alpha_limit(&mut self, voice: &Voice, alpha: f32) {
        self.alpha_limits.insert(*voice, alpha);
    }

    pub fn set_feedback(&mut self, voice: &Voice, feedback: f32) {
        self.feedback.insert(*voice, feedback);
    }

    pub fn set_gravity(&mut self, voice: &Voice, gravity: f32) {
        self.forces.set_circle_center_bias_by_voice(voice, gravity);
    }

    pub fn set_is_spawning(&mut self, voice: &Voice, is_spawning: bool) {
        self.emitters.iter_mut().for_each(|emitter| {
            if emitter.parent_voice() == *voice {
                emitter.set_is_spawning(is_spawning);
            }
        });
    }

    pub fn set_strength(&mut self, voice: &Voice, strength: f32) {
        self.forces.set_circle_strength_by_voice(voice, strength);
    }

    pub fn set_num_particles(&mut self, voice: &Voice, num_particles: f32) {
        let limit = (self.default_particle_limit as f32 * num_particles) as usize;
        self.particle_limits.insert(*voice, limit);

        let spawn_rate_factor = self
            .particle_num_factors
            .insert(*voice, num_particles)
            .unwrap_or(0.5);
        self.emitters.iter_mut().for_each(|emitter| {
            if emitter.parent_voice() == *voice {
                emitter.set_spawn_rate_factor(spawn_rate_factor);
            }
        });
    }

    pub fn set_radius_outer(&mut self, voice: &Voice, val: f32) {
        let Some(mask) = self.masks.get(voice) else {
            println!("Can't set inner radius: No mask found for voice: {}", voice);
            return;
        };

        // The maximum outer radius is half the largest side of the mask
        let max_radius = (mask.size.x.max(mask.size.y) + 50.0) / 2.0;
        let radius = max_radius * val;

        self.forces.set_circle_outer_radius_by_voice(voice, radius);
    }

    pub fn set_radius_inner(&mut self, voice: &Voice, val: f32) {
        self.forces.set_circle_inner_radius_by_voice(voice, val);
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

    /// In this draw mode, particles are only drawn if they are within the bounds of the mask
    /// associated with the emitter that spawned them.
    pub fn draw(&self, draw: &Draw) {
        for (voice, particles) in self.particles.iter() {
            for particle in particles.iter() {
                let Some(mask) = self.masks.get(voice) else {
                    continue;
                };

                let feedback = self.feedback.get(voice).unwrap_or(&0.0);

                if mask.contains(particle.position()) {
                    particle.draw(draw, *feedback, self.dpi_scale);
                }
            }
        }
    }

    /// Draw the forces and emitters
    pub fn draw_forces(&self, draw: &Draw, scale_x: f32, scale_y: f32) {
        self.draw_origin(draw, scale_x, scale_y);
        self.forces.wind_field.draw(draw, scale_x, scale_y);
        self.draw_emitters(draw, scale_x, scale_y);
        for circle in self.forces.wind_circles.values() {
            circle.draw_center(draw, scale_x, scale_y);
        }
    }

    /// Draw the origin
    pub fn draw_origin(&self, draw: &Draw, scale_x: f32, scale_y: f32) {
        draw.ellipse()
            .xy(self.origin * vec2(scale_x, scale_y))
            .w_h(10.0 * scale_x, 10.0 * scale_y)
            .color(rgba(1.0, 0.0, 1.0, 0.2));
    }

    /// Draw the emitters
    pub fn draw_emitters(&self, draw: &Draw, scale_x: f32, scale_y: f32) {
        for emitter in self.emitters.iter() {
            emitter.draw(draw, scale_x, scale_y);
        }
    }
}

/// Helper to make a Rgba from an Rgb and alpha value
fn rgba_from(rgb: Rgb, alpha: f32) -> Rgba {
    rgba(rgb.red, rgb.green, rgb.blue, alpha)
}
