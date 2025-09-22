// src/terminals/drone_parameters_display.rs
//
// Live display of drone parameters with color highlighting for recent updates

use super::terminal_view::{TerminalView, TerminalViewParams};
use crate::{
    groups::VoiceId,
    model::controller::{Command, CommandInner},
};
use nannou::prelude::*;
use std::{collections::HashMap, time::Instant};

#[derive(Clone)]
pub struct ParameterUpdate {
    pub value: f32,
    pub last_updated: Instant,
}

// Voice-level parameters (shown first)
const VOICE_PARAMETERS: &[&str] = &["brightness", "volume", "feedback", "vibration"];

// Circle-specific parameters (shown for each circle)
const CIRCLE_PARAMETERS: &[&str] = &[
    "gravity",
    "force",
    "outerRadius",
    "innerRadius",
    "noise",
    "centerX",
    "centerY",
];

pub struct DroneParametersDisplay {
    terminal_view: TerminalView,
    voice_id: VoiceId,
    voice_parameters: HashMap<String, ParameterUpdate>,
    circle_parameters: HashMap<usize, HashMap<String, ParameterUpdate>>,
    voice_param_to_line: HashMap<String, usize>,
    circle_param_to_line: HashMap<(usize, String), usize>,
    known_circles: std::collections::BTreeSet<usize>,
}

impl DroneParametersDisplay {
    pub fn new(voice: VoiceId, params: TerminalViewParams) -> Self {
        let mut terminal_view = TerminalView::new(voice, params);
        let mut voice_param_to_line = HashMap::new();

        // Add initial header
        let header = format!("Voice {} Parameters:", voice.to_i32());
        terminal_view.add_text(&header, false);

        let mut line_index = 1; // Start after header

        // Pre-populate voice-level parameter lines
        for &param_name in VOICE_PARAMETERS {
            voice_param_to_line.insert(param_name.to_string(), line_index);
            let placeholder_line = format!("{}(-.--)", param_name);
            terminal_view.update_line_at_index(line_index, &placeholder_line, true);
            line_index += 1;
        }

        // Add Wind Circles separator
        terminal_view.update_line_at_index(line_index, "--- Wind Circles ---", false);
        line_index += 1;

        Self {
            terminal_view,
            voice_id: voice,
            voice_parameters: HashMap::new(),
            circle_parameters: HashMap::new(),
            voice_param_to_line,
            circle_param_to_line: HashMap::new(),
            known_circles: std::collections::BTreeSet::new(),
        }
    }

    /// Extract parameter updates from a command and update internal state
    pub fn process_command(&mut self, command: &Command) {
        let now = Instant::now();

        match &command.command {
            // Voice-level parameters
            CommandInner::Alpha { voice_id, value } => {
                if *voice_id == self.voice_id {
                    self.update_voice_parameter("brightness", *value, now);
                }
            }
            CommandInner::Volume { voice_id, value } => {
                if *voice_id == self.voice_id {
                    self.update_voice_parameter("volume", *value, now);
                }
            }
            CommandInner::Feedback {
                voice_id: voice,
                value,
            } => {
                if *voice == self.voice_id {
                    self.update_voice_parameter("feedback", *value, now);
                }
            }
            CommandInner::Vibration { voice_id, value } => {
                if *voice_id == self.voice_id {
                    self.update_voice_parameter("vibration", *value, now);
                }
            }

            // Circle-specific parameters
            CommandInner::OuterRadius {
                voice_id,
                circle_id,
                value,
            } => {
                if *voice_id == self.voice_id {
                    self.update_circle_parameter(*circle_id, "outerRadius", *value, now);
                }
            }
            CommandInner::InnerRadius {
                voice_id,
                circle_id,
                value,
            } => {
                if *voice_id == self.voice_id {
                    self.update_circle_parameter(*circle_id, "innerRadius", *value, now);
                }
            }
            CommandInner::Force {
                voice_id,
                circle_id,
                value,
            } => {
                if *voice_id == self.voice_id {
                    self.update_circle_parameter(*circle_id, "force", *value, now);
                }
            }
            CommandInner::Gravity {
                voice_id,
                circle_id,
                value,
            } => {
                if *voice_id == self.voice_id {
                    self.update_circle_parameter(*circle_id, "gravity", *value, now);
                }
            }
            CommandInner::Noise {
                voice_id,
                circle_id,
                value,
            } => {
                if *voice_id == self.voice_id {
                    self.update_circle_parameter(*circle_id, "noise", *value, now);
                }
            }
            CommandInner::ForceCenterX {
                voice_id,
                circle_id,
                value,
            } => {
                if *voice_id == self.voice_id {
                    self.update_circle_parameter(*circle_id, "centerX", *value, now);
                }
            }
            CommandInner::ForceCenterY {
                voice_id,
                circle_id,
                value,
            } => {
                if *voice_id == self.voice_id {
                    self.update_circle_parameter(*circle_id, "centerY", *value, now);
                }
            }
            CommandInner::CreateDrone { config } => {
                if config.voice_enum() == self.voice_id {
                    self.process_drone_config(config, now);
                }
            }
            CommandInner::ModifyDrone {
                voice_id: voice,
                config,
            } => {
                if *voice == self.voice_id {
                    self.process_drone_config(config, now);
                }
            }
            CommandInner::ListCircles { .. } => {
                // ListCircles is a query command that doesn't modify parameters
                // No action needed for parameter display
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
        // Voice-level parameters
        if let Some(brightness) = config.brightness {
            self.update_voice_parameter("brightness", brightness, now);
        }
        if let Some(volume) = config.volume {
            self.update_voice_parameter("volume", volume, now);
        }
        if let Some(feedback) = config.feedback {
            self.update_voice_parameter("feedback", feedback, now);
        }
        if let Some(vibration) = config.vibration {
            self.update_voice_parameter("vibration", vibration, now);
        }

        // Circle-specific parameters - assume circle 0 for DroneConfig
        // (DroneConfig typically comes from makeDrone/createDrone which creates the first circle)
        let circle_id = 0;
        if let Some(gravity) = config.gravity {
            self.update_circle_parameter(circle_id, "gravity", gravity, now);
        }
        if let Some(force) = config.force {
            self.update_circle_parameter(circle_id, "force", force, now);
        }
        if let Some(outer_radius) = config.outer_radius {
            self.update_circle_parameter(circle_id, "outerRadius", outer_radius, now);
        }
        if let Some(inner_radius) = config.inner_radius {
            self.update_circle_parameter(circle_id, "innerRadius", inner_radius, now);
        }
        if let Some(noise) = config.noise {
            self.update_circle_parameter(circle_id, "noise", noise, now);
        }
        if let Some(center_x) = config.center_x {
            self.update_circle_parameter(circle_id, "centerX", center_x, now);
        }
        if let Some(center_y) = config.center_y {
            self.update_circle_parameter(circle_id, "centerY", center_y, now);
        }
    }

    /// Update a voice-level parameter value and timestamp
    fn update_voice_parameter(&mut self, name: &str, value: f32, timestamp: Instant) {
        self.voice_parameters.insert(
            name.to_string(),
            ParameterUpdate {
                value,
                last_updated: timestamp,
            },
        );
        self.update_voice_parameter_line(name, value, timestamp);
    }

    /// Update a circle-specific parameter value and timestamp
    fn update_circle_parameter(
        &mut self,
        circle_id: usize,
        name: &str,
        value: f32,
        timestamp: Instant,
    ) {
        // Ensure circle exists in our tracking
        if !self.known_circles.contains(&circle_id) {
            self.add_circle_display(circle_id);
        }

        // Update the parameter
        self.circle_parameters
            .entry(circle_id)
            .or_insert_with(HashMap::new)
            .insert(
                name.to_string(),
                ParameterUpdate {
                    value,
                    last_updated: timestamp,
                },
            );

        self.update_circle_parameter_line(circle_id, name, value, timestamp);
    }

    /// Update a voice-level parameter line in the terminal view
    fn update_voice_parameter_line(&mut self, name: &str, value: f32, timestamp: Instant) {
        let Some(&line_index) = self.voice_param_to_line.get(name) else {
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

    /// Update a circle-specific parameter line in the terminal view
    fn update_circle_parameter_line(
        &mut self,
        circle_id: usize,
        name: &str,
        value: f32,
        timestamp: Instant,
    ) {
        let key = (circle_id, name.to_string());
        let Some(&line_index) = self.circle_param_to_line.get(&key) else {
            return; // Parameter not found in mapping
        };

        // Format the parameter line with indentation for circles
        let line_text = format!("  {}({:.2})", name, value);

        // Check if this parameter was updated recently for highlighting
        let now = Instant::now();
        let was_recently_updated = now.duration_since(timestamp).as_secs_f32() < 0.1; // Very recent

        // Update the specific line directly in the terminal view
        self.update_line_at_index(line_index, &line_text, was_recently_updated);
    }

    /// Add display lines for a new circle
    fn add_circle_display(&mut self, circle_id: usize) {
        if self.known_circles.contains(&circle_id) {
            return; // Already added
        }

        self.known_circles.insert(circle_id);

        // Calculate where to insert the new circle section
        let mut line_index = 1 + VOICE_PARAMETERS.len() + 1; // header + voice params + separator

        // Account for existing circles
        for &existing_circle in &self.known_circles {
            if existing_circle < circle_id {
                line_index += 1 + CIRCLE_PARAMETERS.len(); // circle header + circle params
            }
        }

        // Add circle header
        let circle_header = format!("Circle {}:", circle_id);
        self.terminal_view
            .update_line_at_index(line_index, &circle_header, false);
        line_index += 1;

        // Add parameter lines for this circle
        for &param_name in CIRCLE_PARAMETERS {
            let key = (circle_id, param_name.to_string());
            self.circle_param_to_line.insert(key, line_index);

            let placeholder_line = format!("  {}(-.--)", param_name);
            self.terminal_view
                .update_line_at_index(line_index, &placeholder_line, true);
            line_index += 1;
        }

        // Update line indices for circles that come after this one
        self.update_circle_line_indices();
    }

    /// Update line indices for all circles after changes
    fn update_circle_line_indices(&mut self) {
        let mut line_index = 1 + VOICE_PARAMETERS.len() + 1; // header + voice params + separator

        for &circle_id in &self.known_circles {
            line_index += 1; // circle header

            for &param_name in CIRCLE_PARAMETERS {
                let key = (circle_id, param_name.to_string());
                self.circle_param_to_line.insert(key, line_index);
                line_index += 1;
            }
        }
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
        self.voice_parameters.clear();
        self.circle_parameters.clear();
        self.circle_param_to_line.clear();
        self.known_circles.clear();
        self.terminal_view.clear();
    }

    /// Get the voice this display is tracking
    pub fn voice(&self) -> VoiceId {
        self.voice_id
    }
}
