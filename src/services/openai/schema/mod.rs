// src/services/openai/schema/mod.rs
// App-specific types for use with the OpenAI API but are not part of the API itself

pub mod request;
pub mod response;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename = "rhythm_response_object_schema")]
/// Struct describing the JSON response schema desired from OpenAI API
pub struct RhythmObject {
    pub capacity: usize,
    pub sequence: SequenceObject,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poem: Option<String>,
}

/// Serializable sequencer step. Each parameter is a number from 0.0 to 1.0.
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
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct SequenceObject {
    pub rhythm: String,
    pub content: Vec<RhythmSlotObject>,
}
