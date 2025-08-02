// src/view/player.rs
//
// This module defines a terminal view

use nannou::{prelude::*, text::*};
use std::{collections::HashMap, time::Instant};

use crate::view::Voice;

#[derive(Clone)]
pub struct Line {
    pub chars: Vec<char>,
    pub displayed_chars: usize,
    pub color: Rgba,
    pub does_fade: bool,
    pub last_text_update: Instant,
    pub last_char_update: Instant,
}

impl Line {
    pub fn type_text(&mut self, now: Instant, chars_per_second: f32) {
        let elapsed = now.duration_since(self.last_char_update);
        let t = elapsed.as_secs_f32() * chars_per_second;
        let line_length = self.chars.len();
        self.displayed_chars = ((line_length as f32 * t) as usize).min(line_length);
        self.last_text_update = now;
    }

    pub fn new_for_start_sequence(lines_text: Vec<String>, color: Rgba) -> Vec<Self> {
        lines_text
            .iter()
            .map(|text| Self {
                chars: text.chars().collect(),
                displayed_chars: 0,
                color,
                does_fade: false,
                last_text_update: Instant::now(),
                last_char_update: Instant::now(),
            })
            .collect()
    }
}

pub struct StartSequence {
    pub lines: Vec<Line>,
    pub current_line: usize,
}

impl StartSequence {
    pub fn new(
        color: Rgba,
        brightness: f32,
        volume: f32,
        force: f32,
        gravity: f32,
        trail: f32,
    ) -> Self {
        let lines_text = vec![
            "> make drone  ".to_string(),
            format!("brightness? {}  ", brightness),
            format!("volume? {}  ", volume),
            format!("force? {}  ", force),
            format!("gravity? {}  ", gravity),
            format!("feedback? {}  ", trail),
            "... drone created            ".to_string(),
            " ".to_string(),
            " ".to_string(),
            " ".to_string(),
            " ".to_string(),
            " ".to_string(),
            " ".to_string(),
            " ".to_string(),
        ];

        let lines = Line::new_for_start_sequence(lines_text, color);
        Self {
            lines,
            current_line: 0,
        }
    }
}

pub struct Terminal {
    lines: HashMap<usize, Option<Line>>,
    line_positions: HashMap<usize, Vec2>,

    pub voice: Voice,
    pub params: TerminalParams,
    pub rect: Rect,

    pub start_sequence: Option<StartSequence>,
    pub in_start_sequence: bool,

    dpi_scale: f32,
}

pub struct TerminalParams {
    pub origin: Vec2,
    pub num_lines: usize,
    pub line_width: f32,
    pub line_margin: f32,
    pub start_color: Rgba,
    pub end_color: Rgba,
    pub color_fade_secs: f32,
    pub chars_per_second: f32,
    pub font: Font,
    pub font_size: u32,
}

// Generate the positions of the lines in the terminal, also return total height
fn generate_line_positions(params: &TerminalParams) -> (HashMap<usize, Vec2>, f32) {
    let mut line_positions = HashMap::new();
    let line_height = params.font_size as f32 + params.line_margin * 2.0;
    let total_height = line_height * params.num_lines as f32;

    // Start from the top of the terminal and go down
    let top_y = params.origin.y + total_height / 2.0;

    for i in 0..params.num_lines {
        line_positions.insert(
            i,
            Vec2::new(
                params.origin.x,
                top_y - (i as f32 + 0.5) * line_height, // Center each line within its slot
            ),
        );
    }

    (line_positions, total_height)
}

impl Terminal {
    pub fn new_with_params(voice: Voice, params: TerminalParams, dpi_scale: f32) -> Self {
        let (line_positions, total_height) = generate_line_positions(&params);

        let rect = Rect::from_x_y_w_h(
            params.origin.x,
            params.origin.y,
            params.line_width,
            total_height,
        );

        Self {
            lines: HashMap::new(),
            line_positions,

            voice,
            rect,
            params,

            start_sequence: None,
            in_start_sequence: false,

            dpi_scale,
        }
    }

    pub fn update(&mut self, draw: &Draw) -> Option<bool> {
        if self.in_start_sequence {
            // returns Some(true) when start_sequence has just finished
            self.update_start_sequence(draw)
        } else {
            let now = Instant::now();
            self.update_color(now);
            self.type_text(now);
            self.draw_lines(draw);

            // No need to return anything
            None
        }
    }

    // returns true when start sequence has just finished
    pub fn update_start_sequence(&mut self, draw: &Draw) -> Option<bool> {
        // retrieve start sequence or do nothing
        let Some(start_sequence) = self.start_sequence.as_mut() else {
            self.in_start_sequence = false;
            return None;
        };

        let now = Instant::now();
        let bottom_idx = self.params.num_lines - 1; // Bottom line index

        // Check if we need to shift lines up (bottom line is fully displayed)
        let mut should_shift_up = false;
        if let Some(Some(bottom_line)) = self.lines.get(&bottom_idx) {
            if bottom_line.displayed_chars >= bottom_line.chars.len() {
                should_shift_up = true;
            }
        }

        // Shift all lines up one position if needed
        if should_shift_up {
            // Move lines up: line[i] = line[i+1], starting from top
            for i in 0..self.params.num_lines - 1 {
                let line_below = self.lines.remove(&(i + 1)).unwrap_or(None);
                self.lines.insert(i, line_below);
            }
            // Clear the bottom line after shifting
            self.lines.insert(bottom_idx, None);
        }

        // Add next StartSequence line to bottom if bottom is empty
        if matches!(self.lines.get(&bottom_idx), None | Some(None)) {
            if start_sequence.current_line < start_sequence.lines.len() {
                // Clone the next line from StartSequence
                let next_line = start_sequence.lines[start_sequence.current_line].clone();

                if next_line.chars == vec![' '] {
                    self.params.chars_per_second = 6.0;
                } else {
                    self.params.chars_per_second = 6.0; // 0.6 was a nice speed for audience
                }

                // Reset timing for this line instance
                let new_line = Line {
                    chars: next_line.chars,
                    displayed_chars: 0,
                    color: self.params.start_color,
                    does_fade: false,
                    last_text_update: now,
                    last_char_update: now,
                };

                self.lines.insert(bottom_idx, Some(new_line));
                start_sequence.current_line += 1;
            } else {
                // All StartSequence lines have been processed
                self.in_start_sequence = false;
                self.start_sequence = None;
                return Some(true);
            }
        }

        // Update typing for all current lines
        self.type_text(now);

        // Update colors and draw
        //self.update_color(now);
        self.draw_lines(draw);

        // return false since start sequence is not finished yet
        Some(false)
    }

    pub fn begin_start_sequence(
        &mut self,
        brightness: f32,
        volume: f32,
        force: f32,
        gravity: f32,
        trail: f32,
    ) {
        let start_sequence = StartSequence::new(
            self.params.start_color,
            brightness,
            force,
            volume,
            gravity,
            trail,
        );
        self.start_sequence = Some(start_sequence);
        self.in_start_sequence = true;
    }

    pub fn type_text(&mut self, now: Instant) {
        for line in self.lines.values_mut() {
            let Some(line) = line else {
                continue;
            };

            if line.displayed_chars >= line.chars.len() {
                continue;
            }

            line.type_text(now, self.params.chars_per_second);
        }
    }

    pub fn update_color(&mut self, now: Instant) {
        for line in self.lines.values_mut() {
            let Some(line) = line else {
                continue;
            };
            // If the color is already the end color, do nothing
            if line.color == self.params.end_color {
                continue;
            }

            // If it's not a fading line, do nothing
            if !line.does_fade {
                continue;
            }

            let elapsed = (now.duration_since(line.last_text_update)).as_secs_f32();

            // Wait for 1 second before starting the color fade
            if elapsed <= 1.0 {
                continue;
            }

            let t = (elapsed - 1.0) / self.params.color_fade_secs;

            line.color = if t >= 1.0 {
                self.params.end_color
            } else {
                color_lerp(self.params.start_color, self.params.end_color, t)
            };
        }
    }

    pub fn add_text_to_line(&mut self, idx: usize, text: String, does_fade: bool) {
        let chars = text.chars().collect();
        let line = Line {
            chars,
            displayed_chars: 0,
            color: self.params.start_color,
            does_fade,
            last_text_update: Instant::now(),
            last_char_update: Instant::now(),
        };
        self.lines.insert(idx, Some(line));
    }

    pub fn draw_lines(&self, draw: &Draw) {
        for (idx, line) in self.lines.iter() {
            let Some(line) = line else {
                continue;
            };

            // Get position of the line
            let pos = self.line_positions.get(idx).unwrap_or(&self.params.origin);

            // it's ok to clone the font here because the font is a static resource
            let font = self.params.font.clone();

            let text = line.chars[..line.displayed_chars]
                .iter()
                .collect::<String>();

            draw.text(&text)
                .left_justify()
                .w(self.params.line_width) // Constrain text to terminal width
                .color(line.color)
                .font_size(self.params.font_size)
                .font(font)
                .x_y(pos.x, pos.y);
        }
    }
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
