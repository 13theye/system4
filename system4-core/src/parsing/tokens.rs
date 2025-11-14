// system4-core/src/parsing/tokens.rs
// Token types for the System4 command language

use super::errors::ParseError;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Identifier(String),
    Dot,
    LeftParen,
    RightParen,
    String(String),
    Number(f32),
    Semicolon,
}

/// Tokenizer for System4 command language
pub struct Tokenizer {
    input: Vec<char>,
    position: usize,
}

impl Tokenizer {
    pub fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            position: 0,
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, ParseError> {
        let mut tokens = Vec::new();

        while self.position < self.input.len() {
            self.skip_whitespace();

            if self.position >= self.input.len() {
                break;
            }

            let token = self.next_token()?;
            tokens.push(token);
        }

        Ok(tokens)
    }

    fn skip_whitespace(&mut self) {
        while self.position < self.input.len() && self.current_char().is_whitespace() {
            self.position += 1;
        }
    }

    fn current_char(&self) -> char {
        self.input[self.position]
    }

    fn next_token(&mut self) -> Result<Token, ParseError> {
        let ch = self.current_char();

        match ch {
            '.' => {
                self.position += 1;
                Ok(Token::Dot)
            }
            '(' => {
                self.position += 1;
                Ok(Token::LeftParen)
            }
            ')' => {
                self.position += 1;
                Ok(Token::RightParen)
            }
            ';' => {
                self.position += 1;
                Ok(Token::Semicolon)
            }
            '"' => self.read_string(),
            '+' => {
                // Always treat + as a symbol (identifier)
                Ok(Token::Identifier(self.read_symbols()))
            }
            '-' => {
                // Lookahead: if followed by digit, it's a negative number; otherwise it's a symbol
                if self.position + 1 < self.input.len()
                    && self.input[self.position + 1].is_ascii_digit()
                {
                    self.read_number()
                } else {
                    Ok(Token::Identifier(self.read_symbols()))
                }
            }
            c if c.is_ascii_digit() => self.read_number(),
            c if c.is_ascii_alphabetic() || c == '_' => {
                Ok(Token::Identifier(self.read_identifier()))
            }
            _ => Err(ParseError::UnexpectedToken {
                expected: "identifier, number, string, or operator".to_string(),
                found: ch.to_string(),
            }),
        }
    }

    fn read_string(&mut self) -> Result<Token, ParseError> {
        self.position += 1; // Skip opening quote
        let start = self.position;

        while self.position < self.input.len() && self.current_char() != '"' {
            self.position += 1;
        }

        if self.position >= self.input.len() {
            return Err(ParseError::UnterminatedString);
        }

        let string_content: String = self.input[start..self.position].iter().collect();
        self.position += 1; // Skip closing quote

        Ok(Token::String(string_content))
    }

    fn read_number(&mut self) -> Result<Token, ParseError> {
        let start = self.position;

        if self.current_char() == '-' {
            self.position += 1;
        }

        while self.position < self.input.len()
            && (self.current_char().is_ascii_digit() || self.current_char() == '.')
        {
            self.position += 1;
        }

        let number_str: String = self.input[start..self.position].iter().collect();
        let number = number_str
            .parse::<f32>()
            .map_err(|_| ParseError::InvalidNumber(number_str))?;

        Ok(Token::Number(number))
    }

    fn read_identifier(&mut self) -> String {
        let start = self.position;

        while self.position < self.input.len()
            && (self.current_char().is_ascii_alphanumeric() || self.current_char() == '_')
        {
            self.position += 1;
        }

        self.input[start..self.position].iter().collect()
    }

    fn read_symbols(&mut self) -> String {
        let start = self.position;

        while self.position < self.input.len() {
            let ch = self.current_char();
            if ch == '+' || ch == '-' {
                self.position += 1;
            } else {
                break;
            }
        }

        self.input[start..self.position].iter().collect()
    }
}
