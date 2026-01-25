//! src/forces2/wind_circle.rs
//!
//! WindCircle implementation for Forces v2

use crate::{forces::wind::Wind, groups::VoiceId};
use nannou::prelude::*;

/// Maximum wind angle deviation in radians (90 degrees)
const MAX_WIND_ANGLE_DEVIATION: f32 = std::f32::consts::PI;

/// A circular wind force that affects particles within a donut-shaped region
#[derive(Clone)]
pub struct WindCircle {
    pub id: usize,
    pub parent_voice: VoiceId,
    params: WindCircleParams, // Params of the circle
}

#[allow(clippy::too_many_arguments)]
impl WindCircle {
    pub fn new(
        id: usize,
        parent_voice: VoiceId,
        center: Vec2,
        radius: f32,
        width: f32,
        strength: f32,
        center_bias: f32,
        noise: f32,
    ) -> Self {
        let config = WindCircleParams {
            center,
            outer_radius: radius,
            inner_radius: width,
            force: strength,
            gravity: center_bias,
            noise,
        };
        Self {
            id,
            parent_voice,
            params: config,
        }
    }

    /// Calculate the Wind force for a given location, if any
    pub fn wind_for_position(&self, position: Vec2, noise_factor: f64) -> Option<Wind> {
        let params = &self.params;
        let distance_to_center = (position - params.center).length();
        let inner_radius = params.inner_radius;
        let outer_radius = params.outer_radius;

        if distance_to_center >= inner_radius && distance_to_center <= outer_radius {
            // Wind generation logic specific to circular fields
            let radius_vector = position - params.center;
            let radius_dir = radius_vector.normalize();
            let tangent_dir = vec2(radius_dir.y, -radius_dir.x); // tangential, 90 deg CCW from radial

            // Rotate the tangent vector by bias * 90 degrees
            let angle = params.gravity * -std::f32::consts::FRAC_PI_2; // PI/2 = 90 deg
            let angle_offset = params.noise * noise_factor as f32 * MAX_WIND_ANGLE_DEVIATION;
            let angle = angle + angle_offset;

            let sin_a = angle.sin();
            let cos_a = angle.cos();

            // Rotate tangent_dir by 'angle'
            let blended_direction = vec2(
                tangent_dir.x * cos_a - tangent_dir.y * sin_a,
                tangent_dir.x * sin_a + tangent_dir.y * cos_a,
            );

            Some(Wind::new_with(blended_direction, params.force))
        } else {
            None
        }
    }

    /// Returns a bounding Rect in screen coordinates that encompasses the entire WindCircle
    pub fn rect(&self) -> Rect {
        let center = self.params.center;
        let outer_radius = self.params.outer_radius;

        Rect::from_x_y_w_h(center.x, center.y, outer_radius * 2.0, outer_radius * 2.0)
    }

    /******************* Methods to change circle properties *******************/
    // These functions require scale parameters because they are meant to be drawn in the performer window.

    /// Return a reference to the WindCircleParams
    pub fn params(&self) -> &WindCircleParams {
        &self.params
    }

    /// Return a mutable reference to the WindCircleParams
    pub fn params_mut(&mut self) -> &mut WindCircleParams {
        &mut self.params
    }

    /// Draw the center of the WindCircle
    pub fn draw_center(&self, draw: &Draw, scale_x: f32, scale_y: f32) {
        let center = self.params.center * vec2(scale_x, scale_y);
        draw.ellipse()
            .xy(center)
            .w_h(40.0 * scale_x, 40.0 * scale_y)
            .color(rgba(1.0, 0.2, 0.0, 0.2));
    }

    /// Draw the WindCircle with outer and inner radius circles
    pub fn draw(&self, draw: &Draw, scale_x: f32, scale_y: f32) {
        let center = self.params.center * vec2(scale_x, scale_y);
        let outer_radius = self.params.outer_radius;
        let inner_radius = self.params.inner_radius;

        // Draw outer radius circle
        draw.ellipse()
            .xy(center)
            .w_h(outer_radius * scale_x * 2.0, outer_radius * scale_y * 2.0)
            .stroke_color(rgba(0.8, 0.4, 0.0, 0.6))
            .stroke_weight(2.0)
            .no_fill();

        // Draw inner radius circle
        draw.ellipse()
            .xy(center)
            .w_h(inner_radius * scale_x * 2.0, inner_radius * scale_y * 2.0)
            .stroke_color(rgba(0.8, 0.4, 0.0, 0.4))
            .stroke_weight(1.0)
            .no_fill();
    }
}

/// Parameters for a WindCircle.
/// - Center: The centerpoint of the circle in ParticleSystem space
/// - Outer radius: The outer radius of the circle
/// - Inner radius: The radius of the hole in the center of the circle
/// - Force: The strength of the wind applied within the circle
/// - Gravity: 0.0 is tangential, 1.0 is radial inward, 2.0 is tangential in the opposite direction
/// - Noise: Amount of random angle variation (0.0-1.0, where 1.0 = �90� deviation)
#[derive(Clone, Debug)]
pub struct WindCircleParams {
    /// center of the circle in the ParticleSystem space
    pub center: Vec2,
    /// outer circle radius
    pub outer_radius: f32,
    /// inner hole radius
    pub inner_radius: f32,
    /// strength of the wind
    pub force: f32,
    /// 0.0 = purely tangential, 1.0 = purely radial inward, 2.0 = tangential in the opposite direction
    pub gravity: f32,
    /// 0.0-1.0 factor for random angle variation, where 1.0 = full �90� deviation
    pub noise: f32,
}

impl Default for WindCircleParams {
    fn default() -> Self {
        Self {
            center: Vec2::ZERO,
            outer_radius: 0.0,
            inner_radius: 0.0,
            force: 0.0,
            gravity: 0.0,
            noise: 0.0,
        }
    }
}

impl WindCircleParams {
    /// Set the center of the WindCircle
    pub fn set_center(&mut self, center: Vec2) {
        if self.center != center {
            self.center = center;
        }
    }

    pub fn set_center_x(&mut self, x: f32) {
        if self.center.x != x {
            self.center.x = x;
        }
    }

    pub fn set_center_y(&mut self, y: f32) {
        if self.center.y != y {
            self.center.y = y;
        }
    }

    /// Set the OR of the WindCircle
    pub fn set_outer_radius(&mut self, radius: f32) {
        if self.outer_radius != radius {
            self.outer_radius = radius;
        }
    }

    /// Set the IR of the WindCircle
    pub fn set_inner_radius(&mut self, radius: f32) {
        if self.inner_radius != radius {
            self.inner_radius = radius;
        }
    }

    /// Set the strength of the WindCircle
    pub fn set_force(&mut self, force: f32) {
        if self.force != force {
            self.force = force;
        }
    }

    /// Set the center bias of the WindCircle
    pub fn set_gravity(&mut self, gravity: f32) {
        if self.gravity != gravity {
            self.gravity = gravity;
        }
    }

    /// Set the angle variation of the WindCircle
    pub fn set_noise(&mut self, noise: f32) {
        let clamped_noise = noise.clamp(0.0, 1.0);
        if self.noise != clamped_noise {
            self.noise = clamped_noise;
        }
    }
}
