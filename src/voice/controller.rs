// src/voice/voice_controller.rs
//
// Extending the Model to provide an interface for per-Voice control

use super::Voice;
use crate::{forces::WindCircleParams, model::Model};
use nannou::prelude::*;
use nannou::wgpu::{Device, Queue};

#[derive(Debug, Clone)]
pub enum VoiceParameterChange {
    Alpha {
        voice: Voice,
        value: f32,
    },
    Volume {
        voice: Voice,
        value: f32,
    },
    Feedback {
        voice: Voice,
        value: f32,
    },
    OuterRadius {
        voice: Voice,
        value: f32,
    },
    InnerRadius {
        voice: Voice,
        value: f32,
    },
    Strength {
        voice: Voice,
        value: f32,
    },
    CenterBias {
        voice: Voice,
        value: f32,
    },
    AngleVariation {
        voice: Voice,
        value: f32,
    },
    PositionOffset {
        voice: Voice,
        value: f32,
    },
    ForceCenterX {
        voice: Voice,
        value: f32,
    },
    ForceCenterY {
        voice: Voice,
        value: f32,
    },
    MaskChangeBounds {
        voice: Voice,
        rect: Rect,
        duration: f32,
    },
}

impl Model {
    /// Get the params of a circle
    pub fn get_wind_circle_params(&self, voice: Voice) -> Option<&WindCircleParams> {
        self.particle_system.forces.get_wind_circle_params(voice)
    }

    /// Get the alpha limit of a Voice ("brightness")
    pub fn get_alpha_limit(&self, voice: Voice) -> f32 {
        self.particle_system
            .alpha_limits
            .get(&voice)
            .copied()
            .unwrap_or(1.0)
    }

    /// Get the center bias of a Voice's WindCircle ("gravity")
    pub fn get_center_bias(&mut self, voice: Voice) -> f32 {
        self.particle_system.forces.get_center_bias(&voice)
    }

    /// Get the angle variation of a Voice's WindCircle ("noise")
    pub fn get_angle_variation(&self, voice: Voice) -> f32 {
        self.particle_system.forces.get_angle_variation(&voice)
    }

    /// Get the position offset factor of a Voice ("vibration")
    pub fn get_position_offset_factor(&self, voice: Voice) -> f32 {
        self.particle_system.get_position_offset_factor(voice)
    }

    pub fn get_volume(&self, voice: Voice) -> f32 {
        self.particle_system
            .particle_num_factors
            .get(&voice)
            .copied()
            .unwrap_or(1.0)
    }

    pub fn get_feedback(&self, voice: Voice) -> f32 {
        self.particle_system
            .feedback
            .get(&voice)
            .copied()
            .unwrap_or(0.0)
    }

    /// Apply a parameter change to the model
    pub fn apply_voice_parameter_change(&mut self, change: VoiceParameterChange) {
        match change {
            VoiceParameterChange::Alpha { voice, value } => {
                self.particle_system.set_alpha_limit(&voice, value);
            }
            VoiceParameterChange::Volume { voice, value } => {
                self.particle_system.set_volume(&voice, value);
            }
            VoiceParameterChange::Feedback { voice, value } => {
                self.particle_system.set_feedback(&voice, value);
            }
            VoiceParameterChange::OuterRadius { voice, value } => {
                self.particle_system.forces.set_outer_radius(&voice, value);
            }
            VoiceParameterChange::InnerRadius { voice, value } => {
                self.particle_system.forces.set_inner_radius(&voice, value);
            }
            VoiceParameterChange::Strength { voice, value } => {
                let strength = value.min(30.0); // 30 is the max strength of the wind circle
                self.particle_system.forces.set_strength(&voice, strength);
            }
            VoiceParameterChange::CenterBias { voice, value } => {
                self.particle_system.forces.set_center_bias(&voice, value);
            }
            VoiceParameterChange::AngleVariation { voice, value } => {
                self.particle_system
                    .forces
                    .set_angle_variation(&voice, value);
            }
            VoiceParameterChange::PositionOffset { voice, value } => {
                self.particle_system
                    .set_position_offset_factor(&voice, value);
            }
            VoiceParameterChange::ForceCenterX { voice, value } => {
                if let Some(circle) = self.particle_system.forces.get_wind_circle_mut(voice) {
                    let current_y = circle.params().center.y;
                    circle.params_mut().set_center(vec2(value, current_y));
                }
            }
            VoiceParameterChange::ForceCenterY { voice, value } => {
                if let Some(circle) = self.particle_system.forces.get_wind_circle_mut(voice) {
                    let current_x = circle.params().center.x;
                    circle.params_mut().set_center(vec2(current_x, value));
                }
            }
            VoiceParameterChange::MaskChangeBounds {
                voice,
                rect,
                duration,
            } => {
                if let Some(mask) = self.particle_system.masks.get_mut(&voice) {
                    mask.change_bounds(rect, duration);
                }
            }
        }
    }
}

/********** Functions for changing renderer properties ***************** */

/// Update the "Feedback" feature
pub fn update_feedback(model: &mut Model, device: &Device, queue: &Queue) {
    // Read feedback value for segment length before updating particle system
    let voice1_feedback = model.get_feedback(Voice::Voice1);
    let voice4_feedback = model.get_feedback(Voice::Voice4);

    // Update segment length based on Voice1 feedback slider
    model
        .segment_renderer1
        .set_segment_length(device, queue, voice1_feedback);

    // Update segment length based on Voice1 feedback slider
    model
        .segment_renderer4
        .set_segment_length(device, queue, voice4_feedback);
}
