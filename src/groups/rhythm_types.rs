//use nannou::rand::{rngs::ThreadRng, seq::SliceRandom, Rng};
use prat::BeatSubdivision;
use serde::{Deserialize, Serialize};

use crate::terminals::commands::rhythm::RangeSize;

/// Defines the parameters of a single rhythm firing
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct RhythmSlot {
    pub velocity: f32,
    pub length: f32,
    pub cutoff: f32,
}

#[derive(Debug, Clone)]
pub struct RhythmParams {
    // Total number of beat subdivisions / slots in this rhythm
    pub capacity: usize,
    // Number of filled slots out of the total capacity
    pub num_wings: usize,
    // The beat subdivision length of each slot
    pub subdivision: BeatSubdivision,
    // The possible range of the length parameter
    pub length_range: RangeSize,
    // The possible range of the velocity parameter
    pub velocity_range: RangeSize,
    // The possible range of the cutoff parameter
    pub cutoff_range: RangeSize,
    // Which slots are currently "filled", represented as indices
    pub wings: Vec<usize>,
    // A LIFO buffer of wings that have been removed. Facilitates undo
    pub wings_buffer: Vec<usize>,
    // The OSC parameters to be sent for each slot
    pub slots: Vec<RhythmSlot>,
}

impl Default for RhythmParams {
    fn default() -> Self {
        Self {
            capacity: 0,
            num_wings: 0,
            subdivision: BeatSubdivision::Eighth,
            length_range: RangeSize::default(),
            velocity_range: RangeSize::default(),
            cutoff_range: RangeSize::default(),
            wings: Vec::new(),
            wings_buffer: Vec::new(),
            slots: Vec::new(),
        }
    }
}

impl RhythmParams {
    pub fn new(
        capacity: usize,
        num_wings: usize,
        subdivision: BeatSubdivision,
        length_range: RangeSize,
        velocity_range: RangeSize,
        pitch_range: RangeSize,
    ) -> Self {
        Self {
            capacity,
            num_wings,
            subdivision,
            length_range,
            velocity_range,
            cutoff_range: pitch_range,
            wings: Vec::new(),
            wings_buffer: Vec::new(),
            slots: Vec::new(),
        }
    }
}

/// A serializable version of `RhythmSlot`
pub type RhythmSlotIndexed = (usize, RhythmSlot);
