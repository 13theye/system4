use nannou::prelude::*;
use prat::BeatSubdivision;
use std::collections::HashMap;
use std::time::Instant;

use crate::{
    groups::{RhythmParams, VoiceId},
    view::rhythm::{RhythmCircleFormation, RhythmFormation, RhythmLinesFormation},
};

pub enum RhythmFormationType {
    Circle { radius: f32 },
    Lines,
}

#[derive(Debug, Clone, Copy)]
pub struct RhythmViewUpdateParams {
    pub current_slot: Option<usize>,
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
        now: Instant,
    ) {
        let mut formation: Box<dyn RhythmFormation> = match typ {
            RhythmFormationType::Circle { radius } => Box::new(RhythmCircleFormation::new(
                vec2(0.0, 0.0),
                radius,
                rhythm_params.capacity,
                now,
            )),
            RhythmFormationType::Lines => Box::new(RhythmLinesFormation::new(
                vec2(-960.0, 0.0),
                1800.0,
                500.0,
                rhythm_params.capacity,
            )),
        };

        formation.initialize_rhythm(rhythm_params, now);
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
        formation.reinitialize_rhythm(rhythm_params, now);
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
        rhythm_params: &RhythmParams,
        update_params: &RhythmViewUpdateParams,
        now: Instant,
    ) {
        let Some(formation) = self.formations.get_mut(voice_id) else {
            return;
        };
        formation.update_active(rhythm_params, update_params, now);
    }

    pub fn update_all_transitions(&mut self, now: Instant) {
        for formation in self.formations.values_mut() {
            formation.update_transitions(now);
        }
    }

    pub fn draw_all(&self, draw: &Draw) {
        for formation in self.formations.values() {
            formation.draw(draw);
        }
    }
}
