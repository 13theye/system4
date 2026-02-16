//! src/view/rhythm/animation.rs
//!
//! Animation logic for the `RhythmElement`
//!

use nannou::prelude::*;
use rand::{rngs::ThreadRng, Rng};
use std::time::Instant;

use crate::utils::tween;

// Initialization animation duration in secs
pub(super) const INIT_ANIMATION_DURATION: f32 = 3.0;
// Clear animation duration in secs
pub(super) const CLEAR_ANIMATION_DURATION: f32 = 0.8;
// Wings reinit animation duration in secs
pub(super) const WINGS_REINIT_ANIMATION_DURATION: f32 = 1.3;
// The factor by which the animation speed can vary
pub(super) const ANIMATION_VARIATION: f32 = 0.2;

// The percentage of a Activation rotation that a color change lasts
pub(super) const COLOR_CHANGE_FRACTION: f32 = 1.0;
// How much to stretch the gradient ellipse: >1.0 = more elliptical
pub(super) const COLOR_GRADIENT_ELLIPSE_RATIO: f32 = 1.2;
pub(super) const COLOR_DWELL: f32 = 0.6;

// The minimum and maximum element radii in pixels
pub(super) const MIN_ELEMENT_RADIUS: f32 = 50.0;
pub(super) const MAX_ELEMENT_RADIUS: f32 = 100.0;

// The minimum and maximum formation radii in pixels
pub(super) const MIN_FORMATION_RADIUS: f32 = 450.0;
pub(super) const MAX_FORMATION_RADIUS: f32 = 750.0;

pub(super) const LIQUID_BASE_DISTANCE: f32 = 20.0;
pub(super) const LIQUID_DISTANCE_VARIATION: f32 = 0.2;
pub(super) const LIQUID_BASE_DURATION: f32 = 3.0;
pub(super) const LIQUID_DURATION_VARIATION: f32 = 0.3;

/// Helper function to get a animation variation factor
pub fn adjusted_duration(base_duration: f32, rng: &mut ThreadRng) -> f32 {
    let variation = rng.random_range(-ANIMATION_VARIATION..ANIMATION_VARIATION);
    base_duration * (1.0 + variation)
}

/// Animation appearing as a wandering motion
#[derive(Copy, Clone, Debug)]
pub struct LiquidMotion {
    pub start_time: Instant,
    // The offset where the current motion started
    pub start_offset: Vec2,
    pub duration: f32,
    // The distance of the motion target from the Element position
    pub target_distance: f32,
    // The angle of the motion target from the Element position
    pub angle: f32,
}

impl LiquidMotion {
    pub fn new(now: Instant) -> Self {
        let (duration, target_distance, angle) = Self::generate_parameters();

        Self {
            start_time: now,
            start_offset: vec2(0.0, 0.0),
            duration,
            target_distance,
            angle,
        }
    }

    fn generate_parameters() -> (f32, f32, f32) {
        let mut rng = ThreadRng::default();

        let duration = LIQUID_BASE_DURATION
            + rng.random_range(
                -LIQUID_DURATION_VARIATION * LIQUID_BASE_DURATION
                    ..LIQUID_DURATION_VARIATION * LIQUID_BASE_DURATION,
            );
        let target_distance = LIQUID_BASE_DISTANCE
            + rng.random_range(
                -LIQUID_DISTANCE_VARIATION * LIQUID_BASE_DISTANCE
                    ..LIQUID_DISTANCE_VARIATION * LIQUID_BASE_DISTANCE,
            );
        let angle = rng.random_range(0.0..std::f32::consts::TAU);

        (duration, target_distance, angle)
    }

    fn target_offset(&self) -> Vec2 {
        vec2(
            self.angle.cos() * self.target_distance,
            self.angle.sin() * self.target_distance,
        )
    }

    /// Returns the current offset and whether the motion is complete
    pub fn current_offset(&self, now: Instant) -> (Vec2, bool) {
        let t = now.duration_since(self.start_time).as_secs_f32();

        if t >= self.duration {
            (self.target_offset(), true)
        } else {
            use nannou::ease::sine::*;
            let f = ease_out::<f32>;
            let offset =
                tween::ease_vec2(f, t, self.start_offset, self.target_offset(), self.duration);
            (offset, false)
        }
    }

    /// Renew the motion with new random parameters, starting from the previous end position
    pub fn renew(&mut self, now: Instant) {
        let final_offset = self.target_offset();
        let (duration, target_distance, angle) = Self::generate_parameters();

        self.start_time = now;
        self.start_offset = final_offset;
        self.duration = duration;
        self.target_distance = target_distance;
        self.angle = angle;
    }
}
