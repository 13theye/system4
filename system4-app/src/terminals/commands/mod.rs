// src/terminals/commands/mod.rs
//
// Command types with app-specific extensions

pub mod drone;
pub mod rhythm;

// Re-export main command types from core
pub use system4_core::commands::{TerminalCommand, TerminalCommandBuilder};

// Note: drone and rhythm modules provide core types with app-specific extension traits
