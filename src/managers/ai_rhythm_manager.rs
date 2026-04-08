use crate::{
    groups::{RhythmParams, VoiceId},
    managers::{AIRhythm, AiRhythmResult, AiStreamEvent},
    settings::OpenAIServiceConfig,
};

/// RhythmManager handles AI rhythm-related state and operations.
/// This includes:
/// - Rhythm lifecycle (creation, removal)
/// - Rhythm state access (mutable and immutable)
/// - Composite operations requiring coordinated access to sequencer service and RNG
pub struct AIRhythmManager {
    ai_rhythm: AIRhythm,
}

impl AIRhythmManager {
    pub fn init(config: &OpenAIServiceConfig) -> Self {
        Self {
            ai_rhythm: AIRhythm::new(config),
        }
    }

    /// Send the current rhythm state for Voice1 to the AI service.
    ///
    /// The AI-generated rhythm will be applied to Voice2.
    pub fn request_ai_rhythm(&mut self, sample_rhythm_params: RhythmParams, target_voice: VoiceId) {
        // Delegate AI request construction and sending to AIRhythm.
        self.ai_rhythm
            .request_rhythm_for_voice(&sample_rhythm_params, target_voice);
    }

    /// Poll the AI service for completed responses
    pub fn poll_ai(&mut self) -> (Option<Vec<AiRhythmResult>>, Option<Vec<AiStreamEvent>>) {
        self.ai_rhythm.poll_stream()
    }

    /// Return true if an AI rhythm request is currently in flight.
    pub fn is_ai_request_pending(&self) -> bool {
        self.ai_rhythm.is_request_pending()
    }

    /// Clear AI state for the given voice (reasoning text + in-flight request).
    pub fn clear_ai_status_for_voice(&mut self, voice_id: VoiceId) {
        self.ai_rhythm.clear_status_for_voice(voice_id);
    }

    /// Expose current AI reasoning text for UI rendering.
    ///
    /// Prefer using `current_ai_status_text` for user-facing display, which
    /// includes a fallback "AI is thinking..." message while a request is in
    /// flight but no reasoning text has been streamed yet.
    pub fn current_ai_reasoning_text(&self) -> Option<&str> {
        self.ai_rhythm.current_reasoning_text()
    }

    /// Expose a user-facing status string for the AI rhythm request.
    ///
    /// When a request is pending but no reasoning text has arrived yet,
    /// this returns "AI is thinking..." so the performer has immediate
    /// feedback that the system is waiting on the model.
    /// Also returns TRUE if the ai status text should fade out as normal.
    pub fn current_ai_status_text(&self) -> (Option<&str>, bool) {
        if self.ai_rhythm.is_request_pending() {
            if let Some(text) = self.ai_rhythm.current_reasoning_text() {
                (Some(text), true)
            } else {
                (Some("The AI is generating a rhythmic response..."), false)
            }
        } else {
            (self.ai_rhythm.current_reasoning_text(), true)
        }
    }
}
