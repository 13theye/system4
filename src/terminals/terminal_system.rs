// src/terminals/terminal_system.rs
//
// Container for managing multiple terminals.

use nannou::prelude::*;
use std::collections::HashMap;

use crate::terminals::{Terminal, TerminalParams};

#[derive(Default)]
pub struct TerminalSystem {
    pub terminals: HashMap<usize, Terminal>,
}

impl TerminalSystem {
    pub fn new() -> Self {
        Self {
            terminals: HashMap::new(),
        }
    }

    pub fn add_new_terminal(
        &mut self,
        id: usize,
        params: TerminalParams,
        dpi_scale: f32,
    ) -> Option<&mut Terminal> {
        let terminal = Terminal::new_with_params(id, params, dpi_scale);
        self.terminals.insert(id, terminal);
        self.terminals.get_mut(&id)
    }

    pub fn update(&mut self, draw: &Draw) -> Option<(usize, bool)> {
        for (id, terminal) in self.terminals.iter_mut() {
            if let Some(finish_signal) = terminal.update(draw) {
                return Some((*id, finish_signal));
            }
        }
        None
    }

    pub fn add_text_line(&mut self, id: usize, text: String, does_fade: bool) {
        self.terminals
            .get_mut(&id)
            .unwrap()
            .add_text_to_line(1, text, does_fade);
    }
}
