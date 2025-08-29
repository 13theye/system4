// src/view/masks.rs
//
// This module defines the masks that can be used with the ParticleSystem.

use crate::voice::Voice;
use nannou::prelude::*;
use std::time::Instant;

const SIDE_MARGIN: f32 = 100.0;
const TOP_BOTTOM_MARGIN: f32 = 300.0;

#[derive(Debug, Clone)]
pub struct Mask {
    pub voice: Voice,
    pub origin: Vec2,
    pub rect: Rect,
    // Animation state
    target_rect: Option<Rect>,
    animation_start_time: Option<Instant>,
    animation_duration: f32,
}

impl Mask {
    pub fn make_drone(voice: Voice) -> Self {
        let origin = match voice {
            Voice::Voice1 => vec2(-1280.0, 200.0),
            //Voice::Voice1 => vec2(0.0, 0.0),
            Voice::Voice4 => vec2(1280.0, 200.0),
            _ => vec2(0.0, 0.0),
        };

        let size = vec2(800.0, 1200.0);
        //let size = vec2(3840.0, 2160.0);
        let rect = Rect::from_x_y_w_h(origin.x, origin.y, size.x, size.y);
        Self {
            voice,
            origin,
            rect,
            target_rect: None,
            animation_start_time: None,
            animation_duration: 0.0,
        }
    }

    pub fn full_screen(voice: Voice) -> Self {
        let origin = vec2(0.0, 0.0);
        let size = vec2(3840.0, 2160.0);
        let rect = Rect::from_x_y_w_h(origin.x, origin.y, size.x, size.y);
        Self {
            voice,
            origin,
            rect,
            target_rect: None,
            animation_start_time: None,
            animation_duration: 0.0,
        }
    }

    pub fn contains(&self, point: Vec2) -> bool {
        self.rect.contains(point)
    }

    /// Initiates a smooth transition to new bounds over the specified duration
    pub fn change_bounds(&mut self, new_bounds: Rect, secs: f32) {
        self.target_rect = Some(new_bounds);
        self.animation_start_time = Some(Instant::now());
        self.animation_duration = secs;
    }

    /// Updates the animation state and returns true if animation is complete
    pub fn update_animation(&mut self) -> bool {
        if let (Some(target), Some(start_time)) = (self.target_rect, self.animation_start_time) {
            let elapsed = start_time.elapsed().as_secs_f32();
            let progress = (elapsed / self.animation_duration).min(1.0);
            
            if progress >= 1.0 {
                // Animation complete
                self.rect = target;
                self.origin = vec2(target.x(), target.y());
                self.target_rect = None;
                self.animation_start_time = None;
                true
            } else {
                // Interpolate between current and target rect
                self.rect = lerp_rect(self.rect, target, progress);
                self.origin = vec2(self.rect.x(), self.rect.y());
                false
            }
        } else {
            true // No animation in progress
        }
    }
}

/// Linear interpolation between two rectangles
fn lerp_rect(start: Rect, end: Rect, t: f32) -> Rect {
    let x = start.x() + (end.x() - start.x()) * t;
    let y = start.y() + (end.y() - start.y()) * t;
    let w = start.w() + (end.w() - start.w()) * t;
    let h = start.h() + (end.h() - start.h()) * t;
    Rect::from_x_y_w_h(x, y, w, h)
}

pub enum MaskType {
    Drone,
    FullScreen,
}
