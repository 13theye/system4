// src/model/queries.rs
//
// Read-only (or read-mostly) accessors used by UI/rendering.

use crate::{forces::wind_circle::WindCircleParams, groups::VoiceId, model::Model};

impl Model {
    /// Get all wind circle IDs for a voice.
    pub fn get_wind_circle_ids(&self, voice: VoiceId) -> Vec<usize> {
        let Some(voice) = self.voice_manager.voices().get(&voice) else {
            return Vec::new();
        };
        let mut ids: Vec<usize> = voice.wind_circles.keys().copied().collect();
        ids.sort();
        ids
    }

    /// Get the params of a circle.
    pub fn get_wind_circle_params(&self, voice: VoiceId, id: usize) -> Option<&WindCircleParams> {
        let voice = self.voice_manager.voices().get(&voice)?;
        voice.wind_circles.get(&id).map(|circle| circle.params())
    }

    /// Get the alpha limit of a Voice ("brightness").
    pub fn get_alpha_limit(&self, voice: VoiceId) -> f32 {
        let Some(voice) = self.voice_manager.voices().get(&voice) else {
            return 0.0;
        };

        voice.params.alpha_limit
    }

    /// Get the center bias of a Voice's WindCircle ("gravity").
    pub fn get_center_bias(&self, voice: VoiceId, id: usize) -> f32 {
        let Some(voice) = self.voice_manager.voices().get(&voice) else {
            return 0.0;
        };

        voice
            .wind_circles
            .get(&id)
            .map(|circle| circle.params().gravity)
            .unwrap_or(0.0)
    }

    /// Get the angle variation of a Voice's WindCircle by id ("noise").
    pub fn get_noise(&self, voice: VoiceId, id: usize) -> f32 {
        let Some(voice) = self.voice_manager.voices().get(&voice) else {
            return 0.0;
        };

        voice
            .wind_circles
            .get(&id)
            .map(|circle| circle.params().noise)
            .unwrap_or(0.0)
    }

    /// Get the position offset factor of a Voice ("vibration").
    pub fn get_vibration(&self, voice: VoiceId) -> f32 {
        let Some(voice) = self.voice_manager.voices().get(&voice) else {
            return 0.0;
        };

        voice.params.vibration
    }

    pub fn get_emitter_position(&self, voice: VoiceId) -> f32 {
        let Some(voice) = self.voice_manager.voices().get(&voice) else {
            return 0.0;
        };

        voice.params.emitter_position
    }

    pub fn get_volume(&self, voice: VoiceId) -> f32 {
        let Some(voice) = self.voice_manager.voices().get(&voice) else {
            return 0.0;
        };

        voice.params.volume
    }

    pub fn get_feedback(&self, voice: VoiceId) -> f32 {
        let Some(voice) = self.voice_manager.voices().get(&voice) else {
            return 0.0;
        };

        voice.params.feedback
    }
}
