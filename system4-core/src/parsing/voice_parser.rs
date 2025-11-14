// system4-core/src/parsing/voice_parser.rs
// Voice command parser

use super::{
    categorize_parameter, ParameterCategory, ParameterValue, ParseError, ParsingUtils, Token,
    VoiceType,
};
use crate::commands::{DroneBuilder, RhythmBuilder, TerminalCommand, TerminalCommandBuilder};

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
        "makeDrone" => parse_make_drone_from_voice(tokens, position, voice_id),
        "makeRhythm" => parse_make_rhythm_from_voice(tokens, position, voice_id),
        "addWings" => parse_wing_command(tokens, position, voice_id, true),
        "removeWings" => parse_wing_command(tokens, position, voice_id, false),
        "newCircle" => parse_new_circle_from_voice(tokens, position, voice_id),
        "removeCircle" => parse_remove_circle_from_voice(tokens, position, voice_id),
        "clear" => parse_clear_from_voice(tokens, position, voice_id),
        _ => {
            // Check if this is a parameter modification command
            match categorize_parameter(&sub_command) {
                Some(ParameterCategory::Drone) => parse_voice_parameter_modification(
                    tokens,
                    position,
                    voice_id,
                    sub_command,
                    VoiceType::Drone,
                ),
                Some(ParameterCategory::Rhythm) => parse_voice_parameter_modification(
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
    // Parse: voice(voice_id).makeDrone().method().begin();
    ParsingUtils::expect_token(tokens, position, &Token::LeftParen)?;
    ParsingUtils::expect_token(tokens, position, &Token::RightParen)?;

    let mut builder = DroneBuilder::new();
    builder.set_parameter("voice", ParameterValue::Number(voice_id as f32))?;

    // Parse method chain until .begin()
    parse_method_chain_until_terminator(tokens, position, &mut builder, "begin")?;

    Ok(TerminalCommand::CreateDrone(builder.build()))
}

fn parse_make_rhythm_from_voice(
    tokens: &[Token],
    position: &mut usize,
    voice_id: i32,
) -> Result<TerminalCommand, ParseError> {
    // Parse: voice(voice_id).makeRhythm().method().begin();
    ParsingUtils::expect_token(tokens, position, &Token::LeftParen)?;
    ParsingUtils::expect_token(tokens, position, &Token::RightParen)?;

    let mut builder = RhythmBuilder::new();
    builder.set_parameter("voice", ParameterValue::Number(voice_id as f32))?;

    // Parse method chain until .begin()
    parse_method_chain_until_terminator(tokens, position, &mut builder, "begin")?;

    Ok(TerminalCommand::CreateSequencer {
        config: builder.build(),
    })
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

fn parse_new_circle_from_voice(
    tokens: &[Token],
    position: &mut usize,
    voice_id: i32,
) -> Result<TerminalCommand, ParseError> {
    // Parse: voice(voice_id).newCircle().parameters().add();
    ParsingUtils::expect_token(tokens, position, &Token::LeftParen)?;
    ParsingUtils::expect_token(tokens, position, &Token::RightParen)?;

    let mut builder = DroneBuilder::new();
    builder.set_parameter("voice", ParameterValue::Number(voice_id as f32))?;

    // Parse method chain until .add()
    parse_method_chain_until_terminator(tokens, position, &mut builder, "add")?;

    Ok(TerminalCommand::NewCircle {
        voice_id,
        config: builder.build(),
    })
}

fn parse_clear_from_voice(
    tokens: &[Token],
    position: &mut usize,
    voice_id: i32,
) -> Result<TerminalCommand, ParseError> {
    // Parse: voice(voice_id).clearRhythm();
    ParsingUtils::expect_token(tokens, position, &Token::LeftParen)?;
    ParsingUtils::expect_token(tokens, position, &Token::RightParen)?;
    ParsingUtils::parse_optional_semicolon(tokens, position)?;

    Ok(TerminalCommand::Clear { voice_id })
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
    // Parse: voice(voice_id).parameter(value).set();
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

    // Parse additional method calls in the chain until .set()
    while ParsingUtils::is_at_dot(tokens, *position) {
        ParsingUtils::expect_token(tokens, position, &Token::Dot)?;
        let method_name = ParsingUtils::expect_identifier_any(tokens, position)?;

        if method_name == "set" {
            ParsingUtils::expect_token(tokens, position, &Token::LeftParen)?;
            ParsingUtils::expect_token(tokens, position, &Token::RightParen)?;
            ParsingUtils::parse_optional_semicolon(tokens, position)?;
            break;
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
                        expected: "valid parameter name".to_string(),
                        found: method_name,
                    });
                }
            }
        }
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
        (false, false) => Err(ParseError::UnexpectedToken {
            expected: "valid parameter name".to_string(),
            found: first_param,
        }),
    }
}

/// Generic method chain parser that works with any builder until a terminator
fn parse_method_chain_until_terminator<T: TerminalCommandBuilder>(
    tokens: &[Token],
    position: &mut usize,
    builder: &mut T,
    terminator: &str,
) -> Result<(), ParseError> {
    while ParsingUtils::is_at_dot(tokens, *position) {
        ParsingUtils::expect_token(tokens, position, &Token::Dot)?;
        let method_name = ParsingUtils::expect_identifier_any(tokens, position)?;

        if method_name == terminator {
            ParsingUtils::expect_token(tokens, position, &Token::LeftParen)?;
            ParsingUtils::expect_token(tokens, position, &Token::RightParen)?;
            ParsingUtils::parse_optional_semicolon(tokens, position)?;
            return Ok(());
        } else {
            // Parse method call with parameter
            let parameter = ParsingUtils::parse_parameter_call(tokens, position)?;
            builder.set_parameter(&method_name, parameter)?;
        }
    }

    match terminator {
        "begin" => Err(ParseError::MissingBegin),
        "add" => Err(ParseError::MissingAdd),
        _ => Err(ParseError::UnexpectedToken {
            expected: format!(".{}()", terminator),
            found: "end of input".to_string(),
        }),
    }
}
