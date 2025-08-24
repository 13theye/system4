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

    /// Emit particles, update forces, update particles, and cull particles
    pub fn _update(&mut self, rng: &mut ThreadRng, show_forces: bool) -> &[ParticleGpu] {
        self.handle_particle_emission(rng);
        self.cull_excess_particles();

        self.forces.update(show_forces);

        // Reuse existing buffer to avoid allocations
        self.gpu_particle_buffer.clear();

        for (voice, particles) in self.particles.iter_mut() {
            let color_limit = self.color_limits.get(voice).copied();
            let alpha_limit = self.alpha_limits.get(voice).copied();

            // Skip if no limits for this voice
            if color_limit.is_none() || alpha_limit.is_none() {
                continue;
            }

            let (color_limit, alpha_limit) = (color_limit.unwrap(), alpha_limit.unwrap());

            // Parallel update, collect positions
            let gpu_particle_group: Vec<ParticleGpu> = particles
                .par_iter_mut()
                .map(|particle| {
                    self.forces.apply_forces_to_particle(particle);

                    particle.update(color_limit, alpha_limit);

                    if particle.is_out_of_bounds(self.bounds_rect) {
                        particle.kill();
                    }

                    particle.to_gpu()
                })
                .collect();

            self.gpu_particle_buffer.extend(gpu_particle_group);

            // Cull dead particles
            particles.retain(|particle| particle.is_alive());
        }

        &self.gpu_particle_buffer
    }

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

    /********************* Particle data extraction ********************************** */

    /// Extract all live particle positions for GPU processing
    pub fn get_live_particle_positions(&self) -> Vec<Vec2> {
        self.particles
            .values()
            .flat_map(|particles| particles.iter())
            .filter(|p| p.is_alive())
            .map(|p| p.position())
            .collect()
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

    /// Draw smooth heatmap background based on particle density
    pub fn draw_heatmap_background(&self, draw: &Draw, bounds: Rect) {
        let grid_size = 256; // Higher resolution for finer detail
        let mut heatmap = vec![vec![0.0f32; grid_size]; grid_size];
        let blur_radius = 10.0; // Blur radius for smoothing

        // Count particles in each grid cell with Gaussian influence
        for particles in self.particles.values() {
            for particle in particles.iter() {
                if !particle.is_alive() {
                    continue;
                }

                let pos = particle.position();

                // Convert world coordinates to grid coordinates
                let x_norm = (pos.x - bounds.left()) / bounds.w();
                let y_norm = (pos.y - bounds.bottom()) / bounds.h();

                if (0.0..=1.0).contains(&x_norm) && (0.0..=1.0).contains(&y_norm) {
                    let grid_x_f = x_norm * grid_size as f32;
                    let grid_y_f = y_norm * grid_size as f32;

                    // Apply Gaussian influence to nearby cells
                    let min_x = ((grid_x_f - blur_radius) as i32).max(0) as usize;
                    let max_x =
                        ((grid_x_f + blur_radius) as i32).min(grid_size as i32 - 1) as usize;
                    let min_y = ((grid_y_f - blur_radius) as i32).max(0) as usize;
                    let max_y =
                        ((grid_y_f + blur_radius) as i32).min(grid_size as i32 - 1) as usize;

                    for gy in min_y..=max_y {
                        for gx in min_x..=max_x {
                            let dx = gx as f32 - grid_x_f;
                            let dy = gy as f32 - grid_y_f;
                            let distance_sq = dx * dx + dy * dy;

                            // Gaussian falloff
                            let sigma = blur_radius / 3.0;
                            let weight = (-distance_sq / (2.0 * sigma * sigma)).exp();

                            heatmap[gy][gx] += weight;
                        }
                    }
                }
            }
        }

        // Find max density for normalization
        let max_density = heatmap
            .iter()
            .flat_map(|row| row.iter())
            .fold(0.0f32, |max, &val| max.max(val));

        if max_density == 0.0 {
            return;
        }

        let cell_width = bounds.w() / grid_size as f32;
        let cell_height = bounds.h() / grid_size as f32;

        // Draw each grid cell with smooth heatmap color
        for (y, row) in heatmap.iter().enumerate() {
            for (x, &density) in row.iter().enumerate() {
                if density < 0.01 {
                    continue; // Skip very low density cells
                }

                // Normalize density to 0-1 range
                let intensity = (density / max_density).min(1.0);

                // Create 4-color gradient: soft blue -> lavender -> soft pink -> bright pink -> amber
                let color = if intensity < 0.25 {
                    // Soft blue to lavender
                    let t = intensity / 0.25;
                    let soft_blue = vec3(0.7, 0.8, 1.0);
                    let lavender = vec3(0.85, 0.75, 1.0);
                    let rgb = soft_blue + (lavender - soft_blue) * t;
                    rgba(rgb.x, rgb.y, rgb.z, 0.6 + intensity * 0.3)
                } else if intensity < 0.5 {
                    // Lavender to soft pink
                    let t = (intensity - 0.25) / 0.25;
                    let lavender = vec3(0.85, 0.75, 1.0);
                    let soft_pink = vec3(1.0, 0.8, 0.9);
                    let rgb = lavender + (soft_pink - lavender) * t;
                    rgba(rgb.x, rgb.y, rgb.z, 0.7 + intensity * 0.2)
                } else if intensity < 0.75 {
                    // Soft pink to bright pink
                    let t = (intensity - 0.5) / 0.25;
                    let soft_pink = vec3(1.0, 0.8, 0.9);
                    let bright_pink = vec3(1.0, 0.2, 0.6);
                    let rgb = soft_pink + (bright_pink - soft_pink) * t;
                    rgba(rgb.x, rgb.y, rgb.z, 0.8 + intensity * 0.1)
                } else {
                    // Bright pink to amber
                    let t = (intensity - 0.75) / 0.25;
                    let bright_pink = vec3(1.0, 0.2, 0.6);
                    let amber = vec3(1.0, 0.75, 0.0);
                    let rgb = bright_pink + (amber - bright_pink) * t;
                    rgba(rgb.x, rgb.y, rgb.z, 0.9)
                };

                // Calculate cell position
                let cell_x = bounds.left() + (x as f32 + 0.5) * cell_width;
                let cell_y = bounds.bottom() + (y as f32 + 0.5) * cell_height;

                // Draw the cell - make cells slightly larger to avoid gaps
                draw.rect()
                    .xy(vec2(cell_x, cell_y))
                    .w_h(cell_width * 1.1, cell_height * 1.1)
                    .color(color);
            }
        }
    }

    /// Draw analytical per-pixel heatmap for maximum smoothness with parallel processing
    pub fn draw_analytical_heatmap_background(&self, draw: &Draw, bounds: Rect) {
        let resolution_scale = 0.2; // Increased resolution since we have parallel processing
        let pixel_width = bounds.w() * resolution_scale;
        let pixel_height = bounds.h() * resolution_scale;
        let pixels_x = pixel_width as usize;
        let pixels_y = pixel_height as usize;

        let max_influence_radius = 80.0; // Maximum distance a particle can influence
        let max_influence_radius_sq = max_influence_radius * max_influence_radius;

        // Collect all live particles for faster iteration
        let live_particles: Vec<_> = self
            .particles
            .values()
            .flat_map(|particles| particles.iter())
            .filter(|p| p.is_alive())
            .map(|p| p.position()) // Only store positions for thread safety
            .collect();

        if live_particles.is_empty() {
            return;
        }

        let pixel_size_x = bounds.w() / pixels_x as f32;
        let pixel_size_y = bounds.h() / pixels_y as f32;

        // Generate all pixel coordinates
        let pixel_coords: Vec<(usize, usize)> = (0..pixels_y)
            .flat_map(|y| (0..pixels_x).map(move |x| (x, y)))
            .collect();

        // Process pixels in parallel
        let results: Vec<(Vec2, Rgba)> = pixel_coords
            .par_iter()
            .filter_map(|&(x, y)| {
                let pixel_world_x = bounds.left() + (x as f32 + 0.5) * pixel_size_x;
                let pixel_world_y = bounds.bottom() + (y as f32 + 0.5) * pixel_size_y;
                let pixel_pos = vec2(pixel_world_x, pixel_world_y);

                let mut total_intensity = 0.0;

                // Calculate influence from each particle
                for &particle_pos in &live_particles {
                    let dx = pixel_pos.x - particle_pos.x;
                    let dy = pixel_pos.y - particle_pos.y;
                    let distance_sq = dx * dx + dy * dy;

                    // Early exit for distant particles
                    if distance_sq > max_influence_radius_sq {
                        continue;
                    }

                    // Gaussian falloff
                    let sigma = 25.0; // Controls the spread
                    let influence = (-distance_sq / (2.0 * sigma * sigma)).exp();
                    total_intensity += influence;
                }

                // Skip pixels with very low intensity
                if total_intensity < 0.01 {
                    return None;
                }

                // Normalize and apply color mapping
                let intensity = (total_intensity * 0.3).min(1.0);

                // Same 4-color gradient: soft blue -> lavender -> soft pink -> bright pink -> amber
                let color = if intensity < 0.25 {
                    let t = intensity / 0.25;
                    let soft_blue = vec3(0.7, 0.8, 1.0);
                    let lavender = vec3(0.85, 0.75, 1.0);
                    let rgb = soft_blue + (lavender - soft_blue) * t;
                    rgba(rgb.x, rgb.y, rgb.z, 0.6 + intensity * 0.3)
                } else if intensity < 0.5 {
                    let t = (intensity - 0.25) / 0.25;
                    let lavender = vec3(0.85, 0.75, 1.0);
                    let soft_pink = vec3(1.0, 0.8, 0.9);
                    let rgb = lavender + (soft_pink - lavender) * t;
                    rgba(rgb.x, rgb.y, rgb.z, 0.7 + intensity * 0.2)
                } else if intensity < 0.75 {
                    let t = (intensity - 0.5) / 0.25;
                    let soft_pink = vec3(1.0, 0.8, 0.9);
                    let bright_pink = vec3(1.0, 0.2, 0.6);
                    let rgb = soft_pink + (bright_pink - soft_pink) * t;
                    rgba(rgb.x, rgb.y, rgb.z, 0.8 + intensity * 0.1)
                } else {
                    let t = (intensity - 0.75) / 0.25;
                    let bright_pink = vec3(1.0, 0.2, 0.6);
                    let amber = vec3(1.0, 0.75, 0.0);
                    let rgb = bright_pink + (amber - bright_pink) * t;
                    rgba(rgb.x, rgb.y, rgb.z, 0.9)
                };

                Some((pixel_pos, color))
            })
            .collect();

        // Draw all results (this must be single-threaded due to nannou draw API)
        let line_size = (pixel_size_x + pixel_size_y) * 0.5;
        for (pixel_pos, color) in results {
            draw.line()
                .xy(pixel_pos / 2.0)
                .start(pixel_pos / 2.0 + vec2(line_size / 2.0, 0.0))
                .end(pixel_pos / 2.0 + vec2(0.0, line_size / 2.0))
                .stroke_weight(line_size * 1.5)
                .color(color);
        }
    }
}

/// Helper to make a Rgba from an Rgb and alpha value
fn rgba_from(rgb: Rgb, alpha: f32) -> Rgba {
    rgba(rgb.red, rgb.green, rgb.blue, alpha)
}
