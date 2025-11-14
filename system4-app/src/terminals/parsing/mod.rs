// src/terminals/parsing/mod.rs
//
// Re-exports of parsing types from core

// Re-export all parsing types and functions from core
pub use system4_core::parsing::{
    categorize_parameter, parse_drone_command, parse_rhythm_command, parse_voice_command,
    ParameterCategory, ParameterValue, ParseError, ParsingUtils, Token, VoiceType,
};

use crate::terminals::commands::TerminalCommand;

/// Main command parser that dispatches to specialized parsers
pub struct CommandParser {
    tokens: Vec<Token>,
    position: usize,
}

impl CommandParser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            position: 0,
        }
    }

    pub fn parse(&mut self) -> Result<TerminalCommand, ParseError> {
        if self.tokens.is_empty() {
            return Err(ParseError::EmptyInput);
        }

        // Get the first command token to dispatch on
        let command_type =
            ParsingUtils::expect_identifier_any(&self.tokens, &mut self.position)?;

        // Dispatch to appropriate specialized parser function
        match command_type.as_str() {
            "drone" => parse_drone_command(&self.tokens, &mut self.position),
            "rhythm" => parse_rhythm_command(&self.tokens, &mut self.position),
            "voice" => parse_voice_command(&self.tokens, &mut self.position),
            _ => Err(ParseError::UnknownCommand(command_type)),
        }
    }
}