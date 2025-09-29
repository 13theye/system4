// src/terminals/parsing/parameter.rs

use std::fmt;

/// Voice type for parameter validation
#[derive(Debug, Clone, PartialEq)]
pub enum VoiceType {
    Drone,
    Rhythm,
}

/// Parameter categories for validation
#[derive(Debug, Clone, PartialEq)]
pub enum ParameterCategory {
    Drone,
    Rhythm,
}

/// Parameter::Value pair
#[derive(Debug, Clone)]
pub enum ParameterValue {
    String(String),
    Number(f32),
}

impl fmt::Display for ParameterValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParameterValue::String(s) => write!(f, "\"{}\"", s),
            ParameterValue::Number(n) => write!(f, "{}", n),
        }
    }
}

/// Categorize a parameter name for validation
pub fn categorize_parameter(param_name: &str) -> Option<ParameterCategory> {
    match param_name {
        // Drone parameters
        "brightness" | "volume" | "feedback" | "vibration" | "gravity" | "force"
        | "outerRadius" | "innerRadius" | "noise" | "centerX" | "centerY" | "newCircle"
        | "removeCircle" => Some(ParameterCategory::Drone),
        // Rhythm parameters
        "capacity" | "wings" | "sub" | "addWings" | "removeWings" => {
            Some(ParameterCategory::Rhythm)
        }
        // Voice parameter (used in both contexts)
        "voice" => None, // Special case - not categorized
        // Unknown parameter
        _ => None,
    }
}