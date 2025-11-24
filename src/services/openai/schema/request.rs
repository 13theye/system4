// src/services/openai/schema/request.rs
//
// Request types for OpenAI Responses API according to schema at
// https://platform.openai.com/docs/api-reference/responses/create

use serde::{Deserialize, Serialize};

/// Main request body for creating a response.
///
/// This is a minimal subset of the full API; many optional fields are omitted
/// because they are not used by the application.
#[derive(Debug, Serialize, Deserialize)]
pub struct OpenAIRequest {
    pub model: OpenAIModelName,
    /// Convenience shorthand: a simple string input.
    /// The full API also supports structured input items, which are not modeled here.
    pub input: String,
    /// System-level instructions for the model.
    pub instructions: String,
    /// Optional reasoning configuration; omit to disable additional reasoning.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<OpenAIReasoningConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum OpenAIModelName {
    #[serde(rename = "openai/gpt-oss-20b")]
    GptOss20b,
}

/// Reasoning configuration for the request.
#[derive(Debug, Serialize, Deserialize)]
pub struct OpenAIReasoningConfig {
    pub effort: OpenAIReasoningEffort,
}

/// Level of reasoning effort requested from the model.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpenAIReasoningEffort {
    Low,
    Medium,
    High,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn serializes_request_without_reasoning() {
        let req = OpenAIRequest {
            model: OpenAIModelName::GptOss20b,
            input: "hello".to_string(),
            instructions: "Be concise.".to_string(),
            reasoning: None,
        };

        let value = serde_json::to_value(&req).expect("failed to serialize request");

        assert_eq!(value["model"], json!("openai/gpt-oss-20b"));
        assert_eq!(value["input"], json!("hello"));
        assert_eq!(value["instructions"], json!("Be concise."));
        assert!(value.get("reasoning").is_none());
    }

    #[test]
    fn serializes_request_with_reasoning_effort() {
        let req = OpenAIRequest {
            model: OpenAIModelName::GptOss20b,
            input: "hello".to_string(),
            instructions: "Be concise.".to_string(),
            reasoning: Some(OpenAIReasoningConfig {
                effort: OpenAIReasoningEffort::High,
            }),
        };

        let value = serde_json::to_value(&req).expect("failed to serialize request with reasoning");

        assert_eq!(value["reasoning"]["effort"], json!("high"));
    }
}
