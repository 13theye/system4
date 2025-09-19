// src/voice/controller.rs
//
// Command extension for Model

use crate::{
    forces::WindCircleParams,
    groups::{Voice, VoiceId},
    model::Model,
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
            self.terminal_manager
                .borrow_mut()
                .process_command_for_drone_displays(command);
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

                if self.voices.get(&voice_id).is_some() {
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
                let Some(voice) = self.voices.get_mut(&voice_id) else {
                    return;
                };
                voice.set_alpha_limit(value);
            }
            CommandInner::Volume { voice_id, value } => {
                let Some(voice) = self.voices.get_mut(&voice_id) else {
                    return;
                };
                voice.set_volume(value);
            }
            CommandInner::Feedback { voice_id, value } => {
                let Some(voice) = self.voices.get_mut(&voice_id) else {
                    return;
                };
                voice.set_feedback(value);
            }
            CommandInner::Vibration { voice_id, value } => {
                let Some(voice) = self.voices.get_mut(&voice_id) else {
                    return;
                };
                voice.set_vibration(value);
            }
            CommandInner::OuterRadius {
                voice_id,
                circle_id,
                value,
            } => {
                let Some(voice) = self.voices.get_mut(&voice_id) else {
                    return;
                };

                voice.set_circle_outer_radius(circle_id, value);
            }
            CommandInner::InnerRadius {
                voice_id,
                circle_id,
                value,
            } => {
                let Some(voice) = self.voices.get_mut(&voice_id) else {
                    return;
                };

                voice.set_circle_inner_radius(circle_id, value);
            }
            CommandInner::Force {
                voice_id,
                circle_id,
                value,
            } => {
                let strength = value.min(30.0); // 30 is the max strength of the wind circle
                let Some(voice) = self.voices.get_mut(&voice_id) else {
                    return;
                };

                voice.set_circle_force(circle_id, strength);
            }
            CommandInner::Gravity {
                voice_id,
                circle_id,
                value,
            } => {
                let Some(voice) = self.voices.get_mut(&voice_id) else {
                    return;
                };

                voice.set_circle_gravity(circle_id, value);
            }
            CommandInner::Noise {
                voice_id,
                circle_id,
                value,
            } => {
                let Some(voice) = self.voices.get_mut(&voice_id) else {
                    return;
                };

                voice.set_circle_noise(circle_id, value);
            }

            CommandInner::ForceCenterX {
                voice_id,
                circle_id,
                value,
            } => {
                if let Some(voice) = self.voices.get_mut(&voice_id) {
                    voice.set_circle_center_x(circle_id, value);
                }
            }
            CommandInner::ForceCenterY {
                voice_id,
                circle_id,
                value,
            } => {
                if let Some(voice) = self.voices.get_mut(&voice_id) {
                    voice.set_circle_center_y(circle_id, value);
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
