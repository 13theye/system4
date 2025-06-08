// src/view/voices.rs
//
// An enum to associate a set of force objects, emitters, etc

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum Voice {
    Voice1,
    Voice2,
    Voice3,
    Voice4,
    Invalid,
}

impl Voice {
    pub fn to_i32(&self) -> i32 {
        match self {
            Voice::Voice1 => 1,
            Voice::Voice2 => 2,
            Voice::Voice3 => 3,
            Voice::Voice4 => 4,
            Voice::Invalid => 0,
        }
    }

    pub fn from_i32(i: i32) -> Voice {
        match i {
            1 => Voice::Voice1,
            2 => Voice::Voice2,
            3 => Voice::Voice3,
            4 => Voice::Voice4,
            _ => Voice::Invalid,
        }
    }
}

impl std::fmt::Display for Voice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Voice: {}", self.to_i32())
    }
}
