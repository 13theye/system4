// src/model/command_helpers.rs
//
// Small helpers for constructing common commands.

use crate::{
    command_engine::{Command, CommandInner, CommandSource, SimpleCommand},
    groups::VoiceId,
};

/// Create a drone using the unified command system.
pub fn make_drone_command(
    voice_id: i32,
    brightness: f32,
    volume: f32,
    force: f32,
    gravity: f32,
    feedback: f32,
    source: CommandSource,
) -> Command {
    use crate::terminals::commands::drone::DroneConfig;

    let config = DroneConfig {
        voice: VoiceId::from_i32(voice_id),
        brightness: Some(brightness),
        volume: Some(volume),
        gravity: Some(gravity),
        force: Some(force),
        feedback: Some(feedback),
        // Let other values use defaults from get_defaults_for_voice
        outer_radius: None,
        inner_radius: None,
        noise: None,
        vibration: None,
        center_x: None,
        center_y: None,
        additional_parameters: std::collections::HashMap::new(),
    };

    config.to_create_command(source)
}

/// Create an erase drone command using the unified command system.
pub fn erase_drone_command(voice: VoiceId, source: CommandSource) -> Command {
    Command::new(
        CommandInner::Simple(SimpleCommand::ClearDrone { voice_id: voice }),
        source,
    )
}
