//use nannou::rand::{rngs::ThreadRng, seq::SliceRandom, Rng};
use prat::BeatSubdivision;

use crate::terminals::commands::rhythm::RangeSize;

/// Defines the velocity, length, and cutoffparameters of a single sequencer step. Each parameter is a number from 0.0 to 1.0.
#[derive(Default, Debug, Clone, Copy)]
pub struct RhythmSlotParams {
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
    pub slot_params: Vec<RhythmSlotParams>,
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
            slot_params: Vec::new(),
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
            slot_params: Vec::new(),
        }
    }

    /// Simpler test function that gathers filled slots for LLM
    pub fn to_rhythm_string(&self) -> String {
        let mut output = String::new();
        for i in 0..self.capacity {
            if self.wings.contains(&i) {
                output.push('X');
            } else {
                output.push('-');
            }
        }

        output
    }
}
