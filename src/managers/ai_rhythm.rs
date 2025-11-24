use crate::{
    services::openai::{self, OpenAIService},
    settings::OpenAIServiceConfig,
};

use openai::schema::response::{OpenAIOutputContent, OpenAIOutputItem};

pub struct AIRhythm {
    ai_service: OpenAIService,
}

impl AIRhythm {
    pub fn new(config: &OpenAIServiceConfig) -> Self {
        let ai_service = OpenAIService::new(config);

        Self { ai_service }
    }

    /// Send a request to OpenAI.
    pub fn send_openai(&mut self, rhythm: &str) {
        println!("AIRhythm: starting OpenAI request");
        self.ai_service.send(rhythm).unwrap();
    }

    /// Polls OpenAIService for any completed responses and extracts text content.
    ///
    /// Returns a Vec of all OutputText strings found in the response. The
    /// caller is responsible for interpreting the content.
    pub fn poll_openai(&mut self) -> Vec<String> {
        let mut texts = Vec::new();

        if let Some(response) = self.ai_service.try_recv() {
            println!(
                "AIRhythm: received OpenAI response with {} output item(s)",
                response.output.len()
            );

            for item in response.output {
                if let OpenAIOutputItem::Message(message) = item {
                    for content in message.content {
                        if let OpenAIOutputContent::OutputText { text } = content {
                            texts.push(text);
                        }
                    }
                }
            }
        }

        texts
    }
}
