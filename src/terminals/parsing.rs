// src/terminals/parsing/mod.rs

pub mod drone_parser;
pub mod errors;
pub mod parameter;
pub mod rhythm_parser;
pub mod utils;
pub mod voice_parser;

// Re-export main types for compatibility
pub use drone_parser::DroneParser;
pub use errors::ParseError;
pub use parameter::{categorize_parameter, ParameterCategory, ParameterValue, VoiceType};
pub use rhythm_parser::RhythmParser;
pub use voice_parser::VoiceParser;

use crate::terminals::{commands::TerminalCommand, tokens::Token};

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
            utils::ParsingUtils::expect_identifier_any(&self.tokens, &mut self.position)?;

        // Dispatch to appropriate specialized parser
        match command_type.as_str() {
            "drone" => DroneParser::parse_drone_command(&self.tokens, &mut self.position),
            "rhythm" => RhythmParser::parse_rhythm_command(&self.tokens, &mut self.position),
            "voice" => VoiceParser::parse_voice_command(&self.tokens, &mut self.position),
            _ => Err(ParseError::UnknownCommand(command_type)),
        }
    }
}
