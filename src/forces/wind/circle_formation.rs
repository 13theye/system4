//! The WindCircleFormation trait unifies the control of WindCircle-related force formations under the same set of parameters.

use nannou::prelude::*;

use super::{wind_circle::WindCircleParams, Wind};
use crate::groups::VoiceId;

pub trait CircleFormation: Sync + Send {
    /// Returns a bounding Rect in screen coordinates that encompasses the entire WindFormation
    fn rect(&self) -> Rect;

    /// Calculate the Wind force for a given location, if any
    fn wind_for_position(&self, position: Vec2, noise_factor: f64) -> Option<Wind>;

    /// Returns the id of the WindCircleFormation
    fn id(&self) -> usize;

    /// Returns the VoiceId of the WindCircleFormation
    fn parent_voice(&self) -> VoiceId;

    /// Get a ref to a single set of WindCircleParams that describe the formation
    fn params(&self) -> &WindCircleParams;

    /// Set the center of the WindFormation
    fn set_center(&mut self, center: Vec2);
    fn set_center_x(&mut self, x: f32);
    fn set_center_y(&mut self, y: f32);

    /// Set the Outer Radius of the WindCircleFormation
    fn set_outer_radius(&mut self, radius: f32);

    /// Set the Inner Radius of the WindCircleFormation
    fn set_inner_radius(&mut self, radius: f32);

    /// Set the Force of the WindCircleFormation
    fn set_force(&mut self, force: f32);

    /// Set the center bias of the WindCircleFormation
    fn set_gravity(&mut self, gravity: f32);

    /// Set the angle variation of the WindCircleFormation
    fn set_noise(&mut self, noise: f32);

    /// Draw the center of the WindCircleFormation
    fn draw_center(&self, draw: &Draw, scale_x: f32, scale_y: f32);

    /// Draw the WindCircleFormation
    fn draw(&self, draw: &Draw, scale_x: f32, scale_y: f32);
}
