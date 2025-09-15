// src/terminals/drone_parameters_display.rs
//
// Live display of drone parameters with color highlighting for recent updates

use super::terminal_view::{TerminalView, TerminalViewParams};
use crate::voice::{controller::Command, Voice};
use nannou::prelude::*;
use std::{collections::HashMap, time::Instant};

#[derive(Clone)]
pub struct ParameterUpdate {
    pub value: f32,
    pub last_updated: Instant,
}

pub struct DroneParametersDisplay {
    terminal_view: TerminalView,
    voice: Voice,
    parameters: HashMap<String, ParameterUpdate>,
}

impl DroneParametersDisplay {
    pub fn new(voice: Voice, params: TerminalViewParams) -> Self {
        let mut terminal_view = TerminalView::new(voice, params);

        // Add initial placeholder text
        let placeholder = format!("Voice {} - Waiting for parameters...", voice.to_i32());
        terminal_view.add_text(&placeholder, false);

        Self {
            terminal_view,
            voice,
            parameters: HashMap::new(),
        }
    }

    /// Extract parameter updates from a command and update internal state
    pub fn process_command(&mut self, command: &Command) {
        let now = Instant::now();

        match &command.command {
            crate::voice::controller::CommandInner::Alpha { voice, value } => {
                if *voice == self.voice {
                    self.update_parameter("brightness", *value, now);
                }
            }
            crate::voice::controller::CommandInner::Volume { voice, value } => {
                if *voice == self.voice {
                    self.update_parameter("volume", *value, now);
                }
            }
            crate::voice::controller::CommandInner::Feedback { voice, value } => {
                if *voice == self.voice {
                    self.update_parameter("feedback", *value, now);
                }
            }
            crate::voice::controller::CommandInner::OuterRadius { voice, value } => {
                if *voice == self.voice {
                    self.update_parameter("outerRadius", *value, now);
                }
            }
            crate::voice::controller::CommandInner::InnerRadius { voice, value } => {
                if *voice == self.voice {
                    self.update_parameter("innerRadius", *value, now);
                }
            }
            crate::voice::controller::CommandInner::Strength { voice, value } => {
                if *voice == self.voice {
                    self.update_parameter("gravity", *value, now);
                }
            }
            crate::voice::controller::CommandInner::CenterBias { voice, value } => {
                if *voice == self.voice {
                    self.update_parameter("centerBias", *value, now);
                }
            }
            crate::voice::controller::CommandInner::Noise { voice, value } => {
                if *voice == self.voice {
                    self.update_parameter("noise", *value, now);
                }
            }
            crate::voice::controller::CommandInner::Vibration { voice, value } => {
                if *voice == self.voice {
                    self.update_parameter("vibration", *value, now);
                }
            }
            crate::voice::controller::CommandInner::ForceCenterX { voice, value } => {
                if *voice == self.voice {
                    self.update_parameter("centerX", *value, now);
                }
            }
            crate::voice::controller::CommandInner::ForceCenterY { voice, value } => {
                if *voice == self.voice {
                    self.update_parameter("centerY", *value, now);
                }
            }
            crate::voice::controller::CommandInner::CreateDrone { config } => {
                if config.voice_enum() == self.voice {
                    self.process_drone_config(config, now);
                }
            }
            crate::voice::controller::CommandInner::ModifyDrone { voice, config } => {
                if *voice == self.voice {
                    self.process_drone_config(config, now);
                }
            }
            _ => {} // Ignore other command types
        }
    }

    /// Process DroneConfig to extract parameter updates
    fn process_drone_config(&mut self, config: &crate::terminals::commands::drone::DroneConfig, now: Instant) {
        if let Some(brightness) = config.brightness {
            self.update_parameter("brightness", brightness, now);
        }
        if let Some(volume) = config.volume {
            self.update_parameter("volume", volume, now);
        }
        if let Some(gravity) = config.gravity {
            self.update_parameter("gravity", gravity, now);
        }
        if let Some(force) = config.force {
            self.update_parameter("force", force, now);
        }
        if let Some(feedback) = config.feedback {
            self.update_parameter("feedback", feedback, now);
        }
        if let Some(outer_radius) = config.outer_radius {
            self.update_parameter("outerRadius", outer_radius, now);
        }
        if let Some(inner_radius) = config.inner_radius {
            self.update_parameter("innerRadius", inner_radius, now);
        }
        if let Some(noise) = config.noise {
            self.update_parameter("noise", noise, now);
        }
        if let Some(vibration) = config.vibration {
            self.update_parameter("vibration", vibration, now);
        }
        if let Some(center_x) = config.center_x {
            self.update_parameter("centerX", center_x, now);
        }
        if let Some(center_y) = config.center_y {
            self.update_parameter("centerY", center_y, now);
        }
    }

    /// Update a parameter value and timestamp
    fn update_parameter(&mut self, name: &str, value: f32, timestamp: Instant) {
        self.parameters.insert(
            name.to_string(),
            ParameterUpdate {
                value,
                last_updated: timestamp,
            }
        );
        self.refresh_display();
    }

    /// Generate the display string and update the terminal view
    fn refresh_display(&mut self) {
        if self.parameters.is_empty() {
            // Show a placeholder when no parameters are set
            let placeholder = format!("Voice {} - No parameters set", self.voice.to_i32());
            self.terminal_view.clear();
            self.terminal_view.add_text(&placeholder, false);
            return;
        }

        // Sort parameters by name for consistent display order
        let mut sorted_params: Vec<_> = self.parameters.iter().collect();
        sorted_params.sort_by_key(|(name, _)| *name);

        // Format as "param(value) | param(value)..."
        let display_parts: Vec<String> = sorted_params
            .iter()
            .map(|(name, update)| format!("{}({:.2})", name, update.value))
            .collect();

        let display_text = display_parts.join(" | ");

        // Check if any parameter was updated recently for color highlighting
        let now = Instant::now();
        let has_recent_update = self.parameters.values().any(|update| {
            now.duration_since(update.last_updated).as_secs_f32() < 2.0
        });

        // Clear the terminal and add the new line
        self.terminal_view.clear();
        self.terminal_view.add_text(&display_text, has_recent_update);
    }

    /// Update the terminal view (handles animations, etc.)
    pub fn update(&mut self, draw: &Draw) {
        self.terminal_view.update(draw);
    }

    /// Get a reference to the underlying terminal view
    pub fn terminal_view(&self) -> &TerminalView {
        &self.terminal_view
    }

    /// Get a mutable reference to the underlying terminal view
    pub fn terminal_view_mut(&mut self) -> &mut TerminalView {
        &mut self.terminal_view
    }

    /// Clear all parameters
    pub fn clear(&mut self) {
        self.parameters.clear();
        self.terminal_view.clear();
    }

    /// Get the voice this display is tracking
    pub fn voice(&self) -> Voice {
        self.voice
    }
}