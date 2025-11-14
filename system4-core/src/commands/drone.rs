// system4-core/src/commands/drone.rs
// Drone command builder and configuration

use super::TerminalCommandBuilder;
use crate::parsing::{ParameterValue, ParseError};
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
            voice: self.voice.unwrap_or(0), // Voice 0 is default
            brightness: self.brightness,
            volume: self.volume,
            feedback: self.feedback,
            vibration: self.vibration,
            gravity: self.gravity,
            force: self.force,
            outer_radius: self.outer_radius,
            inner_radius: self.inner_radius,
            noise: self.noise,
            center_x: self.center_x,
            center_y: self.center_y,
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
    /// Get default values for a specific voice
    pub fn get_defaults_for_voice(voice: i32) -> Self {
        let (default_center_x, default_center_y) = match voice {
            0 => (-950.0, 0.0),  // Voice0
            3 => (950.0, 0.0),   // Voice3
            _ => (0.0, 0.0),
        };

        let gravity = match voice {
            0 => 0.5,   // Voice0
            3 => 1.5,   // Voice3
            _ => 0.0,
        };

        Self {
            voice,
            brightness: Some(1.0),
            volume: Some(1.0),
            feedback: Some(0.8),
            vibration: Some(0.2),
            center_x: Some(default_center_x),
            center_y: Some(default_center_y),
            outer_radius: Some(1600.0),
            inner_radius: Some(600.0),
            gravity: Some(gravity),
            force: Some(4.0),
            noise: Some(0.1),
            additional_parameters: HashMap::new(),
        }
    }

    /// Merge with another config, preferring values from the other config
    pub fn merge_with(&self, other: &DroneConfig) -> Self {
        Self {
            voice: other.voice, // Always use the voice from other
            brightness: other.brightness.or(self.brightness),
            volume: other.volume.or(self.volume),
            feedback: other.feedback.or(self.feedback),
            vibration: other.vibration.or(self.vibration),
            center_x: other.center_x.or(self.center_x),
            center_y: other.center_y.or(self.center_y),
            outer_radius: other.outer_radius.or(self.outer_radius),
            inner_radius: other.inner_radius.or(self.inner_radius),
            gravity: other.gravity.or(self.gravity),
            force: other.force.or(self.force),
            noise: other.noise.or(self.noise),
            additional_parameters: {
                let mut params = self.additional_parameters.clone();
                params.extend(other.additional_parameters.clone());
                params
            },
        }
    }
}

impl fmt::Display for DroneConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "  voice: {}", self.voice)?;
        if let Some(b) = self.brightness {
            writeln!(f, "  brightness: {}", b)?;
        }
        if let Some(v) = self.volume {
            writeln!(f, "  volume: {}", v)?;
        }
        if let Some(fb) = self.feedback {
            writeln!(f, "  feedback: {}", fb)?;
        }
        if let Some(vib) = self.vibration {
            writeln!(f, "  vibration: {}", vib)?;
        }
        if let Some(cx) = self.center_x {
            writeln!(f, "  centerX: {}", cx)?;
        }
        if let Some(cy) = self.center_y {
            writeln!(f, "  centerY: {}", cy)?;
        }
        if let Some(or) = self.outer_radius {
            writeln!(f, "  outerRadius: {}", or)?;
        }
        if let Some(ir) = self.inner_radius {
            writeln!(f, "  innerRadius: {}", ir)?;
        }
        if let Some(g) = self.gravity {
            writeln!(f, "  gravity: {}", g)?;
        }
        if let Some(force) = self.force {
            writeln!(f, "  force: {}", force)?;
        }
        if let Some(n) = self.noise {
            writeln!(f, "  noise: {}", n)?;
        }
        for (key, value) in &self.additional_parameters {
            writeln!(f, "  {}: {}", key, value)?;
        }
        Ok(())
    }
}
