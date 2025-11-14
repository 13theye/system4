// system4-core/src/parsing/errors.rs
// Pure error types for command parsing

use std::fmt;

#[derive(Debug, Clone)]
pub enum ParseError {
    UnexpectedToken { expected: String, found: String },
    UnexpectedEnd,
    InvalidNumber(String),
    UnterminatedString,
    EmptyInput,
    MissingBegin,
    MissingSet,
    MissingAdd,
    UnknownCommand(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::UnexpectedToken { expected, found } => {
                write!(f, "Expected {}, found {}", expected, found)
            }
            ParseError::UnexpectedEnd => write!(f, "Unexpected end of input"),
            ParseError::InvalidNumber(s) => write!(f, "Invalid number: {}", s),
            ParseError::UnterminatedString => write!(f, "Unterminated string"),
            ParseError::EmptyInput => write!(f, "Empty input"),
            ParseError::MissingBegin => write!(f, "Missing .begin() call"),
            ParseError::MissingSet => write!(f, "Missing .set() call"),
            ParseError::MissingAdd => write!(f, "Missing .add() call"),
            ParseError::UnknownCommand(cmd) => write!(f, "Unknown command: {}", cmd),
        }
    }
}

impl std::error::Error for ParseError {}
