// src/terminals/commands/mod.rs
//
// Command definitions and builder traits for NTerminal

pub mod drone;
pub mod rhythm;

use super::parsing::{ParameterValue, ParseError};
use crate::command_engine::commands::FormationType;
use std::fmt;

/// All possible commands that can be parsed
#[derive(Debug, Clone)]
pub enum TerminalCommand {
    CreateDrone(drone::DroneConfig),
    CreateSequencer {
        config: rhythm::RhythmConfig,
    },
    ModifyVoice {
        voice_id: i32,
        config: drone::DroneConfig,
    },
    ModifyVoiceCircle {
        voice_id: i32,
        circle_id: i32,
        config: drone::DroneConfig,
    },
    ModifyDroneParams {
        voice_id: i32,
        config: drone::DroneConfig,
    },
    ModifyRhythmParams {
        voice_id: i32,
        config: rhythm::RhythmConfig,
    },
    ModifyVoiceParams {
        voice_id: i32,
        drone_config: Option<drone::DroneConfig>,
        rhythm_config: Option<rhythm::RhythmConfig>,
    },
    ListCircles {
        voice_id: i32,
    },
    NewFormation {
        voice_id: i32,
        config: drone::DroneConfig,
        formation_type: FormationType,
    },
    RemoveCircle {
        voice_id: i32,
        circle_id: i32,
    },
    AddWings {
        voice_id: i32,
        count: usize,
    },
    RemoveWings {
        voice_id: i32,
        count: usize,
    },
    Clear {
        voice_id: i32,
    },
    /// Trigger AI-based rhythm generation for a voice.
    ///
    /// Example: voice(2).generateRhythm();
    GenerateRhythm {
        voice_id: i32,
    },
}

impl fmt::Display for TerminalCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TerminalCommand::CreateDrone(config) => {
                writeln!(f, "CreateDrone:")?;
                write!(f, "{}", config)
            }
            TerminalCommand::CreateSequencer { config } => {
                writeln!(f, "CreateSequencer for voice {}:", config.voice)?;
                write!(f, "{}", config)
            }
            TerminalCommand::ModifyVoice { voice_id, config } => {
                writeln!(f, "ModifyDrone voice {}:", voice_id)?;
                write!(f, "{}", config)
            }
            TerminalCommand::ModifyVoiceCircle {
                voice_id,
                circle_id,
                config,
            } => {
                writeln!(f, "ModifyDrone voice {} circle {}:", voice_id, circle_id)?;
                write!(f, "{}", config)
            }
            TerminalCommand::ListCircles { voice_id } => {
                write!(f, "ListCircles for voice {}", voice_id)
            }
            TerminalCommand::NewFormation {
                voice_id,
                config,
                formation_type,
            } => {
                writeln!(f, "New{:?} for voice {}:", formation_type, voice_id)?;
                write!(f, "{}", config)
            }
            TerminalCommand::RemoveCircle {
                voice_id,
                circle_id,
            } => write!(
                f,
                "RemoveCircle for voice {} circle {}:",
                voice_id, circle_id
            ),
            TerminalCommand::ModifyDroneParams { voice_id, config } => {
                writeln!(f, "ModifyDroneParams voice {}:", voice_id)?;
                write!(f, "{}", config)
            }
            TerminalCommand::ModifyRhythmParams { voice_id, config } => {
                writeln!(f, "ModifyRhythmParams voice {}:", voice_id)?;
                write!(f, "{}", config)
            }
            TerminalCommand::ModifyVoiceParams {
                voice_id,
                drone_config,
                rhythm_config,
            } => {
                writeln!(f, "ModifyVoiceParams voice {}:", voice_id)?;
                if let Some(config) = drone_config {
                    writeln!(f, "Drone params:")?;
                    writeln!(f, "{}", config)?;
                }
                if let Some(config) = rhythm_config {
                    writeln!(f, "Rhythm params:")?;
                    write!(f, "{}", config)?;
                }
                Ok(())
            }
            TerminalCommand::AddWings { voice_id, count } => {
                write!(f, "AddWings voice {} count {}", voice_id, count)
            }
            TerminalCommand::RemoveWings { voice_id, count } => {
                write!(f, "RemoveWings voice {} count {}", voice_id, count)
            }
            TerminalCommand::Clear { voice_id } => {
                write!(f, "Clear voice {}", voice_id)
            }
            TerminalCommand::GenerateRhythm { voice_id } => {
                write!(f, "GenerateRhythm voice {}", voice_id)
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
