// src/text/view.rs
//
// View model for rendering fixed line slots.
//
// This is the spiritual successor to `terminals::terminal_view::TerminalView`, but it:
// - takes already-composed lines (no business logic)
// - uses a more general layout model (`text::layout`)
// - retains the core animation logic (typewriter + fade)

use crate::text::{TextFadeMode, TextLine, TextStyle};
use nannou::{prelude::*, text::Font};
use std::time::Instant;

use super::layout::{HorizontalJustify, TextBoxLayout};

#[derive(Debug, Clone, Copy)]
pub struct TextTheme {
    pub normal_bright: Rgba,
    pub normal_regular: Rgba,

    pub error_bright: Rgba,
    pub error_regular: Rgba,

    pub ai_bright: Rgba,
    pub ai_regular: Rgba,
}

impl Default for TextTheme {
    fn default() -> Self {
        Self {
            normal_bright: rgba(0.7, 0.7, 0.7, 1.0),
            normal_regular: rgba(0.2, 0.2, 0.2, 0.85),
            error_bright: rgba(1.0, 0.3, 0.3, 1.0),
            error_regular: rgba(0.4, 0.1, 0.1, 0.85),
            ai_bright: rgba(0.6, 0.9, 0.6, 1.0),
            ai_regular: rgba(0.2, 0.4, 0.2, 0.85),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TextPaneViewParams {
    pub layout: TextBoxLayout,
    pub font: Font,
    pub font_size: u32,

    /// Extra spacing around each line (in addition to font_size-derived line height).
    pub line_spacing: f32,

    /// After this delay, fading begins.
    pub fade_delay_secs: f32,

    /// Fade duration.
    pub color_fade_secs: f32,

    /// Typewriter speed. If 0, all text is shown immediately.
    pub chars_per_second: f32,

    pub theme: TextTheme,
}

#[derive(Debug, Clone)]
struct LineState {
    raw_text: String,
    chars: Vec<char>,
    char_idx: usize,

    /// Starting (bright) color and target (regular) color.
    bright_color: Rgba,
    regular_color: Rgba,

    /// Current color.
    color: Rgba,

    fade_mode: TextFadeMode,

    last_text_update: Instant,
    last_char_update: Instant,
}

impl LineState {
    fn empty(now: Instant) -> Self {
        Self {
            raw_text: String::new(),
            chars: Vec::new(),
            char_idx: 0,
            bright_color: rgba(0.0, 0.0, 0.0, 0.0),
            regular_color: rgba(0.0, 0.0, 0.0, 0.0),
            color: rgba(0.0, 0.0, 0.0, 0.0),
            fade_mode: TextFadeMode::NoFade,
            last_text_update: now,
            last_char_update: now,
        }
    }

    fn set_from_text_line(&mut self, line: &TextLine, bright: Rgba, regular: Rgba, now: Instant) {
        self.raw_text = line.text.clone();
        self.chars = self.raw_text.chars().collect();
        self.char_idx = 0;
        self.bright_color = bright;
        self.regular_color = regular;
        self.color = bright;
        self.fade_mode = line.fade;
        self.last_text_update = now;
        self.last_char_update = now;
    }

    fn show_all_chars(&mut self) {
        self.char_idx = self.chars.len();
    }

    fn displayed_text(&self) -> String {
        self.chars[..self.char_idx].iter().collect::<String>()
    }
}

/// Renders an already-composed list of lines into a fixed layout.
#[derive(Debug, Clone)]
pub struct TextPaneView {
    params: TextPaneViewParams,
    // One slot per physical line.
    slots: Vec<Option<LineState>>,
}

impl TextPaneView {
    pub fn new(params: TextPaneViewParams) -> Self {
        let slots = vec![None; params.layout.num_lines()];
        Self { params, slots }
    }

    pub fn params(&self) -> &TextPaneViewParams {
        &self.params
    }

    /// Update internal slot states to match the provided composed lines.
    ///
    /// - When a slot changes text, animation timers reset.
    /// - When a slot becomes empty, it is cleared.
    pub fn set_composed_lines(&mut self, lines: &[Option<TextLine>], now: Instant) {
        let n = self.params.layout.num_lines();

        // Keep our slots sized to the layout.
        if self.slots.len() != n {
            self.slots = vec![None; n];
        }

        for idx in 0..n {
            let incoming = lines.get(idx).and_then(|l| l.as_ref());

            let Some(text_line) = incoming else {
                self.slots[idx] = None;
                continue;
            };

            let (bright, regular) = self.colors_for_style(text_line.style);
            let slot = &mut self.slots[idx];

            // If same text and same style/fade, keep the animation state.
            let is_same = slot.as_ref().is_some_and(|s| {
                s.raw_text == text_line.text
                    && s.fade_mode == text_line.fade
                    && s.bright_color == bright
                    && s.regular_color == regular
            });

            if is_same {
                continue;
            }

            let mut state = LineState::empty(now);
            state.set_from_text_line(text_line, bright, regular, now);

            if self.params.chars_per_second <= 0.0 {
                state.show_all_chars();
            }

            *slot = Some(state);
        }
    }

    pub fn update(&mut self, now: Instant) {
        self.update_color(now);
        self.type_text(now);
    }

    pub fn draw(&self, draw: &Draw) {
        for idx in 0..self.params.layout.num_lines() {
            let Some(state) = self.slots.get(idx).and_then(|s| s.as_ref()) else {
                continue;
            };

            let Some(pos) = self.params.layout.line_position(idx) else {
                continue;
            };

            let text = state.displayed_text();

            // Our layout positions store the left edge for the line.
            // Nannou's text builder positions by its center, so offset by width/2.
            let center_x = pos.x + self.params.layout.width / 2.0;

            let mut builder = draw
                .text(&text)
                .w(self.params.layout.width)
                .no_line_wrap()
                .color(state.color)
                .font_size(self.params.font_size)
                .font(self.params.font.clone())
                .x_y(center_x, pos.y);

            match self.params.layout.horizontal_justify {
                HorizontalJustify::Left => builder = builder.left_justify(),
                HorizontalJustify::Center => builder = builder.center_justify(),
                HorizontalJustify::Right => builder = builder.right_justify(),
            };

            // Keep builder alive (it performs draw on drop).
            let _ = builder;
        }
    }

    fn type_text(&mut self, now: Instant) {
        if self.params.chars_per_second <= 0.0 {
            return;
        }

        for slot in &mut self.slots {
            let Some(state) = slot.as_mut() else {
                continue;
            };

            if state.char_idx >= state.chars.len() {
                continue;
            }

            let elapsed = now.duration_since(state.last_char_update);
            let t = elapsed.as_secs_f32() * self.params.chars_per_second;
            let line_length = state.chars.len();
            state.char_idx = ((line_length as f32 * t) as usize).min(line_length);
            state.last_text_update = now;
        }
    }

    fn update_color(&mut self, now: Instant) {
        for slot in &mut self.slots {
            let Some(state) = slot.as_mut() else {
                continue;
            };

            if state.fade_mode == TextFadeMode::NoFade {
                continue;
            }

            // If the color is already the end color, do nothing.
            if state.color == state.regular_color {
                continue;
            }

            let elapsed = now.duration_since(state.last_text_update).as_secs_f32();
            if elapsed <= self.params.fade_delay_secs {
                continue;
            }

            let t = (elapsed - self.params.fade_delay_secs) / self.params.color_fade_secs;
            state.color = if t >= 1.0 {
                state.regular_color
            } else {
                color_lerp(state.bright_color, state.regular_color, t)
            };
        }
    }

    fn colors_for_style(&self, style: TextStyle) -> (Rgba, Rgba) {
        match style {
            TextStyle::Normal | TextStyle::Bright => {
                // Bright is treated as normal but can be forced with NoFade.
                (
                    self.params.theme.normal_bright,
                    self.params.theme.normal_regular,
                )
            }
            TextStyle::Error => (
                self.params.theme.error_bright,
                self.params.theme.error_regular,
            ),
            TextStyle::Ai => (self.params.theme.ai_bright, self.params.theme.ai_regular),
        }
    }
}

fn color_lerp(start: Rgba, end: Rgba, t: f32) -> Rgba {
    let start_hsla = Hsla::from(start);
    let end_hsla = Hsla::from(end);

    let mut hue_diff = end_hsla.hue.to_degrees() - start_hsla.hue.to_degrees();
    if hue_diff.abs() > 180.0 {
        hue_diff -= 360.0 * hue_diff.signum();
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
