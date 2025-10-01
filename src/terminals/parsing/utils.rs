// src/terminals/parsing/utils.rs

use super::errors::ParseError;
use super::parameter::ParameterValue;
use crate::terminals::tokens::Token;

/// Common utilities for parsing operations
pub struct ParsingUtils;

impl ParsingUtils {
    /// Parse parentheses with a number inside: (123)
    pub fn parse_parentheses_with_number(
        tokens: &[Token],
        position: &mut usize,
        context: &str,
    ) -> Result<i32, ParseError> {
        Self::expect_token(tokens, position, &Token::LeftParen)?;

        let number = match tokens.get(*position) {
            Some(Token::Number(n)) => {
                let id = *n as i32;
                *position += 1;
                id
            }
            Some(token) => {
                return Err(ParseError::UnexpectedToken {
                    expected: format!("{} number", context),
                    found: format!("{:?}", token),
                })
            }
            None => return Err(ParseError::UnexpectedEnd),
        };

        Self::expect_token(tokens, position, &Token::RightParen)?;
        Ok(number)
    }

    /// Parse a method call with parameter: method(param)
    pub fn parse_parameter_call(
        tokens: &[Token],
        position: &mut usize,
    ) -> Result<ParameterValue, ParseError> {
        Self::expect_token(tokens, position, &Token::LeftParen)?;
        let parameter = Self::parse_parameter(tokens, position)?;
        Self::expect_token(tokens, position, &Token::RightParen)?;
        Ok(parameter)
    }

    /// Parse a parameter value (string, identifier, or number)
    pub fn parse_parameter(
        tokens: &[Token],
        position: &mut usize,
    ) -> Result<ParameterValue, ParseError> {
        match tokens.get(*position) {
            Some(Token::String(s)) => {
                let value = s.clone();
                *position += 1;
                Ok(ParameterValue::String(value))
            }
            Some(Token::Identifier(id)) => {
                let value = id.clone();
                *position += 1;
                Ok(ParameterValue::String(value))
            }
            Some(Token::Number(n)) => {
                let value = *n;
                *position += 1;
                Ok(ParameterValue::Number(value))
            }
            Some(token) => Err(ParseError::UnexpectedToken {
                expected: "string, identifier, or number".to_string(),
                found: format!("{:?}", token),
            }),
            None => Err(ParseError::UnexpectedEnd),
        }
    }

    /// Parse terminator with semicolon: .begin(); or .set(); or .add();
    pub fn parse_terminator(
        tokens: &[Token],
        position: &mut usize,
        expected_terminator: &str,
    ) -> Result<(), ParseError> {
        Self::expect_token(tokens, position, &Token::Dot)?;
        let terminator = Self::expect_identifier_any(tokens, position)?;

        if terminator != expected_terminator {
            return Err(ParseError::UnexpectedToken {
                expected: expected_terminator.to_string(),
                found: terminator,
            });
        }

        Self::expect_token(tokens, position, &Token::LeftParen)?;
        Self::expect_token(tokens, position, &Token::RightParen)?;

        // Expect semicolon at the end if there are more tokens
        if *position < tokens.len() {
            Self::expect_token(tokens, position, &Token::Semicolon)?;
        }

        Ok(())
    }

    /// Parse optional semicolon at end
    pub fn parse_optional_semicolon(tokens: &[Token], position: &mut usize) -> Result<(), ParseError> {
        if *position < tokens.len() {
            Self::expect_token(tokens, position, &Token::Semicolon)?;
        }
        Ok(())
    }

    /// Expect a specific token
    pub fn expect_token(
        tokens: &[Token],
        position: &mut usize,
        expected: &Token,
    ) -> Result<(), ParseError> {
        if let Some(token) = tokens.get(*position) {
            if std::mem::discriminant(token) == std::mem::discriminant(expected) {
                *position += 1;
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

    /// Expect any identifier and return its value
    pub fn expect_identifier_any(
        tokens: &[Token],
        position: &mut usize,
    ) -> Result<String, ParseError> {
        if let Some(Token::Identifier(name)) = tokens.get(*position) {
            let name = name.clone();
            *position += 1;
            Ok(name)
        } else {
            Err(ParseError::UnexpectedToken {
                expected: "identifier".to_string(),
                found: tokens
                    .get(*position)
                    .map(|t| format!("{:?}", t))
                    .unwrap_or("EOF".to_string()),
            })
        }
    }

    /// Check if we're at a dot (method chain continues)
    pub fn is_at_dot(tokens: &[Token], position: usize) -> bool {
        matches!(tokens.get(position), Some(Token::Dot))
    }

    /// Check if position is at end of tokens
    pub fn is_at_end(tokens: &[Token], position: usize) -> bool {
        position >= tokens.len()
    }
}