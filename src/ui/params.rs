// Parameter extraction for UI rendering
// These structs hold pre-extracted parameter values to avoid borrow checker conflicts

use crate::forces::WindCircleParams;
use crate::groups::{VoiceId, VoiceParams};
use crate::model::Model;

/// All parameters needed to render a drone voice panel (Voice 0 or Voice 3)
/// Uses domain structs directly to avoid duplication
#[derive(Clone, Debug)]
pub struct DroneVoiceParams {
    pub voice_id: VoiceId,
    /// List of (circle_id, circle_params) tuples
    pub circles: Vec<(usize, WindCircleParams)>,
    /// Voice-level parameters (contains alpha, volume, feedback, vibration, emitter_position, etc.)
    pub voice_params: VoiceParams,
}

impl DroneVoiceParams {
    /// Extract all parameters for a drone voice from the model
    pub fn extract(model: &Model, voice_id: VoiceId) -> Self {
        let circle_ids = model.get_wind_circle_ids(voice_id);

        let circles: Vec<(usize, WindCircleParams)> = circle_ids
            .iter()
            .filter_map(|&id| {
                model
                    .get_wind_circle_params(voice_id, id)
                    .cloned()
                    .map(|params| (id, params))
            })
            .collect();

        // Get voice params directly from the voice
        let voice_params = model
            .voice_manager
            .voices()
            .get(&voice_id)
            .map(|v| v.params.clone())
            .unwrap_or_default();

        Self {
            voice_id,
            circles,
            voice_params,
        }
    }
}
