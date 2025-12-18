// src/terminals/parsing/rhythm_parser.rs

use super::{errors::ParseError, parameter::ParameterValue, utils::ParsingUtils};
use crate::terminals::{
    commands::{rhythm::RhythmBuilder, TerminalCommand, TerminalCommandBuilder},
    tokens::Token,
};

pub struct RhythmParser;

impl RhythmParser {
    pub fn parse_rhythm_command(
        tokens: &[Token],
        position: &mut usize,
    ) -> Result<TerminalCommand, ParseError> {
        // Parse: rhythm(voice_id).method().method();
        // .set() is now optional and may be omitted.
        let voice_id = ParsingUtils::parse_parentheses_with_number(tokens, position, "voice ID")?;

        let mut builder = RhythmBuilder::new();
        builder.set_parameter("voice", ParameterValue::Number(voice_id as f32))?;

        // Parse method chain, allowing optional trailing .set()
        Self::parse_method_chain(tokens, position, &mut builder)?;

        // Consume optional trailing semicolon (or the one after .set())
        ParsingUtils::parse_optional_semicolon(tokens, position)?;

        Ok(TerminalCommand::ModifyRhythmParams {
            voice_id,
            config: builder.build(),
        })
    }

    /// Parse method chain, allowing optional .set() terminator
    fn parse_method_chain(
        tokens: &[Token],
        position: &mut usize,
        builder: &mut RhythmBuilder,
    ) -> Result<(), ParseError> {
        while ParsingUtils::is_at_dot(tokens, *position) {
            ParsingUtils::expect_token(tokens, position, &Token::Dot)?;
            let method_name = ParsingUtils::expect_identifier_any(tokens, position)?;

            if method_name == "set" {
                // Legacy terminator - optional now
                ParsingUtils::expect_token(tokens, position, &Token::LeftParen)?;
                ParsingUtils::expect_token(tokens, position, &Token::RightParen)?;
                break;
            } else {
                // Parse method call with parameter
                let parameter = ParsingUtils::parse_parameter_call(tokens, position)?;
                builder.set_parameter(&method_name, parameter)?;
            }
        }

        Ok(())
    }
}
