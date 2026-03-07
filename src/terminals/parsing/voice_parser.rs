// src/terminals/parsing/voice_parser.rs

use super::{
    errors::ParseError,
    parameter::{categorize_parameter, ParameterCategory, ParameterValue, VoiceType},
    utils::ParsingUtils,
};
use crate::{
    command_engine::commands::FormationType,
    terminals::{
        commands::{
            drone::DroneBuilder, rhythm::RhythmBuilder, TerminalCommand, TerminalCommandBuilder,
        },
        tokens::Token,
    },
};

pub struct VoiceParser;

impl VoiceParser {
    pub fn parse_voice_command(
        tokens: &[Token],
        position: &mut usize,
    ) -> Result<TerminalCommand, ParseError> {
        // Parse: voice(voice_id).makeCommand()...
        // We've already parsed "voice", now expect (voice_id)
        let voice_id = ParsingUtils::parse_parentheses_with_number(tokens, position, "voice ID")?;

        // Expect dot before the sub-command
        ParsingUtils::expect_token(tokens, position, &Token::Dot)?;

        // Get the sub-command type
        let sub_command = ParsingUtils::expect_identifier_any(tokens, position)?;

        match sub_command.as_str() {
            "makeDrone" => Self::parse_make_drone_from_voice(tokens, position, voice_id),
            "makeRhythm" => Self::parse_make_rhythm_from_voice(tokens, position, voice_id),
            "addWings" => Self::parse_wing_command(tokens, position, voice_id, true),
            "removeWings" => Self::parse_wing_command(tokens, position, voice_id, false),
            "newCircle" => Self::parse_new_formation_from_voice(tokens, position, voice_id, FormationType::WindCircle),
            "doubleCircle" => Self::parse_new_formation_from_voice(tokens, position, voice_id, FormationType::DoubleCircle),
            "removeCircle" => Self::parse_remove_circle_from_voice(tokens, position, voice_id),
            "clear" => Self::parse_clear_from_voice(tokens, position, voice_id),
            "generate" => Self::parse_generate_rhythm_from_voice(tokens, position, voice_id),
            _ => {
                // Check if this is a parameter modification command
                match categorize_parameter(&sub_command) {
                    Some(ParameterCategory::Drone) => Self::parse_voice_parameter_modification(
                        tokens,
                        position,
                        voice_id,
                        sub_command,
                        VoiceType::Drone,
                    ),
                    Some(ParameterCategory::Rhythm) => Self::parse_voice_parameter_modification(
                        tokens,
                        position,
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
        tokens: &[Token],
        position: &mut usize,
        voice_id: i32,
    ) -> Result<TerminalCommand, ParseError> {
        // Parse: voice(voice_id).makeDrone().param(...).param(...);
        // Parameters after makeDrone() are treated as creation parameters
        ParsingUtils::expect_token(tokens, position, &Token::LeftParen)?;
        ParsingUtils::expect_token(tokens, position, &Token::RightParen)?;

        let mut builder = DroneBuilder::new();
        builder.set_parameter("voice", ParameterValue::Number(voice_id as f32))?;

        // Parse an optional chain of drone parameter methods after makeDrone()
        while ParsingUtils::is_at_dot(tokens, *position) {
            ParsingUtils::expect_token(tokens, position, &Token::Dot)?;
            let method_name = ParsingUtils::expect_identifier_any(tokens, position)?;

            if method_name == "set" {
                // Allow optional legacy terminator: .set();
                ParsingUtils::expect_token(tokens, position, &Token::LeftParen)?;
                ParsingUtils::expect_token(tokens, position, &Token::RightParen)?;
                ParsingUtils::parse_optional_semicolon(tokens, position)?;
                break;
            } else if method_name == "alt" {
                ParsingUtils::expect_token(tokens, position, &Token::LeftParen)?;
                ParsingUtils::expect_token(tokens, position, &Token::RightParen)?;
                builder.formation_type = FormationType::DoubleCircle;
            } else {
                // Treat all other methods as drone parameters
                let parameter = ParsingUtils::parse_parameter_call(tokens, position)?;
                builder.set_parameter(&method_name, parameter)?;
            }
        }

        // Consume optional trailing semicolon if it wasn't already consumed
        ParsingUtils::parse_optional_semicolon(tokens, position)?;

        Ok(TerminalCommand::CreateDrone(builder.build()))
    }

    fn parse_make_rhythm_from_voice(
        tokens: &[Token],
        position: &mut usize,
        voice_id: i32,
    ) -> Result<TerminalCommand, ParseError> {
        // Parse: voice(voice_id).makeRhythm().param(...).param(...);
        // Parameters after makeRhythm() are treated as creation parameters
        ParsingUtils::expect_token(tokens, position, &Token::LeftParen)?;
        ParsingUtils::expect_token(tokens, position, &Token::RightParen)?;

        let mut builder = RhythmBuilder::new();
        builder.set_parameter("voice", ParameterValue::Number(voice_id as f32))?;

        // Parse an optional chain of rhythm parameter methods after makeRhythm()
        while ParsingUtils::is_at_dot(tokens, *position) {
            ParsingUtils::expect_token(tokens, position, &Token::Dot)?;
            let method_name = ParsingUtils::expect_identifier_any(tokens, position)?;

            if method_name == "set" {
                // Allow optional legacy terminator: .set();
                ParsingUtils::expect_token(tokens, position, &Token::LeftParen)?;
                ParsingUtils::expect_token(tokens, position, &Token::RightParen)?;
                ParsingUtils::parse_optional_semicolon(tokens, position)?;
                break;
            } else {
                // Treat all other methods as rhythm parameters
                let parameter = ParsingUtils::parse_parameter_call(tokens, position)?;
                builder.set_parameter(&method_name, parameter)?;
            }
        }

        // Build the RhythmConfig and validate
        let config = builder.build();
        config.validate_voice()?;

        // Consume optional trailing semicolon if it wasn't already consumed
        ParsingUtils::parse_optional_semicolon(tokens, position)?;

        Ok(TerminalCommand::CreateSequencer { config })
    }

    fn parse_wing_command(
        tokens: &[Token],
        position: &mut usize,
        voice_id: i32,
        is_add: bool,
    ) -> Result<TerminalCommand, ParseError> {
        // Parse: voice(voice_id).addWings(count); or voice(voice_id).removeWings(count);
        let count =
            ParsingUtils::parse_parentheses_with_number(tokens, position, "wing count")? as usize;

        ParsingUtils::parse_optional_semicolon(tokens, position)?;

        if is_add {
            Ok(TerminalCommand::AddWings { voice_id, count })
        } else {
            Ok(TerminalCommand::RemoveWings { voice_id, count })
        }
    }

    fn parse_new_formation_from_voice(
        tokens: &[Token],
        position: &mut usize,
        voice_id: i32,
        formation_type: FormationType,
    ) -> Result<TerminalCommand, ParseError> {
        ParsingUtils::expect_token(tokens, position, &Token::LeftParen)?;
        ParsingUtils::expect_token(tokens, position, &Token::RightParen)?;

        let mut builder = DroneBuilder::new();
        builder.set_parameter("voice", ParameterValue::Number(voice_id as f32))?;

        Ok(TerminalCommand::NewFormation {
            voice_id,
            config: builder.build(),
            formation_type,
        })
    }

    fn parse_clear_from_voice(
        tokens: &[Token],
        position: &mut usize,
        voice_id: i32,
    ) -> Result<TerminalCommand, ParseError> {
        // Parse: voice(voice_id).clear();
        ParsingUtils::expect_token(tokens, position, &Token::LeftParen)?;
        ParsingUtils::expect_token(tokens, position, &Token::RightParen)?;
        ParsingUtils::parse_optional_semicolon(tokens, position)?;

        Ok(TerminalCommand::Clear { voice_id })
    }

    fn parse_generate_rhythm_from_voice(
        tokens: &[Token],
        position: &mut usize,
        voice_id: i32,
    ) -> Result<TerminalCommand, ParseError> {
        // Parse: voice(voice_id).generateRhythm();
        ParsingUtils::expect_token(tokens, position, &Token::LeftParen)?;
        ParsingUtils::expect_token(tokens, position, &Token::RightParen)?;
        ParsingUtils::parse_optional_semicolon(tokens, position)?;

        Ok(TerminalCommand::GenerateRhythm { voice_id })
    }

    fn parse_remove_circle_from_voice(
        tokens: &[Token],
        position: &mut usize,
        voice_id: i32,
    ) -> Result<TerminalCommand, ParseError> {
        // Parse: voice(voice_id).removeCircle(circle_id);
        let circle_id = ParsingUtils::parse_parentheses_with_number(tokens, position, "circle ID")?;
        ParsingUtils::parse_optional_semicolon(tokens, position)?;

        Ok(TerminalCommand::RemoveCircle {
            voice_id,
            circle_id,
        })
    }

    fn parse_voice_parameter_modification(
        tokens: &[Token],
        position: &mut usize,
        voice_id: i32,
        first_param: String,
        param_type: VoiceType,
    ) -> Result<TerminalCommand, ParseError> {
        // Parse: voice(voice_id).param(value).param(value)...[.set()]?;
        // Also supports chaining makeRhythm() anywhere in the chain to create a new sequencer
        let parameter_value = ParsingUtils::parse_parameter_call(tokens, position)?;

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

        let mut has_drone_params = param_type == VoiceType::Drone;
        let mut has_rhythm_params = param_type == VoiceType::Rhythm;
        let mut saw_make_rhythm = false;
        let mut saw_make_drone = false;

        // Parse additional method calls in the chain until no more dots
        while ParsingUtils::is_at_dot(tokens, *position) {
            ParsingUtils::expect_token(tokens, position, &Token::Dot)?;
            let method_name = ParsingUtils::expect_identifier_any(tokens, position)?;

            if method_name == "set" {
                // Legacy terminator - optional now
                ParsingUtils::expect_token(tokens, position, &Token::LeftParen)?;
                ParsingUtils::expect_token(tokens, position, &Token::RightParen)?;
                // Optional semicolon handled after loop
                break;
            } else if method_name == "makeRhythm" {
                // Allow patterns like voice(1).capacity(5).makeRhythm();
                ParsingUtils::expect_token(tokens, position, &Token::LeftParen)?;
                ParsingUtils::expect_token(tokens, position, &Token::RightParen)?;
                saw_make_rhythm = true;
                // Continue parsing so that parameters after makeRhythm() are also applied
            } else if method_name == "makeDrone" {
                // Allow patterns like voice(1).brightness(0.5).makeDrone();
                ParsingUtils::expect_token(tokens, position, &Token::LeftParen)?;
                ParsingUtils::expect_token(tokens, position, &Token::RightParen)?;
                saw_make_drone = true;
                // Continue parsing so that parameters after makeDrone() are also applied
            } else if method_name == "alt" {
                ParsingUtils::expect_token(tokens, position, &Token::LeftParen)?;
                ParsingUtils::expect_token(tokens, position, &Token::RightParen)?;
                drone_builder.formation_type = FormationType::DoubleCircle;
            } else {
                // Parse additional parameter
                let parameter = ParsingUtils::parse_parameter_call(tokens, position)?;

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
                            expected: "valid parameter name, makeRhythm, or makeDrone".to_string(),
                            found: method_name,
                        });
                    }
                }
            }
        }

        // Consume optional trailing semicolon (or the one after .set())
        ParsingUtils::parse_optional_semicolon(tokens, position)?;

        if saw_make_rhythm {
            // Treat the accumulated rhythm parameters (before and after makeRhythm) as
            // creation parameters for a new sequencer.
            if !has_rhythm_params {
                return Err(ParseError::UnexpectedToken {
                    expected: "at least one rhythm parameter when using makeRhythm".to_string(),
                    found: first_param,
                });
            }

            let config = rhythm_builder.build();
            config.validate_voice()?;
            Ok(TerminalCommand::CreateSequencer { config })
        } else if saw_make_drone {
            // Treat the accumulated drone parameters (before and after makeDrone) as
            // creation parameters for a new drone.
            if !has_drone_params {
                return Err(ParseError::UnexpectedToken {
                    expected: "at least one drone parameter when using makeDrone".to_string(),
                    found: first_param,
                });
            }

            Ok(TerminalCommand::CreateDrone(drone_builder.build()))
        } else {
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
                (false, false) => Err(ParseError::UnexpectedToken {
                    expected: "valid parameter name".to_string(),
                    found: first_param,
                }),
            }
        }
    }
}
