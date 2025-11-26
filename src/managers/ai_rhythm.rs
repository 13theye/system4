use crate::{
    services::openai::schema::response::OpenAIOutputItem, services::openai::OpenAIService,
    settings::OpenAIServiceConfig,
};

use serde_json::Value;

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
    /// Returns a Vec of all text strings found in "Output Message" and
    /// "Reasoning" objects in the `output` array of the ResponseObject.
    pub fn poll_openai(&mut self) -> Vec<OpenAIOutputItem> {
        let mut output = Vec::new();

        if let Some(response) = self.ai_service.try_recv() {
            match response.output {
                Some(Value::Array(items)) => {
                    println!(
                        "AIRhythm: received OpenAI response with {} output item(s)",
                        items.len()
                    );

                    for item_val in items {
                        // Push the output item into the output vector if valid
                        match serde_json::from_value::<OpenAIOutputItem>(item_val) {
                            Ok(output_item) => {
                                output.push(output_item);
                            }

                            Err(e) => {
                                println!(
                                    "AIRhythm: failed to parse output item into OpenAIOutputItem: {}",
                                    e
                                );
                            }
                        }
                    }
                }
                Some(other) => {
                    println!(
                        "AIRhythm: received OpenAI response with non-array output: {:?}",
                        other
                    );
                }
                None => {
                    println!("AIRhythm: received OpenAI response with no output field");
                }
            }
        }

        output
    }
}
