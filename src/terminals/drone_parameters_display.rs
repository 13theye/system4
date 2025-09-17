// src/terminals/drone_parameters_display.rs
//
// Live display of drone parameters with color highlighting for recent updates

use super::terminal_view::{TerminalView, TerminalViewParams};
use crate::groups::{controller::Command, Voice};
use nannou::prelude::*;
use std::{collections::HashMap, time::Instant};

#[derive(Clone)]
pub struct ParameterUpdate {
    pub value: f32,
    pub last_updated: Instant,
}

// Fixed parameter order for consistent line positioning
const PARAMETER_ORDER: &[&str] = &[
    "brightness",
    "volume",
    "gravity",
    "force",
    "feedback",
    "outerRadius",
    "innerRadius",
    "noise",
    "vibration",
    "centerX",
    "centerY",
    "centerBias",
];

pub struct DroneParametersDisplay {
    terminal_view: TerminalView,
    voice: Voice,
    parameters: HashMap<String, ParameterUpdate>,
    parameter_to_line: HashMap<String, usize>,
}

impl DroneParametersDisplay {
    pub fn new(voice: Voice, params: TerminalViewParams) -> Self {
        let mut terminal_view = TerminalView::new(voice, params);
        let mut parameter_to_line = HashMap::new();

        // Add initial header
        let header = format!("Voice {} Parameters:", voice.to_i32());
        terminal_view.add_text(&header, false);

        // Pre-populate all parameter lines with default values
        for (index, &param_name) in PARAMETER_ORDER.iter().enumerate() {
            parameter_to_line.insert(param_name.to_string(), index + 1); // +1 for header line

            // Add placeholder line with default value using direct line update
            let placeholder_line = format!("{}(-.--)", param_name);
            terminal_view.update_line_at_index(index + 1, &placeholder_line, true);
        }

        Self {
            terminal_view,
            voice,
            parameters: HashMap::new(),
            parameter_to_line,
        }
    }

    /// Extract parameter updates from a command and update internal state
    pub fn process_command(&mut self, command: &Command) {
        let now = Instant::now();

        match &command.command {
            crate::groups::controller::CommandInner::Alpha { voice, value } => {
                if *voice == self.voice {
                    self.update_parameter("brightness", *value, now);
                }
            }
            crate::groups::controller::CommandInner::Volume { voice, value } => {
                if *voice == self.voice {
                    self.update_parameter("volume", *value, now);
                }
            }
            crate::groups::controller::CommandInner::Feedback { voice, value } => {
                if *voice == self.voice {
                    self.update_parameter("feedback", *value, now);
                }
            }
            crate::groups::controller::CommandInner::OuterRadius { voice, value } => {
                if *voice == self.voice {
                    self.update_parameter("outerRadius", *value, now);
                }
            }
            crate::groups::controller::CommandInner::InnerRadius { voice, value } => {
                if *voice == self.voice {
                    self.update_parameter("innerRadius", *value, now);
                }
            }
            crate::groups::controller::CommandInner::Force { voice, value } => {
                if *voice == self.voice {
                    self.update_parameter("force", *value, now);
                }
            }
            crate::groups::controller::CommandInner::Gravity { voice, value } => {
                if *voice == self.voice {
                    self.update_parameter("gravity", *value, now);
                }
            }
            crate::groups::controller::CommandInner::Noise { voice, value } => {
                if *voice == self.voice {
                    self.update_parameter("noise", *value, now);
                }
            }
            crate::groups::controller::CommandInner::Vibration { voice, value } => {
                if *voice == self.voice {
                    self.update_parameter("vibration", *value, now);
                }
            }
            crate::groups::controller::CommandInner::ForceCenterX { voice, value } => {
                if *voice == self.voice {
                    self.update_parameter("centerX", *value, now);
                }
            }
            crate::groups::controller::CommandInner::ForceCenterY { voice, value } => {
                if *voice == self.voice {
                    self.update_parameter("centerY", *value, now);
                }
            }
            crate::groups::controller::CommandInner::CreateDrone { config } => {
                if config.voice_enum() == self.voice {
                    self.process_drone_config(config, now);
                }
            }
            crate::groups::controller::CommandInner::ModifyDrone { voice, config } => {
                if *voice == self.voice {
                    self.process_drone_config(config, now);
                }
            }
            _ => {} // Ignore other command types
        }
    }

    /// Process DroneConfig to extract parameter updates
    fn process_drone_config(
        &mut self,
        config: &crate::terminals::commands::drone::DroneConfig,
        now: Instant,
    ) {
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
            },
        );
        self.update_parameter_line(name, value, timestamp);
    }

    /// Update a specific parameter line in the terminal view
    fn update_parameter_line(&mut self, name: &str, value: f32, timestamp: Instant) {
        let Some(&line_index) = self.parameter_to_line.get(name) else {
            return; // Parameter not found in mapping
        };

        // Format the parameter line
        let line_text = format!("{}({:.2})", name, value);

        // Check if this parameter was updated recently for highlighting
        let now = Instant::now();
        let was_recently_updated = now.duration_since(timestamp).as_secs_f32() < 0.1; // Very recent

        // Update the specific line directly in the terminal view
        self.update_line_at_index(line_index, &line_text, was_recently_updated);
    }

    /// Update a specific line in the terminal view by index
    fn update_line_at_index(&mut self, line_index: usize, text: &str, should_highlight: bool) {
        // Use the new update_line_at_index method to update in place
        self.terminal_view
            .update_line_at_index(line_index, text, should_highlight);
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
