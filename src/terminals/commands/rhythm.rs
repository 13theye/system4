// src/terminals/commands/rhythm.rs
//
// Rhythm command builder for sequencer creation

use super::TerminalCommandBuilder;
use crate::groups::{RhythmParams, VoiceId};
use crate::terminals::parsing::{ParameterValue, ParseError};
use prat::BeatSubdivision;
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone)]
pub struct RhythmBuilder {
    pub voice: Option<i32>,
    pub capacity: Option<usize>,
    pub num_wings: Option<usize>,
    pub subdivision: Option<BeatSubdivision>,
    pub parameters: HashMap<String, ParameterValue>,
}

#[derive(Debug, Clone)]
pub struct RhythmConfig {
    pub voice: i32,
    pub capacity: Option<usize>,
    pub num_wings: Option<usize>,
    pub subdivision: Option<BeatSubdivision>,
    pub additional_parameters: HashMap<String, ParameterValue>,
}

impl TerminalCommandBuilder for RhythmBuilder {
    type Config = RhythmConfig;

    fn new() -> Self {
        Self {
            voice: None,
            capacity: None,
            num_wings: None,
            subdivision: None,
            parameters: HashMap::new(),
        }
    }

    fn set_parameter(&mut self, name: &str, value: ParameterValue) -> Result<(), ParseError> {
        match name {
            "voice" => {
                if let ParameterValue::Number(n) = value {
                    self.voice = Some(n as i32);
                } else {
                    return Err(ParseError::UnexpectedToken {
                        expected: "number".to_string(),
                        found: format!("{}", value),
                    });
                }
            }
            "capacity" => {
                if let ParameterValue::Number(n) = value {
                    self.capacity = Some(n as usize);
                } else {
                    return Err(ParseError::UnexpectedToken {
                        expected: "number".to_string(),
                        found: format!("{}", value),
                    });
                }
            }
            "wings" => {
                if let ParameterValue::Number(n) = value {
                    self.num_wings = Some(n as usize);
                } else {
                    return Err(ParseError::UnexpectedToken {
                        expected: "number".to_string(),
                        found: format!("{}", value),
                    });
                }
            }
            "sub" => {
                if let ParameterValue::Number(n) = value {
                    self.subdivision = Some(number_to_subdivision(n as i32)?);
                } else {
                    return Err(ParseError::UnexpectedToken {
                        expected: "number".to_string(),
                        found: format!("{}", value),
                    });
                }
            }
            _ => {
                // Store unknown parameters for future extensibility
                self.parameters.insert(name.to_string(), value);
            }
        }
        Ok(())
    }

    fn build(self) -> RhythmConfig {
        RhythmConfig {
            voice: self.voice.unwrap_or(2), // Default to Voice2
            capacity: self.capacity,
            num_wings: self.num_wings,
            subdivision: self.subdivision,
            additional_parameters: self.parameters,
        }
    }
}

/// Convert a number to BeatSubdivision following the user's expected mapping
fn number_to_subdivision(n: i32) -> Result<BeatSubdivision, ParseError> {
    match n {
        1 => Ok(BeatSubdivision::Whole),
        2 => Ok(BeatSubdivision::Half),
        3 => Ok(BeatSubdivision::Triplet),
        4 => Ok(BeatSubdivision::Quarter),
        8 => Ok(BeatSubdivision::Eighth),
        16 => Ok(BeatSubdivision::Sixteenth),
        _ => Err(ParseError::UnexpectedToken {
            expected: "1, 2, 3, 4, 8, or 16".to_string(),
            found: n.to_string(),
        }),
    }
}

/// Determine which voice should be used for the rhythm sequencer
/// Returns Voice2 if available, otherwise Voice3, otherwise Voice2 as fallback
pub fn determine_rhythm_voice(voice_hint: Option<i32>) -> VoiceId {
    match voice_hint {
        Some(2) => VoiceId::Voice1,
        Some(3) => VoiceId::Voice2,
        _ => VoiceId::Voice1, // Default to Voice2
    }
}

impl RhythmConfig {
    /// Get default values for a specific voice
    pub fn get_defaults_for_voice(voice: i32) -> Self {
        let (default_capacity, default_num_wings, default_subdivision) = match voice {
            2 => (8, 3, BeatSubdivision::Eighth), // Voice2: Fast, simple rhythm
            3 => (16, 5, BeatSubdivision::Quarter), // Voice3: Longer, more complex rhythm
            _ => (8, 4, BeatSubdivision::Eighth), // Fallback
        };

        Self {
            voice,
            capacity: Some(default_capacity),
            num_wings: Some(default_num_wings),
            subdivision: Some(default_subdivision),
            additional_parameters: HashMap::new(),
        }
    }

    /// Merge this config with defaults, keeping specified values and using defaults for None values
    pub fn merge_with_defaults(self) -> Self {
        let defaults = Self::get_defaults_for_voice(self.voice);

        Self {
            voice: self.voice,
            capacity: self.capacity.or(defaults.capacity),
            num_wings: self.num_wings.or(defaults.num_wings),
            subdivision: self.subdivision.or(defaults.subdivision),
            additional_parameters: self.additional_parameters,
        }
    }

    /// Convert this RhythmConfig to RhythmParams (without wings - let Rhythm generate those)
    pub fn to_rhythm_params(self) -> RhythmParams {
        // Merge with defaults first to ensure all required fields are present
        let resolved_config = self.merge_with_defaults();

        RhythmParams::new(
            resolved_config.capacity.unwrap(),
            resolved_config.num_wings.unwrap(),
            resolved_config.subdivision.unwrap(),
        )
    }

    /// Validate that the voice is valid for rhythm commands (2 or 3)
    pub fn validate_voice(&self) -> Result<(), ParseError> {
        match self.voice {
            2 | 3 => Ok(()),
            1 => Err(ParseError::UnexpectedToken {
                expected: "voice 2 or 3".to_string(),
                found: format!("voice {} (Voice 1 is reserved for drones)", self.voice),
            }),
            4 => Err(ParseError::UnexpectedToken {
                expected: "voice 2 or 3".to_string(),
                found: format!("voice {} (Voice 4 is reserved for drones)", self.voice),
            }),
            _ => Err(ParseError::UnexpectedToken {
                expected: "voice 2 or 3".to_string(),
                found: format!("voice {} (invalid voice number)", self.voice),
            }),
        }
    }

    /// Convert this RhythmConfig's voice ID to a VoiceId enum
    pub fn voice_id(&self) -> VoiceId {
        match self.voice {
            2 => VoiceId::Voice1,
            3 => VoiceId::Voice2,
            _ => VoiceId::Voice1, // Default fallback
        }
    }
}

impl fmt::Display for RhythmConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "RhythmConfig {{")?;
        writeln!(f, "  voice: {}", self.voice)?;

        // Display optional parameters - show "default" for None values
        match self.capacity {
            Some(val) => writeln!(f, "  capacity: {}", val)?,
            None => writeln!(f, "  capacity: default")?,
        }
        match self.num_wings {
            Some(val) => writeln!(f, "  num_wings: {}", val)?,
            None => writeln!(f, "  num_wings: default")?,
        }
        match self.subdivision {
            Some(ref val) => writeln!(f, "  subdivision: {:?}", val)?,
            None => writeln!(f, "  subdivision: default")?,
        }

        if !self.additional_parameters.is_empty() {
            writeln!(f, "  additional_parameters: {{")?;
            for (key, value) in &self.additional_parameters {
                writeln!(f, "    {}: {}", key, value)?;
            }
            writeln!(f, "  }}")?;
        }

        write!(f, "}}")
    }
}
