// src/rhythm/rhythm_sys.rs
//
// Rhythm system

use std::collections::HashMap;

use crate::{rhythm::Rhythm, view::Voice};

pub struct RhythmSystem {
    pub rhythms: HashMap<Voice, Rhythm>,
}

impl RhythmSystem {
    pub fn new() -> Self {
        Self {
            rhythms: HashMap::new(),
        }
    }
}

impl Default for RhythmSystem {
    fn default() -> Self {
        Self::new()
    }
}
