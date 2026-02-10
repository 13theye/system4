//! src/view/rhythm2/animation.rs
//!
//! Animation logic for the `RhythmElement`
//!

use rand::{rngs::ThreadRng, Rng};

// Number of seconds for the initialization animation
pub(super) const INIT_ANIMATION_DURATION: f32 = 3.0;
// Number of seconds for the clear animation
pub(super) const CLEAR_ANIMATION_DURATION: f32 = 0.8;
// The fraction of the animation that is the ramp up
pub(super) const ANIMATION_IN_FRAC: f32 = 0.3;
// The factor by which the animation speed can vary
pub(super) const ANIMATION_VARIATION: f32 = 0.2;

// The minimum and maximum element radii in pixels
pub(super) const MIN_ELEMENT_RADIUS: f32 = 50.0;
pub(super) const MAX_ELEMENT_RADIUS: f32 = 120.0;

// The minimum and maximum formation radii in pixels
pub(super) const MIN_FORMATION_RADIUS: f32 = 450.0;
pub(super) const MAX_FORMATION_RADIUS: f32 = 750.0;

/// Helper function to get a animation variation factor
pub fn adjusted_duration(base_duration: f32, rng: &mut ThreadRng) -> f32 {
    let variation = rng.random_range(-ANIMATION_VARIATION..ANIMATION_VARIATION);
    base_duration * (1.0 + variation)
}
