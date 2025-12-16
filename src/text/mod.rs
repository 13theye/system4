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
    CommandInput,
    AiStream,
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
    live_slots: HashMap<TextSlot, Vec<TextLine>>,
    slot_line_budgets: HashMap<TextSlot, usize>,
}

impl TextPane {
    pub fn new(capacity_lines: usize) -> Self {
        Self {
            capacity_lines,
            history: TextRingBuffer::new(capacity_lines),
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
        self.live_slots.clear();
    }

    pub fn set_slot_line_budget(&mut self, slot: TextSlot, max_lines: usize) {
        self.slot_line_budgets.insert(slot, max_lines);
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

    /// Compose final visible lines in top-to-bottom order, clipped to pane capacity.
    ///
    /// Current rule:
    /// - Allocate bottom-up space to configured live slots (in a deterministic order).
    /// - Fill the remaining top lines with history (most recent history lines).
    pub fn compose(&self) -> Vec<Option<TextLine>> {
        let cap = self.capacity_lines;
        if cap == 0 {
            return Vec::new();
        }

        // Deterministic slot order (so layout doesn't jitter).
        let slot_order = [TextSlot::CommandInput, TextSlot::AiStream];

        // Gather live lines in final bottom order (CommandInput first, then AI below it
        // or vice versa depending on preferences). We choose: history at top, then
        // command input, then AI at very bottom.
        let mut live_lines: Vec<TextLine> = Vec::new();

        for slot in slot_order {
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

            live_lines.extend(clipped);
        }

        // Clip live lines to total capacity (keep tail so newest ends at bottom).
        if live_lines.len() > cap {
            live_lines = live_lines[live_lines.len() - cap..].to_vec();
        }

        let remaining_for_history = cap.saturating_sub(live_lines.len());

        // Take most recent N history lines.
        let history_vec = self.history.to_vec();
        let history_tail = if history_vec.len() > remaining_for_history {
            history_vec[history_vec.len() - remaining_for_history..].to_vec()
        } else {
            history_vec
        };

        // Build final vector.
        let mut out: Vec<Option<TextLine>> = Vec::with_capacity(cap);
        for line in history_tail {
            out.push(Some(line));
        }
        for line in live_lines {
            out.push(Some(line));
        }

        // Pad at end if we have fewer than cap lines.
        while out.len() < cap {
            out.push(None);
        }

        out
    }
}
