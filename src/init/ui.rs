use fps::FpsManager;
use nannou::prelude::*;
use nannou_egui::Egui;

use system4::{text::overlay::TextOverlay, ui::UiState};

use crate::init::WindowIds;

pub fn init_ui(app: &App, window_ids: WindowIds, text_overlay: TextOverlay) -> UiState {
    let Some(control_window) = app.window(window_ids.control) else {
        eprintln!("Control window not found. Exiting app.");
        std::process::exit(1);
    };

    // Set up egui
    let egui = Egui::from_window(&control_window);

    // Create FPS manager
    let mut fps = FpsManager::new_with(true, false);
    let performer_rect = app.window(window_ids.performer).unwrap().rect();
    fps.set_draw_position(pt2(
        performer_rect.left() + 40.0,
        performer_rect.top() - 10.0,
    ));

    // Create UI state
    UiState::new(egui, fps, text_overlay)
}
