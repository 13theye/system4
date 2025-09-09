// Test script to verify makeDrone command syntax
use system4::terminals::new_terminal::NTerminal;

fn main() {
    println!("=== Testing makeDrone Command Syntax ===\n");

    // Test the new makeDrone syntax
    let test_commands = vec![
        "makeDrone.name(\"Test Drone\").brightness(0.8).begin();",
        "makeDrone.force(15.5).trail(0.2).begin();",
        "makeDrone.name(\"Bright Drone\").brightness(0.1).volume(0.5).gravity(0.7).begin();",
        "makeDrone.begin();", // Minimal command with defaults
    ];

    for command in &test_commands {
        println!("Testing: {}", command);
        match NTerminal::parse_command(command) {
            Ok(parsed_command) => {
                println!("✅ Successfully parsed:");
                println!("{}\n", parsed_command);
            }
            Err(err) => {
                println!("❌ Parse error: {}\n", err);
            }
        }
    }

    // Test error cases
    println!("=== Testing Error Cases ===\n");
    let error_commands = vec![
        "makeDrone.name(\"Test\");",      // Missing begin()
        "makeDrone.name(123).begin();",  // Wrong parameter type
        "makeDrone.brightness(0.5).build();", // Should use begin() not build()
    ];

    for command in &error_commands {
        println!("Testing error case: {}", command);
        match NTerminal::parse_command(command) {
            Ok(_) => println!("❌ Unexpected success\n"),
            Err(err) => println!("✅ Expected error: {}\n", err),
        }
    }
}