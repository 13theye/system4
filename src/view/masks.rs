// src/view/masks.rs
//
// This module defines the masks that can be used with the ParticleSystem.

use nannou::prelude::*;

const SIDE_MARGIN: f32 = 100.0;
const TOP_BOTTOM_MARGIN: f32 = 300.0;

pub struct Mask {
    pub origin: Vec2,
    pub size: Vec2,
    pub rect: Rect,
}

impl Mask {
    pub fn make_drone_1() -> Self {
        let origin = vec2(-605.0, 0.0);
        let size = vec2(500.0, 1000.0);
        let rect = Rect::from_x_y_w_h(origin.x, origin.y, size.x, size.y);
        Self { origin, size, rect }
    }

    pub fn full_screen() -> Self {
        let origin = vec2(0.0, 0.0);
        let size = vec2(3840.0, 2160.0);
        let rect = Rect::from_x_y_w_h(origin.x, origin.y, size.x, size.y);
        Self { origin, size, rect }
    }

    pub fn contains(&self, point: Vec2) -> bool {
        self.rect.contains(point)
    }
}
