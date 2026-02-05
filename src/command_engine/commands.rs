//! src/command_engine/commands.rs
//!
//! Command types for the system

use crate::{
    groups::VoiceId,
    terminals::commands::{
        drone::DroneConfig,
        rhythm::{RangeSize, RhythmConfig, RhythmParamModification},
    },
};

use nannou::prelude::Vec2;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandSource {
    Terminal,
    OSC,
    UI,
}

#[derive(Debug, Clone)]
pub struct Command {
    pub command: CommandInner,
    pub source: CommandSource,
}

#[derive(Debug, Clone)]
pub enum SimpleCommand {
    Alpha {
        voice_id: VoiceId,
        value: f32,
    },
    Volume {
        voice_id: VoiceId,
        value: f32,
    },
    Feedback {
        voice_id: VoiceId,
        value: f32,
    },
    Vibration {
        voice_id: VoiceId,
        value: f32,
    },
    MoveEmitters {
        voice_id: VoiceId,
        value: f32,
    },
    OuterRadius {
        voice_id: VoiceId,
        circle_id: usize,
        value: f32,
    },
    InnerRadius {
        voice_id: VoiceId,
        circle_id: usize,
        value: f32,
    },
    Force {
        voice_id: VoiceId,
        circle_id: usize,
        value: f32,
    },
    Gravity {
        voice_id: VoiceId,
        circle_id: usize,
        value: f32,
    },
    Noise {
        voice_id: VoiceId,
        circle_id: usize,
        value: f32,
    },
    CenterX {
        voice_id: VoiceId,
        circle_id: usize,
        value: f32,
    },
    CenterY {
        voice_id: VoiceId,
        circle_id: usize,
        value: f32,
    },
    ListCircles {
        voice_id: VoiceId,
    },
    AddWings {
        voice_id: VoiceId,
        count: usize,
    },
    RemoveWings {
        voice_id: VoiceId,
        count: usize,
    },
    ClearRhythm {
        voice_id: VoiceId,
    },
    ClearDrone {
        voice_id: VoiceId,
    },
    RemoveCircle {
        voice_id: VoiceId,
        circle_id: i32,
    },
    // Rhythm structure parameters
    RhythmCapacity {
        voice_id: VoiceId,
        value: usize,
    },
    RhythmNumWings {
        voice_id: VoiceId,
        value: usize,
    },
    RhythmSubdivision {
        voice_id: VoiceId,
        value: prat::BeatSubdivision,
    },
    // Slot range parameters (for creation)
    RhythmLengthRange {
        voice_id: VoiceId,
        range: RangeSize,
    },
    RhythmVelocityRange {
        voice_id: VoiceId,
        range: RangeSize,
    },
    RhythmCutoffRange {
        voice_id: VoiceId,
        range: RangeSize,
    },
    // Slot modification parameters (for editing)
    RhythmModifyLength {
        voice_id: VoiceId,
        modification: RhythmParamModification,
    },
    RhythmModifyVelocity {
        voice_id: VoiceId,
        modification: RhythmParamModification,
    },
    RhythmModifyCutoff {
        voice_id: VoiceId,
        modification: RhythmParamModification,
    },
    MaskAnimation {
        voice_id: VoiceId,
        new_origin: Vec2,
        new_size: Vec2,
        duration: Duration,
    },
}

#[derive(Debug, Clone)]
pub enum CompositeCommand {
    CreateDrone {
        config: DroneConfig,
    },
    CreateRhythm {
        config: RhythmConfig,
    },
    ModifyDrone {
        voice_id: VoiceId,
        config: DroneConfig,
    },
    ModifyRhythm {
        voice_id: VoiceId,
        config: RhythmConfig,
    },
    NewCircle {
        voice_id: VoiceId,
        circle_config: DroneConfig,
    },
    Clear {
        voice_id: VoiceId,
    },
}

#[derive(Debug, Clone)]
pub enum CommandInner {
    Simple(SimpleCommand),
    Composite(CompositeCommand),
}

impl Command {
    pub fn new(command: CommandInner, source: CommandSource) -> Self {
        Self { command, source }
    }

    /// Generate a unique key for each voice/parameter combination to identify conflicts
    pub fn dedup_key(&self) -> String {
        match &self.command {
            CommandInner::Composite(composite) => match composite {
                CompositeCommand::CreateDrone { config } => {
                    format!("CreateDrone_{}", config.voice)
                }
                CompositeCommand::CreateRhythm { config } => {
                    format!("CreateRhythm_{:?}", config.voice)
                }
                CompositeCommand::ModifyDrone { voice_id, .. } => {
                    format!("ModifyDrone_{:?}", voice_id)
                }
                CompositeCommand::ModifyRhythm { voice_id, .. } => {
                    format!("ModifyRhythm_{:?}", voice_id)
                }
                CompositeCommand::NewCircle { voice_id, .. } => {
                    format!("NewCircle_{:?}", voice_id)
                }
                CompositeCommand::Clear { voice_id, .. } => {
                    format!("Clear_{:?}", voice_id)
                }
            },
            CommandInner::Simple(atomic) => match atomic {
                SimpleCommand::Alpha {
                    voice_id: voice, ..
                } => format!("Alpha_{:?}", voice),
                SimpleCommand::Volume {
                    voice_id: voice, ..
                } => format!("Volume_{:?}", voice),
                SimpleCommand::Feedback {
                    voice_id: voice, ..
                } => format!("Feedback_{:?}", voice),
                SimpleCommand::Vibration {
                    voice_id: voice, ..
                } => format!("Vibration_{:?}", voice),
                SimpleCommand::MoveEmitters {
                    voice_id: voice, ..
                } => format!("MoveEmitters_{:?}", voice),
                SimpleCommand::OuterRadius {
                    voice_id: voice, ..
                } => format!("OuterRadius_{:?}", voice),
                SimpleCommand::InnerRadius {
                    voice_id: voice, ..
                } => format!("InnerRadius_{:?}", voice),
                SimpleCommand::Force {
                    voice_id: voice, ..
                } => format!("Strength_{:?}", voice),
                SimpleCommand::Gravity {
                    voice_id: voice, ..
                } => format!("CenterBias_{:?}", voice),
                SimpleCommand::Noise {
                    voice_id: voice, ..
                } => format!("AngleVariation_{:?}", voice),
                SimpleCommand::CenterX {
                    voice_id: voice, ..
                } => format!("ForceCenterX_{:?}", voice),
                SimpleCommand::CenterY {
                    voice_id: voice, ..
                } => format!("ForceCenterY_{:?}", voice),
                SimpleCommand::ListCircles {
                    voice_id: voice, ..
                } => format!("ListCircles_{:?}", voice),
                SimpleCommand::AddWings {
                    voice_id: voice, ..
                } => format!("AddWings_{:?}", voice),
                SimpleCommand::RemoveWings {
                    voice_id: voice, ..
                } => format!("RemoveWings_{:?}", voice),
                SimpleCommand::ClearRhythm {
                    voice_id: voice, ..
                } => format!("ClearRhythm_{:?}", voice),
                SimpleCommand::ClearDrone {
                    voice_id: voice, ..
                } => format!("EraseDrone_{:?}", voice),
                SimpleCommand::RemoveCircle {
                    voice_id: voice,
                    circle_id,
                    ..
                } => format!("RemoveCircle_{:?}_{}", voice, circle_id),
                // Rhythm parameter commands
                SimpleCommand::RhythmCapacity { voice_id, .. } => {
                    format!("RhythmCapacity_{:?}", voice_id)
                }
                SimpleCommand::RhythmNumWings { voice_id, .. } => {
                    format!("RhythmNumWings_{:?}", voice_id)
                }
                SimpleCommand::RhythmSubdivision { voice_id, .. } => {
                    format!("RhythmSubdivision_{:?}", voice_id)
                }
                SimpleCommand::RhythmLengthRange { voice_id, .. } => {
                    format!("RhythmLengthRange_{:?}", voice_id)
                }
                SimpleCommand::RhythmVelocityRange { voice_id, .. } => {
                    format!("RhythmVelocityRange_{:?}", voice_id)
                }
                SimpleCommand::RhythmCutoffRange { voice_id, .. } => {
                    format!("RhythmCutoffRange_{:?}", voice_id)
                }
                SimpleCommand::RhythmModifyLength { voice_id, .. } => {
                    format!("RhythmModifyLength_{:?}", voice_id)
                }
                SimpleCommand::RhythmModifyVelocity { voice_id, .. } => {
                    format!("RhythmModifyVelocity_{:?}", voice_id)
                }
                SimpleCommand::RhythmModifyCutoff { voice_id, .. } => {
                    format!("RhythmModifyCutoff_{:?}", voice_id)
                }
                SimpleCommand::MaskAnimation { voice_id, .. } => {
                    format!("MaskAnimation_{:?}", voice_id)
                }
            },
        }
    }
}
