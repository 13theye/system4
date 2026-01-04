use crate::{groups::VoiceId, terminals::command_input::CommandInput, text::overlay::TextOverlay};
use fps::FpsManager;
use nannou_egui::Egui;
use std::{cell::RefCell, collections::HashMap};

/// UiState encapsulates all UI and debug-related state.
/// This includes:
/// - UI framework (egui)
/// - FPS display manager
/// - Debug visualization flags
/// - Command input interface
/// - Text overlay (rendered in audience window)
/// - UI state (active tabs, etc.)
pub struct UiState {
    // UI framework
    pub egui: Egui,

    // FPS display
    pub fps: FpsManager,

    // Debug visualization flags
    pub debug_unlock: bool,
    pub show_bounds: bool,
    pub show_forces: bool,

    // Per-voice command inputs
    pub command_inputs: HashMap<VoiceId, CommandInput>,

    // Unified text overlay (data + view)
    pub text_overlay: RefCell<TextOverlay>,

    // UI state
    pub active_tab: usize, // 0 = Voices, 1 = NTerminal

    /// When enabled, automatically trigger AI rhythm generation for Voice2
    /// whenever the rhythm for Voice1 is created or modified.
    pub auto_ai_from_voice1: bool,
}

impl UiState {
    /// Create a new UiState with all UI components
    pub fn new(egui: Egui, fps: FpsManager, text_overlay: TextOverlay) -> Self {
        Self {
            egui,
            fps,
            debug_unlock: false,
            show_bounds: false,
            show_forces: false,
            command_inputs: HashMap::from([
                (VoiceId::Voice0, CommandInput::new()),
                (VoiceId::Voice1, CommandInput::new()),
                (VoiceId::Voice2, CommandInput::new()),
                (VoiceId::Voice3, CommandInput::new()),
            ]),
            text_overlay: RefCell::new(text_overlay),
            active_tab: 0,
            auto_ai_from_voice1: false,
        }
    }

    /// Toggle bounds visualization
    pub fn toggle_bounds(&mut self) {
        self.show_bounds = !self.show_bounds;
    }

    /// Toggle force vectors visualization
    pub fn toggle_forces(&mut self) {
        self.show_forces = !self.show_forces;
    }

    /// Set the active tab
    pub fn set_active_tab(&mut self, tab_index: usize) {
        self.active_tab = tab_index;
    }
}
