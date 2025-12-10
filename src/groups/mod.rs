// src/groups/mod.rs

pub mod voice_id;
pub use voice_id::VoiceId;

pub mod voice;
pub use voice::{Voice, VoiceParams};

pub mod rhythm;
pub use rhythm::Rhythm;

pub mod rhythm_types;
pub use rhythm_types::{RhythmParams, RhythmSlotParams};
