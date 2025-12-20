// Helpers for generating OpenAI Response types described in the crate async-openai

use async_openai::types::responses::{
    Reasoning, ReasoningEffort, ResponseFormatJsonSchema, ResponseTextParam,
    TextResponseFormatConfiguration,
};
use schemars::schema_for;

use crate::openai::types::RhythmObject;

/// Helper function to allow default generation of ResponseTextParam for this app.
pub fn response_text_params_for_system4_schema(
    schema_description: Option<String>,
) -> ResponseTextParam {
    ResponseTextParam {
        format: TextResponseFormatConfiguration::JsonSchema(
            response_format_json_schema_for_rhythm_object(schema_description),
        ),
        verbosity: None,
    }
}

/// Helper function for generatiing a ResponseFormatJsonSchema for this app.
fn response_format_json_schema_for_rhythm_object(
    description: Option<String>,
) -> ResponseFormatJsonSchema {
    let schema = Some(schema_for!(RhythmObject).into());

    ResponseFormatJsonSchema {
        description,
        name: "rhythm_response_object_schema".to_owned(),
        schema,
        strict: Some(true),
    }
}

/// Helper function to allow default generation of ReasoningConfig for this app.
pub fn reasoning_config() -> Reasoning {
    Reasoning {
        effort: Some(ReasoningEffort::Low),
        summary: None,
    }
}
