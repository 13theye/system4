// src/voice/controller.rs
//
// Command extension for Model

use crate::{
    forces::WindCircleParams,
    groups::{Voice, VoiceId},
    model::{
        command_builder::{ValidationResult, VoiceValidator},
        Model,
    },
    terminals::commands::drone::DroneConfig,
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
pub enum CommandInner {
    CreateDrone {
        config: DroneConfig,
    },
    EraseDrone {
        voice_id: VoiceId,
    },
    ModifyDrone {
        voice_id: VoiceId,
        config: DroneConfig,
    },
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

    ForceCenterX {
        voice_id: VoiceId,
        circle_id: usize,
        value: f32,
    },
    ForceCenterY {
        voice_id: VoiceId,
        circle_id: usize,
        value: f32,
    },
    ListCircles {
        voice_id: VoiceId,
    },
    NewCircle {
        voice_id: VoiceId,
        config: crate::terminals::commands::drone::DroneConfig,
    },
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
        // Global commands that don't conflict with each other
        CommandInner::CreateDrone { config } => {
            format!("CreateDrone_{}", config.voice)
        }
        CommandInner::EraseDrone { voice_id: voice } => {
            format!("EraseDrone_{:?}", voice)
        }
        CommandInner::ModifyDrone {
            voice_id: voice, ..
        } => {
            format!("ModifyDrone_{:?}", voice)
        }
        // Parameter commands that conflict by voice+parameter type
        CommandInner::Alpha {
            voice_id: voice, ..
        } => format!("Alpha_{:?}", voice),
        CommandInner::Volume {
            voice_id: voice, ..
        } => format!("Volume_{:?}", voice),
        CommandInner::Feedback {
            voice_id: voice, ..
        } => format!("Feedback_{:?}", voice),
        CommandInner::OuterRadius {
            voice_id: voice, ..
        } => format!("OuterRadius_{:?}", voice),
        CommandInner::InnerRadius {
            voice_id: voice, ..
        } => format!("InnerRadius_{:?}", voice),
        CommandInner::Force {
            voice_id: voice, ..
        } => format!("Strength_{:?}", voice),
        CommandInner::Gravity {
            voice_id: voice, ..
        } => format!("CenterBias_{:?}", voice),
        CommandInner::Noise {
            voice_id: voice, ..
        } => format!("AngleVariation_{:?}", voice),
        CommandInner::Vibration {
            voice_id: voice, ..
        } => format!("Vibration_{:?}", voice),
        CommandInner::ForceCenterX {
            voice_id: voice, ..
        } => format!("ForceCenterX_{:?}", voice),
        CommandInner::ForceCenterY {
            voice_id: voice, ..
        } => format!("ForceCenterY_{:?}", voice),
        CommandInner::ListCircles {
            voice_id: voice, ..
        } => format!("ListCircles_{:?}", voice),
        CommandInner::NewCircle {
            voice_id: voice, ..
        } => format!("NewCircle_{:?}", voice),
    }
}

/// Extension of Model that add controller functions
impl Model {
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

    /// Add a command to the queue for later processing
    pub fn queue_command(&mut self, command: Command) {
        self.command_queue.push(command);
    }

    /// Process all queued commands with priority resolution (Terminal > OSC > UI)
    pub fn process_command_queue(&mut self) {
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

        // Process commands through drone parameter displays
        for command in &final_commands {
            self.terminal_manager.borrow_mut().process_command(command);
        }

        // Execute all final commands
        for command in final_commands {
            self.execute_command(command);
        }
    }

    /// Apply a command immediately without queueing
    pub fn execute_command(&mut self, command: Command) {
        match command.command {
            CommandInner::CreateDrone { config } => {
                // Convert voice ID to Voice enum
                let voice_id = VoiceId::from_i32(config.voice);

                if self.voices.contains_key(&voice_id) {
                    // Voice already exists, do nothing
                    println!("Controller: Voice {} already exists", voice_id);
                    return;
                }

                let mut voice = Voice::new_with_id(voice_id);

                voice.begin_drone(
                    &config,
                    self.particle_system.default_particle_color,
                    self.particle_system.global_max_spawn_rate,
                    &mut self.id_generator,
                );

                self.osc_send.send_drone_on_off(config.voice, 1);
                voice.set_is_spawning(true);

                self.voices.insert(voice_id, voice);

                // Send parameter update commands to ensure DroneParametersDisplay gets updated
                // with the actual values that were used (including defaults)
                let resolved_config = config.merge_with_defaults();
                let parameter_commands = resolved_config.generate_parameter_commands(
                    voice_id,
                    0, // First circle created by begin_drone
                    command.source,
                );

                for cmd in parameter_commands {
                    self.terminal_manager.borrow_mut().process_command(&cmd);
                }
            }
            CommandInner::EraseDrone { voice_id } => {
                self.kill_voice(voice_id);
                self.osc_send.send_drone_on_off(voice_id.to_i32(), 0);
            }
            CommandInner::ModifyDrone { voice_id, config } => {
                // Apply selective modifications - only set parameters that are Some(value)
                if let Some(brightness) = config.brightness {
                    let Some(voice) = self.voices.get_mut(&voice_id) else {
                        return;
                    };
                    voice.set_alpha_limit(brightness);
                }
                if let Some(volume) = config.volume {
                    let Some(voice) = self.voices.get_mut(&voice_id) else {
                        return;
                    };
                    voice.set_volume(volume);
                }
                if let Some(feedback) = config.feedback {
                    let Some(voice) = self.voices.get_mut(&voice_id) else {
                        return;
                    };
                    voice.set_feedback(feedback);
                }

                /*
                if let Some(outer_radius) = config.outer_radius {
                    self.particle_system
                        .forces
                        .set_outer_radius(&voice_id, outer_radius);
                }
                if let Some(inner_radius) = config.inner_radius {
                    self.particle_system
                        .forces
                        .set_inner_radius(&voice_id, inner_radius);
                }
                if let Some(force) = config.force {
                    self.particle_system
                        .forces
                        .set_strength(&voice_id, force.min(30.0));
                }
                if let Some(gravity) = config.gravity {
                    self.particle_system
                        .forces
                        .set_center_bias(&voice_id, gravity);
                }
                if let Some(noise) = config.noise {
                    self.particle_system.forces.set_noise(&voice_id, noise);
                }
                if let Some(vibration) = config.vibration {
                    let Some(voice) = self.voices.get_mut(&voice_id) else {
                        return;
                    };
                    voice.set_vibration(vibration);
                }


                // Handle center position - only update if at least one coordinate is specified
                if config.center_x.is_some() || config.center_y.is_some() {
                    if let Some(circle) = self.particle_system.forces.get_wind_circle_mut(voice_id)
                    {
                        let current_center = circle.params().center;
                        let new_x = config.center_x.unwrap_or(current_center.x);
                        let new_y = config.center_y.unwrap_or(current_center.y);
                        circle.params_mut().set_center(vec2(new_x, new_y));
                    }
                }
                */
            }

            CommandInner::Alpha { voice_id, value } => {
                let validation = self.validate_voice(voice_id);
                if !self.validate_and_handle_error(validation, "Alpha") {
                    return;
                }

                if let Some(voice) = self.voices.get_mut(&voice_id) {
                    voice.set_alpha_limit(value);
                }
            }
            CommandInner::Volume { voice_id, value } => {
                let validation = self.validate_voice(voice_id);
                if !self.validate_and_handle_error(validation, "Volume") {
                    return;
                }

                if let Some(voice) = self.voices.get_mut(&voice_id) {
                    voice.set_volume(value);
                }
            }
            CommandInner::Feedback { voice_id, value } => {
                let validation = self.validate_voice(voice_id);
                if !self.validate_and_handle_error(validation, "Feedback") {
                    return;
                }

                if let Some(voice) = self.voices.get_mut(&voice_id) {
                    voice.set_feedback(value);
                }
            }
            CommandInner::Vibration { voice_id, value } => {
                let validation = self.validate_voice(voice_id);
                if !self.validate_and_handle_error(validation, "Vibration") {
                    return;
                }

                if let Some(voice) = self.voices.get_mut(&voice_id) {
                    voice.set_vibration(value);
                }
            }
            CommandInner::OuterRadius {
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
            CommandInner::InnerRadius {
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
            CommandInner::Force {
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
            CommandInner::Gravity {
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
            CommandInner::Noise {
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

            CommandInner::ForceCenterX {
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
            CommandInner::ForceCenterY {
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
            CommandInner::ListCircles { voice_id } => {
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
            CommandInner::NewCircle { voice_id, config } => {
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

                // Send parameter update commands to DroneParametersDisplay
                let parameter_commands = vec![
                    Command::new(
                        CommandInner::Gravity {
                            voice_id,
                            circle_id,
                            value: gravity,
                        },
                        command.source.clone(),
                    ),
                    Command::new(
                        CommandInner::Force {
                            voice_id,
                            circle_id,
                            value: force,
                        },
                        command.source.clone(),
                    ),
                    Command::new(
                        CommandInner::OuterRadius {
                            voice_id,
                            circle_id,
                            value: outer_radius,
                        },
                        command.source.clone(),
                    ),
                    Command::new(
                        CommandInner::InnerRadius {
                            voice_id,
                            circle_id,
                            value: inner_radius,
                        },
                        command.source.clone(),
                    ),
                    Command::new(
                        CommandInner::Noise {
                            voice_id,
                            circle_id,
                            value: noise,
                        },
                        command.source.clone(),
                    ),
                    Command::new(
                        CommandInner::ForceCenterX {
                            voice_id,
                            circle_id,
                            value: center_x,
                        },
                        command.source.clone(),
                    ),
                    Command::new(
                        CommandInner::ForceCenterY {
                            voice_id,
                            circle_id,
                            value: center_y,
                        },
                        command.source.clone(),
                    ),
                ];

                for cmd in parameter_commands {
                    self.terminal_manager.borrow_mut().process_command(&cmd);
                }

                // Set success message
                let status_message = format!(
                    "Voice {} - Added WindCircle {}",
                    voice_id.to_i32(),
                    circle_id
                );
                println!("{}", status_message);
                self.command_input.set_success_message(status_message);
            }
        }
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
        voice: voice_id,
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
    Command::new(CommandInner::EraseDrone { voice_id: voice }, source)
}

/********** Functions for changing renderer properties ***************** */

/// Update the "Feedback" feature because it's owned by the model's renderer
pub fn update_feedback(model: &mut Model, device: &Device, queue: &Queue) {
    // Read feedback value for segment length before updating particle system
    let voice1_feedback = model.get_feedback(VoiceId::Voice1);
    let voice4_feedback = model.get_feedback(VoiceId::Voice4);

    // Update segment length based on Voice1 feedback slider
    model
        .segment_renderer1
        .set_segment_length(device, queue, voice1_feedback);

    // Update segment length based on Voice1 feedback slider
    model
        .segment_renderer4
        .set_segment_length(device, queue, voice4_feedback);
}

// Implement VoiceValidator trait for Model to enable centralized validation
impl VoiceValidator for Model {
    fn voice_exists(&self, voice_id: VoiceId) -> bool {
        self.voices.contains_key(&voice_id)
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
