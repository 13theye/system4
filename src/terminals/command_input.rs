// src/terminals/command_input.rs
//
// Multi-line command input for NTerminal

use super::{new_terminal::NTerminal, TerminalCommand, ParseError};
use std::ops::Range;

#[derive(Debug, Clone)]
pub struct CommandInput {
    /// Raw text content for editing
    raw_text: String,
    /// Formatted display of the command
    formatted_display: String,
    /// Whether the last command had an error
    last_error: Option<ParseError>,
    /// Successfully parsed command waiting for execution
    pending_command: Option<TerminalCommand>,
    /// Last successful command execution message
    last_success: Option<String>,
}

impl Default for CommandInput {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandInput {
    pub fn new() -> Self {
        let mut instance = Self {
            raw_text: String::new(),
            formatted_display: String::new(),
            last_error: None,
            pending_command: None,
            last_success: None,
        };
        instance.update_display();
        instance
    }

    /// Try to execute the current command text
    pub fn try_execute(&mut self) -> Option<TerminalCommand> {
        match NTerminal::parse_command(&self.raw_text) {
            Ok(command) => {
                self.last_error = None;
                self.last_success = Some(format!("Command executed: {:?}", command));
                self.pending_command = Some(command.clone());
                self.update_display();
                Some(command)
            }
            Err(error) => {
                self.last_error = Some(error);
                self.last_success = None;
                self.update_display();
                None
            }
        }
    }

    /// Check if command ends with semicolon (ready for execution)
    pub fn is_ready_for_execution(&self) -> bool {
        self.raw_text.trim().ends_with(';')
    }

    /// Update the formatted display from raw text
    fn update_display(&mut self) {
        self.formatted_display.clear();

        if self.raw_text.is_empty() {
            return;
        }

        // Format the raw text by adding indentation to continuation lines
        let lines: Vec<&str> = self.raw_text.split('\n').collect();
        for (i, line) in lines.iter().enumerate() {
            if i == 0 {
                // First line - no indent
                self.formatted_display.push_str(line);
            } else {
                // Subsequent lines - add indent
                self.formatted_display.push('\n');
                self.formatted_display.push_str("  ");
                self.formatted_display.push_str(line);
            }
        }
    }

    /// Clear the current command
    pub fn clear(&mut self) {
        self.raw_text.clear();
        self.formatted_display.clear();
        self.last_error = None;
        self.last_success = None;
        self.pending_command = None;
    }

    /// Get the formatted display string
    pub fn display(&self) -> &str {
        &self.formatted_display
    }

    /// Get the last error if any
    pub fn last_error(&self) -> Option<&ParseError> {
        self.last_error.as_ref()
    }

    /// Get the last success message if any
    pub fn last_success(&self) -> Option<&str> {
        self.last_success.as_ref().map(|s| s.as_str())
    }

    /// Check if there's a pending command ready for execution
    pub fn has_pending_command(&self) -> bool {
        self.pending_command.is_some()
    }

    /// Take the pending command (consuming it)
    pub fn take_pending_command(&mut self) -> Option<TerminalCommand> {
        self.pending_command.take()
    }

    /// Check if the input is currently empty
    pub fn is_empty(&self) -> bool {
        self.raw_text.trim().is_empty()
    }

    /// Get example commands for help text
    pub fn get_examples() -> Vec<&'static str> {
        vec![
            "makeDrone(1).brightness(0.8).outerRadius(500.0).begin();",
            "makeDrone().force(15.5).noise(0.3).feedback(0.9).begin();",
            "drone(1).brightness(0.2).centerX(100.0).centerY(-50.0).set();",
        ]
    }

    /// Format examples for display
    pub fn format_example(command: &str) -> String {
        let mut formatted = String::new();
        let parts: Vec<&str> = command.split('.').collect();

        for (i, part) in parts.iter().enumerate() {
            if i == 0 {
                formatted.push_str(part);
            } else {
                formatted.push('\n');
                formatted.push_str("  .");
                formatted.push_str(part);
            }
        }

        formatted
    }
    /// Get byte position from character position in raw text
    fn byte_index_from_char_index(&self, char_index: usize) -> usize {
        let mut byte_pos = 0;

        for (char_pos, ch) in self.raw_text.chars().enumerate() {
            if char_pos >= char_index {
                break;
            }
            byte_pos += ch.len_utf8();
        }

        byte_pos
    }
}

/// Implementation of TextBuffer trait for egui TextEdit integration
impl egui::TextBuffer for CommandInput {
    fn is_mutable(&self) -> bool {
        true
    }

    fn as_str(&self) -> &str {
        &self.raw_text
    }

    fn insert_text(&mut self, text: &str, char_index: usize) -> usize {
        let byte_index = self.byte_index_from_char_index(char_index);
        self.raw_text.insert_str(byte_index, text);
        self.update_display();
        text.chars().count()
    }

    fn delete_char_range(&mut self, char_range: Range<usize>) {
        let start_byte = self.byte_index_from_char_index(char_range.start);
        let end_byte = self.byte_index_from_char_index(char_range.end);
        self.raw_text.drain(start_byte..end_byte);
        self.update_display();
    }

    fn clear(&mut self) {
        self.raw_text.clear();
        self.formatted_display.clear();
        self.last_error = None;
        self.last_success = None;
        self.pending_command = None;
    }

    fn replace(&mut self, text: &str) {
        self.raw_text = text.to_string();
        self.update_display();
    }
}
