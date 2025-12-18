// src/text/mod.rs
//
// Unified text data model: per-pane line composition with history ring buffer
// and optional live slots.
//
// This module is intentionally rendering-agnostic.
//
// Note: the `layout` and `view` submodules are view-oriented and depend on nannou.

pub mod adapters;
pub mod layout;
pub mod line_breaker;
pub mod overlay;
pub mod params_dashboard;
pub mod view;

use crate::groups::VoiceId;
use std::collections::{HashMap, VecDeque};
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextPaneId {
    Voice(VoiceId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextSlot {
    /// Bottom-most live slot: in-progress command input.
    CommandInput,
    /// Live slot above command input: streaming output.
    AiStream,
    /// Top-pinned live slot: stable parameter lines rendered as command-like fragments.
    Params,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextFadeMode {
    Fade,
    NoFade,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextStyle {
    Normal,
    Bright,
    Error,
    Ai,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WrapPolicy {
    /// Wrap on whitespace boundaries where possible.
    WordWrap,
    /// Treat explicit newlines as hard breaks; within each line, still wrap.
    HardWrap,
    /// Keep only the last N visual lines when rendering.
    TruncateTail { max_lines: usize },
    /// Keep only the first N visual lines when rendering.
    TruncateHead { max_lines: usize },
    /// Keep head+tail and insert an ellipsis line when rendering.
    TruncateMiddle { head: usize, tail: usize },
    /// If the text parses as JSON, pretty-print it before wrapping.
    ///
    /// Note: pretty-printing is performed by the caller or by a future
    /// LineBreaker implementation; this policy is metadata for now.
    JsonPrettyPrintIfValid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextBlock {
    pub text: String,
    pub style: TextStyle,
    pub fade: TextFadeMode,
    pub wrap: WrapPolicy,
}

impl TextBlock {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style: TextStyle::Normal,
            fade: TextFadeMode::NoFade,
            wrap: WrapPolicy::WordWrap,
        }
    }

    pub fn style(mut self, style: TextStyle) -> Self {
        self.style = style;
        self
    }

    pub fn fade(mut self, fade: TextFadeMode) -> Self {
        self.fade = fade;
        self
    }

    pub fn wrap(mut self, wrap: WrapPolicy) -> Self {
        self.wrap = wrap;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextLine {
    pub text: String,
    pub style: TextStyle,
    pub fade: TextFadeMode,
    pub ts: Instant,
}

impl TextLine {
    pub fn new(text: impl Into<String>, style: TextStyle, fade: TextFadeMode, ts: Instant) -> Self {
        Self {
            text: text.into(),
            style,
            fade,
            ts,
        }
    }
}

/// Fixed-capacity ring buffer of visual lines.
#[derive(Debug, Clone)]
pub struct TextRingBuffer {
    capacity: usize,
    lines: VecDeque<TextLine>,
}

impl TextRingBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            lines: VecDeque::with_capacity(capacity),
        }
    }

    pub fn pop_back_n(&mut self, n: usize) {
        for _ in 0..n {
            if self.lines.pop_back().is_none() {
                break;
            }
        }
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub fn clear(&mut self) {
        self.lines.clear();
    }

    pub fn push_line(&mut self, line: TextLine) {
        if self.capacity == 0 {
            return;
        }

        if self.lines.len() == self.capacity {
            self.lines.pop_front();
        }
        self.lines.push_back(line);
    }

    pub fn extend_lines<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = TextLine>,
    {
        for line in iter {
            self.push_line(line);
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &TextLine> {
        self.lines.iter()
    }

    pub fn to_vec(&self) -> Vec<TextLine> {
        self.lines.iter().cloned().collect()
    }
}

/// Pure data model for a single on-screen text region.
///
/// - `history` is a scrolling ring buffer of committed visual lines.
/// - `live_slots` contains frequently-updated overlays (e.g. current command
///   input, streaming AI output) that should not spam history.
#[derive(Debug, Clone)]
pub struct TextPane {
    capacity_lines: usize,
    history: TextRingBuffer,

    /// Live slots composed at the top of the pane (pinned).
    top_slots: HashMap<TextSlot, Vec<TextLine>>,

    /// Live slots composed at the bottom of the pane.
    live_slots: HashMap<TextSlot, Vec<TextLine>>,

    /// Max visual lines per slot.
    slot_line_budgets: HashMap<TextSlot, usize>,
}

impl TextPane {
    pub fn new(capacity_lines: usize) -> Self {
        Self {
            capacity_lines,
            history: TextRingBuffer::new(capacity_lines),
            top_slots: HashMap::new(),
            live_slots: HashMap::new(),
            slot_line_budgets: HashMap::new(),
        }
    }

    pub fn capacity_lines(&self) -> usize {
        self.capacity_lines
    }

    pub fn history(&self) -> &TextRingBuffer {
        &self.history
    }

    pub fn history_mut(&mut self) -> &mut TextRingBuffer {
        &mut self.history
    }

    pub fn clear(&mut self) {
        self.history.clear();
        self.top_slots.clear();
        self.live_slots.clear();
    }

    pub fn set_slot_line_budget(&mut self, slot: TextSlot, max_lines: usize) {
        self.slot_line_budgets.insert(slot, max_lines);
    }

    pub fn clear_top_slot(&mut self, slot: TextSlot) {
        self.top_slots.remove(&slot);
    }

    pub fn set_top_slot_lines(&mut self, slot: TextSlot, lines: Vec<TextLine>) {
        self.top_slots.insert(slot, lines);
    }

    pub fn clear_live_slot(&mut self, slot: TextSlot) {
        self.live_slots.remove(&slot);
    }

    pub fn set_live_slot_lines(&mut self, slot: TextSlot, lines: Vec<TextLine>) {
        self.live_slots.insert(slot, lines);
    }

    pub fn push_history_lines(&mut self, lines: impl IntoIterator<Item = TextLine>) {
        self.history.extend_lines(lines);
    }

    /// Remove `old_line_count` lines from the end of history (if present) and append `new_lines`.
    /// This is useful for "status" text that updates frequently but should remain in the
    /// history stream rather than a live slot.
    pub fn replace_tail_history_lines(
        &mut self,
        old_line_count: usize,
        new_lines: impl IntoIterator<Item = TextLine>,
    ) {
        self.history.pop_back_n(old_line_count);
        self.history.extend_lines(new_lines);
    }

    /// Compose final visible lines in top-to-bottom order, clipped to pane capacity.
    ///
    /// Rule:
    /// - Allocate top-down space to configured top slots (pinned).
    /// - Allocate bottom-up space to configured live slots (pinned).
    /// - Fill the remaining middle region with history (most recent lines).
    pub fn compose(&self) -> Vec<Option<TextLine>> {
        let cap = self.capacity_lines;
        if cap == 0 {
            return Vec::new();
        }

        // Deterministic orders (so layout doesn't jitter).
        let top_slot_order = [TextSlot::Params];

        // Bottom-anchored slots. (Command input is intentionally *not* bottom-anchored;
        // it should float up to the highest available line.)
        let bottom_slot_order = [TextSlot::AiStream];

        // --- Top pinned lines (head-clip) ---
        let mut top_lines: Vec<TextLine> = Vec::new();
        for slot in top_slot_order {
            let Some(lines) = self.top_slots.get(&slot) else {
                continue;
            };

            let max_lines = self
                .slot_line_budgets
                .get(&slot)
                .copied()
                .unwrap_or(lines.len());

            let clipped = if lines.len() > max_lines {
                lines[..max_lines].to_vec()
            } else {
                lines.clone()
            };

            top_lines.extend(clipped);
        }

        if top_lines.len() > cap {
            top_lines.truncate(cap);
        }

        let remaining_after_top = cap.saturating_sub(top_lines.len());

        // --- Bottom pinned lines (tail-clip) ---
        let mut bottom_lines: Vec<TextLine> = Vec::new();
        for slot in bottom_slot_order {
            let Some(lines) = self.live_slots.get(&slot) else {
                continue;
            };

            let max_lines = self
                .slot_line_budgets
                .get(&slot)
                .copied()
                .unwrap_or(lines.len());

            let clipped = if lines.len() > max_lines {
                // default: tail
                lines[lines.len() - max_lines..].to_vec()
            } else {
                lines.clone()
            };

            bottom_lines.extend(clipped);
        }

        if bottom_lines.len() > remaining_after_top {
            bottom_lines = bottom_lines[bottom_lines.len() - remaining_after_top..].to_vec();
        }

        // --- Floating command input (tail-clip) ---
        let mut command_lines: Vec<TextLine> = Vec::new();
        if let Some(lines) = self.live_slots.get(&TextSlot::CommandInput) {
            let max_lines = self
                .slot_line_budgets
                .get(&TextSlot::CommandInput)
                .copied()
                .unwrap_or(lines.len());

            command_lines = if lines.len() > max_lines {
                // default: tail
                lines[lines.len() - max_lines..].to_vec()
            } else {
                lines.clone()
            };
        }

        let remaining_for_middle = remaining_after_top.saturating_sub(bottom_lines.len());

        if command_lines.len() > remaining_for_middle {
            // Keep tail so the most recent portion is visible.
            command_lines = command_lines[command_lines.len() - remaining_for_middle..].to_vec();
        }

        let remaining_for_history = remaining_for_middle.saturating_sub(command_lines.len());

        // --- History lines (tail) ---
        let history_vec = self.history.to_vec();
        let history_tail = if history_vec.len() > remaining_for_history {
            history_vec[history_vec.len() - remaining_for_history..].to_vec()
        } else {
            history_vec
        };

        // --- Build output with top + middle(history + command) + bottom ---
        let top_len = top_lines.len();
        let bottom_start = cap - bottom_lines.len();

        let mut out: Vec<Option<TextLine>> = vec![None; cap];

        // Fill top.
        for (i, line) in top_lines.into_iter().enumerate() {
            out[i] = Some(line);
        }

        // Fill bottom.
        for (i, line) in bottom_lines.into_iter().enumerate() {
            out[bottom_start + i] = Some(line);
        }

        let history_tail_len = history_tail.len();

        // Fill history into the middle region, starting just after top.
        for (i, line) in history_tail.into_iter().enumerate() {
            let idx = top_len + i;
            if idx >= bottom_start {
                break;
            }
            out[idx] = Some(line);
        }

        // Fill command input immediately after the visible history tail.
        let command_start = top_len + history_tail_len;
        for (i, line) in command_lines.into_iter().enumerate() {
            let idx = command_start + i;
            if idx >= bottom_start {
                break;
            }
            out[idx] = Some(line);
        }

        out
    }
}
