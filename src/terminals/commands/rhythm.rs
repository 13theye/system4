// src/terminals/commands/rhythm.rs
//
// Rhythm command builder for sequencer creation

use super::TerminalCommandBuilder;
use crate::groups::{RhythmParams, VoiceId};
use crate::terminals::parsing::{ParameterValue, ParseError};
use prat::BeatSubdivision;
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, Copy)]
pub enum ParameterModification {
    Absolute(f32),        // Set to exact value
    Relative(f32),        // Add/subtract delta
    Randomize(RangeSize), // Re-roll within range
}

#[derive(Debug, Default, Clone, Copy)]
pub enum RangeSize {
    XS = 0,
    S = 1,
    #[default]
    M = 2,
    L = 3,
    XL = 4,
}

impl RangeSize {
    pub fn to_range_inclusive(&self) -> std::ops::RangeInclusive<f32> {
        match self {
            RangeSize::XS => 0.0..=0.25,
            RangeSize::S => 0.2..=0.45,
            RangeSize::M => 0.4..=0.66,
            RangeSize::L => 0.5..=0.88,
            RangeSize::XL => 0.5..=1.0,
        }
    }

    /// converts the named range to parameters for SkewNormal.
    /// returns (location, scale, shape)
    pub fn to_skew_distribution_params(&self) -> (f32, f32, f32) {
        match self {
            RangeSize::XS => (0.1, 1.0, 2.0),
            RangeSize::S => (0.3, 1.0, 1.5),
            RangeSize::M => (0.5, 1.0, 0.0),
            RangeSize::L => (1.0, 1.0, -1.5),
            RangeSize::XL => (1.0, 1.0, -5.0),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RhythmBuilder {
    pub voice: Option<VoiceId>,
    pub capacity: Option<usize>,
    pub num_wings: Option<usize>,
    pub subdivision: Option<BeatSubdivision>,
    pub length_range: Option<RangeSize>,
    pub velocity_range: Option<RangeSize>,
    pub cutoff_range: Option<RangeSize>,
    pub length_modification: Option<ParameterModification>,
    pub velocity_modification: Option<ParameterModification>,
    pub cutoff_modification: Option<ParameterModification>,
    pub parameters: HashMap<String, ParameterValue>,
}

#[derive(Debug, Clone)]
pub struct RhythmConfig {
    pub voice: VoiceId,
    pub capacity: Option<usize>,
    pub num_wings: Option<usize>,
    pub subdivision: Option<BeatSubdivision>,
    pub length_range: Option<RangeSize>,
    pub velocity_range: Option<RangeSize>,
    pub cutoff_range: Option<RangeSize>,
    pub length_modification: Option<ParameterModification>,
    pub velocity_modification: Option<ParameterModification>,
    pub cutoff_modification: Option<ParameterModification>,
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
            length_range: None,
            velocity_range: None,
            cutoff_range: None,
            length_modification: None,
            velocity_modification: None,
            cutoff_modification: None,
            parameters: HashMap::new(),
        }
    }

    fn set_parameter(&mut self, name: &str, value: ParameterValue) -> Result<(), ParseError> {
        match name {
            "voice" => {
                if let ParameterValue::Number(n) = value {
                    let voice = VoiceId::from_i32(n as i32);
                    self.voice = Some(voice);
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
            "length" => {
                match value {
                    ParameterValue::Number(n) => {
                        // Absolute value: .length(0.5)
                        self.length_modification = Some(ParameterModification::Absolute(n));
                    }
                    ParameterValue::String(s) => {
                        // Try parsing as range size first, then as relative modification
                        if let Ok(range) = string_to_range_size(s.clone()) {
                            // Set both: range for future slot generation, modification for editing
                            self.length_range = Some(range);
                            self.length_modification =
                                Some(ParameterModification::Randomize(range));
                        } else {
                            self.length_modification = Some(parse_relative_modification(s)?);
                        }
                    }
                }
            }
            "velocity" => {
                match value {
                    ParameterValue::Number(n) => {
                        // Absolute value: .velocity(0.5)
                        self.velocity_modification = Some(ParameterModification::Absolute(n));
                    }
                    ParameterValue::String(s) => {
                        // Try parsing as range size first, then as relative modification
                        if let Ok(range) = string_to_range_size(s.clone()) {
                            // Set both: range for future slot generation, modification for editing
                            self.velocity_range = Some(range);
                            self.velocity_modification =
                                Some(ParameterModification::Randomize(range));
                        } else {
                            self.velocity_modification = Some(parse_relative_modification(s)?);
                        }
                    }
                }
            }
            "cutoff" => {
                match value {
                    ParameterValue::Number(n) => {
                        // Absolute value: .cutoff(0.5)
                        self.cutoff_modification = Some(ParameterModification::Absolute(n));
                    }
                    ParameterValue::String(s) => {
                        // Try parsing as range size first, then as relative modification
                        if let Ok(range) = string_to_range_size(s.clone()) {
                            // Set both: range for future slot generation, modification for editing
                            self.cutoff_range = Some(range);
                            self.cutoff_modification =
                                Some(ParameterModification::Randomize(range));
                        } else {
                            self.cutoff_modification = Some(parse_relative_modification(s)?);
                        }
                    }
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
            voice: self.voice.unwrap_or(VoiceId::Voice1), // Default to Voice2
            capacity: self.capacity,
            num_wings: self.num_wings,
            subdivision: self.subdivision,
            length_range: self.length_range,
            velocity_range: self.velocity_range,
            cutoff_range: self.cutoff_range,
            length_modification: self.length_modification,
            velocity_modification: self.velocity_modification,
            cutoff_modification: self.cutoff_modification,
            additional_parameters: self.parameters,
        }
    }
}

/// Parse relative modification symbols (+, ++, +++, -, --, ---)
fn parse_relative_modification(s: String) -> Result<ParameterModification, ParseError> {
    match s.as_str() {
        "+" => Ok(ParameterModification::Relative(0.1)),
        "++" => Ok(ParameterModification::Relative(0.2)),
        "+++" => Ok(ParameterModification::Relative(0.3)),
        "-" => Ok(ParameterModification::Relative(-0.1)),
        "--" => Ok(ParameterModification::Relative(-0.2)),
        "---" => Ok(ParameterModification::Relative(-0.3)),
        _ => Err(ParseError::UnexpectedToken {
            expected: "+, ++, +++, -, --, ---, or range size (xs, s, m, l, xl)".to_string(),
            found: s,
        }),
    }
}

/// Convert a string to a RangeSize
fn string_to_range_size(s: String) -> Result<RangeSize, ParseError> {
    match s.to_lowercase().as_str() {
        "xs" => Ok(RangeSize::XS),
        "s" => Ok(RangeSize::S),
        "m" => Ok(RangeSize::M),
        "l" => Ok(RangeSize::L),
        "xl" => Ok(RangeSize::XL),
        _ => Err(ParseError::UnexpectedToken {
            expected: "xs, s, m, l, or xl".to_string(),
            found: s,
        }),
    }
}

/// Attempt to convert a string to a number
fn string_to_number(s: String) -> Result<f32, ParseError> {
    match s.parse::<f32>() {
        Ok(n) => Ok(n),
        Err(_) => Err(ParseError::UnexpectedToken {
            expected: "number".to_string(),
            found: s,
        }),
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
    pub fn get_defaults_for_voice(voice: VoiceId) -> Self {
        let (default_capacity, default_num_wings, default_subdivision) = match voice {
            VoiceId::Voice1 => (12, 5, BeatSubdivision::Eighth),
            VoiceId::Voice2 => (8, 3, BeatSubdivision::Quarter),
            _ => (8, 4, BeatSubdivision::Eighth), // Fallback
        };

        let (default_length_range, default_velocity_range, default_pitch_range) = match voice {
            VoiceId::Voice1 => (RangeSize::XL, RangeSize::L, RangeSize::L),
            VoiceId::Voice2 => (RangeSize::XL, RangeSize::L, RangeSize::L),
            _ => (RangeSize::M, RangeSize::M, RangeSize::M), // Fallback
        };

        Self {
            voice,
            capacity: Some(default_capacity),
            num_wings: Some(default_num_wings),
            subdivision: Some(default_subdivision),
            length_range: Some(default_length_range),
            velocity_range: Some(default_velocity_range),
            cutoff_range: Some(default_pitch_range),
            length_modification: None,
            velocity_modification: None,
            cutoff_modification: None,
            additional_parameters: HashMap::new(),
        }
    }

    /// Merge this config with defaults, keeping specified values and using defaults for None values
    pub fn merge_with_defaults(&self) -> Self {
        let defaults = Self::get_defaults_for_voice(self.voice);

        Self {
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

    /// Convert this RhythmConfig to RhythmParams (without wings - let Rhythm generate those)
    pub fn to_rhythm_params(&self) -> RhythmParams {
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

    /// Validate that the voice is valid for rhythm commands (2 or 3)
    pub fn validate_voice(&self) -> Result<(), ParseError> {
        match self.voice {
            VoiceId::Voice1 | VoiceId::Voice2 => Ok(()),
            VoiceId::Voice0 => Err(ParseError::UnexpectedToken {
                expected: "voice 1 or 2".to_string(),
                found: format!("voice {} (Voice 0 is reserved for drones)", self.voice),
            }),
            VoiceId::Voice3 => Err(ParseError::UnexpectedToken {
                expected: "voice 2 or 3".to_string(),
                found: format!("voice {} (Voice 3 is reserved for drones)", self.voice),
            }),
            _ => Err(ParseError::UnexpectedToken {
                expected: "voice 1 or 2".to_string(),
                found: format!("voice {} (invalid voice number)", self.voice),
            }),
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
