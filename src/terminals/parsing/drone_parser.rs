// src/terminals/parsing/drone_parser.rs

use super::{
    errors::ParseError,
    parameter::ParameterValue,
    utils::ParsingUtils,
};
use crate::terminals::{
    commands::{drone::DroneBuilder, TerminalCommand, TerminalCommandBuilder},
    tokens::Token,
};

pub struct DroneParser;

impl DroneParser {
    pub fn parse_drone_command(
        tokens: &[Token],
        position: &mut usize,
    ) -> Result<TerminalCommand, ParseError> {
        // After "drone" we expect (voice_id) for modification
        let voice_id = ParsingUtils::parse_parentheses_with_number(tokens, position, "voice ID")?;

        let mut builder = DroneBuilder::new();
        builder.set_parameter("voice", ParameterValue::Number(voice_id as f32))?;

        let mut found_set = false;
        let mut found_add = false;
        let mut is_new_circle = false;
        let mut circle_id: Option<i32> = None;

        // Parse method chain
        while ParsingUtils::is_at_dot(tokens, *position) {
            ParsingUtils::expect_token(tokens, position, &Token::Dot)?;
            let method_name = ParsingUtils::expect_identifier_any(tokens, position)?;

            match method_name.as_str() {
                "set" => {
                    ParsingUtils::expect_token(tokens, position, &Token::LeftParen)?;
                    ParsingUtils::expect_token(tokens, position, &Token::RightParen)?;
                    found_set = true;
                    break;
                }
                "add" => {
                    ParsingUtils::expect_token(tokens, position, &Token::LeftParen)?;
                    ParsingUtils::expect_token(tokens, position, &Token::RightParen)?;
                    found_add = true;
                    break;
                }
                "listCircles" => {
                    ParsingUtils::expect_token(tokens, position, &Token::LeftParen)?;
                    ParsingUtils::expect_token(tokens, position, &Token::RightParen)?;
                    ParsingUtils::parse_optional_semicolon(tokens, position)?;
                    return Ok(TerminalCommand::ListCircles { voice_id });
                }
                "newCircle" => {
                    ParsingUtils::expect_token(tokens, position, &Token::LeftParen)?;
                    ParsingUtils::expect_token(tokens, position, &Token::RightParen)?;
                    is_new_circle = true;
                }
                "removeCircle" => {
                    let circle_id_val = ParsingUtils::parse_parentheses_with_number(tokens, position, "circle ID")?;
                    return Ok(TerminalCommand::RemoveCircle {
                        voice_id,
                        circle_id: circle_id_val,
                    });
                }
                "circle" => {
                    // Parse circle ID for specific circle modification
                    circle_id = Some(ParsingUtils::parse_parentheses_with_number(tokens, position, "circle ID")?);
                }
                _ => {
                    // Parse method call with parameter
                    let parameter = ParsingUtils::parse_parameter_call(tokens, position)?;
                    builder.set_parameter(&method_name, parameter)?;
                }
            }
        }

        if !found_set && !found_add {
            return Err(if is_new_circle {
                ParseError::MissingAdd
            } else {
                ParseError::MissingSet
            });
        }

        ParsingUtils::parse_optional_semicolon(tokens, position)?;

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
}