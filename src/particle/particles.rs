// src/particle/particles.rs
//
// Particle struct for the Particle System

use nannou::prelude::*;
use nnpipe::renderers::{ParticleGpu, SegmentGpu};

use super::constants::*;

/// Core particle data for physics updates (~100 bytes)
/// This is the "hot" data that gets accessed every frame during physics calculations
#[derive(Clone, Copy, Debug)]
pub struct ParticleCore {
    pub position: Point2,
    pub velocity: Vec2,
    pub acceleration: Vec2,
    pub age: f32,
    pub remaining_life_span: f32,
    pub age_per_tick: f32,
    // A particle is alive as soon as it is created
    pub is_alive: bool,
    // A particle is activated when it touches a force. Only then will it fade in to visibility.
    pub is_activated: bool,
    pub size: f32,
    pub mass: f32,
    pub rgba: Rgba,
}

/// Feedback/trail data for rendering (~3.4KB)
/// This is the "cold" data that only gets accessed during segment generation
#[derive(Clone, Debug)]
pub struct ParticleFeedback {
    pub positions: [Option<Point2>; FEEDBACK_POSITIONS],
    pub colors: [Option<Rgb>; FEEDBACK_POSITIONS],
    pub current_index: usize,
}

impl Default for ParticleFeedback {
    fn default() -> Self {
        Self {
            positions: [None; FEEDBACK_POSITIONS],
            colors: [None; FEEDBACK_POSITIONS],
            current_index: 0,
        }
    }
}

impl ParticleFeedback {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&mut self, position: Point2, color: Rgb) {
        self.positions[self.current_index] = Some(position);
        self.colors[self.current_index] = Some(color);
        self.current_index = (self.current_index + 1) % FEEDBACK_POSITIONS;
    }
}

impl ParticleCore {
    pub fn new(position: Point2, size: f32, color: Rgba) -> Self {
        Self {
            position,
            velocity: vec2(0.0, 0.0),
            acceleration: vec2(0.0, 0.0),
            age: 0.0,
            remaining_life_span: PARTICLE_LIFE_SPAN,
            age_per_tick: 1.0,
            is_alive: true,
            is_activated: false,
            size,
            mass: PARTICLE_MASS,
            rgba: color,
        }
    }

    pub fn with_velocity(mut self, velocity: Vec2) -> Self {
        self.velocity = velocity;
        self
    }

    /// Update the particle based on forces and age
    /// This is the hot path - keep it tight and cache-friendly
    pub fn update(&mut self, color_limit: Rgb, alpha_limit: f32, framerate_factor: f32) {
        // Apply velocity and reset acceleration
        self.velocity += self.acceleration * framerate_factor;
        self.position += self.velocity * framerate_factor;
        self.acceleration = vec2(0.0, 0.0);

        // Update color if needed
        if self.rgba.color != color_limit {
            self.rgba.color = color_limit;
        }

        // Calculate fade-in factor based on natural age
        let fade_in_factor = if self.age < PARTICLE_FADE_IN_DURATION {
            self.age / PARTICLE_FADE_IN_DURATION
        } else {
            1.0
        };

        // Calculate the maximum alpha this particle has reached so far
        let max_alpha_reached = alpha_limit * fade_in_factor.powi(3);

        // Calculate life-based alpha fade-out using remaining life span
        let end_of_life_alpha = if self.remaining_life_span <= PARTICLE_FADE_OUT_DURATION {
            self.remaining_life_span / PARTICLE_FADE_OUT_DURATION
        } else {
            1.0
        };

        self.rgba.alpha = max_alpha_reached * end_of_life_alpha;

        // Increment natural age and decrement remaining life span
        // Only increment age if the particle is activated
        if self.is_activated {
            self.age += self.age_per_tick * framerate_factor;
            self.remaining_life_span -= self.age_per_tick * framerate_factor;
        }

        if self.remaining_life_span <= 0.0 {
            self.kill();
        }
    }

    #[inline]
    pub fn is_out_of_bounds(&self, bounds_rect: Rect) -> bool {
        let buffer = OOB_BUFFER;
        self.position.x < bounds_rect.left() - buffer
            || self.position.x > bounds_rect.right() + buffer
            || self.position.y < bounds_rect.bottom() - buffer
            || self.position.y > bounds_rect.top() + buffer
    }

    #[inline]
    pub fn kill(&mut self) {
        self.remaining_life_span = 0.0;
        self.is_alive = false;
    }

    #[inline]
    pub fn set_to_fade_out(&mut self) {
        self.remaining_life_span = PARTICLE_FADE_OUT_DURATION;
    }

    #[inline]
    pub fn activate(&mut self) {
        self.is_activated = true;
    }

    /// Convert core particle data to GPU format
    #[inline]
    pub fn to_gpu(&self, offset: Vec2) -> ParticleGpu {
        ParticleGpu::new(
            [self.position.x + offset.x, self.position.y + offset.y],
            [self.rgba.red, self.rgba.green, self.rgba.blue],
            self.rgba.alpha,
        )
    }

    pub fn to_gpu_invisible(&self, offset: Vec2) -> ParticleGpu {
        ParticleGpu::new(
            [self.position.x + offset.x, self.position.y + offset.y],
            [0.0, 0.0, 0.0],
            0.0,
        )
    }

    pub fn fade_out_duration(&self) -> f32 {
        PARTICLE_FADE_OUT_DURATION
    }
}

/// Helper function to generate SegmentGpu from core and feedback data
pub fn to_segment_gpu(
    core: &ParticleCore,
    feedback: &ParticleFeedback,
    offset: Vec2,
    segment_length: f32,
    line_width: f32,
) -> SegmentGpu {
    let mut points = [[0.0f32; 2]; FEEDBACK_POSITIONS];
    let mut colors = [[0.0f32; 3]; FEEDBACK_POSITIONS];

    // First point is current position

    points[0] = [core.position.x + offset.x, core.position.y + offset.y];
    colors[0] = [core.rgba.red, core.rgba.green, core.rgba.blue];

    // Fill remaining points and colors from feedback history (reading from ring buffer)
    let mut last_valid_pos = [core.position.x, core.position.y];
    for i in 0..(FEEDBACK_POSITIONS - 1) {
        // Calculate ring buffer index: read backwards from second most recent
        // (because the most recent is the current position)
        let ring_index = (feedback.current_index + FEEDBACK_POSITIONS - 2 - i) % FEEDBACK_POSITIONS;

        if let Some(feedback_pos) = feedback.positions[ring_index] {
            points[i + 1] = [feedback_pos.x, feedback_pos.y];
            last_valid_pos = [feedback_pos.x, feedback_pos.y];
        } else {
            // If no feedback position, use the last valid position to avoid ray artifacts
            points[i + 1] = last_valid_pos;
        }

        if let Some(feedback_color) = feedback.colors[ring_index] {
            colors[i + 1] = [
                feedback_color.red,
                feedback_color.green,
                feedback_color.blue,
            ];
        } else {
            // If no feedback color, default to black
            colors[i + 1] = [0.0, 0.0, 0.0];
        }
    }

    // Calculate actual history length from particle age, ensuring it's at least 1
    // and doesn't exceed the maximum feedback positions
    let actual_history_length = (core.age as u32).clamp(1, FEEDBACK_POSITIONS as u32);

    SegmentGpu::new(
        points,
        colors,
        core.rgba.alpha,
        segment_length,
        line_width,
        actual_history_length,
    )
}
