// src/view/player.rs
//
// This module defines a terminal view

use nannou::prelude::*;

pub struct Terminal {
    pub player_id: u32,
    pub origin: Vec2,
}

impl Terminal {
    pub fn new(player_id: u32, origin: Vec2) -> Self {
        Self { player_id, origin }
    }
}
