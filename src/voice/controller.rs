// src/voice/voice_controller.rs
//
// Extending the Model to provide an interface for per-Voice control

use super::Voice;
use crate::{forces::WindCircleParams, model::Model};
use nannou::prelude::*;

#[derive(Debug, Clone)]
pub enum VoiceParameterChange {
    Alpha { voice: Voice, value: f32 },
    Volume { voice: Voice, value: f32 },
    Feedback { voice: Voice, value: f32 },
    OuterRadius { voice: Voice, value: f32 },
    InnerRadius { voice: Voice, value: f32 },
    Strength { voice: Voice, value: f32 },
    CenterBias { voice: Voice, value: f32 },
    CenterX { voice: Voice, value: f32 },
    CenterY { voice: Voice, value: f32 },
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
            VoiceParameterChange::CenterX { voice, value } => {
                if let Some(circle) = self.particle_system.forces.get_wind_circle_mut(voice) {
                    let current_y = circle.params().center.y;
                    circle.params_mut().set_center(vec2(value, current_y));
                }
            }
            VoiceParameterChange::CenterY { voice, value } => {
                if let Some(circle) = self.particle_system.forces.get_wind_circle_mut(voice) {
                    let current_x = circle.params().center.x;
                    circle.params_mut().set_center(vec2(current_x, value));
                }
            }
        }
    }
}
