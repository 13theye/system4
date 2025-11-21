// src/model/command_builder.rs
//
// Centralized command building and validation system
// Reduces code duplication and provides consistent error handling

use super::controller::{Command, CommandInner, CommandSource, SimpleCommand};
use crate::groups::VoiceId;
use crate::terminals::commands::drone::DroneConfig;
use crate::terminals::commands::rhythm::RhythmConfig;
use std::collections::HashMap;

/// Parameter types for categorizing drone parameters
#[derive(Debug, Clone, PartialEq)]
pub enum ParameterType {
    Voice,  // Voice-level parameters (brightness, volume, feedback, vibration)
    Circle, // Circle-level parameters (gravity, force, radii, center, noise)
}

/// Parameter definition for command generation
#[derive(Debug, Clone)]
pub struct ParameterDef {
    pub param_type: ParameterType,
    pub name: &'static str,
}

/// Validation result for voice/circle existence
#[derive(Debug, Clone)]
pub enum ValidationResult {
    Success,
    VoiceNotFound(i32),
    CircleNotFound(i32, usize),
}

/// Centralized command builder for drone parameters
pub struct DroneCommandBuilder;

impl DroneCommandBuilder {
    /// Parameter definitions mapping for all drone parameters
    pub fn get_parameter_definitions() -> HashMap<&'static str, ParameterDef> {
        let mut defs = HashMap::new();

        // Voice-level parameters
        defs.insert(
            "brightness",
            ParameterDef {
                param_type: ParameterType::Voice,
                name: "brightness",
            },
        );
        defs.insert(
            "volume",
            ParameterDef {
                param_type: ParameterType::Voice,
                name: "volume",
            },
        );
        defs.insert(
            "feedback",
            ParameterDef {
                param_type: ParameterType::Voice,
                name: "feedback",
            },
        );
        defs.insert(
            "vibration",
            ParameterDef {
                param_type: ParameterType::Voice,
                name: "vibration",
            },
        );

        // Circle-level parameters
        defs.insert(
            "gravity",
            ParameterDef {
                param_type: ParameterType::Circle,
                name: "gravity",
            },
        );
        defs.insert(
            "force",
            ParameterDef {
                param_type: ParameterType::Circle,
                name: "force",
            },
        );
        defs.insert(
            "outer_radius",
            ParameterDef {
                param_type: ParameterType::Circle,
                name: "outer_radius",
            },
        );
        defs.insert(
            "inner_radius",
            ParameterDef {
                param_type: ParameterType::Circle,
                name: "inner_radius",
            },
        );
        defs.insert(
            "noise",
            ParameterDef {
                param_type: ParameterType::Circle,
                name: "noise",
            },
        );
        defs.insert(
            "center_x",
            ParameterDef {
                param_type: ParameterType::Circle,
                name: "center_x",
            },
        );
        defs.insert(
            "center_y",
            ParameterDef {
                param_type: ParameterType::Circle,
                name: "center_y",
            },
        );

        defs
    }

    /// Generate all parameter commands from DroneConfig
    pub fn generate_all_parameter_commands(
        config: &DroneConfig,
        voice_id: VoiceId,
        circle_id: usize,
        source: CommandSource,
    ) -> Vec<Command> {
        let mut commands = Vec::new();

        // Voice-level parameters
        commands.extend(Self::generate_voice_parameter_commands(
            config, voice_id, source,
        ));

        // Circle-level parameters
        commands.extend(Self::generate_circle_parameter_commands(
            config, voice_id, circle_id, source,
        ));

        commands
    }

    /// Generate only voice-level parameter commands
    pub fn generate_voice_parameter_commands(
        config: &DroneConfig,
        voice_id: VoiceId,
        source: CommandSource,
    ) -> Vec<Command> {
        let mut commands = Vec::new();

        if let Some(brightness) = config.brightness {
            commands.push(Command::new(
                CommandInner::Simple(SimpleCommand::Alpha {
                    voice_id,
                    value: brightness,
                }),
                source,
            ));
        }

        if let Some(volume) = config.volume {
            commands.push(Command::new(
                CommandInner::Simple(SimpleCommand::Volume {
                    voice_id,
                    value: volume,
                }),
                source,
            ));
        }

        if let Some(feedback) = config.feedback {
            commands.push(Command::new(
                CommandInner::Simple(SimpleCommand::Feedback {
                    voice_id,
                    value: feedback,
                }),
                source,
            ));
        }

        if let Some(vibration) = config.vibration {
            commands.push(Command::new(
                CommandInner::Simple(SimpleCommand::Vibration {
                    voice_id,
                    value: vibration,
                }),
                source,
            ));
        }

        commands
    }

    /// Generate only circle-level parameter commands
    pub fn generate_circle_parameter_commands(
        config: &DroneConfig,
        voice_id: VoiceId,
        circle_id: usize,
        source: CommandSource,
    ) -> Vec<Command> {
        let mut commands = Vec::new();

        if let Some(gravity) = config.gravity {
            commands.push(Command::new(
                CommandInner::Simple(SimpleCommand::Gravity {
                    voice_id,
                    circle_id,
                    value: gravity,
                }),
                source,
            ));
        }

        if let Some(force) = config.force {
            commands.push(Command::new(
                CommandInner::Simple(SimpleCommand::Force {
                    voice_id,
                    circle_id,
                    value: force,
                }),
                source,
            ));
        }

        if let Some(outer_radius) = config.outer_radius {
            commands.push(Command::new(
                CommandInner::Simple(SimpleCommand::OuterRadius {
                    voice_id,
                    circle_id,
                    value: outer_radius,
                }),
                source,
            ));
        }

        if let Some(inner_radius) = config.inner_radius {
            commands.push(Command::new(
                CommandInner::Simple(SimpleCommand::InnerRadius {
                    voice_id,
                    circle_id,
                    value: inner_radius,
                }),
                source,
            ));
        }

        if let Some(noise) = config.noise {
            commands.push(Command::new(
                CommandInner::Simple(SimpleCommand::Noise {
                    voice_id,
                    circle_id,
                    value: noise,
                }),
                source,
            ));
        }

        if let Some(center_x) = config.center_x {
            commands.push(Command::new(
                CommandInner::Simple(SimpleCommand::CenterX {
                    voice_id,
                    circle_id,
                    value: center_x,
                }),
                source,
            ));
        }

        if let Some(center_y) = config.center_y {
            commands.push(Command::new(
                CommandInner::Simple(SimpleCommand::CenterY {
                    voice_id,
                    circle_id,
                    value: center_y,
                }),
                source,
            ));
        }

        commands
    }

    /// Format standardized error message for voice not found
    pub fn format_voice_error(voice_id: VoiceId) -> String {
        format!("Voice {} does not exist", voice_id.to_i32())
    }

    /// Format standardized error message for circle not found
    pub fn format_circle_error(voice_id: VoiceId, circle_id: usize) -> String {
        format!(
            "Voice {} - Circle {} does not exist",
            voice_id.to_i32(),
            circle_id
        )
    }

    /// Format standardized success message for operations
    pub fn format_success_message(
        operation: &str,
        voice_id: VoiceId,
        circle_id: Option<usize>,
    ) -> String {
        match circle_id {
            Some(cid) => format!(
                "{} applied to Voice {} Circle {}",
                operation,
                voice_id.to_i32(),
                cid
            ),
            None => format!("{} applied to Voice {}", operation, voice_id.to_i32()),
        }
    }
}

/// Trait for types that can validate voice and circle existence
pub trait VoiceValidator {
    fn voice_exists(&self, voice_id: VoiceId) -> bool;
    fn circle_exists(&self, voice_id: VoiceId, circle_id: usize) -> bool;

    /// Validate voice existence and return standardized result
    fn validate_voice(&self, voice_id: VoiceId) -> ValidationResult {
        if self.voice_exists(voice_id) {
            ValidationResult::Success
        } else {
            ValidationResult::VoiceNotFound(voice_id.to_i32())
        }
    }

    /// Validate both voice and circle existence
    fn validate_voice_circle(&self, voice_id: VoiceId, circle_id: usize) -> ValidationResult {
        if !self.voice_exists(voice_id) {
            ValidationResult::VoiceNotFound(voice_id.to_i32())
        } else if !self.circle_exists(voice_id, circle_id) {
            ValidationResult::CircleNotFound(voice_id.to_i32(), circle_id)
        } else {
            ValidationResult::Success
        }
    }
}

impl ValidationResult {
    /// Convert validation result to error message
    pub fn to_error_message(&self) -> Option<String> {
        match self {
            ValidationResult::Success => None,
            ValidationResult::VoiceNotFound(voice_id) => {
                Some(format!("Voice {} does not exist", voice_id))
            }
            ValidationResult::CircleNotFound(voice_id, circle_id) => Some(format!(
                "Voice {} - Circle {} does not exist",
                voice_id, circle_id
            )),
        }
    }

    /// Check if validation was successful
    pub fn is_success(&self) -> bool {
        matches!(self, ValidationResult::Success)
    }
}

/// Centralized command builder for rhythm parameters
pub struct RhythmCommandBuilder;

impl RhythmCommandBuilder {
    /// Generate all parameter commands from RhythmConfig
    pub fn generate_all_parameter_commands(
        config: &RhythmConfig,
        voice_id: VoiceId,
        source: CommandSource,
    ) -> Vec<Command> {
        let mut commands = Vec::new();

        // Structure parameters
        commands.extend(Self::generate_structure_commands(config, voice_id, source));

        // Range parameters (for creation)
        commands.extend(Self::generate_range_commands(config, voice_id, source));

        // Modification parameters (for editing)
        commands.extend(Self::generate_modification_commands(
            config, voice_id, source,
        ));

        commands
    }

    /// Generate structure parameter commands (capacity, num_wings, subdivision)
    pub fn generate_structure_commands(
        config: &RhythmConfig,
        voice_id: VoiceId,
        source: CommandSource,
    ) -> Vec<Command> {
        let mut commands = Vec::new();

        if let Some(capacity) = config.capacity {
            commands.push(Command::new(
                CommandInner::Simple(SimpleCommand::RhythmCapacity {
                    voice_id,
                    value: capacity,
                }),
                source,
            ));
        }

        if let Some(num_wings) = config.num_wings {
            commands.push(Command::new(
                CommandInner::Simple(SimpleCommand::RhythmNumWings {
                    voice_id,
                    value: num_wings,
                }),
                source,
            ));
        }

        if let Some(subdivision) = config.subdivision {
            commands.push(Command::new(
                CommandInner::Simple(SimpleCommand::RhythmSubdivision {
                    voice_id,
                    value: subdivision,
                }),
                source,
            ));
        }

        commands
    }

    /// Generate range parameter commands (for makeRhythm)
    pub fn generate_range_commands(
        config: &RhythmConfig,
        voice_id: VoiceId,
        source: CommandSource,
    ) -> Vec<Command> {
        let mut commands = Vec::new();

        if let Some(range) = config.length_range {
            commands.push(Command::new(
                CommandInner::Simple(SimpleCommand::RhythmLengthRange { voice_id, range }),
                source,
            ));
        }

        if let Some(range) = config.velocity_range {
            commands.push(Command::new(
                CommandInner::Simple(SimpleCommand::RhythmVelocityRange { voice_id, range }),
                source,
            ));
        }

        if let Some(range) = config.cutoff_range {
            commands.push(Command::new(
                CommandInner::Simple(SimpleCommand::RhythmCutoffRange { voice_id, range }),
                source,
            ));
        }

        commands
    }

    /// Generate modification parameter commands (for editing)
    pub fn generate_modification_commands(
        config: &RhythmConfig,
        voice_id: VoiceId,
        source: CommandSource,
    ) -> Vec<Command> {
        let mut commands = Vec::new();

        if let Some(modification) = config.length_modification {
            commands.push(Command::new(
                CommandInner::Simple(SimpleCommand::RhythmModifyLength {
                    voice_id,
                    modification,
                }),
                source,
            ));
        }

        if let Some(modification) = config.velocity_modification {
            commands.push(Command::new(
                CommandInner::Simple(SimpleCommand::RhythmModifyVelocity {
                    voice_id,
                    modification,
                }),
                source,
            ));
        }

        if let Some(modification) = config.cutoff_modification {
            commands.push(Command::new(
                CommandInner::Simple(SimpleCommand::RhythmModifyCutoff {
                    voice_id,
                    modification,
                }),
                source,
            ));
        }

        commands
    }
}
