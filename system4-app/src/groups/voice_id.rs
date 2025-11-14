// src/groups/voice_id.rs
//
// An enum to associate a set of force objects, emitters, etc
// This app-level VoiceId wraps the core VoiceId for domain-specific voice management

pub use system4_core::physics::VoiceId as CoreVoiceId;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum VoiceId {
    Voice0,
    Voice1,
    Voice2,
    Voice3,
    Invalid,
}

impl VoiceId {
    pub fn to_i32(&self) -> i32 {
        match self {
            VoiceId::Voice0 => 0,
            VoiceId::Voice1 => 1,
            VoiceId::Voice2 => 2,
            VoiceId::Voice3 => 3,
            VoiceId::Invalid => -1,
        }
    }

    pub fn from_i32(i: i32) -> VoiceId {
        match i {
            0 => VoiceId::Voice0,
            1 => VoiceId::Voice1,
            2 => VoiceId::Voice2,
            3 => VoiceId::Voice3,
            _ => VoiceId::Invalid,
        }
    }

    /// Convert to core VoiceId for physics operations
    pub fn to_core(&self) -> CoreVoiceId {
        CoreVoiceId(self.to_i32() as u32)
    }

    /// Create from core VoiceId
    pub fn from_core(core_id: CoreVoiceId) -> Self {
        Self::from_i32(core_id.0 as i32)
    }
}

impl std::fmt::Display for VoiceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Voice: {}", self.to_i32())
    }
}
