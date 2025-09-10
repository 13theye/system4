// src/terminals/new_terminal.rs
//
// NTerminal: Builder Pattern Pseudocode Parser
//
// Parses commands like:
//   makeDrone(1).brightness(0.1).volume(0.5).gravity(0.7).begin();
//   drone(1).brightness(0.1).volume(0.5).gravity(0.7).set();

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
            // Create new drones with makeDrone
            "makeDrone(1).brightness(0.8).outerRadius(600.0).begin();",
            "makeDrone(4).brightness(0.1).volume(0.5).gravity(0.7).noise(0.2).begin();",
            "makeDrone().force(15.5).feedback(0.8).innerRadius(150.0).begin();",
            "makeDrone().begin();", // Minimal command with defaults
            // Modify existing drones
            "drone(1).brightness(0.2).centerX(200.0).centerY(-100.0).set();",
            "drone(4).volume(0.8).vibration(0.3).set();",
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
            "makeDrone(1);",                          // Missing begin()
            "makeDrone(\"test\").begin();",           // Wrong parameter type (should be number)
            "",                                       // Empty input
            "notdrone(1).begin();",                   // Wrong object name
            "light().brightness(0.5).begin();",       // Unknown command type
            "drone(\"test\").brightness(0.5).set();", // Wrong parameter type for voice ID
            "drone(1).brightness(0.5).begin();",      // Should use .set() not .begin() for modify
            "makeDrone(1).brightness(0.5).set();",    // Should use .begin() not .set() for create
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
