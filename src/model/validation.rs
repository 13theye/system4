// src/voice/validation.rs
//
// Validation functions for Model commands and queries

use crate::command_engine::{ValidationResult, VoiceValidator};
use crate::groups::VoiceId;
use crate::model::Model;

impl VoiceValidator for Model {
    fn voice_exists(&self, voice_id: VoiceId) -> bool {
        self.drone_manager.validate_drone_exists(voice_id)
            || self.rhythm_manager.has_rhythm(voice_id)
    }

    fn circle_exists(&self, voice_id: VoiceId, circle_id: usize) -> bool {
        if let Some(voice) = self.drone_manager.drones().get(&voice_id) {
            voice.wind_circles.contains_key(&circle_id)
        } else {
            false
        }
    }
}

// Helper function for consistent error handling in command execution
impl Model {
    /// Validate and execute a command with standardized error handling
    pub fn validate_and_handle_error(
        &mut self,
        validation: ValidationResult,
        _operation: &str,
    ) -> bool {
        match validation {
            ValidationResult::Success => true,
            ValidationResult::VoiceNotFound(_) | ValidationResult::CircleNotFound(_, _) => {
                if let Some(error_msg) = validation.to_error_message() {
                    println!("Error: {}", error_msg);
                    // Route validation errors to the per-voice terminal when possible.
                    let voice_for_error: Option<VoiceId> = match validation {
                        ValidationResult::VoiceNotFound(voice_id) => {
                            Some(VoiceId::from_i32(voice_id))
                        }
                        ValidationResult::CircleNotFound(voice_id, _) => {
                            Some(VoiceId::from_i32(voice_id))
                        }
                        ValidationResult::Success => None,
                    };

                    if let Some(voice_id) = voice_for_error {
                        if let Some(input) = self.ui_state.command_inputs.get_mut(&voice_id) {
                            input.set_error_message(error_msg);
                        }
                    }
                }
                false
            }
        }
    }
}
