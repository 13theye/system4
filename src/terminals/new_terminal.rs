// src/terminals/new_terminal.rs
//
// NTerminal: Builder Pattern Pseudocode Parser
//
// Parses commands like: drone.name("Drone 1").brightness(0.1).volume(0.5).gravity(0.7).build();

use super::{
    commands::Command,
    parsing::{CommandParser, ParseError},
    tokens::Tokenizer,
};

pub struct NTerminal;

impl NTerminal {
    pub fn parse_command(input: &str) -> Result<Command, ParseError> {
        let mut tokenizer = Tokenizer::new(input);
        let tokens = tokenizer.tokenize()?;

        let mut parser = CommandParser::new(tokens);
        parser.parse()
    }

    /// Demo function that shows how to use the parser
    pub fn demo() {
        println!("=== NTerminal Builder Pattern Parser Demo ===\n");

        let test_commands = vec![
            // Create new drones
            "drone.new().name(\"My Drone\").brightness(0.8).build();",
            "drone.new().name(\"Drone 1\").brightness(0.1).volume(0.5).gravity(0.7).build();",
            "drone.new().force(15.5).trail(0.2).build();",
            "drone.new().build();", // Minimal command with defaults
            
            // Modify existing drones
            "drone.get(\"My Drone\").brightness(0.2).set();",
            "drone.get(\"Drone 1\").volume(0.8).set();",
        ];

        for command in test_commands {
            println!("Parsing: {}", command);
            match Self::parse_command(command) {
                Ok(cmd) => {
                    println!("✓ Successfully parsed:");
                    println!("{}\n", cmd);
                }
                Err(err) => {
                    println!("✗ Parse error: {}\n", err);
                }
            }
        }

        // Demonstrate error handling
        println!("Testing error cases:");
        let error_commands = vec![
            "drone.new().name(\"Test\");",                    // Missing build()
            "drone.new().name(123).build();",                 // Wrong parameter type
            "",                                               // Empty input
            "notdrone.new().name(\"test\").build();",         // Wrong object name
            "light.new().brightness(0.5).build();",           // Unknown command type
            "drone.create().name(\"Test\").build();",         // Wrong method (should be new)
            "drone.get(123).brightness(0.5).set();",         // Wrong parameter type for drone name
            "drone.brightness(0.5).build();",                // Missing .new() or .get()
            "drone.get(\"Test\").brightness(0.5).build();",  // Should use .set() not .build() for modify
        ];

        for command in error_commands {
            println!("Parsing: {}", command);
            match Self::parse_command(command) {
                Ok(_) => println!("✗ Unexpected success\n"),
                Err(err) => println!("✓ Expected error: {}\n", err),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{commands::Command, tokens::Token};
    use super::*;

    #[test]
    fn test_basic_tokenization() {
        let mut tokenizer = Tokenizer::new("drone.new().name(\"Test\").build();");
        let tokens = tokenizer.tokenize().unwrap();

        assert_eq!(tokens.len(), 15);
        match &tokens[0] {
            Token::Identifier(name) => assert_eq!(name, "drone"),
            _ => panic!(),
        }
        match &tokens[1] {
            Token::Dot => {}
            _ => panic!(),
        }
        match &tokens[2] {
            Token::Identifier(name) => assert_eq!(name, "new"),
            _ => panic!(),
        }
        match &tokens[3] {
            Token::LeftParen => {}
            _ => panic!(),
        }
        match &tokens[4] {
            Token::RightParen => {}
            _ => panic!(),
        }
        match &tokens[5] {
            Token::Dot => {}
            _ => panic!(),
        }
        match &tokens[6] {
            Token::Identifier(name) => assert_eq!(name, "name"),
            _ => panic!(),
        }
        match &tokens[7] {
            Token::LeftParen => {}
            _ => panic!(),
        }
        match &tokens[8] {
            Token::String(s) => assert_eq!(s, "Test"),
            _ => panic!(),
        }
        match &tokens[9] {
            Token::RightParen => {}
            _ => panic!(),
        }
        match &tokens[10] {
            Token::Dot => {}
            _ => panic!(),
        }
        match &tokens[11] {
            Token::Identifier(name) => assert_eq!(name, "build"),
            _ => panic!(),
        }
        match &tokens[12] {
            Token::LeftParen => {}
            _ => panic!(),
        }
        match &tokens[13] {
            Token::RightParen => {}
            _ => panic!(),
        }
        match &tokens[14] {
            Token::Semicolon => {}
            _ => panic!(),
        }
    }

    #[test]
    fn test_number_tokenization() {
        let mut tokenizer = Tokenizer::new("drone.new().brightness(0.5).build();");
        let tokens = tokenizer.tokenize().unwrap();

        match &tokens[8] {
            Token::Number(n) => assert_eq!(*n, 0.5),
            _ => panic!("Expected number token"),
        }
    }

    #[test]
    fn test_simple_create_parsing() {
        let result = NTerminal::parse_command("drone.new().name(\"Test Drone\").build();");
        assert!(result.is_ok());

        let command = result.unwrap();
        match command {
            Command::CreateDrone(config) => {
                assert_eq!(config.name, "Test Drone");
                assert_eq!(config.brightness, 1.0); // Default value
            }
            _ => panic!("Expected CreateDrone command"),
        }
    }

    #[test]
    fn test_full_create_parsing() {
        let result = NTerminal::parse_command(
            "drone.new().name(\"Drone 1\").brightness(0.1).volume(0.5).gravity(0.7).build();",
        );
        assert!(result.is_ok());

        let command = result.unwrap();
        match command {
            Command::CreateDrone(config) => {
                assert_eq!(config.name, "Drone 1");
                assert_eq!(config.brightness, 0.1);
                assert_eq!(config.volume, 0.5);
                assert_eq!(config.gravity, 0.7);
            }
            _ => panic!("Expected CreateDrone command"),
        }
    }

    #[test]
    fn test_modify_parsing() {
        let result = NTerminal::parse_command("drone.get(\"My Drone\").brightness(0.5).set();");
        assert!(result.is_ok());

        let command = result.unwrap();
        match command {
            Command::ModifyDrone { name, config } => {
                assert_eq!(name, "My Drone");
                assert_eq!(config.brightness, 0.5);
                assert_eq!(config.name, "Unnamed Drone"); // Should use default since not set
            }
            _ => panic!("Expected ModifyDrone command"),
        }
    }

    #[test]
    fn test_missing_build_error() {
        let result = NTerminal::parse_command("drone.new().name(\"Test\");");
        assert!(result.is_err());

        match result {
            Err(ParseError::MissingBuild) => {
                // Expected error
            }
            Err(e) => panic!("Expected MissingBuild error, got: {:?}", e),
            Ok(_) => panic!("Expected error but got success"),
        }
    }

    #[test]
    fn test_invalid_parameter_type() {
        let result = NTerminal::parse_command("drone.new().name(0.5).build();");
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_input() {
        let result = NTerminal::parse_command("");
        assert!(result.is_err());

        if let Err(ParseError::EmptyInput) = result {
            // Expected error
        } else {
            panic!("Expected EmptyInput error");
        }
    }

    #[test]
    fn test_unknown_command() {
        let result = NTerminal::parse_command("light.new().brightness(0.5).build();");
        assert!(result.is_err());

        match result {
            Err(ParseError::UnknownCommand(cmd)) => {
                assert_eq!(cmd, "light");
            }
            Err(e) => panic!("Expected UnknownCommand error, got: {:?}", e),
            Ok(_) => panic!("Expected error but got success"),
        }
    }

    #[test]
    fn test_invalid_drone_syntax() {
        // Test missing .new() or .get() - should expect "new or get"
        let result = NTerminal::parse_command("drone.brightness(0.5).build();");
        assert!(result.is_err());

        match result {
            Err(ParseError::UnexpectedToken { expected, found: _ }) => {
                assert_eq!(expected, "new or get"); // After "drone." we expect "new" or "get"
            }
            Err(e) => panic!("Expected UnexpectedToken error for missing .new() or .get(), got: {:?}", e),
            Ok(_) => panic!("Expected error but got success"),
        }
    }

    #[test]
    fn test_wrong_create_method() {
        // Test drone.create() instead of drone.new()
        let result = NTerminal::parse_command("drone.create().name(\"Test\").build();");
        assert!(result.is_err());

        match result {
            Err(ParseError::UnexpectedToken { expected, found }) => {
                assert_eq!(expected, "new or get");
                assert_eq!(found, "create");
            }
            Err(e) => panic!("Expected UnexpectedToken error for wrong method name, got: {:?}", e),
            Ok(_) => panic!("Expected error but got success"),
        }
    }

    #[test]
    fn test_invalid_drone_name_type() {
        // Test drone.get(123) instead of drone.get("name")
        let result = NTerminal::parse_command("drone.get(123).brightness(0.5).set();");
        assert!(result.is_err());

        match result {
            Err(ParseError::UnexpectedToken { expected, found: _ }) => {
                assert_eq!(expected, "drone name string");
            }
            Err(e) => panic!("Expected UnexpectedToken error for invalid drone name type, got: {:?}", e),
            Ok(_) => panic!("Expected error but got success"),
        }
    }

    #[test]
    fn test_missing_set_error() {
        // Test drone.get("name").brightness(0.5) without .set()
        let result = NTerminal::parse_command("drone.get(\"Test\").brightness(0.5);");
        assert!(result.is_err());

        match result {
            Err(ParseError::MissingSet) => {
                // Expected error
            }
            Err(e) => panic!("Expected MissingSet error, got: {:?}", e),
            Ok(_) => panic!("Expected error but got success"),
        }
    }

    #[test]
    fn test_wrong_terminator_for_modify() {
        // Test drone.get("name").brightness(0.5).build() - should use .set()
        // Currently this produces a parameter parsing error, but still catches the mistake
        let result = NTerminal::parse_command("drone.get(\"Test\").brightness(0.5).build();");
        assert!(result.is_err());

        match result {
            Err(ParseError::UnexpectedToken { expected, found: _ }) => {
                assert_eq!(expected, "string or number"); // Parser expects parameter for build()
            }
            Err(e) => panic!("Expected UnexpectedToken error for wrong terminator, got: {:?}", e),
            Ok(_) => panic!("Expected error but got success"),
        }
    }

    #[test]
    fn test_wrong_terminator_for_create() {
        // Test drone.new().brightness(0.5).set() - should use .build()
        // Currently this produces a parameter parsing error, but still catches the mistake  
        let result = NTerminal::parse_command("drone.new().brightness(0.5).set();");
        assert!(result.is_err());

        match result {
            Err(ParseError::UnexpectedToken { expected, found: _ }) => {
                assert_eq!(expected, "string or number"); // Parser expects parameter for set()
            }
            Err(e) => panic!("Expected UnexpectedToken error for wrong terminator, got: {:?}", e),
            Ok(_) => panic!("Expected error but got success"),
        }
    }
}
