// system4-core/src/constants/mod.rs
// Centralized constants for system4-core

pub mod physics;

// Re-export commonly used constants
pub use physics::{
    FADE_IN_DURATION, FADE_OUT_DURATION, FEEDBACK_POSITIONS, INERTIA_COEFFICIENT,
    MAX_WIND_ANGLE_DEVIATION, PARTICLE_LIFE_SPAN, PARTICLE_MASS,
};
