// Extension for OpenAI Response types described in the crate async-openai

use async_openai::types::responses::{
    ReasoningConfig, ReasoningEffort, ResponseFormatJsonSchema, TextConfig, TextResponseFormat,
};
use schemars::schema_for;

use crate::services::openai::schema::RhythmObject;

pub trait TextConfigExt {
    fn generate_for_system4_schema(schema_description: Option<String>) -> TextConfig;
}

impl TextConfigExt for TextConfig {
    fn generate_for_system4_schema(schema_description: Option<String>) -> Self {
        Self {
            format: TextResponseFormat::JsonSchema(
                ResponseFormatJsonSchema::generate_for_rhythm_object(schema_description),
            ),
            verbosity: None,
        }
    }
}

/// Extension trait to allow for generatiing a ResponseFormatJsonSchema for this app.
pub trait ResponseFormatJsonSchemaExt {
    fn generate_for_rhythm_object(schema_description: Option<String>) -> ResponseFormatJsonSchema;
}

impl ResponseFormatJsonSchemaExt for ResponseFormatJsonSchema {
    /// Generate a ResponseFormatJsonSchema for the RhythmObject
    fn generate_for_rhythm_object(description: Option<String>) -> Self {
        let schema = Some(schema_for!(RhythmObject).into());

        Self {
            description,
            name: "rhythm_response_object_schema".to_owned(),
            schema,
            strict: Some(true),
        }
    }
}

/// Extension trait to allow default generation of ReasoningConfig for this app.
pub trait ReasoningConfigExt {
    fn generate() -> ReasoningConfig;
}

impl ReasoningConfigExt for ReasoningConfig {
    fn generate() -> Self {
        Self {
            effort: Some(ReasoningEffort::Low),
            summary: None,
        }
    }
}
