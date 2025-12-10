// src/terminals/terminal.rs
//
// Terminal: Builder Pattern Pseudocode Parser
//
// Parses commands like:
//   makeDrone(1).brightness(0.1).volume(0.5).gravity(0.7);
//   drone(1).brightness(0.1).volume(0.5).gravity(0.7).set();

use super::{
    commands::TerminalCommand,
    parsing::{CommandParser, ParseError},
    tokens::Tokenizer,
};

pub struct Terminal {}

impl Terminal {
    pub fn parse_command(input: &str) -> Result<TerminalCommand, ParseError> {
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
            "voice(0).makeDrone().brightness(0.8).outerRadius(600.0);",
            "voice(3).brightness(0.1).volume(0.5).gravity(0.7).noise(0.2).set();",
            "voice(0).makeDrone().force(15.5).feedback(0.8).innerRadius(150.0);",
            "voice(0).makeDrone();", // Minimal command with defaults
            // Modify existing drones
            "voice(0).brightness(0.2).centerX(200.0).centerY(-100.0).set();",
            "voice(3).volume(0.8).vibration(0.3).set();",
            // List WindCircles for a drone
            "voice(0).listCircles();",
            "voice(3).listCircles();",
            // Add new WindCircles to existing drones
            "voice(0).newCircle().gravity(1.0).force(25.0);",
            "voice(3).newCircle().centerX(-500.0).centerY(200.0);",
            "voice(0).newCircle();", // All defaults
            // Modify specific circles
            "voice(0).circle(1).centerX(-500.0).set();",
            "voice(0).circle(0).gravity(2.0).force(15.0).set();",
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
            "makeDrone(\"test\");",                   // Wrong parameter type (should be number)
            "",                                       // Empty input
            "notdrone(1);",                           // Wrong object name
            "light().brightness(0.5);",               // Unknown command type
            "drone(\"test\").brightness(0.5).set();", // Wrong parameter type for voice ID
            "drone(1).brightness(0.5);",              // Should use .set() for modify
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
