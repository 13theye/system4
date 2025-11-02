// src/particle/particles.rs
//
// Particle struct for the Particle System

use nannou::prelude::*;
use nnpipe::renderers::{ParticleGpu, SegmentGpu};

const PARTICLE_MASS: f32 = 11.0;
const PARTICLE_LIFE_SPAN: f32 = 1800.0;
const FADE_IN_DURATION: f32 = 180.0; // frames to fade in
const FADE_OUT_DURATION: f32 = 100.0;
pub const FEEDBACK_POSITIONS: usize = 128;

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
    pub is_alive: bool,
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
    #[inline]
    pub fn update(&mut self, color_limit: Rgb, alpha_limit: f32) {
        // Apply velocity and reset acceleration
        self.velocity += self.acceleration;
        self.position += self.velocity;
        self.acceleration = vec2(0.0, 0.0);

        // Update color if needed
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

    #[inline]
    pub fn is_out_of_bounds(&self, bounds_rect: Rect) -> bool {
        let buffer = 1500.0;
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
        self.remaining_life_span = FADE_OUT_DURATION;
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

    pub fn fade_out_duration(&self) -> f32 {
        FADE_OUT_DURATION
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

/// Legacy monolithic Particle struct for backward compatibility
/// Will be removed once refactoring is complete
#[derive(Clone, Copy)]
pub struct Particle {
    position: Point2,
    feedback_positions: [Option<Point2>; FEEDBACK_POSITIONS],
    feedback_colors: [Option<Rgb>; FEEDBACK_POSITIONS],
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
            acceleration: vec2(0.0, 0.0),
            velocity: vec2(0.0, 0.0),
            position,
            feedback_positions: [None; FEEDBACK_POSITIONS],
            feedback_colors: [None; FEEDBACK_POSITIONS],
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
        self.record_feedback_position(offset_position, color_limit);

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

    fn record_feedback_position(&mut self, position: Vec2, color: Rgb) {
        self.feedback_positions[self.current_feedback_position] = Some(position);
        self.feedback_colors[self.current_feedback_position] = Some(color);
        self.current_feedback_position = (self.current_feedback_position + 1) % FEEDBACK_POSITIONS;
    }

    fn _record_feedback_position(&mut self, position: Point2) {
        for i in (1..FEEDBACK_POSITIONS).rev() {
            self.feedback_positions[i] = self.feedback_positions[i - 1];
        }
        self.feedback_positions[0] = Some(position);
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
    pub fn to_gpu(&self, offset: Vec2) -> ParticleGpu {
        ParticleGpu::new(
            [self.position.x + offset.x, self.position.y + offset.y],
            [self.rgba.red, self.rgba.green, self.rgba.blue],
            self.rgba.alpha,
        )
    }

    pub fn to_segment_gpu(&self, offset: Vec2, segment_length: f32, line_width: f32) -> SegmentGpu {
        let mut points = [[0.0f32; 2]; FEEDBACK_POSITIONS];
        let mut colors = [[0.0f32; 3]; FEEDBACK_POSITIONS];

        // First point is current position
        points[0] = [self.position.x + offset.x, self.position.y + offset.y];
        colors[0] = [self.rgba.red, self.rgba.green, self.rgba.blue];

        // Fill remaining points and colors from feedback history (reading from ring buffer)
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

            if let Some(feedback_color) = self.feedback_colors[ring_index] {
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
        let actual_history_length = (self.age as u32).clamp(1, FEEDBACK_POSITIONS as u32);

        SegmentGpu::new(
            points,
            colors,
            self.rgba.alpha,
            segment_length,
            line_width,
            actual_history_length,
        )
    }
}
