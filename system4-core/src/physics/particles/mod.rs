// system4-core/src/physics/particles/mod.rs
// Particle data structures and simulation

pub mod particles;
pub use particles::{ParticleCore, ParticleFeedback};

// Re-export FEEDBACK_POSITIONS from constants for convenience
pub use crate::constants::FEEDBACK_POSITIONS;
