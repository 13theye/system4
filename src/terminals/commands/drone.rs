// src/terminals/commands/drone.rs
//
// Drone command builder and configuration

use super::CommandBuilder;
use crate::terminals::parsing::{ParameterValue, ParseError};
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone)]
pub struct DroneBuilder {
    pub name: Option<String>,
    pub brightness: Option<f32>,
    pub volume: Option<f32>,
    pub gravity: Option<f32>,
    pub force: Option<f32>,
    pub trail: Option<f32>,
    pub parameters: HashMap<String, ParameterValue>,
}

impl CommandBuilder for DroneBuilder {
    type Config = DroneConfig;

    fn new() -> Self {
        Self {
            name: None,
            brightness: None,
            volume: None,
            gravity: None,
            force: None,
            trail: None,
            parameters: HashMap::new(),
        }
    }

    fn set_parameter(&mut self, name: &str, value: ParameterValue) -> Result<(), ParseError> {
        match name {
            "name" => {
                if let ParameterValue::String(s) = value {
                    self.name = Some(s);
                } else {
                    return Err(ParseError::UnexpectedToken {
                        expected: "string".to_string(),
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
            "trail" => {
                if let ParameterValue::Number(n) = value {
                    self.trail = Some(n);
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
            name: self.name.unwrap_or_else(|| "Unnamed Drone".to_string()),
            brightness: self.brightness.unwrap_or(1.0),
            volume: self.volume.unwrap_or(0.5),
            gravity: self.gravity.unwrap_or(0.0),
            force: self.force.unwrap_or(1.0),
            trail: self.trail.unwrap_or(0.5),
            additional_parameters: self.parameters,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DroneConfig {
    pub name: String,
    pub brightness: f32,
    pub volume: f32,
    pub gravity: f32,
    pub force: f32,
    pub trail: f32,
    pub additional_parameters: HashMap<String, ParameterValue>,
}

impl fmt::Display for DroneConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "DroneConfig {{")?;
        writeln!(f, "  name: \"{}\"", self.name)?;
        writeln!(f, "  brightness: {}", self.brightness)?;
        writeln!(f, "  volume: {}", self.volume)?;
        writeln!(f, "  gravity: {}", self.gravity)?;
        writeln!(f, "  force: {}", self.force)?;
        writeln!(f, "  trail: {}", self.trail)?;

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
