// src/terminals/commands/mod.rs
//
// Command definitions and builder traits for NTerminal

pub mod drone;

use super::parsing::{ParameterValue, ParseError};
use std::fmt;

/// All possible commands that can be parsed
#[derive(Debug, Clone)]
pub enum Command {
    CreateDrone(drone::DroneConfig),
    ModifyDrone {
        name: String,
        config: drone::DroneConfig,
    },
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Command::CreateDrone(config) => {
                writeln!(f, "CreateDrone:")?;
                write!(f, "{}", config)
            }
            Command::ModifyDrone { name, config } => {
                writeln!(f, "ModifyDrone \"{}\":", name)?;
                write!(f, "{}", config)
            }
        }
    }
}

/// Trait for command builders that follow the builder pattern
pub trait CommandBuilder {
    type Config;

    fn new() -> Self;
    fn set_parameter(&mut self, name: &str, value: ParameterValue) -> Result<(), ParseError>;
    fn build(self) -> Self::Config;
}
