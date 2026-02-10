//! src/view/rhythm2/activation.rs
//!
//! Handles activation of RhythmElements

use nannou::prelude::*;
use std::time::Instant;

use super::{animation::*, element::RhythmElementParams};
use crate::utils::tween;

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
        target_color: Rgba,
        start_time: Instant,
        duration: f32,
    },
}

#[derive(Copy, Clone, Debug)]
pub struct ActivationElement {
    pub angle: f32,
    pub color: Rgba,
    pub movement: ActivationMovement,
    pub color_change: ActivationColor,
}

impl ActivationElement {
    pub fn new(tempo: f64, element_params: &RhythmElementParams, now: Instant) -> Self {
        let rotation_duration = (60.0 / tempo) as f32 * element_params.slot.length;

        let start_angle = 0.0;
        let start_color = element_params.color;
        let target_color = element_params.gradient_color_1;
        let color_duration = rotation_duration * COLOR_CHANGE_FRACTION;

        Self {
            angle: start_angle,
            color: start_color,
            movement: ActivationMovement::Rotating {
                start_angle,
                start_time: now,
                duration: rotation_duration,
            },
            color_change: ActivationColor::Changing {
                start_color,
                target_color,
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
            target_color,
            start_time,
            duration,
        } = self.color_change
        {
            let elapsed = now.duration_since(start_time).as_secs_f32();

            if elapsed >= duration {
                self.color_change = ActivationColor::Idle;
                self.color = target_color;
            } else {
                // Simple linear RGB interpolation (not HSV) for one-way color transition
                // Calculate progress (0.0 to 1.0) with easing curve
                let progress = (elapsed / duration).clamp(0.0, 1.0);

                // Linear interpolation in RGB space (avoids HSV color wheel issues)
                let r = tween::lerp(start_color.red, target_color.red, progress);
                let g = tween::lerp(start_color.green, target_color.green, progress);
                let b = tween::lerp(start_color.blue, target_color.blue, progress);

                self.color = rgba(r, g, b, 1.0);
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
            }
            // else: rotate (todo)
        }
    }

    pub fn draw_big_circle(&self, draw: &Draw, params: &RhythmElementParams) {
        draw.ellipse()
            .xy(params.current_position)
            .radius(params.current_radius)
            .rotate(self.angle)
            .color(self.color);
    }
}
