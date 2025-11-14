// src/model/terminal_processor.rs
//
// Terminal command processing extension for Model

use crate::{
    groups::VoiceId,
    model::{
        controller::{Command, CommandInner, CommandSource, CompositeCommand, SimpleCommand},
        Model,
    },
    terminals::commands::{
        drone::DroneConfigExt, TerminalCommand,
    },
};

impl Model {
    /// Process a TerminalCommand and queue the resulting Command(s)
    pub fn process_terminal_command(&mut self, command: TerminalCommand) {
        match command {
            TerminalCommand::CreateDrone(config) => {
                let voice_command = config.to_create_command(CommandSource::Terminal);
                println!("Queuing CreateDrone command with config: {:?}", config);
                self.queue_command(voice_command);
            }
            TerminalCommand::CreateSequencer { config } => {
                let voice_id = config.voice;
                let voice_command = Command::new(
                    CommandInner::Composite(CompositeCommand::CreateRhythm { config }),
                    CommandSource::Terminal,
                );
                println!("Queuing CreateSequencer command for voice: {:?}", voice_id);
                self.queue_command(voice_command);
            }
            TerminalCommand::ModifyVoice { voice_id, config } => {
                let voice_enum = VoiceId::from_i32(voice_id);
                let voice_command = config.to_modify_command(voice_enum, CommandSource::Terminal);
                println!(
                    "Queuing ModifyVoice command for voice: {:?} with config: {:?}",
                    voice_id, config
                );
                self.queue_command(voice_command);
            }
            TerminalCommand::ModifyVoiceCircle {
                voice_id,
                circle_id,
                config,
            } => {
                let voice_enum = VoiceId::from_i32(voice_id);

                // Generate circle-specific parameter commands
                let parameter_commands = config.generate_circle_parameter_commands(
                    voice_enum,
                    circle_id as usize,
                    CommandSource::Terminal,
                );

                println!(
                    "Queueing {} circle parameter commands for voice: {:?} circle: {:?}",
                    parameter_commands.len(),
                    voice_id,
                    circle_id
                );

                for cmd in parameter_commands {
                    self.queue_command(cmd);
                }
            }
            TerminalCommand::ListCircles { voice_id } => {
                let voice_enum = VoiceId::from_i32(voice_id);
                let voice_command = Command::new(
                    CommandInner::Simple(SimpleCommand::ListCircles {
                        voice_id: voice_enum,
                    }),
                    CommandSource::Terminal,
                );
                println!("Queueing ListCircles command for voice: {:?}", voice_id);
                self.queue_command(voice_command);
            }
            TerminalCommand::NewCircle { voice_id, config } => {
                let voice_enum = VoiceId::from_i32(voice_id);
                let voice_command = Command::new(
                    CommandInner::Composite(CompositeCommand::NewCircle {
                        voice_id: voice_enum,
                        config,
                    }),
                    CommandSource::Terminal,
                );
                println!("Queueing NewCircle command for voice: {:?}", voice_id);
                self.queue_command(voice_command);
            }
            TerminalCommand::RemoveCircle {
                voice_id,
                circle_id,
            } => {
                let voice_enum = VoiceId::from_i32(voice_id);
                let voice_command = Command::new(
                    CommandInner::Simple(SimpleCommand::RemoveCircle {
                        voice_id: voice_enum,
                        circle_id,
                    }),
                    CommandSource::Terminal,
                );
                println!(
                    "Queueing RemoveCircle command for voice: {:?} circle: {}",
                    voice_id, circle_id
                );
                self.queue_command(voice_command);
            }
            TerminalCommand::ModifyDroneParams { voice_id, config } => {
                let voice_enum = VoiceId::from_i32(voice_id);
                let voice_command = config.to_modify_command(voice_enum, CommandSource::Terminal);
                println!(
                    "Queueing ModifyDroneParams command for voice: {:?} with config: {:?}",
                    voice_id, config
                );
                self.queue_command(voice_command);
            }
            TerminalCommand::ModifyRhythmParams { voice_id, config } => {
                let voice_enum = VoiceId::from_i32(voice_id);
                println!(
                    "Queueing ModifyRhythmParams command for voice: {:?} with config: {:?}",
                    voice_id, config
                );
                let voice_command = Command::new(
                    CommandInner::Composite(CompositeCommand::ModifyRhythm {
                        voice_id: voice_enum,
                        config,
                    }),
                    CommandSource::Terminal,
                );
                self.queue_command(voice_command);
            }
            TerminalCommand::ModifyVoiceParams {
                voice_id,
                drone_config,
                rhythm_config,
            } => {
                let voice_enum = VoiceId::from_i32(voice_id);

                if let Some(config) = drone_config {
                    let voice_command =
                        config.to_modify_command(voice_enum, CommandSource::Terminal);
                    println!("Queueing ModifyVoiceParams drone command for voice: {:?} with config: {:?}", voice_id, config);
                    self.queue_command(voice_command);
                }

                if let Some(config) = rhythm_config {
                    println!("Queueing ModifyVoiceParams rhythm command for voice: {:?} with config: {:?}", voice_id, config);
                    let voice_command = Command::new(
                        CommandInner::Composite(CompositeCommand::ModifyRhythm {
                            voice_id: voice_enum,
                            config,
                        }),
                        CommandSource::Terminal,
                    );
                    self.queue_command(voice_command);
                }
            }
            TerminalCommand::AddWings { voice_id, count } => {
                let voice_enum = VoiceId::from_i32(voice_id);
                let wing_command = Command::new(
                    CommandInner::Simple(SimpleCommand::AddWings {
                        voice_id: voice_enum,
                        count,
                    }),
                    CommandSource::Terminal,
                );
                self.queue_command(wing_command);
            }
            TerminalCommand::RemoveWings { voice_id, count } => {
                let voice_enum = VoiceId::from_i32(voice_id);
                let wing_command = Command::new(
                    CommandInner::Simple(SimpleCommand::RemoveWings {
                        voice_id: voice_enum,
                        count,
                    }),
                    CommandSource::Terminal,
                );
                self.queue_command(wing_command);
            }
            TerminalCommand::Clear { voice_id } => {
                let voice_enum = VoiceId::from_i32(voice_id);
                let clear_command = Command::new(
                    CommandInner::Composite(CompositeCommand::Clear {
                        voice_id: voice_enum,
                    }),
                    CommandSource::Terminal,
                );
                self.queue_command(clear_command);
            }
        }
    }
}
