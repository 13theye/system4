//! src/view/windows/control.rs
//!
//! The functions used to render the contents of the control window.
//!
//! fn control_view() - called every frame by main.rs for the
//!                     Nannou update loop.

use crate::model::Model;

use nannou::prelude::*;

/// Draw the control window (UI)
pub fn control_view(app: &App, model: &Model, frame: Frame) {
    // Draw background first
    model.render_state.control_draw.background().color(BLACK);
    let _ = model.render_state.control_draw.to_frame(app, &frame);
    // Then draw egui UI on top
    model.ui_state.egui.draw_to_frame(&frame).unwrap();
}
