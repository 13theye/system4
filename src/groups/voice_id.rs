// src/view/voices.rs
//
// An enum to associate a set of force objects, emitters, etc

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum VoiceId {
    Voice1,
    Voice2,
    Voice3,
    Voice4,
    Invalid,
}

impl VoiceId {
    pub fn to_i32(&self) -> i32 {
        match self {
            VoiceId::Voice1 => 1,
            VoiceId::Voice2 => 2,
            VoiceId::Voice3 => 3,
            VoiceId::Voice4 => 4,
            VoiceId::Invalid => 0,
        }
    }

    pub fn from_i32(i: i32) -> VoiceId {
        match i {
            1 => VoiceId::Voice1,
            2 => VoiceId::Voice2,
            3 => VoiceId::Voice3,
            4 => VoiceId::Voice4,
            _ => VoiceId::Invalid,
        }
    }
}

impl std::fmt::Display for VoiceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Voice: {}", self.to_i32())
    }
}
