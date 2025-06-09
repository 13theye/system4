// src/rhythm/rhythm.rs
//
// Rhythm

use crate::view::Voice;
use nannou::prelude::*;

pub enum Interval {
    Eighth,
    Sixteenth,
}

#[derive(Default)]
pub enum RhythmMovement {
    Stand,
    Wave,
    #[default]
    Circle,
    Comeback,
}

pub struct RhythmParams {
    pub voice: Voice,
    pub interval: Interval,
    pub capacity: usize,
    pub wings: usize,
    pub length: f32,
    pub volume: f32,
    pub guess: f32,
    pub movement: RhythmMovement,
}

pub struct RhythmSlot {
    pub position: Vec2,
    pub rotation: f32, // rotation in degrees
    pub size: Vec2,
    pub default_color: Rgba,
    pub activated_color: Rgba,
    pub color: Rgba,
}

pub struct Rhythm {
    pub origin: Vec2,
    pub slots: Vec<RhythmSlot>,

    pub params: RhythmParams,
}
