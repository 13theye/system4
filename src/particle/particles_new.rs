/// particles_new.rs
///
/// More efficient particles using Struct-of-Arrays
///
use nannou::prelude::*;
use rayon::prelude::*;
use std::{collections::HashMap, ops::Range};

use crate::view::Voice;

const PARTICLE_MASS: f32 = 11.0;
const PARTICLE_LIFE_SPAN: f32 = 3600.0;
const FADE_IN_DURATION: f32 = 400.0; // frames to fade in
const FADE_OUT_DURATION: f32 = 100.0;
const OOB_BUFFER: f32 = 1000.0;

#[derive(Debug, Default)]
pub struct Particles {
    // Positioning and motion
    pub(crate) pos_x: Vec<f32>,            // x position
    pub(crate) pos_y: Vec<f32>,            // y position
    pub(crate) vel_x: Vec<f32>,            // x velocity
    pub(crate) vel_y: Vec<f32>,            // y velocity
    pub(crate) acc_x: Vec<f32>,            // x acceleration
    pub(crate) acc_y: Vec<f32>,            // y acceleration
    feedback_pos_x: Vec<[Option<f32>; 4]>, // x position of last 4 frames
    feedback_pos_y: Vec<[Option<f32>; 4]>, // y position of last 4 frames

    // Lifespan
    age: Vec<f32>,                 // age of particles
    remaining_life_span: Vec<f32>, // remaining life span
    killed: Vec<bool>,             // whether the particle is "marked for death"

    // Size
    size: Vec<f32>,            // size of particles in screen points
    pub(crate) mass: Vec<f32>, // mass of particles

    // Color and alpha
    color: Vec<Rgb>, // rgb rolor of particles
    alpha: Vec<f32>, // alpha value of particles

    // Metadata
    parent_id: Vec<usize>, // id of the parent emitters
    voice: Vec<i32>,       // voice that the particle belongs to
}

impl Particles {
    pub fn new() -> Self {
        Self {
            pos_x: Vec::new(),
            pos_y: Vec::new(),

            vel_x: Vec::new(),
            vel_y: Vec::new(),
            acc_x: Vec::new(),
            acc_y: Vec::new(),
            feedback_pos_x: Vec::new(),
            feedback_pos_y: Vec::new(),

            age: Vec::new(),
            remaining_life_span: Vec::new(),
            killed: Vec::new(),

            size: Vec::new(),
            mass: Vec::new(),
            color: Vec::new(),
            alpha: Vec::new(),
            parent_id: Vec::new(),
            voice: Vec::new(),
        }
    }

    /// Add a moving particle. Used by Emitters.
    /// - default parameters implied:
    /// - particle life span (via constant)
    /// - particle mass (via constant)
    #[allow(clippy::too_many_arguments)]
    pub fn add_new_particle_with_motion(
        &mut self,
        parent_id: usize,
        voice: Voice,
        position: Point2,
        size: f32,
        rgba: Rgba,
        acceleration: Vec2,
        velocity: Vec2,
    ) {
        self.pos_x.push(position.x);
        self.pos_y.push(position.y);
        self.acc_x.push(acceleration.x);
        self.acc_y.push(acceleration.y);
        self.vel_x.push(velocity.x);
        self.vel_y.push(velocity.y);
        self.feedback_pos_x.push([None; 4]);
        self.feedback_pos_y.push([None; 4]);
        self.age.push(0.0);
        self.remaining_life_span.push(PARTICLE_LIFE_SPAN);
        self.killed.push(false);
        self.size.push(size);
        self.mass.push(PARTICLE_MASS);
        self.color.push(rgba.color);
        self.alpha.push(rgba.alpha);
        self.parent_id.push(parent_id);
        self.voice.push(voice.to_i32());
    }

    /************** Per-cycle updates ***************/

    /// Update particles based on forces and age, given externally-determined color and alpha limits
    pub fn update(&mut self, rgba_limits: HashMap<i32, Rgba>, bounds_rect: Rect) {
        // Delete dead particles
        self.delete_dead_particles();

        // Add the current position to the feedback positions
        self.record_feedback_positions_all();

        // Update velocity based on acceleration
        self.update_velocities_all();

        // Update position based on velocity
        self.update_positions_all(bounds_rect);

        // Update color and alpha
        self.update_style_all(rgba_limits);

        // Update lifespan
        self.increment_age_all();
    }

    /// Record the current position in the feedback positions arrays (optimized parallel)
    fn record_feedback_positions_all(&mut self) {
        // Process both x and y coordinates in a single parallel iteration
        self.feedback_pos_x
            .par_iter_mut()
            .zip(self.feedback_pos_y.par_iter_mut())
            .zip(self.pos_x.par_iter())
            .zip(self.pos_y.par_iter())
            .for_each(|(((feedback_x, feedback_y), &pos_x), &pos_y)| {
                // Update x feedback
                feedback_x.rotate_left(1);
                feedback_x[3] = Some(pos_x);

                // Update y feedback
                feedback_y.rotate_left(1);
                feedback_y[3] = Some(pos_y);
            });
    }

    /// Update velocity based on acceleration
    fn update_velocities_all(&mut self) {
        self.vel_x
            .par_iter_mut()
            .zip(self.vel_y.par_iter_mut())
            .zip(self.acc_x.par_iter_mut())
            .zip(self.acc_y.par_iter_mut())
            .for_each(|(((vel_x, vel_y), acc_x), acc_y)| {
                // add acceleration to velocity
                *vel_x += *acc_x;
                *vel_y += *acc_y;

                // reset acceleration for forces to be applied next frame
                *acc_x = 0.0;
                *acc_y = 0.0;
            });
    }

    /// - Update position based on velocity
    /// - Mark particles for death if they go out of bounds
    fn update_positions_all(&mut self, bounds_rect: Rect) {
        self.pos_x
            .par_iter_mut()
            .zip(self.pos_y.par_iter_mut())
            .zip(self.vel_x.par_iter())
            .zip(self.vel_y.par_iter())
            .zip(self.killed.par_iter_mut())
            .for_each(|((((pos_x, pos_y), vel_x), vel_y), killed)| {
                // add velocity to position
                *pos_x += *vel_x;
                *pos_y += *vel_y;

                // Kill particle if out of bounds & buffer zone
                if Self::position_out_of_bounds(*pos_x, *pos_y, bounds_rect) {
                    *killed = true;
                }
            });
    }

    /// - Update color as dictated by UI
    /// - Update alpha as dictated by UI and age
    fn update_style_all(&mut self, rgba_limits: HashMap<i32, Rgba>) {
        self.color
            .par_iter_mut()
            .zip(self.alpha.par_iter_mut())
            .zip(self.voice.par_iter())
            .zip(self.age.par_iter())
            .zip(self.remaining_life_span.par_iter())
            .for_each(|((((color, alpha), voice), age), remaining_life_span)| {
                // Update color
                let rgba_limit = if let Some(limit) = rgba_limits.get(voice) {
                    *limit
                } else {
                    Rgba {
                        color: rgb(0.0, 0.0, 0.0),
                        alpha: 0.0,
                    }
                };

                *color = if *color != rgba_limit.color {
                    rgba_limit.color
                } else {
                    *color
                };

                // Update alpha
                // 1. Calculate fade in factor based on natural age
                let fade_in_factor = if *age < FADE_IN_DURATION {
                    *age / FADE_IN_DURATION
                } else {
                    1.0
                };

                //2. Calculate the maximum alpha this particle has reached so far
                let max_alpha_reached = rgba_limit.alpha * fade_in_factor;

                //3. Calculate fade-out factor based on remaining life span
                let eol_alpha = if *remaining_life_span <= FADE_OUT_DURATION {
                    *remaining_life_span / FADE_OUT_DURATION
                } else {
                    1.0
                };

                //4. Save calculated alpha value
                *alpha = max_alpha_reached * eol_alpha;
            });
    }

    /// Increment age and decrement remaining life span for all particles
    fn increment_age_all(&mut self) {
        self.age
            .par_iter_mut()
            .zip(self.remaining_life_span.par_iter_mut())
            .for_each(|(age, remaining_life_span)| {
                *age += 1.0;
                *remaining_life_span -= 1.0;
            })
    }

    /// Delete all particles when either is true:
    /// remaining_life_span[i] <= 0.0
    /// killed[i] = true
    fn delete_dead_particles(&mut self) {
        let mut write_index = 0;

        for read_index in 0..self.pos_x.len() {
            // Keep particle if it's still alive and not killed
            if self.remaining_life_span[read_index] > 0.0 && !self.killed[read_index] {
                if write_index != read_index {
                    // Move live particle data to the write position
                    self.pos_x[write_index] = self.pos_x[read_index];
                    self.pos_y[write_index] = self.pos_y[read_index];
                    self.vel_x[write_index] = self.vel_x[read_index];
                    self.vel_y[write_index] = self.vel_y[read_index];
                    self.acc_x[write_index] = self.acc_x[read_index];
                    self.acc_y[write_index] = self.acc_y[read_index];
                    self.feedback_pos_x[write_index] = self.feedback_pos_x[read_index];
                    self.feedback_pos_y[write_index] = self.feedback_pos_y[read_index];
                    self.age[write_index] = self.age[read_index];
                    self.remaining_life_span[write_index] = self.remaining_life_span[read_index];
                    self.killed[write_index] = self.killed[read_index];
                    self.size[write_index] = self.size[read_index];
                    self.mass[write_index] = self.mass[read_index];
                    self.color[write_index] = self.color[read_index];
                    self.alpha[write_index] = self.alpha[read_index];
                    self.parent_id[write_index] = self.parent_id[read_index];
                    self.voice[write_index] = self.voice[read_index];
                }
                write_index += 1;
            }
        }

        // Truncate all vectors to remove dead particles
        self.pos_x.truncate(write_index);
        self.pos_y.truncate(write_index);
        self.vel_x.truncate(write_index);
        self.vel_y.truncate(write_index);
        self.acc_x.truncate(write_index);
        self.acc_y.truncate(write_index);
        self.feedback_pos_x.truncate(write_index);
        self.feedback_pos_y.truncate(write_index);
        self.age.truncate(write_index);
        self.remaining_life_span.truncate(write_index);
        self.killed.truncate(write_index);
        self.size.truncate(write_index);
        self.mass.truncate(write_index);
        self.color.truncate(write_index);
        self.alpha.truncate(write_index);
        self.parent_id.truncate(write_index);
        self.voice.truncate(write_index);
    }

    /// Append particles from another Particles struct
    pub fn append(&mut self, mut other: Particles) {
        self.pos_x.append(&mut other.pos_x);
        self.pos_y.append(&mut other.pos_y);
        self.vel_x.append(&mut other.vel_x);
        self.vel_y.append(&mut other.vel_y);
        self.acc_x.append(&mut other.acc_x);
        self.acc_y.append(&mut other.acc_y);
        self.feedback_pos_x.append(&mut other.feedback_pos_x);
        self.feedback_pos_y.append(&mut other.feedback_pos_y);
        self.age.append(&mut other.age);
        self.remaining_life_span
            .append(&mut other.remaining_life_span);
        self.killed.append(&mut other.killed);
        self.size.append(&mut other.size);
        self.mass.append(&mut other.mass);
        self.color.append(&mut other.color);
        self.alpha.append(&mut other.alpha);
        self.parent_id.append(&mut other.parent_id);
        self.voice.append(&mut other.voice);
    }

    /// Cull oldest particles if the number exceeds the limit
    pub fn cull(&mut self, limit: usize) {
        let excess = self.pos_x.len() - limit;
        if excess > 0 {
            self.begin_fade_out(0..excess);
        }
    }

    pub fn cull_by_voice(&mut self, voice: Voice, limit: usize) {
        let excess = self.len_by_voice(voice) - limit;
        let voice = voice.to_i32();
        if excess > 0 {
            let indices_of_excess = self
                .voice
                .iter()
                .enumerate()
                .filter_map(|(index, &v)| if v == voice { Some(index) } else { None })
                .take(excess)
                .collect::<Vec<_>>();

            // Fade out the oldest particles until all excess have been marked
            for i in indices_of_excess {
                self.begin_fade_out_particle(i);
            }
        }
    }

    /// Draw all particles
    pub fn draw_all(&self, draw: &Draw, feedback_values: &HashMap<i32, f32>) {
        self.pos_x.iter().enumerate().for_each(|(index, &pos_x)| {
            let position = Point2::new(pos_x, self.pos_y[index]);
            let scaled_size = self.size[index];

            draw.line()
                .xy(position)
                .start(vec2(scaled_size / 2.0, 0.0))
                .end(vec2(0.0, scaled_size / 2.0))
                .stroke_weight(scaled_size)
                .color(Rgba {
                    color: self.color[index],
                    alpha: self.alpha[index],
                });

            let feedback = if let Some(&feedback) = feedback_values.get(&self.voice[index]) {
                feedback
            } else {
                0.0
            };

            if feedback > 0.01 {
                // Create trail by connecting feedback positions with scaled distances
                let mut prev_pos = position;

                for i in 1..(feedback * 3.0).round().min(3.0) as usize {
                    if let (Some(pos1_x), Some(pos1_y)) =
                        (self.feedback_pos_x[index][i], self.feedback_pos_y[index][i])
                    {
                        let trail_pos = Point2::new(pos1_x, pos1_y);

                        // Work entirely in world coordinates, let draw API handle scaling
                        let direction = trail_pos - position;
                        let extended_pos = position + direction * feedback;

                        // Calculate alpha of the segment
                        let trail_alpha = self.alpha[index] * (1.0 - i as f32 * 0.125);

                        // Draw line connecting positions
                        draw.line()
                            .start(prev_pos)
                            .end(extended_pos)
                            .stroke_weight(scaled_size * (0.5 - i as f32 * 0.05))
                            .color(Rgba {
                                color: self.color[index],
                                alpha: trail_alpha,
                            });

                        prev_pos = extended_pos;
                    }
                }
            }
        });
    }

    /****************** Particle accessors: lifespan  ************************/

    /// Tells all particles within a range to begin fading out
    pub fn begin_fade_out(&mut self, range: Range<usize>) {
        for index in range {
            self.remaining_life_span[index] = FADE_OUT_DURATION;
        }
    }

    /// Tells a particular particle to begin fading out
    pub fn begin_fade_out_particle(&mut self, index: usize) {
        self.remaining_life_span[index] = FADE_OUT_DURATION;
    }

    /// Returns true if the particle has a remaining life span > 0
    pub fn is_alive(&self, index: usize) -> bool {
        self.remaining_life_span[index] > 0.0
    }

    /// Sets remaining life of a particle to 0
    pub fn kill(&mut self, index: usize) {
        self.remaining_life_span[index] = 0.0;
    }

    /// Get a the fade out duration of particles
    pub fn fade_out_duration(&self) -> f32 {
        FADE_OUT_DURATION
    }

    /**************** Particle accessors: motion & position ******************/

    /// Returns all particle positions as an owned Vec of Points
    pub fn positions_to_owned(&self) -> Vec<Point2> {
        self.pos_x
            .iter()
            .zip(self.pos_y.iter())
            .map(|(x, y)| Point2::new(*x, *y))
            .collect()
    }

    /// Returns all particle positions as a reference to the internal arrays
    pub fn positions(&self) -> (&Vec<f32>, &Vec<f32>) {
        (&self.pos_x, &self.pos_y)
    }

    /// Returns true if the particle is outside of the bounds_rect area
    /// - buffer is 1000 pixels, allowing for a particle to leave the area
    ///   and then return
    pub fn position_out_of_bounds(pos_x: f32, pos_y: f32, bounds_rect: Rect) -> bool {
        pos_x < bounds_rect.left() - OOB_BUFFER
            || pos_x > bounds_rect.right() + OOB_BUFFER
            || pos_y < bounds_rect.bottom() - OOB_BUFFER
            || pos_y > bounds_rect.top() + OOB_BUFFER
    }

    /********************** Helper functions *******************************/

    /// Returns how many particles exist overall
    pub fn len(&self) -> usize {
        self.pos_x.len()
    }

    pub fn len_by_voice(&self, voice: Voice) -> usize {
        let voice_num = voice.to_i32();
        self.voice.iter().filter(|&v| *v == voice_num).count()
    }

    /// Returns true if there are no particles
    pub fn is_empty(&self) -> bool {
        self.pos_x.is_empty()
    }
}
