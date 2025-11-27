//use nannou::rand::{rngs::ThreadRng, seq::SliceRandom, Rng};
use prat::BeatSubdivision;

use crate::{
    services::openai::schema::{RhythmObject, RhythmSlotObject, SequenceObject},
    terminals::commands::rhythm::RangeSize,
};

/// Defines the velocity, length, and cutoffparameters of a single sequencer step. Each parameter is a number from 0.0 to 1.0.
#[derive(Debug, Clone, Copy)]
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

    pub fn to_serializable_object(&self) -> RhythmObject {
        let mut content = Vec::with_capacity(self.capacity);

        for i in 0..self.capacity {
            let slot = RhythmSlotObject {
                index: i,
                velocity: self.slot_params[i].velocity,
                length: self.slot_params[i].length,
                cutoff: self.slot_params[i].cutoff,
            };
            content.insert(i, slot);
        }

        let rhythm = self.as_test_ai_rhythm();

        let sequence = SequenceObject { rhythm, content };

        RhythmObject {
            capacity: self.capacity,
            sequence,
            poem: None,
        }
    }

    /// Simpler test function that gathers filled slots for LLM
    pub fn as_test_ai_rhythm(&self) -> String {
        let mut output = String::from("[");
        for i in 0..self.capacity {
            if self.wings.contains(&i) {
                output.push('X');
            } else {
                output.push('O');
            }
        }
        output.push(']');

        output
    }

    pub fn from_rhythm_response_object(object: RhythmObject) -> Self {
        use std::collections::HashMap;

        // Derive capacity from the rhythm string itself so we always match the
        // number of beat positions (including both X and O), regardless of how
        // many entries the model puts into `content`.
        let rhythm_chars: Vec<char> = object
            .sequence
            .rhythm
            .chars()
            .filter(|c| *c == 'X' || *c == 'O')
            .collect();

        // Start from defaults so we inherit sensible subdivision and ranges,
        // then override capacity and fill slot/wings data from the sequence.
        let mut output = RhythmParams {
            capacity: object.capacity,
            slot_params: Vec::with_capacity(object.capacity),
            ..Default::default()
        };

        // Build a lookup table from index -> slot parameters so we can handle
        // sparse `content` arrays where only active slots are provided.
        let mut slot_map: HashMap<usize, RhythmSlotParams> = HashMap::new();
        for slot in &object.sequence.content {
            slot_map.insert(
                slot.index,
                RhythmSlotParams {
                    velocity: slot.velocity,
                    length: slot.length,
                    cutoff: slot.cutoff,
                },
            );
        }

        // For every beat position 0..capacity, either take the model-provided
        // parameters or fall back to a neutral default.
        for i in 0..object.capacity {
            if let Some(params) = slot_map.get(&i) {
                output.slot_params.push(*params);
            } else {
                output.slot_params.push(RhythmSlotParams {
                    velocity: 0.5,
                    length: 0.5,
                    cutoff: 0.5,
                });
            }
        }

        // Wings (active slots) are determined purely by the rhythm string.
        output.wings.clear();
        for (i, c) in rhythm_chars.iter().enumerate() {
            if *c == 'X' {
                output.wings.push(i);
            }
        }

        output.num_wings = output.wings.len();
        output
    }
}
