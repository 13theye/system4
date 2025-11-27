// Extension for OpenAI RequestObject types described in the create openai-api-rs

use schemars::{schema_for, Schema};
use serde::{Deserialize, Serialize};

use crate::services::openai::schema::RhythmObject;

/// The "text" field of an OpenAI RequestObject, allowing for response format definition
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename = "text")]
pub struct TextObject {
    pub format: Option<FormatObject>,
    pub verbosity: Option<VerbosityLevel>,
}

impl TextObject {
    pub fn new(schema_description: Option<String>) -> Self {
        TextObject {
            format: Some(FormatObject::new(schema_description)),
            verbosity: None,
        }
    }
}

/// The "format" field of an OpenAI RequestObject, containing JSON schema definition
#[derive(Debug, Deserialize, Serialize)]
pub struct FormatObject {
    #[serde(rename = "type")]
    pub typ: String,
    pub name: String,
    pub schema: Schema,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

impl FormatObject {
    pub fn new(description: Option<String>) -> Self {
        let schema = schema_for!(RhythmObject);

        Self {
            typ: "json_schema".to_owned(),
            name: "rhythm_response_object_schema".to_owned(),
            schema,
            description,
            strict: Some(true),
        }
    }
}

/// Reasoning verbosity level
#[derive(Debug, Deserialize, Serialize)]
pub enum VerbosityLevel {
    #[serde(rename = "low")]
    Low,
    #[serde(rename = "medium")]
    Medium,
    #[serde(rename = "high")]
    High,
}
