//! src/view/center_line.rs
//!
//! The animated central dividing line drawn in the audience window.

use nannou::prelude::*;
use std::time::{Duration, Instant};

pub const DEFAULT_ANIMATION_DURATION_SECS: f32 = 3.0;

/// The central vertical dividing line, with hide/show animation.
///
/// When hiding, the line splits at the center and each half slides toward the
/// nearest screen edge until fully off-screen. Unchecking reverses from
/// whatever progress was reached, taking proportionally less time.
pub struct CenterLine {
    /// How long a full hide or show animation takes.
    pub animation_duration: Duration,
    state: CenterLineState,
}

enum CenterLineState {
    Visible,
    /// Sliding outward. `start_progress` is the split progress at the moment
    /// this animation began (0.0 = fully joined, 1.0 = fully split).
    Hiding {
        start_time: Instant,
        start_progress: f32,
    },
    Hidden,
    /// Sliding back inward. `start_progress` is the split progress at the
    /// moment the reversal was triggered (1.0 when coming from Hidden).
    Showing {
        start_time: Instant,
        start_progress: f32,
    },
}

impl Default for CenterLine {
    fn default() -> Self {
        Self {
            animation_duration: Duration::from_secs_f32(DEFAULT_ANIMATION_DURATION_SECS),
            state: CenterLineState::Visible,
        }
    }
}

impl CenterLine {
    /// Begin the hide animation from the current position.
    pub fn trigger_hide(&mut self) {
        let now = Instant::now();
        let start_progress = match self.state {
            CenterLineState::Hidden => return,
            CenterLineState::Hiding { .. } => return,
            CenterLineState::Visible => 0.0,
            CenterLineState::Showing {
                start_time,
                start_progress,
            } => current_show_progress(start_time, start_progress, self.animation_duration, now),
        };
        self.state = CenterLineState::Hiding {
            start_time: now,
            start_progress,
        };
    }

    /// Begin the show animation from the current position.
    pub fn trigger_show(&mut self) {
        let now = Instant::now();
        let start_progress = match self.state {
            CenterLineState::Visible => return,
            CenterLineState::Showing { .. } => return,
            CenterLineState::Hidden => 1.0,
            CenterLineState::Hiding {
                start_time,
                start_progress,
            } => current_hide_progress(start_time, start_progress, self.animation_duration, now),
        };
        self.state = CenterLineState::Showing {
            start_time: now,
            start_progress,
        };
    }

    /// Advance animation state. Call once per frame from the update loop.
    pub fn update(&mut self, now: Instant) {
        match self.state {
            CenterLineState::Hiding {
                start_time,
                start_progress,
            } => {
                let p =
                    current_hide_progress(start_time, start_progress, self.animation_duration, now);
                if p >= 1.0 {
                    self.state = CenterLineState::Hidden;
                }
            }
            CenterLineState::Showing {
                start_time,
                start_progress,
            } => {
                let p =
                    current_show_progress(start_time, start_progress, self.animation_duration, now);
                if p <= 0.0 {
                    self.state = CenterLineState::Visible;
                }
            }
            _ => {}
        }
    }

    /// Draw the line (or its animated halves) into `draw`.
    ///
    /// `height` is the full height of the render texture in pixels (centered
    /// coordinates, so the line spans from `height/2` to `-height/2`).
    pub fn draw(&self, draw: &Draw, height: f32) {
        let half_h = height / 2.0;

        let split = match self.state {
            CenterLineState::Visible => 0.0,
            CenterLineState::Hidden => return,
            CenterLineState::Hiding {
                start_time,
                start_progress,
            } => current_hide_progress(
                start_time,
                start_progress,
                self.animation_duration,
                Instant::now(),
            ),
            CenterLineState::Showing {
                start_time,
                start_progress,
            } => current_show_progress(
                start_time,
                start_progress,
                self.animation_duration,
                Instant::now(),
            ),
        };

        draw_line_pair(draw, half_h, split * half_h);
    }
}

/// Split progress (0.0–1.0) for a hide animation given a start_progress and elapsed time.
fn current_hide_progress(
    start_time: Instant,
    start_progress: f32,
    duration: Duration,
    now: Instant,
) -> f32 {
    let remaining = 1.0 - start_progress;
    if remaining <= 0.0 {
        return 1.0;
    }
    let anim_secs = duration.as_secs_f32() * remaining;
    let elapsed = now.duration_since(start_time).as_secs_f32();
    (start_progress + elapsed / anim_secs).min(1.0)
}

/// Split progress (0.0–1.0) for a show animation given a start_progress and elapsed time.
fn current_show_progress(
    start_time: Instant,
    start_progress: f32,
    duration: Duration,
    now: Instant,
) -> f32 {
    if start_progress <= 0.0 {
        return 0.0;
    }
    let anim_secs = duration.as_secs_f32() * start_progress;
    let elapsed = now.duration_since(start_time).as_secs_f32();
    (start_progress - elapsed / anim_secs).max(0.0)
}

/// Draw both half-lines with the given outward offset from center.
///
/// At `offset = 0` the halves meet at y = 0 (full line).
/// At `offset = half_h` each half has slid completely off-screen.
fn draw_line_pair(draw: &Draw, half_h: f32, offset: f32) {
    // Top half: from the top edge down to the current split point.
    draw.line()
        .start(vec2(0.0, half_h))
        .end(vec2(0.0, offset))
        .color(rgba(1.0, 1.0, 1.0, 1.0))
        .stroke_weight(2.0);

    // Bottom half: from the current split point down to the bottom edge.
    draw.line()
        .start(vec2(0.0, -offset))
        .end(vec2(0.0, -half_h))
        .color(rgba(1.0, 1.0, 1.0, 1.0))
        .stroke_weight(2.0);
}
