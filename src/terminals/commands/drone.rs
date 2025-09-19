// src/terminals/commands/drone.rs
//
// Drone command builder and configuration

use super::TerminalCommandBuilder;
use crate::groups::VoiceId;
use crate::model::controller::{Command, CommandInner, CommandSource};
use crate::terminals::parsing::{ParameterValue, ParseError};
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone)]
pub struct DroneBuilder {
    pub voice: Option<i32>,
    pub brightness: Option<f32>,
    pub volume: Option<f32>,
    pub feedback: Option<f32>,
    pub gravity: Option<f32>,
    pub force: Option<f32>,
    pub outer_radius: Option<f32>,
    pub inner_radius: Option<f32>,
    pub noise: Option<f32>,
    pub vibration: Option<f32>,
    pub center_x: Option<f32>,
    pub center_y: Option<f32>,
    pub parameters: HashMap<String, ParameterValue>,
}

impl TerminalCommandBuilder for DroneBuilder {
    type Config = DroneConfig;

    fn new() -> Self {
        Self {
            voice: None,
            brightness: None,
            volume: None,
            feedback: None,
            gravity: None,
            force: None,
            outer_radius: None,
            inner_radius: None,
            noise: None,
            vibration: None,
            center_x: None,
            center_y: None,
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
            "brightness" => {
                if let ParameterValue::Number(n) = value {
                    self.brightness = Some(n);
                } else {
                    return Err(ParseError::UnexpectedToken {
                        expected: "number".to_string(),
                        found: format!("{}", value),
                    });
                }
            }
            "volume" => {
                if let ParameterValue::Number(n) = value {
                    self.volume = Some(n);
                } else {
                    return Err(ParseError::UnexpectedToken {
                        expected: "number".to_string(),
                        found: format!("{}", value),
                    });
                }
            }
            "gravity" => {
                if let ParameterValue::Number(n) = value {
                    self.gravity = Some(n);
                } else {
                    return Err(ParseError::UnexpectedToken {
                        expected: "number".to_string(),
                        found: format!("{}", value),
                    });
                }
            }
            "force" => {
                if let ParameterValue::Number(n) = value {
                    self.force = Some(n);
                } else {
                    return Err(ParseError::UnexpectedToken {
                        expected: "number".to_string(),
                        found: format!("{}", value),
                    });
                }
            }
            "outerRadius" => {
                if let ParameterValue::Number(n) = value {
                    self.outer_radius = Some(n);
                } else {
                    return Err(ParseError::UnexpectedToken {
                        expected: "number".to_string(),
                        found: format!("{}", value),
                    });
                }
            }
            "innerRadius" => {
                if let ParameterValue::Number(n) = value {
                    self.inner_radius = Some(n);
                } else {
                    return Err(ParseError::UnexpectedToken {
                        expected: "number".to_string(),
                        found: format!("{}", value),
                    });
                }
            }
            "noise" => {
                if let ParameterValue::Number(n) = value {
                    self.noise = Some(n);
                } else {
                    return Err(ParseError::UnexpectedToken {
                        expected: "number".to_string(),
                        found: format!("{}", value),
                    });
                }
            }
            "vibration" => {
                if let ParameterValue::Number(n) = value {
                    self.vibration = Some(n);
                } else {
                    return Err(ParseError::UnexpectedToken {
                        expected: "number".to_string(),
                        found: format!("{}", value),
                    });
                }
            }
            "feedback" => {
                if let ParameterValue::Number(n) = value {
                    self.feedback = Some(n);
                } else {
                    return Err(ParseError::UnexpectedToken {
                        expected: "number".to_string(),
                        found: format!("{}", value),
                    });
                }
            }
            "centerX" => {
                if let ParameterValue::Number(n) = value {
                    self.center_x = Some(n);
                } else {
                    return Err(ParseError::UnexpectedToken {
                        expected: "number".to_string(),
                        found: format!("{}", value),
                    });
                }
            }
            "centerY" => {
                if let ParameterValue::Number(n) = value {
                    self.center_y = Some(n);
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

    fn build(self) -> DroneConfig {
        DroneConfig {
            voice: self.voice.unwrap_or(1), // Voice is always required
            brightness: self.brightness,    // None = unchanged, Some = set to value
            volume: self.volume,            // None = unchanged, Some = set to value
            feedback: self.feedback,        // None = unchanged, Some = set to value
            vibration: self.vibration,      // None = unchanged, Some = set to value

            gravity: self.gravity, // None = unchanged, Some = set to value
            force: self.force,     // None = unchanged, Some = set to value
            outer_radius: self.outer_radius, // None = unchanged, Some = set to value
            inner_radius: self.inner_radius, // None = unchanged, Some = set to value
            noise: self.noise,     // None = unchanged, Some = set to value
            center_x: self.center_x, // None = unchanged, Some = set to value
            center_y: self.center_y, // None = unchanged, Some = set to value

            additional_parameters: self.parameters,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DroneConfig {
    pub voice: i32,
    pub brightness: Option<f32>,
    pub volume: Option<f32>,
    pub feedback: Option<f32>,
    pub vibration: Option<f32>,

    pub center_x: Option<f32>,
    pub center_y: Option<f32>,
    pub outer_radius: Option<f32>,
    pub inner_radius: Option<f32>,
    pub gravity: Option<f32>,
    pub force: Option<f32>,
    pub noise: Option<f32>,

    pub additional_parameters: HashMap<String, ParameterValue>,
}

impl DroneConfig {
    /// Convert this DroneConfig to a CreateDrone VoiceCommand
    pub fn to_create_command(&self, source: CommandSource) -> Command {
        Command::new(
            CommandInner::CreateDrone {
                config: self.clone(),
            },
            source,
        )
    }

    /// Convert this DroneConfig to a ModifyDrone VoiceCommand for the specified voice
    pub fn to_modify_command(&self, voice: VoiceId, source: CommandSource) -> Command {
        Command::new(
            CommandInner::ModifyDrone {
                voice_id: voice,
                config: self.clone(),
            },
            source,
        )
    }

    /// Convert this DroneConfig's voice ID to a Voice enum
    pub fn voice_enum(&self) -> VoiceId {
        match self.voice {
            1 => VoiceId::Voice1,
            4 => VoiceId::Voice4,
            _ => VoiceId::Voice1, // Default fallback
        }
    }
}

impl fmt::Display for DroneConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "DroneConfig {{")?;
        writeln!(f, "  voice: {}", self.voice)?;

        // Display optional parameters - show "unchanged" for None values
        match self.brightness {
            Some(val) => writeln!(f, "  brightness: {}", val)?,
            None => writeln!(f, "  brightness: unchanged")?,
        }
        match self.volume {
            Some(val) => writeln!(f, "  volume: {}", val)?,
            None => writeln!(f, "  volume: unchanged")?,
        }
        match self.feedback {
            Some(val) => writeln!(f, "  trail: {}", val)?,
            None => writeln!(f, "  trail: unchanged")?,
        }

        match self.vibration {
            Some(val) => writeln!(f, "  vibration: {}", val)?,
            None => writeln!(f, "  vibration: unchanged")?,
        }
        match self.feedback {
            Some(val) => writeln!(f, "  feedback: {}", val)?,
            None => writeln!(f, "  feedback: unchanged")?,
        }

        /* Moving these to WindCircleParams
        match self.center_x {
            Some(val) => writeln!(f, "  center_x: {}", val)?,
            None => writeln!(f, "  center_x: unchanged")?,
        }
        match self.center_y {
            Some(val) => writeln!(f, "  center_y: {}", val)?,
            None => writeln!(f, "  center_y: unchanged")?,
        }
        match self.outer_radius {
            Some(val) => writeln!(f, "  outer_radius: {}", val)?,
            None => writeln!(f, "  outer_radius: unchanged")?,
        }
        match self.inner_radius {
            Some(val) => writeln!(f, "  inner_radius: {}", val)?,
            None => writeln!(f, "  inner_radius: unchanged")?,
        }
        match self.gravity {
            Some(val) => writeln!(f, "  gravity: {}", val)?,
            None => writeln!(f, "  gravity: unchanged")?,
        }
        match self.force {
            Some(val) => writeln!(f, "  force: {}", val)?,
            None => writeln!(f, "  force: unchanged")?,
        }
        match self.noise {
            Some(val) => writeln!(f, "  noise: {}", val)?,
            None => writeln!(f, "  noise: unchanged")?,
        }
        */

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
