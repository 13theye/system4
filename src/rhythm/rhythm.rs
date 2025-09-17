// src/rhythm/rhythm.rs
//
// Rhythm

use crate::groups::Voice;
use nannou::{
    prelude::*,
    rand::{rngs::ThreadRng, Rng},
};
use std::collections::HashSet;

pub struct Rhythm {
    pub origin: Vec2,
    pub activated_slots: HashSet<usize>, // indices of the activated slots, 1-indexed

    pub params: RhythmParams,
}

impl Rhythm {
    pub fn new(params: RhythmParams) -> Self {
        Self {
            origin: Vec2::new(0.0, 0.0),
            activated_slots: HashSet::new(),
            params,
        }
    }

    pub fn make_new_rhythm(&mut self, rnd: &mut ThreadRng) {
        for i in 1..=self.params.capacity {
            if rnd.gen_range(0.0..1.0) < 0.5 {
                self.activated_slots.insert(i);
            }
        }
    }

    pub fn set_capacity(&mut self, capacity: usize) {
        self.params.capacity = capacity;
    }
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

pub struct VisualElement {
    pub position: Vec2,
    pub rotation: f32, // rotation in degrees
    pub size: Vec2,
    pub default_color: Rgba,
    pub activated_color: Rgba,
    pub color: Rgba,
}

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
