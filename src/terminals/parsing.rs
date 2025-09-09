// src/terminals/parsing.rs

use super::{
    commands::{drone::DroneBuilder, Command, CommandBuilder},
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
    MissingBuild,
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
            ParseError::MissingBuild => write!(f, "Missing .build() call"),
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

    pub fn parse(&mut self) -> Result<Command, ParseError> {
        if self.tokens.is_empty() {
            return Err(ParseError::EmptyInput);
        }

        // Get the first command token to dispatch on
        let command_type = self.expect_identifier_any()?;

        // Dispatch to appropriate command parser
        match command_type.as_str() {
            "drone" => self.parse_drone_command(),
            _ => Err(ParseError::UnknownCommand(command_type)),
        }
    }

    fn parse_drone_command(&mut self) -> Result<Command, ParseError> {
        // After "drone" we expect a dot for either .new() or .get()
        self.expect_token(&Token::Dot)?;
        
        let method_name = self.expect_identifier_any()?;
        
        match method_name.as_str() {
            "new" => self.parse_create_drone_command(),
            "get" => self.parse_modify_drone_command(),
            _ => Err(ParseError::UnexpectedToken {
                expected: "new or get".to_string(),
                found: method_name,
            }),
        }
    }

    fn parse_create_drone_command(&mut self) -> Result<Command, ParseError> {
        // Parse: drone.new().method().method().build();
        // We've already parsed "drone.new", now expect ()
        self.expect_token(&Token::LeftParen)?;
        self.expect_token(&Token::RightParen)?;

        let mut builder = DroneBuilder::new();
        let mut found_build = false;

        // Parse method chain
        while self.position < self.tokens.len() {
            if let Some(Token::Dot) = self.current_token() {
                self.expect_token(&Token::Dot)?;

                let method_name = self.expect_identifier_any()?;

                if method_name == "build" {
                    self.expect_token(&Token::LeftParen)?;
                    self.expect_token(&Token::RightParen)?;
                    found_build = true;
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

        if !found_build {
            return Err(ParseError::MissingBuild);
        }

        // Expect semicolon at the end
        if self.position < self.tokens.len() {
            self.expect_token(&Token::Semicolon)?;
        }

        Ok(Command::CreateDrone(builder.build()))
    }

    fn parse_modify_drone_command(&mut self) -> Result<Command, ParseError> {
        // Parse: drone.get("name").method().method().set();
        // We've already parsed "drone.get", now expect ("name")
        self.expect_token(&Token::LeftParen)?;
        
        // Get the drone name
        let drone_name = match self.current_token() {
            Some(Token::String(s)) => {
                let name = s.clone();
                self.position += 1;
                name
            }
            Some(token) => return Err(ParseError::UnexpectedToken {
                expected: "drone name string".to_string(),
                found: format!("{:?}", token),
            }),
            None => return Err(ParseError::UnexpectedEnd),
        };
        
        self.expect_token(&Token::RightParen)?;

        let mut builder = DroneBuilder::new();
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

        Ok(Command::ModifyDrone { 
            name: drone_name, 
            config: builder.build() 
        })
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
