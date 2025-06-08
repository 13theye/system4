// src/view/masks.rs
//
// This module defines the masks that can be used with the ParticleSystem.

use crate::view::Voice;
use nannou::prelude::*;

const SIDE_MARGIN: f32 = 100.0;
const TOP_BOTTOM_MARGIN: f32 = 300.0;

#[derive(Debug, Clone, PartialEq)]
pub struct Mask {
    pub voice: Voice,
    pub origin: Vec2,
    pub size: Vec2,
    pub rect: Rect,
}

impl Mask {
    pub fn make_drone(voice: Voice) -> Self {
        let origin = match voice {
            Voice::Voice1 => vec2(-1280.0, 200.0),
            Voice::Voice4 => vec2(1280.0, 200.0),
            _ => vec2(0.0, 0.0),
        };

        let size = vec2(800.0, 1200.0);
        let rect = Rect::from_x_y_w_h(origin.x, origin.y, size.x, size.y);
        Self {
            voice,
            origin,
            size,
            rect,
        }
    }

    pub fn full_screen(voice: Voice) -> Self {
        let origin = vec2(0.0, 0.0);
        let size = vec2(3840.0, 2160.0);
        let rect = Rect::from_x_y_w_h(origin.x, origin.y, size.x, size.y);
        Self {
            voice,
            origin,
            size,
            rect,
        }
    }

    pub fn contains(&self, point: Vec2) -> bool {
        self.rect.contains(point)
    }
}

pub enum MaskType {
    Drone,
    FullScreen,
}
