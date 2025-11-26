// src/services/openai/schema/mod.rs

pub mod request;
pub mod response;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::groups::rhythm_types::Sequence;

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename = "rhythm_response_object_schema")]
/// Struct describing the JSON response schema desired from OpenAI API
pub struct RhythmResponseObject {
    pub capacity: usize,
    pub sequence: Sequence,
    pub haiku: String,
}
