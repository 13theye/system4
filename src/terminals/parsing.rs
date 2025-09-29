// src/terminals/parsing.rs

use super::{
    commands::{
        drone::DroneBuilder, rhythm::RhythmBuilder, TerminalCommand, TerminalCommandBuilder,
    },
    tokens::Token,
};
use std::fmt;

/// Voice type for parameter validation
#[derive(Debug, Clone, PartialEq)]
pub enum VoiceType {
    Drone,
    Rhythm,
}

/// Parameter categories for validation
#[derive(Debug, Clone, PartialEq)]
pub enum ParameterCategory {
    Drone,
    Rhythm,
}

/// Categorize a parameter name for validation
pub fn categorize_parameter(param_name: &str) -> Option<ParameterCategory> {
    match param_name {
        // Drone parameters
        "brightness" | "volume" | "feedback" | "vibration" | "gravity" | "force"
        | "outerRadius" | "innerRadius" | "noise" | "centerX" | "centerY" | "newCircle"
        | "removeCircle" => Some(ParameterCategory::Drone),
        // Rhythm parameters
        "capacity" | "wings" | "sub" | "addWings" | "removeWings" => {
            Some(ParameterCategory::Rhythm)
        }
        // Voice parameter (used in both contexts)
        "voice" => None, // Special case - not categorized
        // Unknown parameter
        _ => None,
    }
}

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
            "rhythm" => self.parse_rhythm_command(),
            "voice" => self.parse_voice_command(),
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
        let mut found_add = false;
        let mut is_new_circle = false;
        let mut circle_id: Option<i32> = None;

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
                } else if method_name == "add" {
                    self.expect_token(&Token::LeftParen)?;
                    self.expect_token(&Token::RightParen)?;
                    found_add = true;
                    break;
                } else if method_name == "listCircles" {
                    self.expect_token(&Token::LeftParen)?;
                    self.expect_token(&Token::RightParen)?;
                    // Expect semicolon at the end
                    if self.position < self.tokens.len() {
                        self.expect_token(&Token::Semicolon)?;
                    }
                    return Ok(TerminalCommand::ListCircles { voice_id });
                } else if method_name == "newCircle" {
                    self.expect_token(&Token::LeftParen)?;
                    self.expect_token(&Token::RightParen)?;
                    is_new_circle = true;
                } else if method_name == "removeCircle" {
                    self.expect_token(&Token::LeftParen)?;
                    circle_id = match self.current_token() {
                        Some(Token::Number(n)) => {
                            let id = *n as i32;
                            self.position += 1;
                            Some(id)
                        }
                        Some(token) => {
                            return Err(ParseError::UnexpectedToken {
                                expected: "circle ID number".to_string(),
                                found: format!("{:?}", token),
                            })
                        }
                        None => return Err(ParseError::UnexpectedEnd),
                    };
                    self.expect_token(&Token::RightParen)?;
                    return Ok(TerminalCommand::RemoveCircle {
                        voice_id,
                        circle_id: circle_id.unwrap(),
                    });
                } else if method_name == "circle" {
                    // Parse circle ID
                    self.expect_token(&Token::LeftParen)?;
                    circle_id = match self.current_token() {
                        Some(Token::Number(n)) => {
                            let id = *n as i32;
                            self.position += 1;
                            Some(id)
                        }
                        Some(token) => {
                            return Err(ParseError::UnexpectedToken {
                                expected: "circle ID number".to_string(),
                                found: format!("{:?}", token),
                            })
                        }
                        None => return Err(ParseError::UnexpectedEnd),
                    };
                    self.expect_token(&Token::RightParen)?;
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

        if !found_set && !found_add {
            return Err(if is_new_circle {
                ParseError::MissingAdd
            } else {
                ParseError::MissingSet
            });
        }

        // Expect semicolon at the end
        if self.position < self.tokens.len() {
            self.expect_token(&Token::Semicolon)?;
        }

        // Return appropriate command based on the type of operation
        if is_new_circle {
            Ok(TerminalCommand::NewCircle {
                voice_id,
                config: builder.build(),
            })
        } else if let Some(circle) = circle_id {
            Ok(TerminalCommand::ModifyVoiceCircle {
                voice_id,
                circle_id: circle,
                config: builder.build(),
            })
        } else {
            Ok(TerminalCommand::ModifyDroneParams {
                voice_id,
                config: builder.build(),
            })
        }
    }

    fn parse_voice_command(&mut self) -> Result<TerminalCommand, ParseError> {
        // Parse: voice(voice_id).makeCommand()...
        // We've already parsed "voice", now expect (voice_id)
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

        // Expect dot before the sub-command
        self.expect_token(&Token::Dot)?;

        // Get the sub-command type
        let sub_command = self.expect_identifier_any()?;

        match sub_command.as_str() {
            "makeDrone" => self.parse_make_drone_from_voice(voice_id),
            "makeRhythm" => self.parse_make_rhythm_from_voice(voice_id),
            "addWings" => self.parse_wing_command(voice_id, sub_command, true),
            "removeWings" => self.parse_wing_command(voice_id, sub_command, false),
            "newCircle" => self.parse_new_circle_from_voice(voice_id),
            "removeCircle" => self.parse_remove_circle_from_voice(voice_id),
            _ => {
                // Check if this is a parameter modification command
                // We need to determine what type of parameter this is
                match categorize_parameter(&sub_command) {
                    Some(ParameterCategory::Drone) => self.parse_voice_parameter_modification(
                        voice_id,
                        sub_command,
                        VoiceType::Drone,
                    ),
                    Some(ParameterCategory::Rhythm) => self.parse_voice_parameter_modification(
                        voice_id,
                        sub_command,
                        VoiceType::Rhythm,
                    ),
                    None => Err(ParseError::UnexpectedToken {
                        expected: "makeDrone, makeRhythm, or valid parameter name".to_string(),
                        found: sub_command,
                    }),
                }
            }
        }
    }

    fn parse_make_drone_from_voice(
        &mut self,
        voice_id: i32,
    ) -> Result<TerminalCommand, ParseError> {
        // Parse: voice(voice_id).makeDrone().method().begin();
        // We've already parsed "voice(voice_id).makeDrone", now expect ()
        self.expect_token(&Token::LeftParen)?;
        self.expect_token(&Token::RightParen)?;

        let mut builder = DroneBuilder::new();

        // Set the voice ID that was parsed from the voice() command
        builder.set_parameter("voice", ParameterValue::Number(voice_id as f32))?;

        let mut found_begin = false;

        // Parse method chain (same as existing makeDrone parsing)
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

    fn parse_make_rhythm_from_voice(
        &mut self,
        voice_id: i32,
    ) -> Result<TerminalCommand, ParseError> {
        // Parse: voice(voice_id).makeRhythm().method().begin();
        // We've already parsed "voice(voice_id).makeRhythm", now expect ()
        self.expect_token(&Token::LeftParen)?;
        self.expect_token(&Token::RightParen)?;

        let mut builder = RhythmBuilder::new();

        // Set the voice ID that was parsed from the voice() command
        builder.set_parameter("voice", ParameterValue::Number(voice_id as f32))?;

        let mut found_begin = false;

        // Parse method chain (same as existing makeRhythm parsing)
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

        // Build the RhythmConfig
        let config = builder.build();

        // Validate the voice is appropriate for rhythm commands
        config.validate_voice()?;

        Ok(TerminalCommand::CreateSequencer { config })
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

    fn _expect_identifier(&mut self, expected: &str) -> Result<(), ParseError> {
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

    fn parse_voice_parameter_modification(
        &mut self,
        voice_id: i32,
        first_param: String,
        param_type: VoiceType,
    ) -> Result<TerminalCommand, ParseError> {
        // Parse: voice(voice_id).parameter(value).set();
        // We've already parsed "voice(voice_id).parameter", now expect (value)
        self.expect_token(&Token::LeftParen)?;
        let parameter_value = self.parse_parameter()?;
        self.expect_token(&Token::RightParen)?;

        let mut drone_builder = DroneBuilder::new();
        let mut rhythm_builder = RhythmBuilder::new();

        // Set the voice ID for both builders
        drone_builder.set_parameter("voice", ParameterValue::Number(voice_id as f32))?;
        rhythm_builder.set_parameter("voice", ParameterValue::Number(voice_id as f32))?;

        // Set the first parameter based on its type
        match param_type {
            VoiceType::Drone => {
                drone_builder.set_parameter(&first_param, parameter_value)?;
            }
            VoiceType::Rhythm => {
                rhythm_builder.set_parameter(&first_param, parameter_value)?;
            }
        }

        let mut found_set = false;
        let mut has_drone_params = param_type == VoiceType::Drone;
        let mut has_rhythm_params = param_type == VoiceType::Rhythm;

        // Parse additional method calls in the chain
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
                    // Parse additional parameter
                    self.expect_token(&Token::LeftParen)?;
                    let parameter = self.parse_parameter()?;
                    self.expect_token(&Token::RightParen)?;

                    match categorize_parameter(&method_name) {
                        Some(ParameterCategory::Drone) => {
                            drone_builder.set_parameter(&method_name, parameter)?;
                            has_drone_params = true;
                        }
                        Some(ParameterCategory::Rhythm) => {
                            rhythm_builder.set_parameter(&method_name, parameter)?;
                            has_rhythm_params = true;
                        }
                        None => {
                            return Err(ParseError::UnexpectedToken {
                                expected: "valid parameter name".to_string(),
                                found: method_name,
                            });
                        }
                    }
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

        // Return appropriate command based on what parameters were set
        match (has_drone_params, has_rhythm_params) {
            (true, false) => Ok(TerminalCommand::ModifyDroneParams {
                voice_id,
                config: drone_builder.build(),
            }),
            (false, true) => Ok(TerminalCommand::ModifyRhythmParams {
                voice_id,
                config: rhythm_builder.build(),
            }),
            (true, true) => Ok(TerminalCommand::ModifyVoiceParams {
                voice_id,
                drone_config: Some(drone_builder.build()),
                rhythm_config: Some(rhythm_builder.build()),
            }),
            (false, false) => {
                // This shouldn't happen if categorize_parameter works correctly
                Err(ParseError::UnexpectedToken {
                    expected: "valid parameter name".to_string(),
                    found: first_param,
                })
            }
        }
    }

    fn parse_wing_command(
        &mut self,
        voice_id: i32,
        _command_name: String,
        is_add: bool,
    ) -> Result<TerminalCommand, ParseError> {
        // Parse: voice(voice_id).addWings(count); or voice(voice_id).removeWings(count);
        // We've already parsed "voice(voice_id).addWings/removeWings", now expect (count)
        self.expect_token(&Token::LeftParen)?;

        let count = match self.current_token() {
            Some(Token::Number(n)) => {
                let count = *n as usize;
                self.position += 1;
                count
            }
            Some(token) => {
                return Err(ParseError::UnexpectedToken {
                    expected: "number".to_string(),
                    found: format!("{:?}", token),
                })
            }
            None => return Err(ParseError::UnexpectedEnd),
        };

        self.expect_token(&Token::RightParen)?;

        // Expect semicolon at the end (no .set() required)
        if self.position < self.tokens.len() {
            self.expect_token(&Token::Semicolon)?;
        }

        // Return appropriate command
        if is_add {
            Ok(TerminalCommand::AddWings { voice_id, count })
        } else {
            Ok(TerminalCommand::RemoveWings { voice_id, count })
        }
    }

    fn parse_new_circle_from_voice(
        &mut self,
        voice_id: i32,
    ) -> Result<TerminalCommand, ParseError> {
        // Parse: voice(voice_id).newCircle().parameters().add();
        // We've already parsed "voice(voice_id).newCircle", now expect ()
        self.expect_token(&Token::LeftParen)?;
        self.expect_token(&Token::RightParen)?;

        let mut builder = DroneBuilder::new();

        // Set the voice ID that was parsed from the voice() command
        builder.set_parameter("voice", ParameterValue::Number(voice_id as f32))?;

        let mut found_add = false;

        // Parse method chain until we find .add()
        while self.position < self.tokens.len() {
            if let Some(Token::Dot) = self.current_token() {
                self.expect_token(&Token::Dot)?;

                let method_name = self.expect_identifier_any()?;

                if method_name == "add" {
                    self.expect_token(&Token::LeftParen)?;
                    self.expect_token(&Token::RightParen)?;
                    found_add = true;
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

        if !found_add {
            return Err(ParseError::MissingAdd);
        }

        // Expect semicolon at the end
        if self.position < self.tokens.len() {
            self.expect_token(&Token::Semicolon)?;
        }

        Ok(TerminalCommand::NewCircle {
            voice_id,
            config: builder.build(),
        })
    }

    fn parse_remove_circle_from_voice(
        &mut self,
        voice_id: i32,
    ) -> Result<TerminalCommand, ParseError> {
        // Parse: voice(voice_id).removeCircle(circle_id);
        // We've already parsed "voice(voice_id).removeCircle", now expect (circle_id)
        self.expect_token(&Token::LeftParen)?;

        let circle_id = match self.current_token() {
            Some(Token::Number(n)) => {
                let id = *n as i32;
                self.position += 1;
                id
            }
            Some(token) => {
                return Err(ParseError::UnexpectedToken {
                    expected: "circle ID number".to_string(),
                    found: format!("{:?}", token),
                })
            }
            None => return Err(ParseError::UnexpectedEnd),
        };

        self.expect_token(&Token::RightParen)?;

        // Expect semicolon at the end
        if self.position < self.tokens.len() {
            self.expect_token(&Token::Semicolon)?;
        }

        Ok(TerminalCommand::RemoveCircle {
            voice_id,
            circle_id,
        })
    }

    fn parse_rhythm_command(&mut self) -> Result<TerminalCommand, ParseError> {
        // Parse: rhythm(voice_id).method().method().set();
        // We've already parsed "rhythm", now expect (voice_id)
        self.expect_token(&Token::LeftParen)?;

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

        let mut builder = RhythmBuilder::new();
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

        Ok(TerminalCommand::ModifyRhythmParams {
            voice_id,
            config: builder.build(),
        })
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
