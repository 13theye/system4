// src/particle/particles.rs
//
// Particle struct for the Particle System

use nannou::prelude::*;
use nnpipe::renderers::{ParticleGpu, SegmentGpu};

const PARTICLE_MASS: f32 = 11.0;
const PARTICLE_LIFE_SPAN: f32 = 3600.0;
const FADE_IN_DURATION: f32 = 20.0; // frames to fade in
const FADE_OUT_DURATION: f32 = 100.0;

#[derive(Clone, Copy)]
pub struct Particle {
    pub parent_emitter: usize, // the emitter that spawned this particle
    position: Point2,
    feedback_positions: [Option<Point2>; 15],
    pub velocity: Vec2,
    pub acceleration: Vec2,

    pub age: f32,
    pub remaining_life_span: f32,
    age_per_tick: f32,

    pub is_alive: bool,
    pub size: f32,
    pub mass: f32,
    pub rgba: Rgba,
}

impl Particle {
    pub fn new(parent_id: usize, position: Point2, size: f32, color: Rgba) -> Self {
        Self {
            parent_emitter: parent_id,
            acceleration: vec2(0.0, 0.0),
            velocity: vec2(0.0, 0.0),
            position,
            feedback_positions: [None; 15],
            age: 0.0,
            remaining_life_span: PARTICLE_LIFE_SPAN,
            age_per_tick: 1.0,
            is_alive: true,
            size,
            rgba: color,
            mass: PARTICLE_MASS,
        }
    }

    pub fn new_with_motion(
        parent_id: usize,
        position: Point2,
        size: f32,
        color: Rgba,
        acceleration: Vec2,
        velocity: Vec2,
    ) -> Self {
        Self {
            parent_emitter: parent_id,
            acceleration,
            velocity,
            position,
            feedback_positions: [None; 15],
            age: 0.0,
            remaining_life_span: PARTICLE_LIFE_SPAN,
            age_per_tick: 1.0,
            is_alive: true,
            size,
            rgba: color,
            mass: PARTICLE_MASS,
        }
    }

    /// Update the particle based on forces and age, given externally-determined color and alpha limits
    pub fn update(&mut self, color_limit: Rgb, alpha_limit: f32) {
        // Add the current position to the feedback positions
        self.record_feedback_position();

        self.velocity += self.acceleration;
        self.position += self.velocity;

        // Reset acceleration for next frame (forces will be reapplied)
        self.acceleration = vec2(0.0, 0.0);

        if self.rgba.color != color_limit {
            self.rgba.color = color_limit;
        }

        // Calculate fade-in factor based on natural age
        let fade_in_factor = if self.age < FADE_IN_DURATION {
            self.age / FADE_IN_DURATION
        } else {
            1.0
        };

        // Calculate the maximum alpha this particle has reached so far
        let max_alpha_reached = alpha_limit * fade_in_factor;

        // Calculate life-based alpha fade-out using remaining life span
        let end_of_life_alpha = if self.remaining_life_span <= FADE_OUT_DURATION {
            self.remaining_life_span / FADE_OUT_DURATION
        } else {
            1.0
        };

        self.rgba.alpha = max_alpha_reached * end_of_life_alpha;

        // Increment natural age and decrement remaining life span
        self.age += self.age_per_tick;
        self.remaining_life_span -= self.age_per_tick;

        if self.remaining_life_span <= 0.0 {
            self.kill();
        }
    }

    pub fn position(&self) -> Point2 {
        self.position
    }

    fn record_feedback_position(&mut self) {
        for i in (1..15).rev() {
            self.feedback_positions[i] = self.feedback_positions[i - 1];
        }
        self.feedback_positions[0] = Some(self.position);
    }

    /// Deprecated draw command that uses Nannou::draw to draw the particle as a short line.
    /// FOR REFERENCE ONLY!
    /// Particle drawing is now handled by Nnpipe::ParticleRenderer
    /// Trails are now handled by Nnpipe::SegmentRenderer
    pub fn draw(&self, draw: &Draw, _feedback: f32, dpi_scale: f32) {
        let scaled_size = self.size / dpi_scale;

        draw.line()
            .xy(self.position)
            .start(vec2(scaled_size / 2.0, 0.0))
            .end(vec2(-scaled_size / 2.0, 0.0))
            .stroke_weight(scaled_size)
            .color(self.rgba);

        /* Old Feedback/Trail functionality has been moved to GPU as a post-processing component
        if feedback > 0.01 {
            // Create trail by connecting feedback positions with scaled distances
            let mut prev_pos = self.position;

            for i in 1..(feedback * 4.0).round().min(4.0) as usize {
                if let Some(trail_pos) = self.feedback_positions[i] {
                    // Work entirely in world coordinates, let draw API handle scaling
                    let direction = trail_pos - self.position;
                    let extended_pos = self.position + direction * feedback;

                    // Draw trail segment
                    let trail_alpha = self.rgba.alpha * (1.0 - i as f32 * 0.125);
                    let trail_color =
                        rgba(self.rgba.red, self.rgba.green, self.rgba.blue, trail_alpha);

                    // Draw line connecting positions
                    draw.line()
                        .start(prev_pos)
                        .end(extended_pos)
                        .stroke_weight(scaled_size * (0.5 - i as f32 * 0.05))
                        .color(trail_color);

                    prev_pos = extended_pos;
                }
            }
        }
        */
    }

    /// True if the particle is out of bounds, with a buffer of 1000 pixels
    pub fn is_out_of_bounds(&self, bounds_rect: Rect) -> bool {
        let buffer = 1000.0;
        self.position.x < bounds_rect.left() - buffer
            || self.position.x > bounds_rect.right() + buffer
            || self.position.y < bounds_rect.bottom() - buffer
            || self.position.y > bounds_rect.top() + buffer
    }

    pub fn is_within_rect(&self, rect: Rect) -> bool {
        rect.contains(self.position)
    }

    pub fn kill(&mut self) {
        self.remaining_life_span = 0.0;
        self.is_alive = false;
    }

    pub fn set_age_per_tick(&mut self, age_per_tick: f32) {
        self.age_per_tick = age_per_tick;
    }

    pub fn is_dead(&self) -> bool {
        if self.remaining_life_span <= 0.0 || !self.is_alive {
            return true;
        }
        false
    }

    pub fn is_alive(&self) -> bool {
        self.is_alive
    }

    /********************* Accessors *********************/
    pub fn set_position(&mut self, position: Point2) {
        self.position = position;
    }

    pub fn set_velocity(&mut self, velocity: Vec2) {
        self.velocity = velocity;
    }

    pub fn set_acceleration(&mut self, acceleration: Vec2) {
        self.acceleration = acceleration;
    }

    pub fn set_life_span(&mut self, remaining_life_span: f32) {
        self.remaining_life_span = remaining_life_span;
    }

    pub fn set_to_fade_out(&mut self) {
        self.remaining_life_span = FADE_OUT_DURATION;
    }

    pub fn set_size(&mut self, size: f32) {
        self.size = size;
    }

    pub fn set_color(&mut self, color: Rgba) {
        self.rgba = color;
    }

    pub fn fade_out_duration(&self) -> f32 {
        FADE_OUT_DURATION
    }

    /********************* Convert to GPU *********************/
    pub fn to_gpu(&self) -> ParticleGpu {
        ParticleGpu::new(
            [self.position.x, self.position.y],
            [self.rgba.red, self.rgba.green, self.rgba.blue],
            self.rgba.alpha,
        )
    }

    pub fn to_segment_gpu(&self) -> SegmentGpu {
        let mut points = [[0.0f32; 2]; 16];

        // First point is current position
        points[0] = [self.position.x, self.position.y];

        // Fill remaining points from feedback positions
        for i in 0..15 {
            if let Some(feedback_pos) = self.feedback_positions[i] {
                points[i + 1] = [feedback_pos.x, feedback_pos.y];
            } else {
                // If no feedback position, use the current position
                points[i + 1] = [self.position.x, self.position.y];
            }
        }

        SegmentGpu::new(
            points,
            [self.rgba.red, self.rgba.green, self.rgba.blue],
            self.rgba.alpha,
        )
    }
}
