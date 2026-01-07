// src/view/voices.rs
//
// An enum to associate a set of force objects, emitters, etc

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

    pub fn all() -> Vec<VoiceId> {
        vec![
            VoiceId::Voice0,
            VoiceId::Voice1,
            VoiceId::Voice2,
            VoiceId::Voice3,
        ]
    }

    pub fn all_drones() -> Vec<VoiceId> {
        vec![VoiceId::Voice0, VoiceId::Voice3]
    }

    pub fn all_rhythms() -> Vec<VoiceId> {
        vec![VoiceId::Voice1, VoiceId::Voice2]
    }
}

impl std::fmt::Display for VoiceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Voice: {}", self.to_i32())
    }
}
