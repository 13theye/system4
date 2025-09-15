// src/voice/controller.rs
//
// Command extension for Model

use super::Voice;
use crate::{forces::WindCircleParams, model::Model, terminals::commands::drone::DroneConfig};
use nannou::prelude::*;
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
        voice: Voice,
    },
    ModifyDrone {
        voice: Voice,
        config: DroneConfig,
    },
    Alpha {
        voice: Voice,
        value: f32,
    },
    Volume {
        voice: Voice,
        value: f32,
    },
    Feedback {
        voice: Voice,
        value: f32,
    },
    OuterRadius {
        voice: Voice,
        value: f32,
    },
    InnerRadius {
        voice: Voice,
        value: f32,
    },
    Strength {
        voice: Voice,
        value: f32,
    },
    CenterBias {
        voice: Voice,
        value: f32,
    },
    Noise {
        voice: Voice,
        value: f32,
    },
    Vibration {
        voice: Voice,
        value: f32,
    },
    ForceCenterX {
        voice: Voice,
        value: f32,
    },
    ForceCenterY {
        voice: Voice,
        value: f32,
    },
    MaskChangeBounds {
        voice: Voice,
        rect: Rect,
        duration: f32,
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
        CommandInner::EraseDrone { voice } => {
            format!("EraseDrone_{:?}", voice)
        }
        CommandInner::ModifyDrone { voice, .. } => {
            format!("ModifyDrone_{:?}", voice)
        }
        // Parameter commands that conflict by voice+parameter type
        CommandInner::Alpha { voice, .. } => format!("Alpha_{:?}", voice),
        CommandInner::Volume { voice, .. } => format!("Volume_{:?}", voice),
        CommandInner::Feedback { voice, .. } => format!("Feedback_{:?}", voice),
        CommandInner::OuterRadius { voice, .. } => format!("OuterRadius_{:?}", voice),
        CommandInner::InnerRadius { voice, .. } => format!("InnerRadius_{:?}", voice),
        CommandInner::Strength { voice, .. } => format!("Strength_{:?}", voice),
        CommandInner::CenterBias { voice, .. } => format!("CenterBias_{:?}", voice),
        CommandInner::Noise { voice, .. } => format!("AngleVariation_{:?}", voice),
        CommandInner::Vibration { voice, .. } => format!("Vibration_{:?}", voice),
        CommandInner::ForceCenterX { voice, .. } => format!("ForceCenterX_{:?}", voice),
        CommandInner::ForceCenterY { voice, .. } => format!("ForceCenterY_{:?}", voice),
        CommandInner::MaskChangeBounds { voice, .. } => format!("MaskChangeBounds_{:?}", voice),
    }
}

/// Extension of Model that add controller functions
impl Model {
    /// Get the params of a circle
    pub fn get_wind_circle_params(&self, voice: Voice) -> Option<&WindCircleParams> {
        self.particle_system.forces.get_wind_circle_params(voice)
    }

    /// Get the alpha limit of a Voice ("brightness")
    pub fn get_alpha_limit(&self, voice: Voice) -> f32 {
        self.particle_system
            .alpha_limits
            .get(&voice)
            .copied()
            .unwrap_or(1.0)
    }

    /// Get the center bias of a Voice's WindCircle ("gravity")
    pub fn get_center_bias(&mut self, voice: Voice) -> f32 {
        self.particle_system.forces.get_center_bias(&voice)
    }

    /// Get the angle variation of a Voice's WindCircle ("noise")
    pub fn get_angle_variation(&self, voice: Voice) -> f32 {
        self.particle_system.forces.get_angle_variation(&voice)
    }

    /// Get the position offset factor of a Voice ("vibration")
    pub fn get_vibration_offset_factor(&self, voice: Voice) -> f32 {
        self.particle_system.get_vibration_factor(voice)
    }

    pub fn get_volume(&self, voice: Voice) -> f32 {
        self.particle_system
            .particle_num_factors
            .get(&voice)
            .copied()
            .unwrap_or(1.0)
    }

    pub fn get_feedback(&self, voice: Voice) -> f32 {
        self.particle_system
            .feedback
            .get(&voice)
            .copied()
            .unwrap_or(0.0)
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
            self.terminal_manager.borrow_mut().process_command_for_drone_displays(command);
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
                let voice = Voice::from_i32(config.voice);

                let _ = self
                    .particle_system
                    .begin_voice(&mut self.id_generator, voice, &config);

                self.osc_send.send_drone_on_off(config.voice, 1);
                self.particle_system.set_is_spawning(&voice, true);
            }
            CommandInner::EraseDrone { voice } => {
                // Reset drone parameters to default values
                self.particle_system.set_alpha_limit(&voice, 0.0);
                self.particle_system.set_volume(&voice, 0.0);
                self.particle_system.set_feedback(&voice, 0.0);
                self.particle_system.forces.set_strength(&voice, 0.0);
                self.osc_send.send_drone_on_off(voice.to_i32(), 0);
            }
            CommandInner::ModifyDrone { voice, config } => {
                // Apply selective modifications - only set parameters that are Some(value)
                if let Some(brightness) = config.brightness {
                    self.particle_system.set_alpha_limit(&voice, brightness);
                }
                if let Some(volume) = config.volume {
                    self.particle_system.set_volume(&voice, volume);
                }
                if let Some(feedback) = config.feedback {
                    self.particle_system.set_feedback(&voice, feedback);
                }
                if let Some(outer_radius) = config.outer_radius {
                    self.particle_system
                        .forces
                        .set_outer_radius(&voice, outer_radius);
                }
                if let Some(inner_radius) = config.inner_radius {
                    self.particle_system
                        .forces
                        .set_inner_radius(&voice, inner_radius);
                }
                if let Some(force) = config.force {
                    self.particle_system
                        .forces
                        .set_strength(&voice, force.min(30.0));
                }
                if let Some(gravity) = config.gravity {
                    self.particle_system.forces.set_center_bias(&voice, gravity);
                }
                if let Some(noise) = config.noise {
                    self.particle_system.forces.set_noise(&voice, noise);
                }
                if let Some(vibration) = config.vibration {
                    self.particle_system.set_vibration_factor(&voice, vibration);
                }

                // Handle center position - only update if at least one coordinate is specified
                if config.center_x.is_some() || config.center_y.is_some() {
                    if let Some(circle) = self.particle_system.forces.get_wind_circle_mut(voice) {
                        let current_center = circle.params().center;
                        let new_x = config.center_x.unwrap_or(current_center.x);
                        let new_y = config.center_y.unwrap_or(current_center.y);
                        circle.params_mut().set_center(vec2(new_x, new_y));
                    }
                }
            }
            CommandInner::Alpha { voice, value } => {
                self.particle_system.set_alpha_limit(&voice, value);
            }
            CommandInner::Volume { voice, value } => {
                self.particle_system.set_volume(&voice, value);
            }
            CommandInner::Feedback { voice, value } => {
                self.particle_system.set_feedback(&voice, value);
            }
            CommandInner::OuterRadius { voice, value } => {
                self.particle_system.forces.set_outer_radius(&voice, value);
            }
            CommandInner::InnerRadius { voice, value } => {
                self.particle_system.forces.set_inner_radius(&voice, value);
            }
            CommandInner::Strength { voice, value } => {
                let strength = value.min(30.0); // 30 is the max strength of the wind circle
                self.particle_system.forces.set_strength(&voice, strength);
            }
            CommandInner::CenterBias { voice, value } => {
                self.particle_system.forces.set_center_bias(&voice, value);
            }
            CommandInner::Noise { voice, value } => {
                self.particle_system.forces.set_noise(&voice, value);
            }
            CommandInner::Vibration { voice, value } => {
                self.particle_system.set_vibration_factor(&voice, value);
            }
            CommandInner::ForceCenterX { voice, value } => {
                if let Some(circle) = self.particle_system.forces.get_wind_circle_mut(voice) {
                    let current_y = circle.params().center.y;
                    circle.params_mut().set_center(vec2(value, current_y));
                }
            }
            CommandInner::ForceCenterY { voice, value } => {
                if let Some(circle) = self.particle_system.forces.get_wind_circle_mut(voice) {
                    let current_x = circle.params().center.x;
                    circle.params_mut().set_center(vec2(current_x, value));
                }
            }
            CommandInner::MaskChangeBounds {
                voice,
                rect,
                duration,
            } => {
                if let Some(mask) = self.particle_system.masks.get_mut(&voice) {
                    mask.change_bounds(rect, duration);
                }
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
        //trail: Some(trail),        // OSC "trail" maps to feedback
        outer_radius: Some(800.0), // Default values
        inner_radius: Some(200.0),
        noise: Some(0.0),
        vibration: Some(0.1),
        feedback: Some(feedback),
        center_x: Some(0.0),
        center_y: Some(0.0),
        additional_parameters: std::collections::HashMap::new(),
    };

    config.to_create_command(source)
}

/// Create an erase drone command using the unified command system
pub fn erase_drone_command(voice: Voice, source: CommandSource) -> Command {
    Command::new(CommandInner::EraseDrone { voice }, source)
}

/********** Functions for changing renderer properties ***************** */

/// Update the "Feedback" feature because it's owned by the model's renderer
pub fn update_feedback(model: &mut Model, device: &Device, queue: &Queue) {
    // Read feedback value for segment length before updating particle system
    let voice1_feedback = model.get_feedback(Voice::Voice1);
    let voice4_feedback = model.get_feedback(Voice::Voice4);

    // Update segment length based on Voice1 feedback slider
    model
        .segment_renderer1
        .set_segment_length(device, queue, voice1_feedback);

    // Update segment length based on Voice1 feedback slider
    model
        .segment_renderer4
        .set_segment_length(device, queue, voice4_feedback);
}
