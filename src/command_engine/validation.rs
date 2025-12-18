// src/command_engine/validation.rs
//
// Command validation types and helpers.

use crate::groups::VoiceId;

/// Validation result for voice/circle existence.
#[derive(Debug, Clone)]
pub enum ValidationResult {
    Success,
    VoiceNotFound(i32),
    CircleNotFound(i32, usize),
}

/// Trait for types that can validate voice and circle existence.
pub trait VoiceValidator {
    fn voice_exists(&self, voice_id: VoiceId) -> bool;
    fn circle_exists(&self, voice_id: VoiceId, circle_id: usize) -> bool;

    /// Validate voice existence and return standardized result.
    fn validate_voice(&self, voice_id: VoiceId) -> ValidationResult {
        if self.voice_exists(voice_id) {
            ValidationResult::Success
        } else {
            ValidationResult::VoiceNotFound(voice_id.to_i32())
        }
    }

    /// Validate both voice and circle existence.
    fn validate_voice_circle(&self, voice_id: VoiceId, circle_id: usize) -> ValidationResult {
        if !self.voice_exists(voice_id) {
            ValidationResult::VoiceNotFound(voice_id.to_i32())
        } else if !self.circle_exists(voice_id, circle_id) {
            ValidationResult::CircleNotFound(voice_id.to_i32(), circle_id)
        } else {
            ValidationResult::Success
        }
    }
}

impl ValidationResult {
    /// Convert validation result to error message.
    pub fn to_error_message(&self) -> Option<String> {
        match self {
            ValidationResult::Success => None,
            ValidationResult::VoiceNotFound(voice_id) => {
                Some(format!("Voice {} does not exist", voice_id))
            }
            ValidationResult::CircleNotFound(voice_id, circle_id) => Some(format!(
                "Voice {} - Circle {} does not exist",
                voice_id, circle_id
            )),
        }
    }

    /// Check if validation was successful.
    pub fn is_success(&self) -> bool {
        matches!(self, ValidationResult::Success)
    }
}
