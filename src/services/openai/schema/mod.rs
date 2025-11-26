// src/services/openai/schema/mod.rs

pub mod request;
pub mod response;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename = "rhythm_response_object_schema")]
/// Struct describing the JSON response schema desired from OpenAI API
pub struct RhythmResponseObject {
    pub capacity: usize,
    pub wings: Vec<usize>,
    pub velocity: Vec<f32>,
    pub length: Vec<f32>,
    pub cutoff: Vec<f32>,
    pub haiku: String,
}
