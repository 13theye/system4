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
    last_ai_status_text: Option<String>,
    last_ai_status_line_count: usize,
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
            self.last_ai_status_line_count = 0;
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
        let block = TextBlock::new(text)
            .style(TextStyle::Ai)
            .fade(TextFadeMode::NoFade)
            .wrap(WrapPolicy::WordWrap);

        let new_lines: Vec<TextLine> = line_breaker::break_block(&block, now, max_chars);
        let new_count = new_lines.len();

        pane.replace_tail_history_lines(self.last_ai_status_line_count, new_lines);
        self.last_ai_status_line_count = new_count;
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
