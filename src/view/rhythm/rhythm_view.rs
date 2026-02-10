use nannou::prelude::*;
use prat::BeatSubdivision;
use std::collections::HashMap;
use std::time::Instant;

use crate::{
    groups::{RhythmParams, VoiceId},
    view::rhythm2::RhythmFormation,
};

#[derive(Debug, Clone, Copy)]
pub struct RhythmViewUpdateParams {
    pub current_slot: Option<usize>,
    pub current_wing: Option<usize>,
    pub tempo: f64,
    pub subdivision: BeatSubdivision,
}

/// Manager for RhythmFormations
#[derive(Default)]
pub struct RhythmView {
    formations: HashMap<VoiceId, RhythmFormation>,
}

impl RhythmView {
    pub fn new() -> Self {
        Self {
            formations: HashMap::new(),
        }
    }

    pub fn add_formation(&mut self, voice_id: VoiceId, rhythm_params: &RhythmParams, now: Instant) {
        let Some(formation) = RhythmFormation::init_rhythm(voice_id, rhythm_params, now) else {
            return;
        };
        self.formations.insert(voice_id, formation);
    }

    pub fn reinitialize_formation(
        &mut self,
        voice_id: VoiceId,
        rhythm_params: &RhythmParams,
        now: Instant,
    ) {
        let Some(formation) = self.formations.get_mut(&voice_id) else {
            return;
        };
        formation.reinit_rhythm(rhythm_params, now);
    }

    pub fn clear_formation(&mut self, voice_id: VoiceId, now: Instant) {
        let Some(formation) = self.formations.get_mut(&voice_id) else {
            return;
        };
        formation.clear_rhythm(now);
    }

    pub fn update_voice(
        &mut self,
        voice_id: &VoiceId,
        update_params: &RhythmViewUpdateParams,
        now: Instant,
    ) {
        let Some(formation) = self.formations.get_mut(voice_id) else {
            return;
        };
        formation.update(update_params, now);
    }

    pub fn update_transitions(&mut self, now: Instant) {
        for formation in self.formations.values_mut() {
            formation.update_transitions(now);
        }
    }

    pub fn draw_alpha_elements(&self, draw: &Draw, show_debug_geometry: bool) {
        self.formations.values().for_each(|formation| {
            formation.draw_elements(draw, show_debug_geometry);
        });
    }
}
