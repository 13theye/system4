// src/terminals/commands/mod.rs
//
// Command definitions and builder traits for NTerminal

pub mod drone;

use super::parsing::{ParameterValue, ParseError};
use std::fmt;

/// All possible commands that can be parsed
#[derive(Debug, Clone)]
pub enum TerminalCommand {
    CreateDrone(drone::DroneConfig),
    ModifyVoice {
        voice: i32,
        config: drone::DroneConfig,
    },
    ModifyVoiceCircle {
        voice: i32,
        circle: i32,
        config: drone::DroneConfig,
    },
}

impl fmt::Display for TerminalCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TerminalCommand::CreateDrone(config) => {
                writeln!(f, "CreateDrone:")?;
                write!(f, "{}", config)
            }
            TerminalCommand::ModifyVoice { voice, config } => {
                writeln!(f, "ModifyDrone voice {}:", voice)?;
                write!(f, "{}", config)
            }
            TerminalCommand::ModifyVoiceCircle {
                voice,
                circle,
                config,
            } => {
                writeln!(f, "ModifyDrone voice {} circle {}:", voice, circle)?;
                write!(f, "{}", config)
            }
        }
    }
}

/// Trait for command builders that follow the builder pattern
pub trait TerminalCommandBuilder {
    type Config;

    fn new() -> Self;
    fn set_parameter(&mut self, name: &str, value: ParameterValue) -> Result<(), ParseError>;
    fn build(self) -> Self::Config;
}
