// src/services/openai/schema/mod.rs
// App-specific types for use with the OpenAI API but are not part of the API itself

pub mod request;
pub mod response;
pub mod stream;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename = "rhythm_object_schema")]
/// Struct describing the JSON response schema desired from OpenAI API.
/// Construct this object's fields in this order: 1. thought_process, 2. sequence, 3. capacity, 4. feeling
/// thought_process: detailed description of the intention behind the sequence. Use present continuous tense.
/// sequence: the sequence object
/// capacity: the number of slots in the sequence
/// summary: a haiku describing the thinking behind the sequence.
pub struct RhythmObject {
    pub thought_process: String,
    pub capacity: usize,
    pub sequence: SequenceObject,
    pub feeling: FeelingObject,
}

/// Serializable sequencer step.
/// i: the index of the sequencer step, beginning at 0
/// vel: velocity parameter for external sound engine, range: 0.0 to 1.0. Truncate to 3 decimal places.
/// len: note length parameter for external sound engine, range: 0.0 to 1.0. Truncate to 3 decimal places.
/// cut: cutoff parameter for external sound engine, range: 0.0 to 1.0. Truncate to 3 decimal places.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, JsonSchema)]
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
/// rhythm: the rhythm of the sequence, expressed as a string of "X" and "_" characters. Length of the string should be equal to capacity of the sequence. X = note on, _ = note off.
/// parameters: array of sound engine parameters for each sequencer step
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct SequenceObject {
    pub rhythm: String,
    pub parameters: Vec<SequenceParametersObject>,
}

/// The haiku describing the thinking behind the sequence.
/// Each line of the poem is a string.
#[derive(Default, Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct FeelingObject {
    pub text: Vec<String>,
}
