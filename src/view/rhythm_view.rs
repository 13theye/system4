// src/views/rhythm_view.rs
//
// Visualization of a rhythm

use nannou::prelude::*;

use crate::groups::Rhythm;

pub struct RhythmShape {
    beat_idx: usize,
    center: Vec2,
    base_size: Vec2,
    base_color: Rgb,
    base_alpha: f32,
}

pub struct RhythmFormation {
    center: Vec2,
    slots: Vec<RhythmShape>,
}

impl RhythmFormation {
    pub fn new(center: Vec2, capacity: usize) -> Self {
        Self {
            center,
            slots: Vec::with_capacity(capacity),
        }
    }
}
