use crate::{
    services::openai::OpenAIService,
    settings::OpenAIServiceConfig,
};

pub struct AIRhythm {
    ai_service: OpenAIService,
}

impl AIRhythm {
    pub fn new(config: &OpenAIServiceConfig) -> Self {
        let ai_service = OpenAIService::new(config);

        Self { ai_service }
    }

    /// Polls OpenAIService for any completed responses.
    ///
    /// This is a placeholder hook; integration with AIRhythm behavior
    /// can be added once response handling is defined.
    pub fn poll_openai(&mut self) {
        if let Some(response) = self.ai_service.try_recv() {
            println!(
                "AIRhythm: received OpenAI response with {} output item(s)",
                response.output.len()
            );
        }
    }
}
