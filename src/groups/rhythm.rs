// src/groups/rhythm.rs

use nannou::rand::{rngs::ThreadRng, Rng};
use prat::BeatSubdivision;

use crate::{
    groups::{VoiceId, VoiceParams},
    particle::emitter::Emitter,
};

#[derive(Debug, Clone)]
pub struct RhythmParams {
    pub capacity: usize,
    pub num_wings: usize,
    pub subdivision: BeatSubdivision,
    pub wings: Option<Vec<usize>>,
}

impl Default for RhythmParams {
    fn default() -> Self {
        Self {
            capacity: 0,
            num_wings: 0,
            subdivision: BeatSubdivision::Eighth,
            wings: None,
        }
    }
}

pub struct Rhythm {
    pub id: VoiceId,
    params: RhythmParams,
    pub voice_params: VoiceParams,
    pub emitters: Vec<Box<dyn Emitter>>,
}

impl Rhythm {
    pub fn new_with_id(id: VoiceId) -> Self {
        Self {
            id,
            params: RhythmParams::default(),
            voice_params: VoiceParams::default(),
            emitters: Vec::new(),
        }
    }

    pub fn get_params(&self) -> &RhythmParams {
        &self.params
    }

    pub fn set_capacity(&mut self, capacity: usize) {
        self.params.capacity = capacity;
    }

    pub fn set_num_wings(&mut self, num_wings: usize) {
        self.params.num_wings = num_wings;
    }

    pub fn set_subdivision(&mut self, subdivision: BeatSubdivision) {
        self.params.subdivision = subdivision;
    }

    pub fn set_wings(&mut self, rng: &mut ThreadRng) {
        self.params.wings = Some(Rhythm::roll_wings(
            rng,
            self.params.capacity,
            self.params.num_wings,
        ));
    }

    fn roll_wings(rng: &mut ThreadRng, capacity: usize, num_wings: usize) -> Vec<usize> {
        let mut wings = Vec::new();
        for _ in 0..num_wings {
            wings.push(rng.gen_range(0..capacity));
        }
        wings
    }
}
