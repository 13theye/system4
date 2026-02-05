//! src/command_engine/mod.rs
//!
//! The main command engine for the application.
//! - Invoked by the Model in process_command_queue()
//! - Only fields are the handlers for different types of commands.
//!
//! Commands follow the following pattern:
//! - `Command` instances are created by text commands or UI
//! - `CommandEngine` receives the Commands as a Vec<Command>
//! - One of the CommandHandlers executes the Command by calling relevant function in `ExecutionContext`

pub mod builders;
pub mod commands;
pub mod context;
pub mod handlers;
//pub mod validation;

pub use builders::{DroneCommandBuilder, RhythmCommandBuilder};
pub use commands::{Command, CommandInner, CommandSource, CompositeCommand, SimpleCommand};
pub use context::ExecutionContext;
//pub use validation::{ValidationResult, VoiceValidator};

use handlers::{CircleCommandHandler, DroneCommandHandler, RhythmCommandHandler};
use std::time::Instant;

#[derive(Default)]
pub struct CommandEngine {
    drone_handler: DroneCommandHandler,
    rhythm_handler: RhythmCommandHandler,
    circle_handler: CircleCommandHandler,
}

impl CommandEngine {
    pub fn new() -> Self {
        Self {
            drone_handler: DroneCommandHandler::new(),
            rhythm_handler: RhythmCommandHandler::new(),
            circle_handler: CircleCommandHandler::new(),
        }
    }

    /// Execute one "tick" worth of commands.
    ///
    /// Note: this consumes a snapshot of commands (typically drained from the
    /// model). Any commands queued via `ExecutionContext::queue_command` during
    /// execution will be enqueued onto the model for the *next* tick.
    pub fn process_commands(
        &self,
        ctx: &mut dyn ExecutionContext,
        commands: Vec<Command>,
        now: Instant,
    ) {
        if commands.is_empty() {
            return;
        }

        let all_commands = commands;

        let mut final_commands = Vec::new();
        let mut map_osc: std::collections::HashMap<String, (usize, CommandSource)> =
            std::collections::HashMap::new();

        for command in all_commands.iter() {
            let key = command.dedup_key();

            if let Some((existing_idx, existing_source)) = map_osc.get(&key) {
                let existing_priority = command_priority(*existing_source);
                let current_priority = command_priority(command.source);

                if current_priority > existing_priority {
                    final_commands[*existing_idx] = None;
                    final_commands.push(Some(command.clone()));
                    map_osc.insert(key, (final_commands.len() - 1, command.source));
                }
            } else {
                final_commands.push(Some(command.clone()));
                map_osc.insert(key, (final_commands.len() - 1, command.source));
            }
        }

        for command in final_commands.into_iter().flatten() {
            self.execute_command(ctx, command, now);
        }
    }

    pub fn execute_command(&self, ctx: &mut dyn ExecutionContext, command: Command, now: Instant) {
        ctx.log_command(&command);

        match command.command {
            CommandInner::Composite(composite) => match composite {
                CompositeCommand::CreateDrone { config } => {
                    self.drone_handler
                        .create_drone(ctx, config, command.source, now);
                }
                CompositeCommand::ModifyDrone { voice_id, config } => {
                    self.drone_handler.modify_drone_params(
                        ctx,
                        voice_id,
                        config,
                        command.source,
                        now,
                    );
                }
                CompositeCommand::CreateRhythm { config } => {
                    self.rhythm_handler
                        .create_rhythm(ctx, config, command.source, now);
                }
                CompositeCommand::ModifyRhythm { voice_id, config } => {
                    self.rhythm_handler
                        .modify_rhythm(ctx, voice_id, config, command.source, now);
                }
                CompositeCommand::NewCircle {
                    voice_id,
                    circle_config,
                } => {
                    self.circle_handler
                        .add_circle(ctx, voice_id, circle_config, command.source);
                }
                CompositeCommand::Clear { voice_id } => {
                    if ctx.has_drone(voice_id) {
                        self.drone_handler.clear_drone(ctx, voice_id);
                    } else if ctx.has_rhythm(voice_id) {
                        self.rhythm_handler.clear_rhythm(ctx, voice_id, now);
                    }
                }
            },
            CommandInner::Simple(simple) => {
                self.execute_simple_command(ctx, simple, now);
            }
        }
    }

    fn execute_simple_command(
        &self,
        ctx: &mut dyn ExecutionContext,
        command: SimpleCommand,
        now: Instant,
    ) {
        use commands::SimpleCommand::*;

        match command {
            // Voice-level commands
            Alpha { voice_id, value } => {
                self.drone_handler.set_alpha(ctx, voice_id, value);
            }
            Volume { voice_id, value } => {
                self.drone_handler.set_volume(ctx, voice_id, value);
            }
            Feedback { voice_id, value } => {
                self.drone_handler.set_feedback(ctx, voice_id, value);
            }
            Vibration { voice_id, value } => {
                self.drone_handler.set_vibration(ctx, voice_id, value);
            }
            MoveEmitters { voice_id, value } => {
                self.drone_handler.move_emitters(ctx, voice_id, value);
            }

            // Circle-level commands
            OuterRadius {
                voice_id,
                circle_id,
                value,
            } => {
                self.circle_handler
                    .set_outer_radius(ctx, voice_id, circle_id, value);
            }
            InnerRadius {
                voice_id,
                circle_id,
                value,
            } => {
                self.circle_handler
                    .set_inner_radius(ctx, voice_id, circle_id, value);
            }
            Force {
                voice_id,
                circle_id,
                value,
            } => {
                self.circle_handler
                    .set_force(ctx, voice_id, circle_id, value);
            }
            Gravity {
                voice_id,
                circle_id,
                value,
            } => {
                self.circle_handler
                    .set_gravity(ctx, voice_id, circle_id, value);
            }
            Noise {
                voice_id,
                circle_id,
                value,
            } => {
                self.circle_handler
                    .set_noise(ctx, voice_id, circle_id, value);
            }
            CenterX {
                voice_id,
                circle_id,
                value,
            } => {
                self.circle_handler
                    .set_center_x(ctx, voice_id, circle_id, value);
            }
            CenterY {
                voice_id,
                circle_id,
                value,
            } => {
                self.circle_handler
                    .set_center_y(ctx, voice_id, circle_id, value);
            }
            ListCircles { voice_id } => {
                self.circle_handler.list_circles(ctx, voice_id);
            }
            RemoveCircle {
                voice_id,
                circle_id,
            } => {
                self.circle_handler.remove_circle(ctx, voice_id, circle_id);
            }

            // Rhythm commands
            RhythmCapacity { voice_id, value } => {
                self.rhythm_handler.set_capacity(ctx, voice_id, value, now);
            }
            RhythmNumWings { voice_id, value } => {
                self.rhythm_handler.set_num_wings(ctx, voice_id, value, now);
            }
            RhythmSubdivision { voice_id, value } => {
                self.rhythm_handler.set_subdivision(ctx, voice_id, value);
            }
            AddWings { voice_id, count } => {
                self.rhythm_handler.add_wings(ctx, voice_id, count, now);
            }
            RemoveWings { voice_id, count } => {
                self.rhythm_handler.remove_wings(ctx, voice_id, count, now);
            }
            ClearRhythm { voice_id } => {
                self.rhythm_handler.clear_rhythm(ctx, voice_id, now);
            }
            ClearDrone { voice_id } => {
                self.drone_handler.clear_drone(ctx, voice_id);
            }

            // Rhythm parameter ranges
            RhythmLengthRange { voice_id, range } => {
                self.rhythm_handler.set_length_range(ctx, voice_id, range);
            }
            RhythmVelocityRange { voice_id, range } => {
                self.rhythm_handler.set_velocity_range(ctx, voice_id, range);
            }
            RhythmCutoffRange { voice_id, range } => {
                self.rhythm_handler.set_cutoff_range(ctx, voice_id, range);
            }

            // Rhythm parameter modifications
            RhythmModifyLength {
                voice_id,
                modification,
            } => {
                self.rhythm_handler
                    .modify_length(ctx, voice_id, modification);
            }
            RhythmModifyVelocity {
                voice_id,
                modification,
            } => {
                self.rhythm_handler
                    .modify_velocity(ctx, voice_id, modification);
            }
            RhythmModifyCutoff {
                voice_id,
                modification,
            } => {
                self.rhythm_handler
                    .modify_cutoff(ctx, voice_id, modification);
            }

            // Mask Animation
            MaskAnimation {
                voice_id,
                new_origin,
                new_size,
                duration,
            } => {
                self.drone_handler
                    .animate_mask_to(ctx, voice_id, new_origin, new_size, duration);
            }
        }
    }
}

fn command_priority(source: CommandSource) -> u8 {
    match source {
        CommandSource::Terminal => 3,
        CommandSource::OSC => 2,
        CommandSource::UI => 1,
    }
}
