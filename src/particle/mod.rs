// src/particle/mod.rs
pub mod constants;
pub use constants::{EMPTY_GPU_PARTICLE_BUFFER, EMPTY_GPU_SEGMENT_BUFFER};

pub mod emitter;

pub mod particles;
pub use particles::{to_segment_gpu, ParticleCore, ParticleFeedback};

pub mod particle_system;
pub use particle_system::ParticleSystem;
