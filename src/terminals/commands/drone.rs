// src/terminals/commands/drone.rs
//
// Drone command builder and configuration

use super::TerminalCommandBuilder;
use crate::command_engine::commands::FormationType;
use crate::command_engine::DroneCommandBuilder;
use crate::command_engine::{Command, CommandInner, CommandSource, CompositeCommand};
use crate::groups::VoiceId;
use crate::terminals::parsing::{ParameterValue, ParseError};
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone)]
pub struct DroneBuilder {
    pub voice: Option<VoiceId>,
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
    pub formation_type: FormationType,
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
            formation_type: FormationType::WindCircle,
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
            voice: self.voice.unwrap_or(VoiceId::Voice0), // Voice is always required
            brightness: self.brightness,                  // None = unchanged, Some = set to value
            volume: self.volume,                          // None = unchanged, Some = set to value
            feedback: self.feedback,                      // None = unchanged, Some = set to value
            vibration: self.vibration,                    // None = unchanged, Some = set to value

            gravity: self.gravity, // None = unchanged, Some = set to value
            force: self.force,     // None = unchanged, Some = set to value
            outer_radius: self.outer_radius, // None = unchanged, Some = set to value
            inner_radius: self.inner_radius, // None = unchanged, Some = set to value
            noise: self.noise,     // None = unchanged, Some = set to value
            center_x: self.center_x, // None = unchanged, Some = set to value
            center_y: self.center_y, // None = unchanged, Some = set to value

            additional_parameters: self.parameters,
            formation_type: self.formation_type,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DroneConfig {
    pub voice: VoiceId,
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
    pub formation_type: FormationType,
}

impl DroneConfig {
    /// Get default values for a specific voice
    pub fn get_defaults_for_voice(voice: VoiceId) -> Self {
        let (default_center_x, default_center_y) = match voice {
            VoiceId::Voice0 => (-950.0, 0.0),
            VoiceId::Voice3 => (950.0, 0.0),
            _ => (0.0, 0.0),
        };

        let gravity = match voice {
            VoiceId::Voice0 => 0.5,
            VoiceId::Voice3 => 1.5,
            _ => 0.0,
        };

        Self {
            voice,
            brightness: Some(0.0),
            volume: Some(0.0),
            gravity: Some(gravity),
            force: Some(0.0),
            feedback: Some(0.0),
            outer_radius: Some(800.0),
            inner_radius: Some(200.0),
            noise: Some(0.0),
            vibration: Some(0.0),
            center_x: Some(default_center_x),
            center_y: Some(default_center_y),
            additional_parameters: HashMap::new(),
            formation_type: FormationType::WindCircle,
        }
    }

    /// Merge this config with defaults, keeping specified values and using defaults for None values
    pub fn merge_with_defaults(self) -> Self {
        let defaults = Self::get_defaults_for_voice(self.voice);

        Self {
            voice: self.voice,
            brightness: self.brightness.or(defaults.brightness),
            volume: self.volume.or(defaults.volume),
            gravity: self.gravity.or(defaults.gravity),
            force: self.force.or(defaults.force),
            feedback: self.feedback.or(defaults.feedback),
            outer_radius: self.outer_radius.or(defaults.outer_radius),
            inner_radius: self.inner_radius.or(defaults.inner_radius),
            noise: self.noise.or(defaults.noise),
            vibration: self.vibration.or(defaults.vibration),
            center_x: self.center_x.or(defaults.center_x),
            center_y: self.center_y.or(defaults.center_y),
            additional_parameters: self.additional_parameters,
            formation_type: self.formation_type,
        }
    }

    /// Generate parameter update commands from this config (assumes all values are Some)
    pub fn generate_parameter_commands(
        &self,
        voice_id: VoiceId,
        circle_id: usize,
        source: CommandSource,
    ) -> Vec<Command> {
        DroneCommandBuilder::generate_all_parameter_commands(self, voice_id, circle_id, source)
    }

    /// Generate only circle-specific parameter commands (for modifying existing circles)
    pub fn generate_circle_parameter_commands(
        &self,
        voice_id: VoiceId,
        circle_id: usize,
        source: CommandSource,
    ) -> Vec<Command> {
        DroneCommandBuilder::generate_circle_parameter_commands(self, voice_id, circle_id, source)
    }

    /// Convert this DroneConfig to a CreateDrone VoiceCommand
    pub fn to_create_command(&self, source: CommandSource) -> Command {
        Command::new(
            CommandInner::Composite(CompositeCommand::CreateDrone {
                config: self.clone(),
            }),
            source,
        )
    }

    /// Convert this DroneConfig to a ModifyDrone VoiceCommand for the specified voice
    pub fn to_modify_command(&self, voice: VoiceId, source: CommandSource) -> Command {
        Command::new(
            CommandInner::Composite(CompositeCommand::ModifyDrone {
                voice_id: voice,
                config: self.clone(),
            }),
            source,
        )
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
