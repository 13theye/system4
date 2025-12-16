// src/text/overlay.rs
//
// Glue between text data (TextPane) and rendering (TextPaneView).

use crate::text::{line_breaker, TextBlock, TextLine, TextPane, TextPaneId, TextSlot};
use nannou::prelude::*;
use std::collections::HashMap;
use std::time::Instant;

use super::view::TextPaneView;

#[derive(Debug, Default)]
pub struct TextOverlay {
    panes: HashMap<TextPaneId, TextPane>,
    views: HashMap<TextPaneId, TextPaneView>,
}

impl TextOverlay {
    pub fn new() -> Self {
        Self::default()
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

    pub fn set_live_block(
        &mut self,
        id: TextPaneId,
        slot: TextSlot,
        block: TextBlock,
        now: Instant,
    ) {
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
        pane.set_live_slot_lines(slot, lines);
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
