//! A Double Circle is a force object consisting of two concentric WindCircles that can be controlled using a single set of parameters
//!

use super::wind_circle::{WindCircle, WindCircleParams, MAX_WIND_ANGLE_DEVIATION};

use crate::groups::VoiceId;
use nannou::prelude::*;

#[derive(Clone)]
pub struct DoubleCircle {
    pub id: usize,
    pub parent_voice: VoiceId,
    pub outer: WindCircle,
    pub inner: WindCircle,
}

impl DoubleCircle {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: usize,
        parent_voice: VoiceId,
        center: Vec2,
        outer_radius: f32,
        inner_radius: f32,
        strength: f32,
        center_bias: f32,
        noise: f32,
    ) -> Self {
        Self {
            id,
            parent_voice,
            outer: WindCircle::new(
                id,
                parent_voice,
                center,
                outer_radius,
                inner_radius,
                strength,
                center_bias,
                noise,
            ),
            inner: WindCircle::new(
                id,
                parent_voice,
                center,
                outer_radius,
                0.0,
                strength,
                center_bias,
                noise,
            ),
        }
    }
}
