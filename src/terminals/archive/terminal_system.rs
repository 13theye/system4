// src/terminals/terminal_system.rs
//
// Container for managing multiple terminals.

use nannou::prelude::*;
use std::collections::HashMap;

use crate::{
    terminals::{Terminal, TerminalParams},
    voice::Voice,
};

#[derive(Default)]
pub struct TerminalSystem {
    pub terminals: HashMap<Voice, Terminal>,
}

impl TerminalSystem {
    pub fn new() -> Self {
        Self {
            terminals: HashMap::new(),
        }
    }

    pub fn add_new_terminal(
        &mut self,
        voice: Voice,
        params: TerminalParams,
        dpi_scale: f32,
    ) -> Option<&mut Terminal> {
        let terminal = Terminal::new_with_params(voice, params, dpi_scale);
        self.terminals.insert(voice, terminal);
        self.terminals.get_mut(&voice)
    }

    pub fn update(&mut self, draw: &Draw) -> Vec<(Voice, bool)> {
        let mut finish_signals = Vec::new();

        for (voice, terminal) in self.terminals.iter_mut() {
            if let Some(finish_signal) = terminal.update(draw) {
                finish_signals.push((*voice, finish_signal));
            }
        }

        finish_signals
    }

    pub fn add_text_line(&mut self, voice: Voice, text: String, does_fade: bool) {
        self.terminals
            .get_mut(&voice)
            .unwrap()
            .add_text_to_line(1, text, does_fade);
    }
}
