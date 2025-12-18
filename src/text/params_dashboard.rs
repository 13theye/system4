// src/text/params_dashboard.rs
//
// Rendering-agnostic model for a per-voice, pinned set of parameter lines.
//
// Goal: look like real-time command fragments (e.g. `brightness(0.25)`,
// `circle(0).gravity(0.70)`), while retaining stable line positions per parameter.

use crate::groups::VoiceId;

use super::{TextFadeMode, TextLine, TextStyle};
use std::collections::HashMap;
use std::time::Instant;

/// Identifies a pinned parameter line.
///
/// Note: the formatting here intentionally matches `src/text/adapters.rs` so the
/// pinned lines feel like the same “language” as the scrolling history.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ParamKey {
    /// Voice-level parameter, formatted like `brightness(0.25)`.
    Voice(&'static str),

    /// Circle-level parameter, formatted like `circle(0).gravity(0.70)`.
    Circle {
        circle_id: usize,
        name: &'static str,
    },
}

#[derive(Debug, Default, Clone, Copy)]
struct ParamState {
    value: Option<f32>,
    last_updated: Option<Instant>,
}

#[derive(Debug, Clone)]
pub struct ParamsDashboard {
    #[allow(dead_code)]
    voice: VoiceId,
    tracked: Vec<ParamKey>,
    state: HashMap<ParamKey, ParamState>,

    /// Used when `include_placeholders = true` and a param has never been set.
    placeholder_token: &'static str,
}

impl ParamsDashboard {
    pub fn new(voice: VoiceId, tracked: impl IntoIterator<Item = ParamKey>) -> Self {
        let tracked: Vec<ParamKey> = tracked.into_iter().collect();

        let mut state = HashMap::new();
        for key in &tracked {
            state.insert(key.clone(), ParamState::default());
        }

        Self {
            voice,
            tracked,
            state,
            placeholder_token: "--",
        }
    }

    /// Convenience constructor matching the existing parameter vocabulary used elsewhere.
    pub fn new_default(voice: VoiceId) -> Self {
        Self::new(voice, default_tracked_keys())
    }

    pub fn set_placeholder_token(&mut self, token: &'static str) {
        self.placeholder_token = token;
    }

    /// Update a tracked parameter. Returns `true` if it was tracked and updated.
    pub fn update(&mut self, key: &ParamKey, value: f32, now: Instant) -> bool {
        let Some(state) = self.state.get_mut(key) else {
            return false;
        };

        state.value = Some(value);
        state.last_updated = Some(now);
        true
    }

    /// Render current pinned lines as command-like `TextLine`s.
    ///
    /// - `include_placeholders`: when true, unset params render as `name(--)`.
    ///   when false, unset params are omitted.
    pub fn render_lines(&self, now: Instant, include_placeholders: bool) -> Vec<TextLine> {
        let mut out = Vec::new();

        for key in &self.tracked {
            let state = self.state.get(key).copied().unwrap_or_default();
            let (value, ts) = match (state.value, state.last_updated) {
                (Some(v), Some(ts)) => (Some(v), ts),
                (Some(v), None) => (Some(v), now),
                (None, Some(ts)) => (None, ts),
                (None, None) => (None, now),
            };

            if value.is_none() && !include_placeholders {
                continue;
            }

            out.push(TextLine::new(
                format_command_like(key, value, self.placeholder_token),
                TextStyle::Normal,
                TextFadeMode::Fade,
                ts,
            ));
        }

        out
    }
}

const VOICE_KEYS: &[&str] = &["brightness", "volume", "feedback", "vibration"];
const CIRCLE_KEYS: &[&str] = &[
    "gravity",
    "force",
    "outerRadius",
    "innerRadius",
    "noise",
    "centerX",
    "centerY",
];

pub fn default_tracked_keys() -> Vec<ParamKey> {
    let mut out = Vec::new();

    for &name in VOICE_KEYS {
        out.push(ParamKey::Voice(name));
    }

    // Default to circle(0) to match current DroneConfig assumptions and existing adapters.
    for &name in CIRCLE_KEYS {
        out.push(ParamKey::Circle { circle_id: 0, name });
    }

    out
}

fn format_command_like(key: &ParamKey, value: Option<f32>, placeholder_token: &str) -> String {
    match key {
        ParamKey::Voice(name) => match value {
            Some(v) => format!("{}({:.2})", name, v),
            None => format!("{}({})", name, placeholder_token),
        },
        ParamKey::Circle { circle_id, name } => match value {
            Some(v) => format!("circle({}).{}({:.2})", circle_id, name, v),
            None => format!("circle({}).{}({})", circle_id, name, placeholder_token),
        },
    }
}
