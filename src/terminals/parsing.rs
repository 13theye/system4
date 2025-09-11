// src/terminals/parsing.rs

use super::{
    commands::{drone::DroneBuilder, TerminalCommand, TerminalCommandBuilder},
    tokens::Token,
};
use std::fmt;

/// Parameter::Value pair
#[derive(Debug, Clone)]
pub enum ParameterValue {
    String(String),
    Number(f32),
}

impl fmt::Display for ParameterValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParameterValue::String(s) => write!(f, "\"{}\"", s),
            ParameterValue::Number(n) => write!(f, "{}", n),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ParseError {
    UnexpectedToken { expected: String, found: String },
    UnexpectedEnd,
    InvalidNumber(String),
    UnterminatedString,
    EmptyInput,
    MissingBegin,
    MissingSet,
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
            ParseError::UnknownCommand(cmd) => write!(f, "Unknown command: {}", cmd),
        }
    }
}

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
        let command_type = self.expect_identifier_any()?;

        // Dispatch to appropriate command parser
        match command_type.as_str() {
            "drone" => self.parse_drone_command(),
            "makeDrone" => self.parse_make_drone_command(),
            _ => Err(ParseError::UnknownCommand(command_type)),
        }
    }

    fn parse_drone_command(&mut self) -> Result<TerminalCommand, ParseError> {
        // After "drone" we expect (voice_id) for modification
        self.expect_token(&Token::LeftParen)?;

        // Get the voice ID
        let voice_id = match self.current_token() {
            Some(Token::Number(n)) => {
                let id = *n as i32;
                self.position += 1;
                id
            }
            Some(token) => {
                return Err(ParseError::UnexpectedToken {
                    expected: "voice ID number".to_string(),
                    found: format!("{:?}", token),
                })
            }
            None => return Err(ParseError::UnexpectedEnd),
        };

        self.expect_token(&Token::RightParen)?;

        let mut builder = DroneBuilder::new();

        // Set the voice ID
        builder.set_parameter("voice", ParameterValue::Number(voice_id as f32))?;

        let mut found_set = false;

        // Parse method chain
        while self.position < self.tokens.len() {
            if let Some(Token::Dot) = self.current_token() {
                self.expect_token(&Token::Dot)?;

                let method_name = self.expect_identifier_any()?;

                if method_name == "set" {
                    self.expect_token(&Token::LeftParen)?;
                    self.expect_token(&Token::RightParen)?;
                    found_set = true;
                    break;
                } else {
                    // Parse method call with parameter
                    self.expect_token(&Token::LeftParen)?;
                    let parameter = self.parse_parameter()?;
                    self.expect_token(&Token::RightParen)?;

                    builder.set_parameter(&method_name, parameter)?;
                }
            } else {
                break;
            }
        }

        if !found_set {
            return Err(ParseError::MissingSet);
        }

        // Expect semicolon at the end
        if self.position < self.tokens.len() {
            self.expect_token(&Token::Semicolon)?;
        }

        Ok(TerminalCommand::ModifyDrone {
            voice: voice_id,
            config: builder.build(),
        })
    }

    fn parse_make_drone_command(&mut self) -> Result<TerminalCommand, ParseError> {
        // Parse: makeDrone(voice_id).method().method().begin();
        // We've already parsed "makeDrone", now expect (voice_id)
        self.expect_token(&Token::LeftParen)?;

        // Get the voice ID (optional for makeDrone)
        let voice_id = match self.current_token() {
            Some(Token::Number(n)) => {
                let id = *n as i32;
                self.position += 1;
                Some(id)
            }
            Some(Token::RightParen) => None, // Empty parentheses - no voice provided
            Some(token) => {
                return Err(ParseError::UnexpectedToken {
                    expected: "voice ID number or )".to_string(),
                    found: format!("{:?}", token),
                })
            }
            None => return Err(ParseError::UnexpectedEnd),
        };

        self.expect_token(&Token::RightParen)?;

        let mut builder = DroneBuilder::new();

        // Set the voice if provided in parentheses
        if let Some(voice) = voice_id {
            builder.set_parameter("voice", ParameterValue::Number(voice as f32))?;
        }

        let mut found_begin = false;

        // Parse method chain
        while self.position < self.tokens.len() {
            if let Some(Token::Dot) = self.current_token() {
                self.expect_token(&Token::Dot)?;

                let method_name = self.expect_identifier_any()?;

                if method_name == "begin" {
                    self.expect_token(&Token::LeftParen)?;
                    self.expect_token(&Token::RightParen)?;
                    found_begin = true;
                    break;
                } else {
                    // Parse method call with parameter
                    self.expect_token(&Token::LeftParen)?;
                    let parameter = self.parse_parameter()?;
                    self.expect_token(&Token::RightParen)?;

                    builder.set_parameter(&method_name, parameter)?;
                }
            } else {
                break;
            }
        }

        if !found_begin {
            return Err(ParseError::MissingBegin);
        }

        // Expect semicolon at the end
        if self.position < self.tokens.len() {
            self.expect_token(&Token::Semicolon)?;
        }

        Ok(TerminalCommand::CreateDrone(builder.build()))
    }

    fn current_token(&self) -> Option<&Token> {
        self.tokens.get(self.position)
    }

    fn expect_token(&mut self, expected: &Token) -> Result<(), ParseError> {
        if let Some(token) = self.current_token() {
            if std::mem::discriminant(token) == std::mem::discriminant(expected) {
                self.position += 1;
                Ok(())
            } else {
                Err(ParseError::UnexpectedToken {
                    expected: format!("{:?}", expected),
                    found: format!("{:?}", token),
                })
            }
        } else {
            Err(ParseError::UnexpectedEnd)
        }
    }

    fn expect_identifier(&mut self, expected: &str) -> Result<(), ParseError> {
        if let Some(Token::Identifier(name)) = self.current_token() {
            if name == expected {
                self.position += 1;
                Ok(())
            } else {
                Err(ParseError::UnexpectedToken {
                    expected: expected.to_string(),
                    found: name.clone(),
                })
            }
        } else {
            Err(ParseError::UnexpectedToken {
                expected: expected.to_string(),
                found: self
                    .current_token()
                    .map(|t| format!("{:?}", t))
                    .unwrap_or("EOF".to_string()),
            })
        }
    }

    fn expect_identifier_any(&mut self) -> Result<String, ParseError> {
        if let Some(Token::Identifier(name)) = self.current_token() {
            let name = name.clone();
            self.position += 1;
            Ok(name)
        } else {
            Err(ParseError::UnexpectedToken {
                expected: "identifier".to_string(),
                found: self
                    .current_token()
                    .map(|t| format!("{:?}", t))
                    .unwrap_or("EOF".to_string()),
            })
        }
    }

    fn parse_parameter(&mut self) -> Result<ParameterValue, ParseError> {
        match self.current_token() {
            Some(Token::String(s)) => {
                let value = s.clone();
                self.position += 1;
                Ok(ParameterValue::String(value))
            }
            Some(Token::Number(n)) => {
                let value = *n;
                self.position += 1;
                Ok(ParameterValue::Number(value))
            }
            Some(token) => Err(ParseError::UnexpectedToken {
                expected: "string or number".to_string(),
                found: format!("{:?}", token),
            }),
            None => Err(ParseError::UnexpectedEnd),
        }
    }
}
