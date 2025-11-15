// system4-core/src/physics/particles/mod.rs
// Particle data structures and simulation

pub mod emitter;
pub mod particle_system;
pub mod particles;

pub use emitter::{EmitDirection, Emitter, FullScreenRandomEmitter, LinearEmitter, PointEmitter};
pub use particle_system::{ParticleSystemCore, VoiceParams};
pub use particles::{ParticleCore, ParticleFeedback};

// Re-export FEEDBACK_POSITIONS from constants for convenience
pub use crate::constants::FEEDBACK_POSITIONS;
