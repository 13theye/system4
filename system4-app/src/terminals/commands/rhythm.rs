// src/terminals/commands/rhythm.rs
//
// Re-exports of core rhythm types with app-specific extensions

pub use system4_core::commands::rhythm::{
    ParameterModification, RangeSize, RhythmBuilder, RhythmConfig,
};

use crate::groups::{rhythm::RhythmParams, VoiceId};
use crate::terminals::parsing::ParseError;
use prat::BeatSubdivision;
use std::collections::HashMap;

/// Extension trait for RhythmConfig providing app-specific functionality
pub trait RhythmConfigExt {
    /// Get the voice ID as VoiceId enum
    fn get_voice_id(&self) -> VoiceId;

    /// Get default values for a specific voice
    fn get_defaults_for_voice(voice: VoiceId) -> RhythmConfig;

    /// Merge this config with defaults, keeping specified values and using defaults for None values
    fn merge_with_defaults(&self) -> RhythmConfig;

    /// Convert this RhythmConfig to RhythmParams (without wings - let Rhythm generate those)
    fn to_rhythm_params(&self) -> RhythmParams;

    /// Validate that the voice is valid for rhythm (Voice1 or Voice2)
    fn validate_voice(&self) -> Result<(), ParseError>;
}

impl RhythmConfigExt for RhythmConfig {
    fn get_voice_id(&self) -> VoiceId {
        VoiceId::from_i32(self.voice)
    }

    fn get_defaults_for_voice(voice: VoiceId) -> RhythmConfig {
        let (default_capacity, default_num_wings, default_subdivision) = match voice {
            VoiceId::Voice1 => (12, 5, BeatSubdivision::Eighth),
            VoiceId::Voice2 => (8, 3, BeatSubdivision::Quarter),
            _ => (8, 4, BeatSubdivision::Eighth), // Fallback
        };

        let (default_length_range, default_velocity_range, default_cutoff_range) = match voice {
            VoiceId::Voice1 => (RangeSize::M, RangeSize::M, RangeSize::M),
            VoiceId::Voice2 => (RangeSize::M, RangeSize::M, RangeSize::M),
            _ => (RangeSize::M, RangeSize::M, RangeSize::M), // Fallback
        };

        RhythmConfig {
            voice: voice.to_i32(),
            capacity: Some(default_capacity),
            num_wings: Some(default_num_wings),
            subdivision: Some(default_subdivision),
            length_range: Some(default_length_range),
            velocity_range: Some(default_velocity_range),
            cutoff_range: Some(default_cutoff_range),
            length_modification: None,
            velocity_modification: None,
            cutoff_modification: None,
            additional_parameters: HashMap::new(),
        }
    }

    fn merge_with_defaults(&self) -> RhythmConfig {
        let defaults = Self::get_defaults_for_voice(self.get_voice_id());

        RhythmConfig {
            voice: self.voice,
            capacity: self.capacity.or(defaults.capacity),
            num_wings: self.num_wings.or(defaults.num_wings),
            subdivision: self.subdivision.or(defaults.subdivision),
            length_range: self.length_range.or(defaults.length_range),
            velocity_range: self.velocity_range.or(defaults.velocity_range),
            cutoff_range: self.cutoff_range.or(defaults.cutoff_range),
            length_modification: self.length_modification,
            velocity_modification: self.velocity_modification,
            cutoff_modification: self.cutoff_modification,
            additional_parameters: self.additional_parameters.clone(),
        }
    }

    fn to_rhythm_params(&self) -> RhythmParams {
        // Merge with defaults first to ensure all required fields are present
        let resolved_config = self.merge_with_defaults();

        RhythmParams::new(
            resolved_config.capacity.unwrap(),
            resolved_config.num_wings.unwrap(),
            resolved_config.subdivision.unwrap(),
            resolved_config.length_range.unwrap(),
            resolved_config.velocity_range.unwrap(),
            resolved_config.cutoff_range.unwrap(),
        )
    }

    fn validate_voice(&self) -> Result<(), ParseError> {
        // Convert i32 voice to VoiceId for validation
        let voice_enum = VoiceId::from_i32(self.voice);

        match voice_enum {
            VoiceId::Voice1 | VoiceId::Voice2 => Ok(()),
            VoiceId::Voice0 => Err(ParseError::UnexpectedToken {
                expected: "voice 1 or 2".to_string(),
                found: format!("voice {} (Voice 0 is reserved for drones)", self.voice),
            }),
            VoiceId::Voice3 => Err(ParseError::UnexpectedToken {
                expected: "voice 1 or 2".to_string(),
                found: format!("voice {} (Voice 3 is reserved for drones)", self.voice),
            }),
            _ => Err(ParseError::UnexpectedToken {
                expected: "voice 1 or 2".to_string(),
                found: format!("voice {} (invalid voice number)", self.voice),
            }),
        }
    }
}
