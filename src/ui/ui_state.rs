use crate::terminals::{command_input::CommandInput, terminal_view::TerminalViewManager};
use fps::FpsManager;
use nannou_egui::Egui;
use std::cell::RefCell;

/// UiState encapsulates all UI and debug-related state.
/// This includes:
/// - UI framework (egui)
/// - FPS display manager
/// - Debug visualization flags
/// - Command input interface
/// - Terminal view manager
/// - UI state (active tabs, etc.)
pub struct UiState {
    // UI framework
    pub egui: Egui,

    // FPS display
    pub fps: FpsManager,

    // Debug visualization flags
    pub show_bounds: bool,
    pub show_forces: bool,

    // Command input for NTerminal
    pub command_input: CommandInput,

    // Terminal view manager for on-screen display
    pub terminal_manager: RefCell<TerminalViewManager>,

    // UI state
    pub active_tab: usize, // 0 = Voices, 1 = NTerminal
}

impl UiState {
    /// Create a new UiState with all UI components
    pub fn new(
        egui: Egui,
        fps: FpsManager,
        terminal_manager: TerminalViewManager,
    ) -> Self {
        Self {
            egui,
            fps,
            show_bounds: false,
            show_forces: false,
            command_input: CommandInput::new(),
            terminal_manager: RefCell::new(terminal_manager),
            active_tab: 0,
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

    /// Get the terminal manager
    pub fn terminal_manager(&self) -> &RefCell<TerminalViewManager> {
        &self.terminal_manager
    }
}
