// src/particle/mod.rs
pub mod emitter;
pub use emitter::{EmitDirection, Emitter, LinearEmitter, PointEmitter};

pub mod particles;
pub use particles::Particle;

pub mod particle_system;
pub use particle_system::{ParticleSystem, EMPTY_GPU_BUFFER};
