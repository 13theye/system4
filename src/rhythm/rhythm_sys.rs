// src/rhythm/rhythm_sys.rs
//
// Rhythm system

use nannou::{prelude::*, rand::rngs::ThreadRng};
use std::collections::HashMap;

use crate::{rhythm::Rhythm, groups::Voice};

pub struct RhythmSystem {
    pub rhythms: HashMap<Voice, Rhythm>,

    pub rnd: ThreadRng,
}

impl RhythmSystem {
    pub fn new() -> Self {
        Self {
            rhythms: HashMap::new(),
            rnd: ThreadRng::default(),
        }
    }
}

impl Default for RhythmSystem {
    fn default() -> Self {
        Self::new()
    }
}
