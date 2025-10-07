use nannou::prelude::*;
use prat::BeatSubdivision;
use std::collections::HashMap;

use crate::{
    groups::{rhythm::RhythmParams, VoiceId},
    view::{RhythmCircleFormation, RhythmFormation},
};

pub enum RhythmFormationType {
    Circle,
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
        _typ: RhythmFormationType,
        rhythm_params: &RhythmParams,
    ) {
        let mut formation = Box::new(RhythmCircleFormation::new(
            vec2(0.0, 0.0),
            200.0,
            rhythm_params.capacity,
        ));

        formation.initialize_rhythm(rhythm_params);
        self.formations.insert(voice_id, formation);
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
        formation.update(rhythm_params, update_params, time);
    }

    pub fn draw_all(&self, draw: &Draw) {
        for (_, formation) in self.formations.iter() {
            formation.draw(draw);
        }
    }
}
