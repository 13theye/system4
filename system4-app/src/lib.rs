// /src/lib.rs

// Re-export core modules
pub use system4_core::{config, utils, physics};

// App-specific extensions for core modules
pub mod physics_ext;  // Nannou ↔ Core type conversions and GPU extensions
pub mod utils_ext;   // Nannou-dependent utilities

pub mod forces;
pub mod fps;
pub mod groups;
pub mod model;
pub mod osc;
pub mod particle;
pub mod services;
pub mod terminals;
pub mod view;
