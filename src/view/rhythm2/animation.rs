//! src/view/rhythm2/animation.rs
//!
//! Animation logic for the `RhythmElement`

use std::time::{Duration, Instant};

pub(super) const INIT_ANIMATION_DURATION_SECS: f32 = 0.8;
pub(super) const CLEAR_ANIMATION_DURATION_SECS: f32 = 0.8;
pub(super) const ANIMATION_IN_FRACTION: f32 = 0.3;

pub(super) const MIN_ELEMENT_RADIUS: f32 = 40.0;
pub(super) const MAX_ELEMENT_RADIUS: f32 = 120.0;
