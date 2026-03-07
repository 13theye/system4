//! A Double Circle is a force object consisting of two concentric WindCircles that can be controlled using a single set of parameters
//!

use super::{wind_circle::WindCircleParams, CircleFormation, Wind, WindCircle};

use crate::groups::VoiceId;
use nannou::prelude::*;

#[derive(Clone)]
pub struct DoubleCircle {
    pub id: usize,
    pub parent_voice: VoiceId,
    pub outer: WindCircle,
    pub inner: WindCircle,
    pub central: WindCircle,
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
        let mut checked_outer = outer_radius;
        let mut checked_inner = inner_radius;

        if outer_radius < inner_radius {
            checked_outer = inner_radius;
            checked_inner = outer_radius;
        }

        Self {
            id,
            parent_voice,
            outer: WindCircle::new(
                id,
                parent_voice,
                center,
                checked_outer,
                checked_inner,
                strength,
                center_bias,
                noise,
            ),
            inner: WindCircle::new(
                id,
                parent_voice,
                center,
                checked_inner,
                100.0,
                strength,
                center_bias,
                noise,
            ),
            central: WindCircle::new(
                id,
                parent_voice,
                center,
                100.0,
                0.0,
                strength,
                center_bias,
                1.0,
            ),
        }
    }
}

impl CircleFormation for DoubleCircle {
    fn id(&self) -> usize {
        self.id
    }

    fn parent_voice(&self) -> VoiceId {
        self.parent_voice
    }

    fn wind_for_position(&self, position: Vec2, noise_factor: f64) -> Option<Wind> {
        use super::wind_field::combined_wind_at_pos;
        let formations: Vec<&dyn CircleFormation> = vec![&self.outer, &self.inner, &self.central];
        combined_wind_at_pos(&formations, position, noise_factor)
    }

    fn rect(&self) -> Rect {
        self.outer.rect()
    }

    fn label(&self) -> &'static str {
        "DoubleCircle"
    }

    fn params(&self) -> WindCircleParams {
        WindCircleParams {
            center: self.outer.params().center,
            outer_radius: self.outer.params().outer_radius,
            inner_radius: self.outer.params().inner_radius,
            force: self.outer.params().force,
            gravity: self.inner.params().gravity,
            noise: self.outer.params().noise,
        }
    }

    fn set_center(&mut self, center: Vec2) {
        self.outer.set_center(center);
        self.inner.set_center(center);
        self.central.set_center(center);
    }

    fn set_center_x(&mut self, x: f32) {
        self.outer.set_center_x(x);
        self.inner.set_center_x(x);
        self.central.set_center_x(x);
    }

    fn set_center_y(&mut self, y: f32) {
        self.outer.set_center_y(y);
        self.inner.set_center_y(y);
        self.central.set_center_y(y);
    }

    fn set_outer_radius(&mut self, radius: f32) {
        if radius > self.outer.params().inner_radius + 20.0 {
            self.outer.set_outer_radius(radius);
        }
    }

    fn set_inner_radius(&mut self, radius: f32) {
        self.inner.set_outer_radius(radius);
        self.outer.set_inner_radius(radius);
    }

    fn set_force(&mut self, force: f32) {
        self.outer.set_force(force);
        self.inner.set_force(force);
        self.central.set_force(force * 4.0);
    }

    fn set_gravity(&mut self, gravity: f32) {
        self.inner.set_gravity(gravity);
        let offset = (gravity - 1.0).rem_euclid(2.0);
        self.outer.set_gravity(offset);
        self.central.set_gravity(-offset);
    }

    fn set_noise(&mut self, noise: f32) {
        self.outer.set_noise(noise);
        self.inner.set_noise(noise);
    }

    fn draw_center(&self, draw: &Draw, scale_x: f32, scale_y: f32) {
        self.inner.draw_center(draw, scale_x, scale_y);
    }

    fn draw(&self, draw: &Draw, scale_x: f32, scale_y: f32) {
        self.outer.draw(draw, scale_x, scale_y);
        self.inner.draw(draw, scale_x, scale_y);
        self.central.draw(draw, scale_x, scale_y);
    }
}
