// src/terminals/commands/drone.rs
//
// Drone command builder and configuration

use super::CommandBuilder;
use crate::terminals::parsing::{ParameterValue, ParseError};
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone)]
pub struct DroneBuilder {
    pub voice: Option<i32>,
    pub brightness: Option<f32>,
    pub volume: Option<f32>,
    pub gravity: Option<f32>,
    pub force: Option<f32>,
    pub trail: Option<f32>,
    pub outer_radius: Option<f32>,
    pub inner_radius: Option<f32>,
    pub noise: Option<f32>,
    pub vibration: Option<f32>,
    pub feedback: Option<f32>,
    pub center_x: Option<f32>,
    pub center_y: Option<f32>,
    pub parameters: HashMap<String, ParameterValue>,
}

impl CommandBuilder for DroneBuilder {
    type Config = DroneConfig;

    fn new() -> Self {
        Self {
            voice: None,
            brightness: None,
            volume: None,
            gravity: None,
            force: None,
            trail: None,
            outer_radius: None,
            inner_radius: None,
            noise: None,
            vibration: None,
            feedback: None,
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
            voice: self.voice.unwrap_or(1),                   // Default to Voice1
            brightness: self.brightness.unwrap_or(0.7),       // Default brightness
            volume: self.volume.unwrap_or(0.5),               // Default volume
            gravity: self.gravity.unwrap_or(0.0),             // Default gravity
            force: self.force.unwrap_or(10.0),                // Default force
            trail: self.trail.unwrap_or(0.0),                 // Default trail
            outer_radius: self.outer_radius.unwrap_or(800.0), // Default outer radius
            inner_radius: self.inner_radius.unwrap_or(200.0), // Default inner radius
            noise: self.noise.unwrap_or(0.0),                 // Default noise/angle variation
            vibration: self.vibration.unwrap_or(0.1),         // Default vibration/position offset
            feedback: self.feedback.unwrap_or(0.0),           // Default feedback
            center_x: self.center_x.unwrap_or(0.0),           // Default center X
            center_y: self.center_y.unwrap_or(0.0),           // Default center Y
            additional_parameters: self.parameters,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DroneConfig {
    pub voice: i32,
    pub brightness: f32,
    pub volume: f32,
    pub gravity: f32,
    pub force: f32,
    pub trail: f32,
    pub outer_radius: f32,
    pub inner_radius: f32,
    pub noise: f32,
    pub vibration: f32,
    pub feedback: f32,
    pub center_x: f32,
    pub center_y: f32,
    pub additional_parameters: HashMap<String, ParameterValue>,
}

impl fmt::Display for DroneConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "DroneConfig {{")?;
        writeln!(f, "  voice: {}", self.voice)?;
        writeln!(f, "  brightness: {}", self.brightness)?;
        writeln!(f, "  volume: {}", self.volume)?;
        writeln!(f, "  gravity: {}", self.gravity)?;
        writeln!(f, "  force: {}", self.force)?;
        writeln!(f, "  trail: {}", self.trail)?;
        writeln!(f, "  outer_radius: {}", self.outer_radius)?;
        writeln!(f, "  inner_radius: {}", self.inner_radius)?;
        writeln!(f, "  noise: {}", self.noise)?;
        writeln!(f, "  vibration: {}", self.vibration)?;
        writeln!(f, "  feedback: {}", self.feedback)?;
        writeln!(f, "  center_x: {}", self.center_x)?;
        writeln!(f, "  center_y: {}", self.center_y)?;

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
