use schemars::{schema_for, Schema};
use serde::{Deserialize, Serialize};

use crate::services::openai::schema::RhythmResponseObject;

/// The "text" field of an OpenAI RequestObject
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename = "text")]
pub struct TextObject {
    pub format: Option<FormatObject>,
    pub verbosity: Option<VerbosityLevel>,
}

impl Default for TextObject {
    fn default() -> Self {
        TextObject {
            format: Some(FormatObject::default()),
            verbosity: None,
        }
    }
}

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

impl Default for FormatObject {
    fn default() -> Self {
        let schema = schema_for!(RhythmResponseObject);
        let description = String::from("The schema describes parameters needed to create a rhythm sequence. The field \"capacity\" is the length of the sequence. The field \"wings\" is an array containing the 0-indexed positions of active notes in the sequence. The fields \"velocity\", \"length\", and \"cutoff\" are each arrays of parameters corresponding to notes in the rhythm sequence. Each parameter is a number from 0.0 to 1.0. Finally, the \"haiku\" field is a string containing the haiku that you are asked to generate in the prompt.");

        Self {
            typ: "json_schema".to_owned(),
            name: "rhythm_response_object_schema".to_owned(),
            schema,
            description: Some(description),
            strict: Some(true),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub enum VerbosityLevel {
    #[serde(rename = "low")]
    Low,
    #[serde(rename = "medium")]
    Medium,
    #[serde(rename = "high")]
    High,
}
