//! src/view/rhythm2/activation.rs
//!
//! Handles activation of RhythmElements

use nannou::prelude::*;
use std::time::Instant;

use super::{animation::*, element::RhythmElementParams};
use crate::utils::{color, tween};

#[derive(Copy, Clone, Debug)]
pub enum RotationDirection {
    Clockwise,
    Counterclockwise,
}

impl RotationDirection {
    /// Returns the rotation multiplier: -1.0 for clockwise, 1.0 for counterclockwise
    pub fn multiplier(&self) -> f32 {
        match self {
            RotationDirection::Clockwise => -1.0,
            RotationDirection::Counterclockwise => 1.0,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum ActivationMovement {
    Idle,
    Rotating {
        start_angle: f32,
        start_time: Instant,
        duration: f32,
    },
    Done,
}

#[derive(Copy, Clone, Debug)]
pub enum ActivationColor {
    Idle,
    Changing {
        start_color: Rgba,
        target_color_1: Rgba,
        target_color_2: Rgba,
        start_time: Instant,
        duration: f32,
    },
}

#[derive(Copy, Clone, Debug)]
pub struct ActivationElement {
    pub angle: f32,
    pub outer_color: Rgba,
    pub inner_color: Rgba,
    pub movement: ActivationMovement,
    pub color_change: ActivationColor,
    pub direction: RotationDirection,
}

impl ActivationElement {
    pub fn new(
        tempo: f64,
        element_params: &RhythmElementParams,
        index: usize,
        now: Instant,
    ) -> Self {
        let rotation_duration = (60.0 / tempo) as f32 * element_params.slot.length;

        let start_angle = 0.0;
        let start_color = element_params.color;

        // Determine rotation direction based on index
        // Even index: counterclockwise, Odd index: clockwise
        let direction = if index.is_multiple_of(2) {
            RotationDirection::Counterclockwise
        } else {
            RotationDirection::Clockwise
        };

        // Convert gradient_colors to HSV, scale value by cutoff, then convert back to
        let (_h0, _s0, v0) = color::rgb_to_hsv(start_color.color);
        let (h1, s1, v1) = color::rgb_to_hsv(element_params.gradient_color_1.color);
        let (h2, s2, v2) = color::rgb_to_hsv(element_params.gradient_color_2.color);

        let scaled_v1 = ((v1 - v0) * element_params.slot.cutoff) + v0;
        let target_c1 = color::hsv_to_rgb(h1, s1, scaled_v1);

        let scaled_v2 = ((v2 - v0) * element_params.slot.cutoff) + v0;
        let target_c2 = color::hsv_to_rgb(h2, s2, scaled_v2);

        let color_duration = rotation_duration * COLOR_CHANGE_FRACTION;

        Self {
            angle: start_angle,
            outer_color: start_color,
            inner_color: start_color,
            direction,
            movement: ActivationMovement::Rotating {
                start_angle,
                start_time: now,
                duration: rotation_duration,
            },
            color_change: ActivationColor::Changing {
                start_color,
                target_color_1: rgba(target_c1.red, target_c1.green, target_c1.blue, 1.0),
                target_color_2: rgba(target_c2.red, target_c2.green, target_c2.blue, 1.0),
                start_time: now,
                duration: color_duration,
            },
        }
    }

    pub fn is_done(&self) -> bool {
        matches!(self.movement, ActivationMovement::Done)
    }

    pub fn update(&mut self, now: Instant) {
        self.update_color(now);
        self.update_movement(now);
    }

    fn update_color(&mut self, now: Instant) {
        if let ActivationColor::Changing {
            start_color,
            target_color_1,
            target_color_2,
            start_time,
            duration,
        } = self.color_change
        {
            let elapsed = now.duration_since(start_time).as_secs_f32();

            if elapsed >= duration {
                self.color_change = ActivationColor::Idle;
                self.outer_color = target_color_1;
                self.inner_color = target_color_2;
            } else {
                // Simple linear RGB interpolation (not HSV) for one-way color transition
                // Calculate progress (0.0 to 1.0) with easing curve
                let progress = (elapsed / duration).clamp(0.0, 1.0);

                // Linear interpolation in RGB space (avoids HSV color wheel issues)
                self.outer_color = tween::lerp_rgba(start_color, target_color_1, progress);
                self.inner_color = tween::lerp_rgba(start_color, target_color_2, progress);
            }
        }
    }

    pub fn update_movement(&mut self, now: Instant) {
        if let ActivationMovement::Rotating {
            start_angle,
            start_time,
            duration,
        } = self.movement
        {
            let t = now.duration_since(start_time).as_secs_f32();
            if t >= duration {
                self.movement = ActivationMovement::Done;
                self.angle = start_angle + (TAU * self.direction.multiplier());
            } else {
                // Rotate from start_angle through one full rotation (TAU radians)
                // Apply direction multiplier: positive for counterclockwise, negative for clockwise
                let progress = t / duration;
                self.angle = start_angle + (TAU * progress * self.direction.multiplier());
            }
        }
    }

    /// Calculate the position of the inner circle
    fn inner_position(&self, params: &RhythmElementParams) -> Vec2 {
        let outer_radius = params.current_radius;
        let inner_radius = self.inner_radius(params);

        // Position the inner circle so it touches the inside of the outer circle
        // Distance from outer center to inner center = outer_radius - inner_radius
        let orbit_radius = outer_radius - inner_radius;

        // Calculate the offset based on the current angle
        let x_offset = orbit_radius * self.angle.cos();
        let y_offset = orbit_radius * self.angle.sin();

        params.current_position + vec2(x_offset, y_offset)
    }

    /// Calculate the inner circle radius
    fn inner_radius(&self, params: &RhythmElementParams) -> f32 {
        params.current_radius * 0.5
    }

    pub fn draw(&self, draw: &Draw, params: &RhythmElementParams) {
        self.draw_outer(draw, params);
        self.draw_inner(draw, params);
    }

    pub fn draw_outer(&self, draw: &Draw, params: &RhythmElementParams) {
        draw.ellipse()
            .xy(params.current_position)
            .radius(params.current_radius)
            .color(self.outer_color);
    }

    pub fn draw_inner(&self, draw: &Draw, params: &RhythmElementParams) {
        draw.ellipse()
            .xy(self.inner_position(params))
            .radius(self.inner_radius(params))
            .color(self.inner_color);
    }
}
