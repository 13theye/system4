//! The Voice enum is a wrapper for either a Drone or a Rhythm instance

use crate::groups::{Drone, Rhythm};

pub enum Voice {
    Drone(Drone),
    Rhythm(Rhythm),
}

impl Voice {
    pub fn new_from_drone(drone: Drone) -> Self {
        Voice::Drone(drone)
    }

    pub fn new_from_rhythm(rhythm: Rhythm) -> Self {
        Voice::Rhythm(rhythm)
    }

    /// Returns `true` if the `Voice` is a `Drone`
    pub fn is_drone(&self) -> bool {
        matches!(self, Voice::Drone(_))
    }

    /// Returns the Drone if the `Voice` is a `Drone``
    pub fn as_drone(&self) -> Option<&Drone> {
        match self {
            Voice::Drone(drone) => Some(drone),
            _ => None,
        }
    }

    /// Returns a mutable reference to the `Drone`` if the `Voice`` is a `Drone`
    pub fn as_drone_mut(&mut self) -> Option<&mut Drone> {
        match self {
            Voice::Drone(drone) => Some(drone),
            _ => None,
        }
    }

    /// Returns `true` if the `Voice` is a `Rhythm`
    pub fn is_rhythm(&self) -> bool {
        matches!(self, Voice::Rhythm(_))
    }

    /// Returns the `Rhythm` if the `Voice` is that type
    pub fn as_rhythm(&self) -> Option<&Rhythm> {
        match self {
            Voice::Rhythm(rhythm) => Some(rhythm),
            _ => None,
        }
    }

    /// Returns a mutable reference to the `Rhythm` if the Voices if that type
    pub fn as_rhythm_mut(&mut self) -> Option<&mut Rhythm> {
        match self {
            Voice::Rhythm(rhythm) => Some(rhythm),
            _ => None,
        }
    }
}
