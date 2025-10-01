// src/terminals/terminal_view.rs

use nannou::{prelude::*, text::*};
use std::{collections::HashMap, time::Instant};

use super::{command_input::CommandInput, drone_parameters_view::DroneParametersView};
use crate::{groups::VoiceId, model::controller::Command};

/// Defines the alignment corner of a TerminalView box
#[derive(Clone, Copy, Debug)]
pub enum TerminalViewTextJustification {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// Sets whether a line of text should fade to dark or not
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TerminalViewLineFadeMode {
    Fade,
    NoFade,
}

#[derive(Default)]
/// Struct to collect and manager TerminalView updates
pub struct TerminalViewManager {
    terminal_views: HashMap<String, TerminalView>,
    drone_parameter_displays: HashMap<VoiceId, DroneParametersView>,
}

impl TerminalViewManager {
    pub fn new() -> Self {
        Self {
            terminal_views: HashMap::new(),
            drone_parameter_displays: HashMap::new(),
        }
    }

    pub fn new_terminal_view(&mut self, name: &str, voice: VoiceId, params: TerminalViewParams) {
        let terminal_view = TerminalView::new(voice, params);
        self.terminal_views.insert(name.to_owned(), terminal_view);
    }

    pub fn add_line(&mut self, name: &str, text: &str, fade_mode: TerminalViewLineFadeMode) {
        let Some(terminal_view) = self.terminal_views.get_mut(name) else {
            return;
        };

        terminal_view.add_text(text, fade_mode);
    }

    /// Update a terminal view from CommandInput for live display
    pub fn update_from_command_input(&mut self, name: &str, command_input: &CommandInput) {
        let Some(terminal_view) = self.terminal_views.get_mut(name) else {
            return;
        };

        terminal_view.update_from_command_input(command_input);
    }

    /// Clear a specific terminal view
    pub fn clear_terminal_view(&mut self, name: &str) {
        let Some(terminal_view) = self.terminal_views.get_mut(name) else {
            return;
        };

        terminal_view.clear();
    }

    /// Get a terminal view for direct access
    pub fn get_mut_terminal_view(&mut self, name: &str) -> Option<&mut TerminalView> {
        self.terminal_views.get_mut(name)
    }

    /// Get a terminal view for display/drawing (immutable access)
    pub fn get_terminal_view(&self, name: &str) -> Option<&TerminalView> {
        self.terminal_views.get(name)
    }

    /// Add a new drone parameters display for a voice
    pub fn add_drone_parameters_display(&mut self, voice: VoiceId, params: TerminalViewParams) {
        let display = DroneParametersView::new(voice, params);
        self.drone_parameter_displays.insert(voice, display);
    }

    /// Process a command through all drone parameter displays
    pub fn process_command(&mut self, command: &Command) {
        for display in self.drone_parameter_displays.values_mut() {
            display.process_command(command);
        }
    }

    /// Update all drone parameter displays
    pub fn update_drone_parameter_displays(&mut self, draw: &Draw) {
        for display in self.drone_parameter_displays.values_mut() {
            display.update(draw);
        }
    }

    /// Get a drone parameter display for a specific voice
    pub fn get_mut_drone_parameters_display(
        &mut self,
        voice: VoiceId,
    ) -> Option<&mut DroneParametersView> {
        self.drone_parameter_displays.get_mut(&voice)
    }

    /// Get a drone parameter display for display/drawing (immutable access)
    pub fn get_drone_parameters_display(&self, voice: VoiceId) -> Option<&DroneParametersView> {
        self.drone_parameter_displays.get(&voice)
    }

    /// Clear all drone parameter displays
    pub fn clear_all_drone_parameter_displays(&mut self) {
        for display in self.drone_parameter_displays.values_mut() {
            display.clear();
        }
    }
}

/// View of a Terminal
pub struct TerminalView {
    // Text of each line
    lines: Vec<Option<Line>>,
    // Position of each line
    line_positions: Vec<Vec2>,
    // Terminal style parameters
    params: TerminalViewParams,

    // Voice that this Terminal is associated with
    #[allow(dead_code)]
    voice: VoiceId,

    // Rect of this Terminal
    #[allow(dead_code)]
    rect: Rect,
}

impl TerminalView {
    pub fn new(voice: VoiceId, params: TerminalViewParams) -> Self {
        let (line_positions, total_height) = generate_line_positions(&params);

        // Calculate rect based on justification - origin defines the specified corner
        let rect = match params.justification {
            TerminalViewTextJustification::TopLeft => Rect::from_x_y_w_h(
                params.origin.x + params.width / 2.0,
                params.origin.y - total_height / 2.0,
                params.width,
                total_height,
            ),
            TerminalViewTextJustification::TopRight => Rect::from_x_y_w_h(
                params.origin.x - params.width / 2.0,
                params.origin.y - total_height / 2.0,
                params.width,
                total_height,
            ),
            TerminalViewTextJustification::BottomLeft => Rect::from_x_y_w_h(
                params.origin.x + params.width / 2.0,
                params.origin.y + total_height / 2.0,
                params.width,
                total_height,
            ),
            TerminalViewTextJustification::BottomRight => Rect::from_x_y_w_h(
                params.origin.x - params.width / 2.0,
                params.origin.y + total_height / 2.0,
                params.width,
                total_height,
            ),
        };

        let lines = vec![None; params.num_lines];

        Self {
            lines,
            line_positions,
            voice,
            rect,
            params,
        }
    }

    pub fn clear(&mut self) {
        self.lines = vec![None; self.params.num_lines];
    }

    pub fn add_text(&mut self, text: &str, fade_mode: TerminalViewLineFadeMode) {
        let line = Line::new_from_str(text, self.params.bright_color, fade_mode);
        self.add_text_line(line);
    }

    /// Add a pre-created Line object (helper for add_text and live updates)
    fn add_text_line(&mut self, line: Line) {
        match self.params.justification {
            TerminalViewTextJustification::TopLeft | TerminalViewTextJustification::TopRight => {
                // Top justification: add lines at the end, rotate left when full
                let idx = self.get_next_line_idx();
                if idx < self.params.num_lines {
                    self.lines.insert(idx, Some(line));
                } else {
                    self.lines.rotate_left(1);
                    self.lines.insert(self.params.num_lines - 1, Some(line));
                }
            }
            TerminalViewTextJustification::BottomLeft
            | TerminalViewTextJustification::BottomRight => {
                // Bottom justification: add lines at the beginning, shift everything down
                if self.lines.iter().all(|l| l.is_some()) {
                    // Terminal is full, remove the bottom line and add new line at top
                    self.lines.rotate_right(1);
                }
                self.lines[0] = Some(line);
            }
        }
    }

    /// Update a specific line in place without affecting other lines
    pub fn update_line_at_index(
        &mut self,
        index: usize,
        text: &str,
        fade_mode: TerminalViewLineFadeMode,
    ) {
        if index >= self.params.num_lines {
            return; // Index out of bounds
        }

        let mut line = Line::new_from_str(text, self.params.bright_color, fade_mode);
        // Show all characters immediately (no typewriter effect)
        line.show_all_chars();
        self.lines[index] = Some(line);
    }

    /// Update terminal display from CommandInput - convenience method for integration
    pub fn update_from_command_input(&mut self, command_input: &CommandInput) {
        if command_input.is_empty() {
            self.clear();
        } else {
            // Clear all lines first
            self.lines = vec![None; self.params.num_lines];

            let lines: Vec<&str> = command_input.display().split('\n').collect();
            let first_line = lines.first().unwrap_or(&"");

            // Determine which lines to show (last N lines if exceeding capacity)
            let start_idx = if lines.len() > self.params.num_lines {
                lines.len() - self.params.num_lines
            } else {
                0
            };

            // Place lines based on justification
            match self.params.justification {
                TerminalViewTextJustification::TopLeft
                | TerminalViewTextJustification::TopRight => {
                    // Top justification: place lines top to bottom
                    for (display_idx, line_text) in lines.iter().skip(start_idx).enumerate() {
                        if !line_text.trim().is_empty() || *line_text == *first_line {
                            let mut new_line = Line::new_from_str(
                                line_text,
                                self.params.bright_color,
                                TerminalViewLineFadeMode::NoFade,
                            );
                            new_line.char_idx = new_line.chars.len(); // Show all characters immediately
                            self.lines[display_idx] = Some(new_line);
                        }
                    }
                }
                TerminalViewTextJustification::BottomLeft
                | TerminalViewTextJustification::BottomRight => {
                    // Bottom justification: place lines from bottom up
                    let displayed_lines: Vec<&str> =
                        lines.iter().skip(start_idx).copied().collect();
                    for (i, line_text) in displayed_lines.iter().enumerate() {
                        if !line_text.trim().is_empty() || *line_text == *first_line {
                            let mut new_line = Line::new_from_str(
                                line_text,
                                self.params.bright_color,
                                TerminalViewLineFadeMode::NoFade,
                            );
                            new_line.char_idx = new_line.chars.len(); // Show all characters immediately
                                                                      // Place from bottom: last line goes at bottom index
                            let display_idx = self.params.num_lines - displayed_lines.len() + i;
                            self.lines[display_idx] = Some(new_line);
                        }
                    }
                }
            }
        }
    }

    fn get_next_line_idx(&self) -> usize {
        let mut next_idx: usize = 0;
        for line in self.lines.iter() {
            if line.is_some() {
                next_idx += 1;
            } else {
                break;
            }
        }
        next_idx
    }

    pub fn update(&mut self, draw: &Draw) {
        let now = Instant::now();
        self.update_color(now);
        self.type_text(now);
        self.draw(draw);
    }

    fn type_text(&mut self, now: Instant) {
        for line in &mut self.lines {
            let Some(line) = line else {
                continue;
            };

            if line.char_idx >= line.chars.len() {
                continue;
            }

            line.type_text(now, self.params.chars_per_second);
        }
    }

    fn update_color(&mut self, now: Instant) {
        for line in &mut self.lines {
            let Some(line) = line else {
                continue;
            };

            // If the color is already the end color, do nothing
            if line.color == self.params.regular_color {
                continue;
            }

            // If it's not a fading line, do nothing
            if line.fade_mode == TerminalViewLineFadeMode::NoFade {
                continue;
            }

            let elapsed = now.duration_since(line.last_text_update).as_secs_f32();

            // Wait 1 second before starting color fade
            if elapsed <= 1.0 {
                continue;
            }

            let t = (elapsed - 1.0) / self.params.color_fade_secs;

            line.color = if t >= 1.0 {
                self.params.regular_color
            } else {
                color_lerp(self.params.bright_color, self.params.regular_color, t)
            };
        }
    }

    pub fn draw(&self, draw: &Draw) {
        for (idx, line) in self.lines.iter().enumerate() {
            let Some(line) = line else {
                continue;
            };

            // Get the position of the line
            let pos = self.line_positions.get(idx).unwrap_or(&self.params.origin);

            // it's ok to clone the font becuase the font is a static resource
            let font = self.params.font.clone();

            let text = line.chars[..line.char_idx].iter().collect::<String>();

            // Convert corner-based position to center-based position for nannou
            // pos.x is always the left edge of the text area, so center is pos.x + width/2
            let center_x = pos.x + self.params.width / 2.0;

            let text_builder = draw
                .text(&text)
                .w(self.params.width)
                .color(line.color)
                .font_size(self.params.font_size)
                .font(font)
                .x_y(center_x, pos.y);

            // Apply appropriate text justification based on terminal justification
            match self.params.justification {
                TerminalViewTextJustification::TopLeft
                | TerminalViewTextJustification::BottomLeft => {
                    text_builder.left_justify();
                }
                TerminalViewTextJustification::TopRight
                | TerminalViewTextJustification::BottomRight => {
                    text_builder.right_justify();
                }
            }
        }
    }
}

pub struct TerminalViewParams {
    pub origin: Vec2,
    pub num_lines: usize,
    pub width: f32,
    pub line_spacing: f32,
    pub bright_color: Rgba,
    pub regular_color: Rgba,
    pub color_fade_secs: f32,
    pub chars_per_second: f32,
    pub font: Font,
    pub font_size: u32,
    pub justification: TerminalViewTextJustification,
}

/// Struct defining a Line of text with properties for animations
#[derive(Clone)]
pub struct Line {
    chars: Vec<char>,
    char_idx: usize,
    color: Rgba,
    fade_mode: TerminalViewLineFadeMode,
    last_text_update: Instant,
    last_char_update: Instant,
}

impl Line {
    /// Creates a new Line from a String Slice
    pub fn new_from_str(content: &str, color: Rgba, fade_mode: TerminalViewLineFadeMode) -> Self {
        let now = Instant::now();
        let chars = content.chars().collect();

        Self {
            chars,
            char_idx: 0,
            color,
            fade_mode,
            last_text_update: now,
            last_char_update: now,
        }
    }

    /// Updates own character display index based on time
    pub fn type_text(&mut self, now: Instant, chars_per_second: f32) {
        let elapsed = now.duration_since(self.last_char_update);
        let t = elapsed.as_secs_f32() * chars_per_second;
        let line_length = self.chars.len();
        self.char_idx = ((line_length as f32 * t) as usize).min(line_length);
        self.last_text_update = now;
    }

    /// Show all characters immediately (skip typewriter effect)
    pub fn show_all_chars(&mut self) {
        self.char_idx = self.chars.len();
    }
}

// Generate the positions of the lines in the terminal, also return total height
fn generate_line_positions(params: &TerminalViewParams) -> (Vec<Vec2>, f32) {
    let mut line_positions = Vec::new();
    let line_height = params.font_size as f32 + params.line_spacing * 2.0;
    let total_height = line_height * params.num_lines as f32;

    // Calculate base positions based on justification
    let (base_x, start_y) = match params.justification {
        TerminalViewTextJustification::TopLeft => (params.origin.x, params.origin.y),
        TerminalViewTextJustification::TopRight => {
            (params.origin.x - params.width, params.origin.y)
        }
        TerminalViewTextJustification::BottomLeft => {
            (params.origin.x, params.origin.y + total_height)
        }
        TerminalViewTextJustification::BottomRight => (
            params.origin.x - params.width,
            params.origin.y + total_height,
        ),
    };

    // All justifications flow lines downward, but start from different Y positions
    for i in 0..params.num_lines {
        let line_offset = (i as f32 + 0.5) * line_height;

        line_positions.push(Vec2::new(
            base_x,
            start_y - line_offset, // All lines go down from start_y
        ));
    }

    (line_positions, total_height)
}

fn color_lerp(start: Rgba, end: Rgba, t: f32) -> Rgba {
    let start_hsla = Hsla::from(start);
    let end_hsla = Hsla::from(end);

    let mut hue_diff = end_hsla.hue.to_degrees() - start_hsla.hue.to_degrees();
    if hue_diff.abs() > 180.0 {
        hue_diff -= 360.0 * hue_diff.signum(); // shortest path
    }
    let hue = start_hsla.hue.to_degrees() + hue_diff * t;

    let lerped_hsla = Hsla::new(
        hue,
        start_hsla.saturation + (end_hsla.saturation - start_hsla.saturation) * t,
        start_hsla.lightness + (end_hsla.lightness - start_hsla.lightness) * t,
        start_hsla.alpha + (end_hsla.alpha - start_hsla.alpha) * t,
    );

    Rgba::from(lerped_hsla)
}
