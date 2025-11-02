// src/voice/controller.rs
//
// Command extension for Model

use crate::{
    forces::WindCircleParams,
    groups::{Rhythm, Voice, VoiceId},
    model::{
        command_builder::{
            DroneCommandBuilder, RhythmCommandBuilder, ValidationResult, VoiceValidator,
        },
        Model,
    },
    terminals::commands::{drone::DroneConfig, rhythm::RhythmConfig},
    view::RhythmFormationType,
};
use nannou::wgpu::{Device, Queue};

#[derive(Debug, Clone)]
pub enum CommandSource {
    Terminal,
    Osc,
    Ui,
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
        range: crate::terminals::commands::rhythm::RangeSize,
    },
    RhythmVelocityRange {
        voice_id: VoiceId,
        range: crate::terminals::commands::rhythm::RangeSize,
    },
    RhythmCutoffRange {
        voice_id: VoiceId,
        range: crate::terminals::commands::rhythm::RangeSize,
    },
    // Slot modification parameters (for editing)
    RhythmModifyLength {
        voice_id: VoiceId,
        modification: crate::terminals::commands::rhythm::ParameterModification,
    },
    RhythmModifyVelocity {
        voice_id: VoiceId,
        modification: crate::terminals::commands::rhythm::ParameterModification,
    },
    RhythmModifyCutoff {
        voice_id: VoiceId,
        modification: crate::terminals::commands::rhythm::ParameterModification,
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
        config: DroneConfig,
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
}

/// Get priority value from a command's source (lower = higher priority)
fn get_command_priority(command: &Command) -> u8 {
    match &command.source {
        CommandSource::Terminal => 0,
        CommandSource::Osc => 1,
        CommandSource::Ui => 2,
    }
}

/// Generate a unique key for each voice/parameter combination to identify conflicts
fn get_command_key(command: &Command) -> String {
    match &command.command {
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
        },
    }
}

/// Extension of Model that add controller functions
impl Model {
    /// Add a command to the queue for later processing
    pub fn queue_command(&mut self, command: Command) {
        self.command_queue.push(command);
    }

    /// Process all queued commands with priority resolution (Terminal > OSC > UI)
    pub fn process_command_queue(&mut self, time: f32) {
        if self.command_queue.is_empty() {
            return;
        }

        // Take all commands from queue
        let all_commands: Vec<Command> = self.command_queue.drain(..).collect();

        // Keep only the highest priority command for each voice/parameter combination
        let mut final_commands = Vec::new();

        for command in all_commands {
            let current_priority = get_command_priority(&command);
            let command_key = get_command_key(&command);

            // Check if we already have a command for this voice/parameter combination
            if let Some(existing_index) = final_commands
                .iter()
                .position(|existing_cmd| get_command_key(existing_cmd) == command_key)
            {
                let existing_priority = get_command_priority(&final_commands[existing_index]);

                // Replace if current command has higher priority (lower number = higher priority)
                if current_priority < existing_priority {
                    final_commands[existing_index] = command;
                }
                // Otherwise discard the current command (lower priority)
            } else {
                // No existing command for this combination, add it
                final_commands.push(command);
            }
        }

        // Execute all final commands
        // (display updates happen automatically in execute_command)
        for command in final_commands {
            self.execute_command(command, time);
        }
    }

    /// Apply a command immediately without queueing
    pub fn execute_command(&mut self, command: Command, time: f32) {
        // Send all commands to terminal display for visualization
        self.terminal_manager.borrow_mut().process_command(&command);

        match command.command {
            CommandInner::Composite(composite) => match composite {
                CompositeCommand::CreateDrone { config } => {
                    // Convert voice ID to Voice enum
                    let voice_id = config.voice;

                    if self.voices.contains_key(&config.voice) {
                        // Voice already exists, do nothing
                        println!("Controller: Voice {} already exists", &config.voice);
                        return;
                    }

                    // Merge config with defaults as necessary
                    let resolved_config = config.merge_with_defaults();

                    // Phase 1: Initialize drone structure (WindCircle and emitters)
                    let mut voice = Voice::new_with_id(voice_id);
                    let circle_id = voice.initialize_drone(
                        &resolved_config,
                        self.particle_system.default_particle_color,
                        self.particle_system.global_max_spawn_rate,
                    );

                    self.osc_send
                        .send_drone_on_off(resolved_config.voice.to_i32(), 1);
                    voice.set_is_spawning(true);

                    // Insert voice before applying parameters so validation can find it
                    self.voices.insert(voice_id, voice);

                    // Phase 2: Apply parameters through the command pipeline
                    // This is redundant until we make a default drone maker

                    let parameter_commands = DroneCommandBuilder::generate_all_parameter_commands(
                        &resolved_config,
                        voice_id,
                        circle_id,
                        command.source.clone(),
                    );

                    // Queue parameter commands to avoid recursive execution
                    // They will be processed in the next command queue cycle
                    for param_cmd in parameter_commands {
                        self.command_queue.push(param_cmd);
                    }
                }
                CompositeCommand::CreateRhythm { config } => {
                    let voice_id = config.voice;

                    if self.rhythms.contains_key(&voice_id) {
                        println!("Controller: Rhythm for voice {} already exists", voice_id);
                        return;
                    }

                    // Merge config with defaults
                    let resolved_config = config.merge_with_defaults();

                    // Phase 1: Create basic rhythm structure
                    let params = resolved_config.to_rhythm_params();
                    let mut rhythm = Rhythm::new_with_params(voice_id, params);

                    // Initialize slots and wings
                    rhythm.initialize_slots(&mut self.rng);
                    rhythm.randomize_wings(&mut self.rng);

                    // Start sequencer
                    rhythm.add_sequencer(&mut self.sequencer_service);

                    // Subscribe to sequencer callbacks
                    if let Some(data_rx) = self.sequencer_service.get_data_rx(voice_id) {
                        rhythm.set_sequencer_data_rx(data_rx);
                    }

                    let radius = if voice_id == VoiceId::Voice1 {
                        800.0
                    } else {
                        450.0
                    };

                    // Create the RhythmFormation
                    self.rhythm_view.add_formation(
                        voice_id,
                        RhythmFormationType::Circle { radius },
                        rhythm.get_params(),
                        time,
                    );

                    // Insert rhythm before applying parameters so validation can find it
                    self.rhythms.insert(voice_id, rhythm);

                    // Phase 2: Apply parameters through the command pipeline
                    // This is redundant until we make a default rhythm maker
                    let parameter_commands = RhythmCommandBuilder::generate_all_parameter_commands(
                        &resolved_config,
                        voice_id,
                        command.source.clone(),
                    );

                    // Queue parameter commands to avoid recursive execution
                    // They will be processed in the next command queue cycle
                    for param_cmd in parameter_commands {
                        self.command_queue.push(param_cmd);
                    }

                    // Phase 3: Start all sequencers to sync on the next beat
                    self.sequencer_service.start_all();

                    let status_message =
                        format!("Voice {} - Created rhythm sequencer", voice_id.to_i32());
                    println!("{}", status_message);
                    self.command_input.set_success_message(status_message);
                }
                CompositeCommand::ModifyDrone { voice_id, config } => {
                    let validation = self.validate_voice(voice_id);
                    if !self.validate_and_handle_error(validation, "ModifyDrone") {
                        return;
                    }

                    // Generate atomic commands for voice-level parameters using DroneCommandBuilder
                    let parameter_commands = DroneCommandBuilder::generate_voice_parameter_commands(
                        &config,
                        voice_id,
                        command.source.clone(),
                    );

                    // Queue parameter commands
                    for cmd in parameter_commands {
                        self.command_queue.push(cmd);
                    }

                    // Circle-level parameters require circle_id, which ModifyDrone doesn't specify
                    // These should be handled by explicit circle commands instead
                }
                CompositeCommand::ModifyRhythm { voice_id, config } => {
                    let validation = self.validate_voice(voice_id);
                    if !self.validate_and_handle_error(validation, "ModifyRhythm") {
                        return;
                    }

                    // Check if rhythm exists for this voice
                    if !self.rhythms.contains_key(&voice_id) {
                        let error_message =
                            format!("Voice {} has no rhythm to modify", voice_id.to_i32());
                        println!("Error: {}", error_message);
                        self.command_input.set_error_message(error_message);
                        return;
                    }

                    // Generate and queue parameter commands using RhythmCommandBuilder
                    let parameter_commands = RhythmCommandBuilder::generate_all_parameter_commands(
                        &config,
                        voice_id,
                        command.source.clone(),
                    );

                    for cmd in parameter_commands {
                        self.command_queue.push(cmd);
                    }
                }
                CompositeCommand::NewCircle { voice_id, config } => {
                    let validation = self.validate_voice(voice_id);
                    if !self.validate_and_handle_error(validation, "NewCircle") {
                        return;
                    }

                    // Merge config with defaults
                    let resolved_config = config.merge_with_defaults();

                    // Extract circle parameters (voice-level params are ignored for new circles)
                    let gravity = resolved_config.gravity.unwrap();
                    let force = resolved_config.force.unwrap();
                    let outer_radius = resolved_config.outer_radius.unwrap();
                    let inner_radius = resolved_config.inner_radius.unwrap();
                    let center_x = resolved_config.center_x.unwrap();
                    let center_y = resolved_config.center_y.unwrap();
                    let noise = resolved_config.noise.unwrap();

                    // Create the new WindCircle
                    let voice = self.voices.get_mut(&voice_id).unwrap();
                    let circle_id = voice.issue_wind_circle_idx();

                    let center = nannou::prelude::vec2(center_x, center_y);
                    let circle = crate::forces::WindCircle::new(
                        circle_id,
                        voice_id,
                        center,
                        outer_radius,
                        inner_radius,
                        force,
                        gravity,
                        noise,
                    );

                    // Add the circle to the voice
                    voice.add_wind_circle(circle);

                    // Queue parameter update commands for processing after this command completes
                    let parameter_commands =
                        DroneCommandBuilder::generate_circle_parameter_commands(
                            &resolved_config,
                            voice_id,
                            circle_id,
                            command.source.clone(),
                        );

                    // Add commands to queue instead of executing recursively
                    self.command_queue.extend(parameter_commands);

                    // Set success message
                    let status_message = format!(
                        "Voice {} - Added WindCircle {}",
                        voice_id.to_i32(),
                        circle_id
                    );
                    println!("{}", status_message);
                    self.command_input.set_success_message(status_message);
                }
                CompositeCommand::Clear { voice_id } => {
                    let validation = self.validate_voice(voice_id);
                    if !self.validate_and_handle_error(validation, "Clear") {
                        return;
                    }

                    if self.rhythms.contains_key(&voice_id) {
                        let cmd = Command::new(
                            CommandInner::Simple(SimpleCommand::ClearRhythm { voice_id }),
                            command.source.clone(),
                        );
                        self.command_queue.push(cmd);
                    } else if self.voices.contains_key(&voice_id) {
                        let cmd = Command::new(
                            CommandInner::Simple(SimpleCommand::ClearDrone { voice_id }),
                            command.source.clone(),
                        );
                        self.command_queue.push(cmd);
                    }
                }
            },

            CommandInner::Simple(atomic) => {
                self.execute_simple_command(atomic, time);
            }
        }
    }

    /// Execute atomic parameter commands without display updates (used internally)
    fn execute_simple_command(&mut self, simple: SimpleCommand, time: f32) {
        match simple {
            // Voice-level atomic commands
            SimpleCommand::Alpha { voice_id, value } => {
                let validation = self.validate_voice(voice_id);
                if !self.validate_and_handle_error(validation, "Alpha") {
                    return;
                }

                if let Some(voice) = self.voices.get_mut(&voice_id) {
                    voice.set_alpha_limit(value);
                }
            }
            SimpleCommand::Volume { voice_id, value } => {
                let validation = self.validate_voice(voice_id);
                if !self.validate_and_handle_error(validation, "Volume") {
                    return;
                }

                if let Some(voice) = self.voices.get_mut(&voice_id) {
                    voice.set_volume(value);
                }
            }
            SimpleCommand::Feedback { voice_id, value } => {
                let validation = self.validate_voice(voice_id);
                if !self.validate_and_handle_error(validation, "Feedback") {
                    return;
                }

                if let Some(voice) = self.voices.get_mut(&voice_id) {
                    voice.set_feedback(value);
                }
            }
            SimpleCommand::Vibration { voice_id, value } => {
                let validation = self.validate_voice(voice_id);
                if !self.validate_and_handle_error(validation, "Vibration") {
                    return;
                }

                if let Some(voice) = self.voices.get_mut(&voice_id) {
                    voice.set_vibration(value);
                }
            }
            SimpleCommand::MoveEmitters { voice_id, value } => {
                let validation = self.validate_voice(voice_id);
                if !self.validate_and_handle_error(validation, "MoveEmitters") {
                    return;
                }

                if let Some(voice) = self.voices.get_mut(&voice_id) {
                    voice.set_emitter_position(value);
                }
            }
            // Circle-level atomic commands
            SimpleCommand::OuterRadius {
                voice_id,
                circle_id,
                value,
            } => {
                let validation = self.validate_voice_circle(voice_id, circle_id);
                if !self.validate_and_handle_error(validation, "OuterRadius") {
                    return;
                }

                if let Some(voice) = self.voices.get_mut(&voice_id) {
                    voice.set_circle_outer_radius(circle_id, value);
                }
            }
            SimpleCommand::InnerRadius {
                voice_id,
                circle_id,
                value,
            } => {
                let validation = self.validate_voice_circle(voice_id, circle_id);
                if !self.validate_and_handle_error(validation, "InnerRadius") {
                    return;
                }

                if let Some(voice) = self.voices.get_mut(&voice_id) {
                    voice.set_circle_inner_radius(circle_id, value);
                }
            }
            SimpleCommand::Force {
                voice_id,
                circle_id,
                value,
            } => {
                let validation = self.validate_voice_circle(voice_id, circle_id);
                if !self.validate_and_handle_error(validation, "Force") {
                    return;
                }

                let strength = value.min(30.0); // 30 is the max strength of the wind circle
                if let Some(voice) = self.voices.get_mut(&voice_id) {
                    voice.set_circle_force(circle_id, strength);
                }
            }
            SimpleCommand::Gravity {
                voice_id,
                circle_id,
                value,
            } => {
                let validation = self.validate_voice_circle(voice_id, circle_id);
                if !self.validate_and_handle_error(validation, "Gravity") {
                    return;
                }

                if let Some(voice) = self.voices.get_mut(&voice_id) {
                    voice.set_circle_gravity(circle_id, value);
                }
            }
            SimpleCommand::Noise {
                voice_id,
                circle_id,
                value,
            } => {
                let validation = self.validate_voice_circle(voice_id, circle_id);
                if !self.validate_and_handle_error(validation, "Noise") {
                    return;
                }

                if let Some(voice) = self.voices.get_mut(&voice_id) {
                    voice.set_circle_noise(circle_id, value);
                }
            }
            SimpleCommand::CenterX {
                voice_id,
                circle_id,
                value,
            } => {
                let validation = self.validate_voice_circle(voice_id, circle_id);
                if !self.validate_and_handle_error(validation, "ForceCenterX") {
                    return;
                }

                if let Some(voice) = self.voices.get_mut(&voice_id) {
                    voice.set_circle_center_x(circle_id, value);
                }
            }
            SimpleCommand::CenterY {
                voice_id,
                circle_id,
                value,
            } => {
                let validation = self.validate_voice_circle(voice_id, circle_id);
                if !self.validate_and_handle_error(validation, "ForceCenterY") {
                    return;
                }

                if let Some(voice) = self.voices.get_mut(&voice_id) {
                    voice.set_circle_center_y(circle_id, value);
                }
            }
            SimpleCommand::ListCircles { voice_id } => {
                let circle_ids = self.get_wind_circle_ids(voice_id);
                let circles_str = if circle_ids.is_empty() {
                    "No WindCircles found".to_string()
                } else {
                    format!("WindCircle keys: {:?}", circle_ids)
                };

                let status_message = format!("Voice {} - {}", voice_id.to_i32(), circles_str);
                println!("{}", status_message);

                // Send to Performer Control status line
                self.command_input.set_success_message(status_message);
            }
            SimpleCommand::AddWings { voice_id, count } => {
                // Check if rhythm exists for this voice
                if let Some(rhythm) = self.rhythms.get_mut(&voice_id) {
                    rhythm.add_wings(count, &mut self.rng);
                    rhythm.update_sequencer(&mut self.sequencer_service);
                    self.rhythm_view
                        .reinitialize_formation(voice_id, rhythm.get_params(), time);

                    let status_message = format!(
                        "Voice {} - Added {} wings (total: {})",
                        voice_id.to_i32(),
                        count,
                        rhythm.get_params().wings.len()
                    );
                    println!("{}", status_message);
                    self.command_input.set_success_message(status_message);
                } else {
                    let error_message =
                        format!("Voice {} has no rhythm to add wings to", voice_id.to_i32());
                    println!("Error: {}", error_message);
                    self.command_input.set_error_message(error_message);
                }
            }
            SimpleCommand::RemoveWings { voice_id, count } => {
                // Check if rhythm exists for this voice
                if let Some(rhythm) = self.rhythms.get_mut(&voice_id) {
                    rhythm.remove_wings(count);
                    rhythm.update_sequencer(&mut self.sequencer_service);
                    self.rhythm_view
                        .reinitialize_formation(voice_id, rhythm.get_params(), time);

                    let status_message = format!(
                        "Voice {} - Removed {} wings (total: {})",
                        voice_id.to_i32(),
                        count,
                        rhythm.get_params().wings.len()
                    );
                    println!("{}", status_message);
                    self.command_input.set_success_message(status_message);
                } else {
                    let error_message = format!(
                        "Voice {} has no rhythm to remove wings from",
                        voice_id.to_i32()
                    );
                    println!("Error: {}", error_message);
                    self.command_input.set_error_message(error_message);
                }
            }
            SimpleCommand::ClearRhythm { voice_id } => {
                // Check if rhythm exists for this voice
                if let Some(rhythm) = self.rhythms.get_mut(&voice_id) {
                    // Stop the sequencer before removing the rhythm
                    rhythm.stop_sequencer(&mut self.sequencer_service);

                    // Trigger clearing animation in view
                    self.rhythm_view.clear_formation(voice_id, time);

                    // Remove the rhythm from the model (view continues animating)
                    self.rhythms.remove(&voice_id);

                    let status_message = format!(
                        "Voice {} - Cleared rhythm and stopped sequencer",
                        voice_id.to_i32()
                    );
                    println!("{}", status_message);
                    self.command_input.set_success_message(status_message);
                } else {
                    let error_message =
                        format!("Voice {} has no rhythm to clear", voice_id.to_i32());
                    println!("Error: {}", error_message);
                    self.command_input.set_error_message(error_message);
                }
            }
            SimpleCommand::ClearDrone { voice_id } => {
                self.kill_voice(voice_id);
                self.osc_send.send_drone_on_off(voice_id.to_i32(), 0);
            }
            SimpleCommand::RemoveCircle {
                voice_id,
                circle_id,
            } => {
                let validation = self.validate_voice(voice_id);
                if !self.validate_and_handle_error(validation, "RemoveCircle") {
                    return;
                }

                // Remove the circle from the voice
                let voice = self.voices.get_mut(&voice_id).unwrap();
                let Some(circle) = voice.wind_circles.get_mut(&(circle_id as usize)) else {
                    return;
                };

                let wind_field = &mut self.particle_system.forces.wind_field;
                circle.remove_from_field(wind_field);
                voice.remove_wind_circle(circle_id as usize);

                // Set success message
                let status_message = format!(
                    "Voice {} - Removed WindCircle {}",
                    voice_id.to_i32(),
                    circle_id
                );
                println!("{}", status_message);
                self.command_input.set_success_message(status_message);
            }
            // Rhythm structure parameters
            SimpleCommand::RhythmCapacity { voice_id, value } => {
                if value < 1 {
                    return;
                }

                if let Some(rhythm) = self.rhythms.get_mut(&voice_id) {
                    rhythm.set_capacity(value);
                    rhythm.update_sequencer(&mut self.sequencer_service);
                    self.rhythm_view
                        .reinitialize_formation(voice_id, rhythm.get_params(), time);
                }
            }
            SimpleCommand::RhythmNumWings { voice_id, value } => {
                if let Some(rhythm) = self.rhythms.get_mut(&voice_id) {
                    rhythm.set_num_wings(value);
                    rhythm.reroll_wings(&mut self.rng, &mut self.sequencer_service);
                    self.rhythm_view
                        .reinitialize_formation(voice_id, rhythm.get_params(), time);
                }
            }
            SimpleCommand::RhythmSubdivision { voice_id, value } => {
                if let Some(rhythm) = self.rhythms.get_mut(&voice_id) {
                    rhythm.set_subdivision(value);
                    rhythm.update_sequencer(&mut self.sequencer_service);
                }
            }
            // Slot range parameters (for creation)
            SimpleCommand::RhythmLengthRange { voice_id, range } => {
                if let Some(rhythm) = self.rhythms.get_mut(&voice_id) {
                    rhythm.set_length_range(range);
                }
            }
            SimpleCommand::RhythmVelocityRange { voice_id, range } => {
                if let Some(rhythm) = self.rhythms.get_mut(&voice_id) {
                    rhythm.set_velocity_range(range);
                }
            }
            SimpleCommand::RhythmCutoffRange { voice_id, range } => {
                if let Some(rhythm) = self.rhythms.get_mut(&voice_id) {
                    rhythm.set_cutoff_range(range);
                }
            }
            // Slot modification parameters (for editing)
            SimpleCommand::RhythmModifyLength {
                voice_id,
                modification,
            } => {
                if let Some(rhythm) = self.rhythms.get_mut(&voice_id) {
                    rhythm.modify_all_slots_length(modification, &mut self.rng);
                    rhythm.update_sequencer(&mut self.sequencer_service);
                }
            }
            SimpleCommand::RhythmModifyVelocity {
                voice_id,
                modification,
            } => {
                if let Some(rhythm) = self.rhythms.get_mut(&voice_id) {
                    rhythm.modify_all_slots_velocity(modification, &mut self.rng);
                    rhythm.update_sequencer(&mut self.sequencer_service);
                }
            }
            SimpleCommand::RhythmModifyCutoff {
                voice_id,
                modification,
            } => {
                if let Some(rhythm) = self.rhythms.get_mut(&voice_id) {
                    rhythm.modify_all_slots_cutoff(modification, &mut self.rng);
                    rhythm.update_sequencer(&mut self.sequencer_service);
                }
            }
        }
    }

    /// Get all wind circle IDs for a voice
    pub fn get_wind_circle_ids(&self, voice: VoiceId) -> Vec<usize> {
        let Some(voice) = self.voices.get(&voice) else {
            return Vec::new();
        };
        let mut ids: Vec<usize> = voice.wind_circles.keys().copied().collect();
        ids.sort();
        ids
    }

    /// Get the params of a circle
    pub fn get_wind_circle_params(&self, voice: VoiceId, id: usize) -> Option<&WindCircleParams> {
        let voice = self.voices.get(&voice)?;
        voice.wind_circles.get(&id).map(|circle| circle.params())
    }

    /// Get the alpha limit of a Voice ("brightness")
    pub fn get_alpha_limit(&self, voice: VoiceId) -> f32 {
        let Some(voice) = self.voices.get(&voice) else {
            return 0.0;
        };

        voice.params.alpha_limit
    }

    /// Get the center bias of a Voice's WindCircle ("gravity")
    pub fn get_center_bias(&mut self, voice: VoiceId, id: usize) -> f32 {
        let Some(voice) = self.voices.get(&voice) else {
            return 0.0;
        };

        voice
            .wind_circles
            .get(&id)
            .map(|circle| circle.params().gravity)
            .unwrap_or(0.0)
    }

    /// Get the angle variation of a Voice's WindCircle by id ("noise")
    pub fn get_noise(&self, voice: VoiceId, id: usize) -> f32 {
        let Some(voice) = self.voices.get(&voice) else {
            return 0.0;
        };

        voice
            .wind_circles
            .get(&id)
            .map(|circle| circle.params().noise)
            .unwrap_or(0.0)
    }

    /// Get the position offset factor of a Voice ("vibration")
    pub fn get_vibration(&self, voice: VoiceId) -> f32 {
        let Some(voice) = self.voices.get(&voice) else {
            return 0.0;
        };

        voice.params.vibration
    }

    pub fn get_emitter_position(&self, voice: VoiceId) -> f32 {
        let Some(voice) = self.voices.get(&voice) else {
            return 0.0;
        };

        voice.params.emitter_position
    }

    pub fn get_volume(&self, voice: VoiceId) -> f32 {
        let Some(voice) = self.voices.get(&voice) else {
            return 0.0;
        };

        voice.params.volume
    }

    pub fn get_feedback(&self, voice: VoiceId) -> f32 {
        let Some(voice) = self.voices.get(&voice) else {
            return 0.0;
        };

        voice.params.feedback
    }

    /// End a voice -- remove its wind circles and remove it from the hashmap
    pub fn kill_voice(&mut self, voice_id: VoiceId) {
        // Remove voice's wind circles from force field
        let Some(voice) = self.voices.get_mut(&voice_id) else {
            return;
        };

        let wind_field = &mut self.particle_system.forces.wind_field;

        for (id, circle) in voice.wind_circles.iter_mut() {
            circle.remove_from_field(wind_field);
            println!("Controller: Removed wind circle for {:?}", id);
        }

        // Remove the voice from the hashmap
        self.voices.remove(&voice_id);
    }
}

/********** Command creation helpers ***************** */

/// Create a drone using the unified command system
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

/// Create an erase drone command using the unified command system
pub fn erase_drone_command(voice: VoiceId, source: CommandSource) -> Command {
    Command::new(
        CommandInner::Simple(SimpleCommand::ClearDrone { voice_id: voice }),
        source,
    )
}

/********** Functions for changing renderer properties ***************** */

/// Update the "Feedback" feature by setting segment length in voice parameters
pub fn update_feedback(model: &mut Model, _device: &Device, _queue: &Queue) {
    // Read feedback value for segment length before updating particle system
    let voice1_feedback = model.get_feedback(VoiceId::Voice0);
    let voice4_feedback = model.get_feedback(VoiceId::Voice3);

    // Update segment length based on Voice1 feedback slider
    if let Some(voice1) = model.voices.get_mut(&VoiceId::Voice0) {
        voice1.set_segment_length(voice1_feedback);
    }

    // Update segment length based on Voice4 feedback slider
    if let Some(voice4) = model.voices.get_mut(&VoiceId::Voice3) {
        voice4.set_segment_length(voice4_feedback);
    }
}

// Implement VoiceValidator trait for Model to enable centralized validation
impl VoiceValidator for Model {
    fn voice_exists(&self, voice_id: VoiceId) -> bool {
        self.voices.contains_key(&voice_id) || self.rhythms.contains_key(&voice_id)
    }

    fn circle_exists(&self, voice_id: VoiceId, circle_id: usize) -> bool {
        if let Some(voice) = self.voices.get(&voice_id) {
            voice.wind_circles.contains_key(&circle_id)
        } else {
            false
        }
    }
}

// Helper function for consistent error handling in command execution
impl Model {
    /// Validate and execute a command with standardized error handling
    pub fn validate_and_handle_error(
        &mut self,
        validation: ValidationResult,
        _operation: &str,
    ) -> bool {
        match validation {
            ValidationResult::Success => true,
            ValidationResult::VoiceNotFound(_) | ValidationResult::CircleNotFound(_, _) => {
                if let Some(error_msg) = validation.to_error_message() {
                    println!("Error: {}", error_msg);
                    self.command_input.set_success_message(error_msg);
                }
                false
            }
        }
    }
}
