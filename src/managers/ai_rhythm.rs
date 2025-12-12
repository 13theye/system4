use crate::{
    groups::{RhythmParams, VoiceId},
    openai::{
        schema::{
            response::{MessageContent, OutputItem, OutputMessage, ResponseObject},
            stream::StreamEvent,
            RhythmObject,
        },
        OpenAIService,
    },
    settings::OpenAIServiceConfig,
};

use serde_json::Value;

/// High-level, domain-specific result from the AI service: a set of rhythm
/// parameters that should be applied to a particular target voice.
#[derive(Debug, Clone)]
pub struct AiRhythmResult {
    pub target_voice: VoiceId,
    pub params: RhythmParams,
}

pub struct AIRhythm {
    /// OpenAI REST API handler
    ai_service: OpenAIService,
    /// Voice for which we most recently sent an AI rhythm request.
    /// We assume a single in-flight AI request at a time.
    pending_ai_voice: Option<VoiceId>,
    /// Accumulated reasoning text from the current streamed response, if any.
    reasoning_text: Option<String>,
}

impl AIRhythm {
    pub fn new(config: &OpenAIServiceConfig) -> Self {
        let ai_service = OpenAIService::new(config);

        Self {
            ai_service,
            pending_ai_voice: None,
            reasoning_text: None,
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

        // Reset any previous reasoning text and mark a new request as pending
        self.reasoning_text = None;
        self.pending_ai_voice = Some(target_voice);

        if let Err(e) = self.ai_service.stream(object_str) {
            println!("AIRhythm: failed to send OpenAI request: {}", e);
            self.pending_ai_voice = None;
        }
    }

    /// Poll the underlying OpenAIService for streamed events and, if a complete
    /// AI response is available, convert it into one or more `AiRhythmResult`s.
    ///
    /// This also updates `reasoning_text` incrementally from
    /// `ResponseReasoningTextDelta` / `ResponseReasoningTextDone` events.
    /// Legacy non-streaming path: poll for a completed `ResponseObject` and
    /// parse it into AI rhythm results.
    ///
    /// Currently unused, but kept as a reference implementation and a future
    /// pathway for the non-stream Responses API.
    #[allow(dead_code)]
    pub fn poll_responses(&mut self) -> Option<Vec<AiRhythmResult>> {
        // Poll the OpenAI service for completed responses.
        let response = self.poll_openai_response()?;

        // Determine which voice this AI result belongs to.
        let Some(voice_id) = self.pending_ai_voice.take() else {
            println!(
                "AIRhythm: (warning) received AI rhythm output but no pending voice; ignoring",
            );
            return None;
        };

        let results = self.handle_completed_response(response, voice_id);
        if results.is_empty() {
            None
        } else {
            Some(results)
        }
    }

    /// Streaming path: poll the underlying OpenAIService for streamed events
    /// and, if a complete AI response is available, convert it into one or more
    /// `AiRhythmResult`s.
    ///
    /// This also updates `reasoning_text` incrementally from
    /// `ResponseReasoningTextDelta` / `ResponseReasoningTextDone` events.
    pub fn poll_stream(&mut self) -> Option<Vec<AiRhythmResult>> {
        let mut results = Vec::new();

        while let Some(event) = self.poll_openai_stream() {
            // If there is no pending target voice, we ignore all remaining
            // stream events for this request. This prevents late deltas or
            // completions from updating the UI after a voice has been cleared.
            if self.pending_ai_voice.is_none() {
                continue;
            }

            match event {
                // Incremental reasoning text updates
                StreamEvent::ResponseOutputTextDelta(e) => {
                    let buffer = self.reasoning_text.get_or_insert_with(String::new);
                    buffer.push_str(&e.delta);
                }
                StreamEvent::ResponseOutputTextDone(e) => {
                    self.reasoning_text = Some(e.text);
                }

                // Final response with full output array
                StreamEvent::ResponseCompleted(e) => {
                    let response = e.response;

                    // Determine which voice this AI result belongs to.
                    let Some(voice_id) = self.pending_ai_voice.take() else {
                        println!(
                            "AIRhythm: (warning) received AI rhythm output but no pending voice; ignoring",
                        );
                        continue;
                    };

                    let mut completed_results = self.handle_completed_response(response, voice_id);
                    results.append(&mut completed_results);
                }

                // Error / failure / incomplete events
                StreamEvent::ResponseError(e) => {
                    eprintln!(
                        "AIRhythm: received OpenAI stream error event: code={:?}, param={:?}, message={}",
                        e.code, e.param, e.message
                    );
                    self.pending_ai_voice = None;
                }
                StreamEvent::ResponseFailed(e) => {
                    eprintln!(
                        "AIRhythm: stream failed. status={:?}, incomplete_details={:?}, error={:?}",
                        e.response.status, e.response.incomplete_details, e.response.error
                    );
                    self.pending_ai_voice = None;
                }
                StreamEvent::ResponseIncomplete(e) => {
                    eprintln!(
                        "AIRhythm: stream incomplete. status={:?}, incomplete_details={:?}, error={:?}",
                        e.response.status, e.response.incomplete_details, e.response.error
                    );
                    self.pending_ai_voice = None;
                }
                StreamEvent::ResponseRefusalDelta(e) => {
                    eprintln!("AIRhythm: model partial refusal text: {}", e.delta);
                }
                StreamEvent::ResponseRefusalDone(e) => {
                    eprintln!("AIRhythm: model refusal: {}", e.refusal);
                    self.pending_ai_voice = None;
                }

                // Other events are currently ignored for rhythm generation
                _ => {}
            }
        }

        if results.is_empty() {
            None
        } else {
            Some(results)
        }
    }

    /// Poll the underlying OpenAIService for completed responses (non-stream)
    fn poll_openai_response(&mut self) -> Option<ResponseObject> {
        self.ai_service.try_recv_response()
    }

    /// Poll the underlying OpenAIService for `StreamEvent` events
    fn poll_openai_stream(&mut self) -> Option<StreamEvent> {
        self.ai_service.try_recv_stream()
    }

    /// Shared logic for turning a completed `ResponseObject` into
    /// `AiRhythmResult`s, used by both streaming and non-streaming paths.
    fn handle_completed_response(
        &self,
        response: ResponseObject,
        voice_id: VoiceId,
    ) -> Vec<AiRhythmResult> {
        let mut results = Vec::new();

        let Some(output_items) = process_response_object(response) else {
            return results;
        };

        // Attempt to generate a valid RhythmParams from first output item.
        for output_item in &output_items {
            self.print_output_content(output_item);

            if let OutputItem::Message(message) = output_item {
                if let Some(params) = self.extract_rhythm_params(message) {
                    println!(
                        "AIRhythm: parsed AI rhythm {} for {:?}",
                        params.as_test_ai_rhythm(),
                        voice_id
                    );

                    results.push(AiRhythmResult {
                        target_voice: voice_id,
                        params,
                    });

                    break;
                }
            }
        }

        results
    }

    /// Get the current accumulated reasoning text, if any.
    pub fn current_reasoning_text(&self) -> Option<&str> {
        self.reasoning_text.as_deref()
    }

    /// Returns true if there is an AI rhythm request currently in flight.
    pub fn is_request_pending(&self) -> bool {
        self.pending_ai_voice.is_some()
    }

    /// Clear any pending request for the given voice and set a simple status
    /// message indicating that the voice was cleared. Subsequent stream events
    /// for this request will be ignored.
    pub fn clear_status_for_voice(&mut self, voice_id: VoiceId) {
        // Always update the reasoning text so the UI shows a clear message
        // instead of any previous AI-generated content.
        if voice_id == VoiceId::Voice2 {
            // Use the exact copy requested for Voice2.
            self.reasoning_text = Some("Voice2 cleared".to_string());
        } else {
            self.reasoning_text = Some(format!("Voice {} cleared", voice_id.to_i32()));
        }

        // If this voice had an in-flight request, also clear the pending flag
        // so that further stream events are ignored.
        if self.pending_ai_voice == Some(voice_id) {
            self.pending_ai_voice = None;
        }
    }

    /// Extract a `RhythmParams` from an AI response item by parsing any JSON
    /// object that matches our `RhythmObject` schema out of its text content.
    ///
    /// This is intentionally tolerant of some common LLM "schema drift" issues,
    /// such as:
    /// - wrapping the `sequence` field in an array instead of a single object
    /// - including extra candidate sequences where only one is needed
    fn extract_rhythm_params(&self, message: &OutputMessage) -> Option<RhythmParams> {
        for message_content in &message.content {
            match message_content {
                MessageContent::OutputText { text } => {
                    // Some models return the JSON wrapped in Markdown code
                    // fences (```json ... ```). To be robust, we extract the
                    // substring from the first '{' to the last '}' and attempt
                    // to parse that as JSON.
                    let trimmed = text.trim();

                    let json_candidate =
                        if let (Some(start), Some(end)) = (trimmed.find('{'), trimmed.rfind('}')) {
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
                                    return Some(RhythmParams::from_rhythm_response_object(object));
                                }
                                Err(e) => {
                                    eprintln!(
                                                "AIRhythm: failed to deserialize normalized RhythmObject. Error: {}",
                                                e
                                            );
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!(
                                        "AIRhythm: failed to parse JSON for RhythmObject. Candidate JSON: '{}'. Error: {}",
                                        json_candidate,
                                        e
                                    );
                        }
                    }
                }
                MessageContent::Refusal { refusal } => {
                    println!("AIRhythm: received refusal: {}", refusal);
                }
                MessageContent::Unknown => {
                    println!(
                        "AIRhythm: ignoring unexpected message content type: {:?}",
                        message_content
                    );
                }
                // Safely ignore ReasoningText
                MessageContent::ReasoningText { .. } => {}
            }
        }

        // If we did not find a valid RhythmObject:
        println!("\nAIRhythm: (warning) no valid RhythmObject found in AI response");
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
    fn print_output_content(&self, output_content: &OutputItem) {
        match output_content {
            OutputItem::Message(output_message) => {
                println!("AIRhythm: AI OutputItem::Message received");
                for (idx, output_content) in output_message.content.iter().enumerate() {
                    match output_content {
                        MessageContent::OutputText { text } => {
                            println!("\n--- Message::OutputText #{idx} raw ---");
                            println!("{}", text);
                        }

                        MessageContent::ReasoningText { text } => {
                            println!("\n--- Message::ReasoningText #{idx} raw ---");
                            println!("{}", text);
                        }

                        MessageContent::Refusal { refusal } => {
                            println!("\n--- Message::Refusal #{idx} raw ---");
                            println!("{}", refusal);
                        }
                        _ => {}
                    }
                }
            }
            OutputItem::Reasoning(reasoning_item) => {
                println!("AIRhythm: AI OutputItem::Reasoning received");
                for (idx, content) in reasoning_item.content.iter().enumerate() {
                    let inner_text = content
                        .iter()
                        .map(|c| c.text.to_owned())
                        .collect::<String>();
                    println!("\n--- Reasoning::Reasoning Text #{idx} raw ---");
                    println!("{}", inner_text);
                }
            }
            // Safely ignore Unknown content
            OutputItem::Unknown(val) => {
                println!("AIRhythm: ignoring unknown output item: {}", val);
            }
        }
    }
}

/// Process a received OpenAI response into a vector of `OutputItem`s.
fn process_response_object(response: ResponseObject) -> Option<Vec<OutputItem>> {
    let items = response.output;
    let mut output = Vec::new();

    println!(
        "AIRhythm: received OpenAI response with {} output item(s)",
        items.len()
    );

    items.iter().for_each(|item| match item {
        OutputItem::Message(_) => {
            output.push(item.clone());
        }
        OutputItem::Reasoning(_) => {
            output.push(item.clone());
        }
        OutputItem::Unknown(value) => {
            println!("AIRhythm: (warning)ignoring unknown output item: {}", value);
        }
    });

    if output.is_empty() {
        println!("AIRhythm: no output items found in OpenAI response");
        return None;
    }

    Some(output)
}
