use openai_api_rs::v1::{
    api::OpenAIClient,
    responses::{CreateResponseRequest, ResponseObject},
};
use serde_json::json;

use crate::settings::OpenAIServiceConfig;

pub struct OpenAIService {
    // LLM system prompt
    pub system_prompt: Option<String>,

    // Model name, url
    pub model: String,
    pub url: String,

    pub client: OpenAIClient,
}

impl OpenAIService {
    pub fn new(config: &OpenAIServiceConfig) -> Self {
        let client = OpenAIClient::builder()
            .with_endpoint(config.url.to_owned())
            .build()
            .expect("OpenAIService: Failed to build OpenAI client");

        Self {
            system_prompt: Some(config.system_prompt.to_owned()),
            model: config.model.to_owned(),
            url: config.url.to_owned(),
            client,
        }
    }

    pub async fn send(
        &mut self,
        content: &str,
    ) -> Result<ResponseObject, Box<dyn std::error::Error>> {
        let mut req = CreateResponseRequest::new();
        req.model = Some(self.model.to_owned());
        req.instructions = self.system_prompt.clone();
        req.input = Some(json!(content));

        let response = self.client.create_response(req).await?;

        Ok(response)
    }
}
