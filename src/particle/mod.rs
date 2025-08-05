// src/particle/mod.rs
pub mod emitter;
pub use emitter::{EmitDirection, Emitter, LinearEmitter, PointEmitter};

pub mod particles_new;
pub use particles_new::Particles;

pub mod particle_system_new;
pub use particle_system_new::ParticleSystemNew;
