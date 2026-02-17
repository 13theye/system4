// src/text/overlay.rs
//
// Glue between text data (TextPane) and rendering (TextPaneView).

use crate::{
    groups::VoiceId,
    text::{
        line_breaker, params_dashboard::ParamKey, params_dashboard::ParamsDashboard, TextBlock,
        TextFadeMode, TextLine, TextPane, TextPaneId, TextSlot, TextStyle, WrapPolicy,
    },
};
use nannou::prelude::*;
use std::collections::HashMap;
use std::time::Instant;

use super::view::TextPaneView;

#[derive(Debug, Default)]
pub struct TextOverlay {
    panes: HashMap<TextPaneId, TextPane>,
    views: HashMap<TextPaneId, TextPaneView>,

    // Per-pane params dashboards (rendered into TextSlot::Params).
    params_dashboards: HashMap<TextPaneId, ParamsDashboard>,

    // Avoid duplicate history spam: AI status updates rewrite a tail segment.
    // We keep the previously-wrapped lines so we can preserve timestamps for unchanged
    // lines across incremental updates (so they keep fading instead of re-brightening).
    last_ai_status_text: Option<String>,
    last_ai_status_lines: Vec<TextLine>,
}

impl TextOverlay {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a top slot from pre-broken `TextLine`s (no wrapping performed).
    pub fn set_top_lines(&mut self, id: TextPaneId, slot: TextSlot, lines: Vec<TextLine>) {
        let Some(pane) = self.panes.get_mut(&id) else {
            return;
        };
        pane.set_top_slot_lines(slot, lines);
    }

    /// Set a live (bottom) slot from pre-broken `TextLine`s (no wrapping performed).
    pub fn set_live_lines(&mut self, id: TextPaneId, slot: TextSlot, lines: Vec<TextLine>) {
        let Some(pane) = self.panes.get_mut(&id) else {
            return;
        };
        pane.set_live_slot_lines(slot, lines);
    }

    /// Clear pinned param state and remove the params slot lines for a voice.
    pub fn clear_params_dashboard(&mut self, voice: VoiceId) {
        let id = TextPaneId::Voice(voice);
        self.params_dashboards.remove(&id);

        let Some(pane) = self.panes.get_mut(&id) else {
            return;
        };
        pane.clear_top_slot(TextSlot::Params);
    }

    pub fn apply_param_update(&mut self, voice: VoiceId, key: ParamKey, value: f32, now: Instant) {
        let id = TextPaneId::Voice(voice);

        if !self.panes.contains_key(&id) {
            return;
        }

        let dashboard = self
            .params_dashboards
            .entry(id)
            .or_insert_with(|| ParamsDashboard::new_default(voice));

        if !dashboard.update(&key, value, now) {
            return;
        }

        // Keep placeholders so line positions remain stable.
        let lines = dashboard.render_lines(now, true);
        self.set_top_lines(id, TextSlot::Params, lines);
    }

    pub fn insert_pane(&mut self, id: TextPaneId, pane: TextPane, view: TextPaneView) {
        self.panes.insert(id, pane);
        self.views.insert(id, view);
    }

    pub fn pane_mut(&mut self, id: TextPaneId) -> Option<&mut TextPane> {
        self.panes.get_mut(&id)
    }

    pub fn view_mut(&mut self, id: TextPaneId) -> Option<&mut TextPaneView> {
        self.views.get_mut(&id)
    }

    pub fn set_top_block(
        &mut self,
        id: TextPaneId,
        slot: TextSlot,
        block: TextBlock,
        now: Instant,
    ) {
        let Some(view) = self.views.get(&id) else {
            return;
        };

        let max_chars = line_breaker::estimate_max_chars_per_line(
            view.params().layout.width,
            view.params().font_size,
        );

        let lines: Vec<TextLine> = line_breaker::break_block(&block, now, max_chars);
        self.set_top_lines(id, slot, lines);
    }

    pub fn clear_top_slot(&mut self, id: TextPaneId, slot: TextSlot) {
        if slot == TextSlot::Params {
            self.params_dashboards.remove(&id);
        }

        let Some(pane) = self.panes.get_mut(&id) else {
            return;
        };
        pane.clear_top_slot(slot);
    }

    pub fn set_live_block(
        &mut self,
        id: TextPaneId,
        slot: TextSlot,
        block: TextBlock,
        now: Instant,
    ) {
        let Some(view) = self.views.get(&id) else {
            return;
        };

        let max_chars = line_breaker::estimate_max_chars_per_line(
            view.params().layout.width,
            view.params().font_size,
        );

        let lines: Vec<TextLine> = line_breaker::break_block(&block, now, max_chars);
        self.set_live_lines(id, slot, lines);
    }

    pub fn clear_live_slot(&mut self, id: TextPaneId, slot: TextSlot) {
        let Some(pane) = self.panes.get_mut(&id) else {
            return;
        };
        pane.clear_live_slot(slot);
    }

    pub fn push_history_block(&mut self, id: TextPaneId, block: TextBlock, now: Instant) {
        let Some(pane) = self.panes.get_mut(&id) else {
            return;
        };
        let Some(view) = self.views.get(&id) else {
            return;
        };

        let max_chars = line_breaker::estimate_max_chars_per_line(
            view.params().layout.width,
            view.params().font_size,
        );
        let lines: Vec<TextLine> = line_breaker::break_block(&block, now, max_chars);
        pane.push_history_lines(lines);
    }

    /// Push AI status text into Voice2's history (not live text) when it changes.
    pub fn push_ai_status_history_if_changed(&mut self, text: Option<&str>, now: Instant) {
        let Some(text) = text else {
            self.last_ai_status_text = None;
            self.last_ai_status_lines.clear();
            return;
        };

        if self.last_ai_status_text.as_deref() == Some(text) {
            return;
        }

        self.last_ai_status_text = Some(text.to_string());

        let id = TextPaneId::Voice(VoiceId::Voice2);
        let Some(pane) = self.panes.get_mut(&id) else {
            return;
        };
        let Some(view) = self.views.get(&id) else {
            return;
        };

        let max_chars = line_breaker::estimate_max_chars_per_line(
            view.params().layout.width,
            view.params().font_size,
        );

        // Break into visual lines, then rewrite just the previously-written tail segment.
        // Use Fade so status lines flash bright on change, then dim over time.
        //
        // IMPORTANT: as the AI streams more tokens, we frequently re-wrap the *entire*
        // status string. If we stamp every wrapped line with `now`, older lines will
        // incorrectly regain their bright state. To avoid that, preserve timestamps for
        // any wrapped lines whose text didn't change compared to the previous wrap.
        let block = TextBlock::new(text)
            .style(TextStyle::Ai)
            .fade(TextFadeMode::Fade)
            .wrap(WrapPolicy::WordWrap);

        let freshly_wrapped: Vec<TextLine> = line_breaker::break_block(&block, now, max_chars);

        // Lines belonging to the "feeling" JSON object should remain bright.
        // Find the exact range of wrapped lines that contain the object.
        let feeling_range = freshly_wrapped
            .iter()
            .position(|line| line.text.contains("\"feeling\":"))
            .map(|start| feeling_line_range(&freshly_wrapped, start));

        let old_lines = std::mem::take(&mut self.last_ai_status_lines);
        let stabilized: Vec<TextLine> = freshly_wrapped
            .into_iter()
            .enumerate()
            .map(|(i, line)| {
                let is_feeling = feeling_range.is_some_and(|(s, e)| i >= s && i <= e);

                if is_feeling {
                    TextLine::new(line.text, line.style, TextFadeMode::NoFade, now)
                } else {
                    let preserved_ts = old_lines
                        .get(i)
                        .filter(|old| {
                            old.text == line.text
                                && old.style == line.style
                                && old.fade == line.fade
                        })
                        .map(|old| old.timestamp)
                        .unwrap_or(now);

                    TextLine::new(line.text, line.style, line.fade, preserved_ts)
                }
            })
            .collect();

        let old_count = old_lines.len();
        pane.replace_tail_history_lines(old_count, stabilized.clone());
        self.last_ai_status_lines = stabilized;
    }

    pub fn update_and_draw_all(&mut self, draw: &Draw, now: Instant) {
        // Stable ordering: draw by pane id debug order.
        // (HashMap iteration is non-deterministic; this is good enough for now.)
        let ids: Vec<TextPaneId> = self.panes.keys().copied().collect();

        for id in ids {
            let (Some(pane), Some(view)) = (self.panes.get(&id), self.views.get_mut(&id)) else {
                continue;
            };

            let composed = pane.compose();
            view.set_composed_lines(&composed, now);
            view.update(now);
            view.draw(draw);
        }
    }
}

/// Return the (start, end) line indices (inclusive) that contain the string
/// *values* inside the `"feeling"."text"` JSON array, excluding lines that
/// hold only the `"text":` key, the opening `[`, or the closing `]`.
///
/// Specifically, `start` is the line of the first `"` that opens a string
/// element, and `end` is the line of the last `"` that closes one. For
/// compact JSON where brackets and content share a line, both collapse to
/// that same line. For pretty-printed JSON each haiku line gets its own line
/// and the bracket lines are correctly excluded.
///
/// If the array is never fully closed (still streaming), all remaining lines
/// are included.
fn feeling_line_range(lines: &[TextLine], start_idx: usize) -> (usize, usize) {
    let Some(start_line) = lines.get(start_idx) else {
        return (start_idx, start_idx);
    };

    // Begin scanning just after the "feeling": key.
    const FEELING_KEY: &str = "\"feeling\":";
    let feeling_offset = start_line
        .text
        .find(FEELING_KEY)
        .map(|p| p + FEELING_KEY.len())
        .unwrap_or(0);

    // Phase 1: find the "text": key within the feeling object.
    const TEXT_KEY: &str = "\"text\":";
    let text_key_pos = lines
        .iter()
        .enumerate()
        .skip(start_idx)
        .find_map(|(i, line)| {
            let search_in = if i == start_idx {
                &line.text[feeling_offset.min(line.text.len())..]
            } else {
                &line.text
            };
            let base = if i == start_idx { feeling_offset } else { 0 };
            search_in
                .find(TEXT_KEY)
                .map(|off| (i, base + off + TEXT_KEY.len()))
        });

    let Some((text_line_idx, text_offset)) = text_key_pos else {
        return (start_idx, start_idx);
    };

    // Phase 2: scan from after "text": to find the content of the array.
    // We want only the lines containing the actual string elements, not the
    // lines with the surrounding [ ] or the "text": key itself. Track:
    //   content_start_line — line of the first " that opens a string element
    //   content_end_line   — line of the last  " that closes a string element
    // Return that tighter range when ] closes the array.
    let mut depth: i32 = 0;
    let mut array_opened = false;
    let mut in_string = false;
    let mut escape_next = false;
    let mut content_start_line: Option<usize> = None;
    let mut content_end_line = text_line_idx;

    for (line_idx, line) in lines.iter().enumerate().skip(text_line_idx) {
        let slice = if line_idx == text_line_idx {
            &line.text[text_offset.min(line.text.len())..]
        } else {
            &line.text
        };

        for ch in slice.chars() {
            if escape_next {
                escape_next = false;
                continue;
            }
            if in_string {
                match ch {
                    '\\' => escape_next = true,
                    '"' => {
                        in_string = false;
                        content_end_line = line_idx;
                    }
                    _ => {}
                }
            } else {
                match ch {
                    '"' => {
                        in_string = true;
                        if array_opened && content_start_line.is_none() {
                            content_start_line = Some(line_idx);
                        }
                    }
                    '[' => {
                        if !array_opened {
                            array_opened = true;
                        }
                        depth += 1;
                    }
                    ']' => {
                        depth -= 1;
                        if depth <= 0 {
                            return (
                                content_start_line.unwrap_or(text_line_idx),
                                content_end_line,
                            );
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // Array not yet closed (still streaming): include all remaining lines.
    (
        content_start_line.unwrap_or(text_line_idx),
        lines.len().saturating_sub(1),
    )
}
