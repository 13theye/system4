// system4-core/src/physics/voice_id.rs
// Voice identification for forces and particles

use std::hash::Hash;

/// VoiceId uniquely identifies a voice (drone or rhythm)
/// This is a placeholder type that will be properly implemented when
/// the domain model is extracted to core
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VoiceId(pub u32);

impl VoiceId {
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

impl From<u32> for VoiceId {
    fn from(id: u32) -> Self {
        Self(id)
    }
}

impl From<VoiceId> for u32 {
    fn from(id: VoiceId) -> u32 {
        id.0
    }
}
