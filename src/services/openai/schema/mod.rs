// src/services/openai/schema/mod.rs
// App-specific types for use with the OpenAI API but are not part of the API itself

pub mod request;
pub mod response;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename = "rhythm_response_object_schema")]
/// Struct describing the JSON response schema desired from OpenAI API
/// capacity: the number of slots in the sequence
/// sequence: the sequence object
/// poem: a haiku describing the thinking behind the sequence
pub struct RhythmObject {
    pub capacity: usize,
    pub sequence: SequenceObject,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poem: Option<String>,
}

/// Serializable sequencer step.
/// i: the index of the sequencer step, beginning at 0
/// v: velocity parameter for external sound engine, range: 0.0 to 1.0
/// l: note length parameter for external sound engine, range: 0.0 to 1.0
/// c: cutoff parameter for external sound engine, range: 0.0 to 1.0
#[derive(Debug, Clone, Copy, Deserialize, Serialize, JsonSchema)]
pub struct RhythmSlotObject {
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
/// rhythm: the rhythm of the sequence, expressed as a string of "X" and "O" characters. X = note on, O = note off.
/// content: array of sequencer steps
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct SequenceObject {
    pub rhythm: String,
    pub content: Vec<RhythmSlotObject>,
}
