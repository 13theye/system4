use crate::{
    groups::{RhythmParams, VoiceId},
    services::openai::{schema::RhythmObject, OpenAIService},
    settings::OpenAIServiceConfig,
};

use async_openai::types::responses as openai_response;
use serde_json::Value;

/// High-level, domain-specific result from the AI service: a set of rhythm
/// parameters that should be applied to a particular target voice.
#[derive(Debug, Clone)]
pub struct AiRhythmResult {
    pub target_voice: VoiceId,
    pub params: RhythmParams,
}

pub struct AIRhythm {
    ai_service: OpenAIService,
    /// Voice for which we most recently sent an AI rhythm request.
    /// We assume a single in-flight AI request at a time.
    pending_ai_voice: Option<VoiceId>,
}

impl AIRhythm {
    pub fn new(config: &OpenAIServiceConfig) -> Self {
        let ai_service = OpenAIService::new(config);

        Self {
            ai_service,
            pending_ai_voice: None,
        }
    }

    /// High-level entry point: request an AI-generated rhythm based on the
    /// provided source `RhythmParams`, targeting `target_voice`.
    pub fn request_rhythm_for_voice(
        &mut self,
        source_params: &RhythmParams,
        target_voice: VoiceId,
    ) {
        let object: RhythmObject = source_params.to_serializable_object();
        let object_str = serde_json::to_string(&object).unwrap();

        println!("AIRhythm: starting OpenAI request for {:?}", target_voice);
        println!("{:#?}", object);
        self.pending_ai_voice = Some(target_voice);

        if let Err(e) = self.ai_service.send(object_str) {
            println!("AIRhythm: failed to send OpenAI request: {}", e);
            self.pending_ai_voice = None;
        }
    }

    /// Poll the underlying OpenAIService and, if a complete AI response is
    /// available, convert it into one or more `AiRhythmResult`s.
    pub fn poll_results(&mut self) -> Vec<AiRhythmResult> {
        let mut results = Vec::new();
        let output_items = self.poll_openai();
        if output_items.is_empty() {
            return results;
        }

        // Determine which voice this AI result belongs to.
        let Some(voice_id) = self.pending_ai_voice.take() else {
            println!(
                "AIRhythm: (warning) received AI rhythm output but no pending voice; ignoring"
            );
            return results;
        };

        // Find the first message that yields valid RhythmParams.
        let mut rhythm_params: Option<RhythmParams> = None;
        for output_item in &output_items {
            self.print_output_content(output_item);
            if let Some(params) = self.extract_rhythm_params(output_item) {
                rhythm_params = Some(params);
                break;
            }
        }

        let Some(params) = rhythm_params else {
            println!(
                "AIRhythm: AI response for {:?} did not contain a valid RhythmParams: {:?}",
                voice_id, output_items
            );
            return results;
        };

        println!(
            "AIRhythm: parsed AI rhythm {} for {:?}",
            params.as_test_ai_rhythm(),
            voice_id
        );

        results.push(AiRhythmResult {
            target_voice: voice_id,
            params,
        });

        results
    }

    fn poll_openai(&mut self) -> Vec<openai_response::OutputContent> {
        use openai_response::OutputContent;

        let mut output = Vec::new();

        if let Some(response) = self.ai_service.try_recv() {
            let items = response.output;

            println!(
                "AIRhythm: received OpenAI response with {} output item(s)",
                items.len()
            );

            items.iter().for_each(|item| match item {
                OutputContent::Message(_) => {
                    output.push(item.clone());
                }
                OutputContent::Reasoning(_) => {
                    output.push(item.clone());
                }
                _ => println!("AIRhythm: ignoring unknown output item type"),
            });
        }

        output
    }

    /*
    /// Low-level helper: poll the OpenAIService and decode its `output` array
    /// into `OpenAIOutputItem`s.
    fn poll_openai(&mut self) -> Vec<OpenAIOutputItem> {
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
     */

    /// Extract a `RhythmParams` from an AI response item by parsing any JSON
    /// object that matches our `RhythmObject` schema out of its text content.
    ///
    /// This is intentionally tolerant of some common LLM "schema drift" issues,
    /// such as:
    /// - wrapping the `sequence` field in an array instead of a single object
    /// - including extra candidate sequences where only one is needed
    fn extract_rhythm_params(
        &self,
        output_item: &openai_response::OutputContent,
    ) -> Option<RhythmParams> {
        use openai_response::{Content, OutputContent};

        match output_item {
            OutputContent::Message(message) => {
                for output_content in &message.content {
                    match output_content {
                        Content::OutputText(output_text) => {
                            let text = &output_text.text;
                            // Some models return the JSON wrapped in Markdown code
                            // fences (```json ... ```). To be robust, we extract the
                            // substring from the first '{' to the last '}' and attempt
                            // to parse that as JSON.
                            let trimmed = text.trim();

                            let json_candidate = if let (Some(start), Some(end)) =
                                (trimmed.find('{'), trimmed.rfind('}'))
                            {
                                &trimmed[start..=end]
                            } else {
                                trimmed
                            };

                            // First parse into a generic Value so we can repair
                            // common structural issues, then deserialize into
                            // `RhythmObject` from the normalized value.
                            match serde_json::from_str::<Value>(json_candidate) {
                                Ok(raw_val) => {
                                    let normalized = Self::normalize_rhythm_value(raw_val);
                                    match serde_json::from_value::<RhythmObject>(normalized) {
                                        Ok(object) => {
                                            return Some(
                                                RhythmParams::from_rhythm_response_object(object),
                                            );
                                        }
                                        Err(e) => {
                                            println!(
                                                "AIRhythm: failed to deserialize normalized RhythmObject. Error: {}",
                                                e
                                            );
                                        }
                                    }
                                }
                                Err(e) => {
                                    println!(
                                        "AIRhythm: failed to parse JSON for RhythmObject. Candidate JSON: '{}'. Error: {}",
                                        json_candidate,
                                        e
                                    );
                                }
                            }
                        }
                        Content::Refusal(refusal) => {
                            println!("AIRhythm: received refusal: {}", refusal.refusal);
                        }
                    }
                }
            }
            OutputContent::Reasoning(_) => {}
            _ => {}
        }

        None
    }

    /// Best-effort normalization of loosely-structured JSON from the model
    /// into something that matches the `RhythmObject` schema closely enough
    /// for `serde` to deserialize it.
    fn normalize_rhythm_value(mut v: Value) -> Value {
        // If `sequence` is an array, prefer the first entry that looks like a
        // proper sequence object with a string `rhythm` field.
        if let Some(seq_val) = v.get_mut("sequence") {
            if let Value::Array(arr) = seq_val {
                if !arr.is_empty() {
                    // Try to find the most "object-like" candidate.
                    let mut chosen: Option<Value> = None;

                    for candidate in arr.iter() {
                        if let Value::Object(map) = candidate {
                            let has_string_rhythm =
                                map.get("rhythm").map(|r| r.is_string()).unwrap_or(false);
                            let has_content_array =
                                map.get("content").map(|c| c.is_array()).unwrap_or(false);

                            if has_string_rhythm && has_content_array {
                                chosen = Some(candidate.clone());
                                break;
                            }
                        }
                    }

                    // Fallback: just take the first element if nothing matched
                    // our heuristics.
                    let chosen = chosen.unwrap_or_else(|| arr[0].clone());
                    *seq_val = chosen;
                }
            }
        }

        v
    }

    /// Debug helper: pretty-print the raw AI output item content in a
    /// human-friendly way.
    fn print_output_content(&self, output_content: &openai_response::OutputContent) {
        use openai_response::{Content, OutputContent};

        match output_content {
            OutputContent::Message(output_message) => {
                println!("AIRhythm: AI output message received");
                for (idx, output_content) in output_message.content.iter().enumerate() {
                    if let Content::OutputText(output_text) = output_content {
                        println!("\n--- OutputText #{idx} raw ---");
                        println!("{}", output_text.text);
                    }
                }
            }
            OutputContent::Reasoning(reasoning_item) => {
                println!("AIRhythm: AI output reasoning received");
                for (idx, content) in reasoning_item.content.iter().enumerate() {
                    let inner_text = &content.text;
                    println!("\n--- Reasoning Text #{idx} raw ---");
                    println!("{}", inner_text);
                }
            }
            _ => {
                println!("AIRhythm: ignoring unknown output item type");
            }
        }
    }
}
