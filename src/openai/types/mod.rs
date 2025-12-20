// src/services/openai/schema/mod.rs
// App-specific types for use with the OpenAI API but are not part of the API itself

pub mod request_helpers;
pub mod response;
pub mod stream;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename = "rhythm_object_schema")]
/// Struct describing the JSON response schema desired from OpenAI API.
/// Construct this object's fields in this order: 1. thought_process, 2. sequence, 3. subdivision, 4. dist, 5. capacity, 6. feeling.
/// thought_process: detailed description of the intention behind the sequence. Use present continuous tense.
/// capacity: the number of slots in the sequence.
/// subdivision: the beat subdivision used to advance this sequence.
/// sequence: the sequence object.
/// feeling: a haiku describing the thinking behind the sequence.
pub struct RhythmObject {
    pub thought_process: String,
    pub capacity: usize,
    pub subdivision: SubdivisionObject,
    pub sequence: SequenceObject,
    pub feeling: FeelingObject,
}

/// Serializable beat subdivision enum.
/// Describes the beat subdivision that advances the sequence.
/// Allowable values: "quarter", "eighth", "sixteenth", "triplet"
#[derive(Debug, Clone, Copy, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum SubdivisionObject {
    Quarter,
    Eighth,
    Sixteenth,
    Triplet,
    #[serde(other)]
    Invalid,
}

impl SubdivisionObject {
    pub fn from_beat_subdivision_u8(subdivision: u8) -> SubdivisionObject {
        match subdivision {
            3 => SubdivisionObject::Quarter,
            4 => SubdivisionObject::Eighth,
            5 => SubdivisionObject::Sixteenth,
            0 => SubdivisionObject::Triplet,
            _ => SubdivisionObject::Invalid,
        }
    }
}

/// Serializable sequencer step.
/// i: the index of the sequencer step, beginning at 0
/// vel: velocity parameter for external sound engine, range: 0.0 to 1.0. Truncate to 3 decimal places.
/// len: note length parameter for external sound engine, range: 0.0 to 1.0. Truncate to 3 decimal places.
/// cut: cutoff parameter for external sound engine, range: 0.0 to 1.0. Truncate to 3 decimal places.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct SequenceParametersObject {
    #[serde(rename = "i")]
    pub index: usize,
    #[serde(rename = "v")]
    pub velocity: f32,
    #[serde(rename = "l")]
    pub length: f32,
    #[serde(rename = "c")]
    pub cutoff: f32,
}

/// Serializable sequence.
/// rhythm: the rhythm of the sequence, expressed as a string of "X" and "-" characters. Length of the string should be equal to capacity of the sequence. X = note on, _ = note off.
/// parameters: array of sound engine parameters for each sequencer step
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct SequenceObject {
    pub rhythm: String,
    pub parameters: Vec<SequenceParametersObject>,
}

impl SequenceObject {
    pub fn clean_rhythm_string(&self) -> Vec<char> {
        self.rhythm
            .chars()
            .filter(|c| *c == 'X' || *c == '-')
            .collect()
    }
}

/// The haiku describing the thinking behind the sequence.
/// Each line of the poem is a string.
#[derive(Default, Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct FeelingObject {
    pub text: Vec<String>,
}

// Conversion implementations for rhythm types
use crate::groups::RhythmParams;
use prat::BeatSubdivision;

impl From<&RhythmParams> for RhythmObject {
    fn from(params: &RhythmParams) -> Self {
        let mut content = Vec::with_capacity(params.capacity);

        let subdivision = SubdivisionObject::from_beat_subdivision_u8(params.subdivision as u8);

        for i in 0..params.capacity {
            let slot = SequenceParametersObject {
                index: i,
                velocity: (params.slot_params[i].velocity * 1000.0).round() / 1000.0,
                length: (params.slot_params[i].length * 1000.0).round() / 1000.0,
                cutoff: (params.slot_params[i].cutoff * 1000.0).round() / 1000.0,
            };
            content.insert(i, slot);
        }

        let rhythm = params.to_rhythm_string();

        let sequence = SequenceObject {
            rhythm,
            parameters: content,
        };

        RhythmObject {
            thought_process: String::from(""),
            capacity: params.capacity,
            subdivision,
            sequence,
            feeling: FeelingObject::default(),
        }
    }
}

impl From<RhythmObject> for RhythmParams {
    fn from(object: RhythmObject) -> Self {
        use std::collections::HashMap;

        let subdivision = match object.subdivision {
            SubdivisionObject::Quarter => BeatSubdivision::Quarter,
            SubdivisionObject::Eighth => BeatSubdivision::Eighth,
            SubdivisionObject::Sixteenth => BeatSubdivision::Sixteenth,
            SubdivisionObject::Triplet => BeatSubdivision::Triplet,
            // Default to eighth
            SubdivisionObject::Invalid => BeatSubdivision::Eighth,
        };

        // Start from defaults so we inherit sensible subdivision and ranges,
        // then override capacity and fill slot/wings data from the sequence.
        let mut output = RhythmParams {
            capacity: object.capacity,
            slot_params: Vec::with_capacity(object.capacity),
            subdivision,
            ..Default::default()
        };

        // Build a lookup table from index -> slot parameters so we can handle
        // sparse `content` arrays where only active slots are provided.
        let mut slot_map: HashMap<usize, crate::groups::RhythmSlotParams> = HashMap::new();
        for slot in &object.sequence.parameters {
            slot_map.insert(
                slot.index,
                crate::groups::RhythmSlotParams {
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
                output.slot_params.push(crate::groups::RhythmSlotParams {
                    velocity: 0.5,
                    length: 0.5,
                    cutoff: 0.5,
                });
            }
        }

        // Derive capacity from the rhythm string itself so we always match the
        // number of beat positions (including both X and -), regardless of how
        // many entries the model puts into `content`.
        let rhythm_chars: Vec<char> = object.sequence.clean_rhythm_string();
        output.wings.clear();

        for (i, c) in rhythm_chars.iter().enumerate() {
            if *c == 'X' {
                output.wings.push(i);
            }
        }

        // Wings (active slots) are determined purely by the rhythm string.
        output.num_wings = output.wings.len();
        output
    }
}
