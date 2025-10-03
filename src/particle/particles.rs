// src/particle/particles.rs
//
// Particle struct for the Particle System

use nannou::prelude::*;
use nnpipe::renderers::{ParticleGpu, SegmentGpu};

const PARTICLE_MASS: f32 = 11.0;
const PARTICLE_LIFE_SPAN: f32 = 900.0;
const FADE_IN_DURATION: f32 = 180.0; // frames to fade in
const FADE_OUT_DURATION: f32 = 100.0;
const FEEDBACK_POSITIONS: usize = 128;

#[derive(Clone, Copy)]
pub struct Particle {
    pub particle_id: u32, // Stable ID for GPU history buffer
    position: Point2,
    feedback_positions: [Option<Point2>; FEEDBACK_POSITIONS],
    current_feedback_position: usize,
    pub velocity: Vec2,
    pub acceleration: Vec2,

    pub age: f32,
    pub remaining_life_span: f32,
    age_per_tick: f32,

    is_alive: bool,
    // a particle is activated when it has been affected by a force
    is_activated: bool,
    pub size: f32,
    pub mass: f32,
    pub rgba: Rgba,
}

impl Particle {
    pub fn new(position: Point2, size: f32, color: Rgba) -> Self {
        Self {
            particle_id: 0, // Will be set by particle system
            acceleration: vec2(0.0, 0.0),
            velocity: vec2(0.0, 0.0),
            position,
            feedback_positions: [None; FEEDBACK_POSITIONS],
            current_feedback_position: 0,
            age: 0.0,
            remaining_life_span: PARTICLE_LIFE_SPAN,
            age_per_tick: 1.0,
            is_alive: true,
            is_activated: false,
            size,
            rgba: color,
            mass: PARTICLE_MASS,
        }
    }

    pub fn set_particle_id(&mut self, id: u32) {
        self.particle_id = id;
    }

    pub fn with_velocity(mut self, velocity: Vec2) -> Self {
        self.velocity = velocity;
        self
    }

    /// Update the particle based on forces and age, given externally-determined color and alpha limits, and offset for feedback recording
    pub fn update(&mut self, color_limit: Rgb, alpha_limit: f32, position_offset: Vec2) {
        // Record the offset position for feedback trails
        let offset_position = if position_offset.length_squared() > 0.0 {
            self.position + position_offset
        } else {
            self.position
        };
        //self.record_feedback_position(offset_position);

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
        let max_alpha_reached = alpha_limit * fade_in_factor.powi(3);

        // Calculate life-based alpha fade-out using remaining life span
        let end_of_life_alpha = if self.remaining_life_span <= FADE_OUT_DURATION {
            self.remaining_life_span / FADE_OUT_DURATION
        } else {
            1.0
        };

        self.rgba.alpha = max_alpha_reached * end_of_life_alpha;

        // Increment natural age and decrement remaining life span
        // Only increment age if the particle is activated
        if self.is_activated {
            self.age += self.age_per_tick;
            self.remaining_life_span -= self.age_per_tick;
        }

        if self.remaining_life_span <= 0.0 {
            self.kill();
        }
    }

    pub fn position(&self) -> Point2 {
        self.position
    }

    /// Record the current position in a ring buffer for feedback trails
    fn record_feedback_position(&mut self, position: Vec2) {
        self.feedback_positions[self.current_feedback_position] = Some(position);
        self.current_feedback_position = (self.current_feedback_position + 1) % FEEDBACK_POSITIONS;
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

    /// True if the particle is out of bounds, with a buffer
    pub fn is_out_of_bounds(&self, bounds_rect: Rect) -> bool {
        let buffer = 1500.0;
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

    pub fn is_activated(&self) -> bool {
        self.is_activated
    }

    pub fn activate(&mut self) {
        self.is_activated = true;
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
    pub fn to_gpu(&self, offset: Vec2, segment_length: f32) -> ParticleGpu {
        ParticleGpu::new(
            self.particle_id,
            [self.position.x + offset.x, self.position.y + offset.y],
            [self.rgba.red, self.rgba.green, self.rgba.blue],
            self.rgba.alpha,
            self.age,
            segment_length,
        )
    }

    pub fn to_segment_gpu(&self, offset: Vec2, segment_length: f32, line_width: f32) -> SegmentGpu {
        let mut points = [[0.0f32; 2]; FEEDBACK_POSITIONS];

        // First point is current position
        points[0] = [self.position.x + offset.x, self.position.y + offset.y];

        // Fill remaining points from feedback positions (reading from ring buffer)
        let mut last_valid_pos = [self.position.x, self.position.y];
        for i in 0..(FEEDBACK_POSITIONS - 1) {
            // Calculate ring buffer index: read backwards from most recent
            let ring_index =
                (self.current_feedback_position + FEEDBACK_POSITIONS - 1 - i) % FEEDBACK_POSITIONS;

            if let Some(feedback_pos) = self.feedback_positions[ring_index] {
                points[i + 1] = [feedback_pos.x, feedback_pos.y];
                last_valid_pos = [feedback_pos.x, feedback_pos.y];
            } else {
                // If no feedback position, use the last valid position to avoid ray artifacts
                points[i + 1] = last_valid_pos;
            }
        }

        // Calculate actual history length from particle age, ensuring it's at least 1
        // and doesn't exceed the maximum feedback positions
        let actual_history_length = (self.age as u32).clamp(1, FEEDBACK_POSITIONS as u32);

        SegmentGpu::new(
            points,
            [self.rgba.red, self.rgba.green, self.rgba.blue],
            self.rgba.alpha,
            segment_length,
            line_width,
            actual_history_length,
        )
    }
}
