// system4-core/src/commands/rhythm.rs
// Rhythm command builder for sequencer creation

use super::TerminalCommandBuilder;
use crate::parsing::{ParameterValue, ParseError};
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
    /// All ranges produce values in [0.0, 1.0] with different skew:
    /// - XS, S: skewed toward 0.0
    /// - M: normal distribution (no skew)
    /// - L, XL: skewed toward 1.0
    pub fn to_skew_distribution_params(&self) -> (f32, f32, f32) {
        match self {
            RangeSize::XS => (0.0, 0.25, 8.0),
            RangeSize::S => (0.0, 0.25, 5.0),
            RangeSize::M => (0.5, 0.25, 0.0),
            RangeSize::L => (0.9, 0.25, -5.0),
            RangeSize::XL => (0.9, 0.25, -8.0),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RhythmBuilder {
    pub voice: Option<i32>,
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
    pub voice: i32,
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
            voice: self.voice.unwrap_or(1), // Default to Voice1
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

/// Parse relative modification strings like "+", "++", "-", "--"
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

impl fmt::Display for RhythmConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "  voice: {}", self.voice)?;
        if let Some(c) = self.capacity {
            writeln!(f, "  capacity: {}", c)?;
        }
        if let Some(w) = self.num_wings {
            writeln!(f, "  wings: {}", w)?;
        }
        if let Some(sub) = self.subdivision {
            writeln!(f, "  subdivision: {:?}", sub)?;
        }
        if let Some(range) = self.length_range {
            writeln!(f, "  length_range: {:?}", range)?;
        }
        if let Some(range) = self.velocity_range {
            writeln!(f, "  velocity_range: {:?}", range)?;
        }
        if let Some(range) = self.cutoff_range {
            writeln!(f, "  cutoff_range: {:?}", range)?;
        }
        for (key, value) in &self.additional_parameters {
            writeln!(f, "  {}: {}", key, value)?;
        }
        Ok(())
    }
}
