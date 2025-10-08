use nannou::prelude::*;
use prat::BeatSubdivision;
use std::collections::HashMap;

use crate::{
    groups::{rhythm::RhythmParams, VoiceId},
    view::{RhythmCircleFormation, RhythmFormation, RhythmLinesFormation},
};

pub enum RhythmFormationType {
    Circle,
    Lines,
}

#[derive(Debug, Clone, Copy)]
pub struct RhythmViewUpdateParams {
    pub current_wing: Option<usize>,
    pub tempo: f64,
    pub subdivision: BeatSubdivision,
}

#[derive(Default)]
pub struct RhythmView {
    formations: HashMap<VoiceId, Box<dyn RhythmFormation>>,
}

impl RhythmView {
    pub fn new() -> Self {
        Self {
            formations: HashMap::new(),
        }
    }

    pub fn add_formation(
        &mut self,
        voice_id: VoiceId,
        typ: RhythmFormationType,
        rhythm_params: &RhythmParams,
        time: f32,
    ) {
        let mut formation: Box<dyn RhythmFormation> = match typ {
            RhythmFormationType::Circle => Box::new(RhythmCircleFormation::new(
                vec2(0.0, 0.0),
                900.0,
                rhythm_params.capacity,
                time,
            )),
            RhythmFormationType::Lines => Box::new(RhythmLinesFormation::new(
                vec2(-960.0, 0.0),
                1800.0,
                500.0,
                rhythm_params.capacity,
            )),
        };

        formation.initialize_rhythm(rhythm_params, time);
        self.formations.insert(voice_id, formation);
    }

    pub fn reinitialize_formation(
        &mut self,
        voice_id: VoiceId,
        rhythm_params: &RhythmParams,
        time: f32,
    ) {
        let Some(formation) = self.formations.get_mut(&voice_id) else {
            return;
        };
        formation.reinitialize_rhythm(rhythm_params, time);
    }

    pub fn clear_formation(&mut self, voice_id: VoiceId, time: f32) {
        let Some(formation) = self.formations.get_mut(&voice_id) else {
            return;
        };
        formation.clear_rhythm(time);
    }

    pub fn update_voice(
        &mut self,
        voice_id: &VoiceId,
        rhythm_params: &RhythmParams,
        update_params: &RhythmViewUpdateParams,
        time: f32,
    ) {
        let Some(formation) = self.formations.get_mut(voice_id) else {
            return;
        };
        formation.update_active(rhythm_params, update_params, time);
    }

    pub fn update_all_transitions(&mut self, time: f32) {
        for formation in self.formations.values_mut() {
            formation.update_transitions(time);
        }
    }

    pub fn draw_all(&self, draw: &Draw) {
        for formation in self.formations.values() {
            formation.draw(draw);
        }
    }
}
