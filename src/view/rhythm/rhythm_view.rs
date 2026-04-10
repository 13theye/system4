use nannou::prelude::*;
use prat::BeatSubdivision;
use std::collections::HashMap;
use std::time::Instant;

use crate::{
    groups::{RhythmParams, VoiceId},
    settings::Settings,
    view::rhythm::{
        animation::{
            MIN_FORMATION_RADIUS, MAX_FORMATION_RADIUS,
            MIN_ELEMENT_RADIUS, MAX_ELEMENT_RADIUS,
        },
        RhythmFormation,
    },
};

#[derive(Debug, Clone, Copy)]
pub struct RhythmViewUpdateParams {
    pub current_slot: Option<usize>,
    pub current_wing: Option<usize>,
    pub tempo: f64,
    pub subdivision: BeatSubdivision,
}

/// Manager for RhythmFormations
pub struct RhythmView {
    formations: HashMap<VoiceId, RhythmFormation>,
    pub min_formation_radius: f32,
    pub max_formation_radius: f32,
    pub min_element_radius: f32,
    pub max_element_radius: f32,
    pub size_scale: f32,
    // Canonical base colors for all rhythm elements (edited via UI, persisted to config)
    pub base_color: [f32; 4],
    pub gradient_color_1: [f32; 4],
    pub gradient_color_2: [f32; 4],
    pub rhythm_color_save_status: Option<String>,
}

impl Default for RhythmView {
    fn default() -> Self {
        Self::new()
    }
}

impl RhythmView {
    pub fn new() -> Self {
        Self {
            formations: HashMap::new(),
            min_formation_radius: MIN_FORMATION_RADIUS,
            max_formation_radius: MAX_FORMATION_RADIUS,
            min_element_radius: MIN_ELEMENT_RADIUS,
            max_element_radius: MAX_ELEMENT_RADIUS,
            size_scale: 1.0,
            base_color: [0.247, 0.349, 0.353, 1.0],
            gradient_color_1: [1.0, 0.162, 0.190, 1.0],
            gradient_color_2: [0.689, 1.0, 0.995, 1.0],
            rhythm_color_save_status: None,
        }
    }

    pub fn new_from_settings(settings: &Settings) -> Self {
        let mut view = Self::new();
        if let Some(rv) = &settings.rhythm_vis {
            view.base_color = rv.base_color.to_array();
            view.gradient_color_1 = rv.gradient_1.to_array();
            view.gradient_color_2 = rv.gradient_2.to_array();
        }
        view
    }

    /// Propagate the current canonical colors to all elements in all formations.
    pub fn update_element_colors(&mut self) {
        let c = self.base_color;
        let g1 = self.gradient_color_1;
        let g2 = self.gradient_color_2;
        let color = rgba(c[0], c[1], c[2], c[3]);
        let grad1 = rgba(g1[0], g1[1], g1[2], g1[3]);
        let grad2 = rgba(g2[0], g2[1], g2[2], g2[3]);
        for formation in self.formations.values_mut() {
            formation.set_element_colors(color, grad1, grad2);
        }
    }

    pub fn add_formation(&mut self, voice_id: VoiceId, rhythm_params: &RhythmParams, now: Instant) {
        let Some(formation) = RhythmFormation::init_rhythm(
            voice_id,
            rhythm_params,
            self.min_formation_radius,
            self.max_formation_radius,
            self.min_element_radius,
            self.max_element_radius,
            now,
        ) else {
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

    pub fn update_formation_params(
        &mut self,
        voice_id: VoiceId,
        rhythm_params: &RhythmParams,
        now: Instant,
    ) {
        let Some(formation) = self.formations.get_mut(&voice_id) else {
            return;
        };
        formation.update_element_params(rhythm_params, now);
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

    pub fn draw_alpha_elements(&self, draw: &Draw, debug_draw: Option<&Draw>) {
        self.formations.values().for_each(|formation| {
            formation.draw_elements(draw, debug_draw);
        });
    }

    pub fn draw_debug_geometry(&self, draw: &Draw, scale_x: f32, scale_y: f32) {
        self.formations.values().for_each(|formation| {
            formation.draw_debug_geometry(draw, scale_x, scale_y);
        });
    }

    pub fn draw_activations(&self, draw: &Draw) {
        self.formations.values().for_each(|formation| {
            formation.draw_activations(draw);
        });
    }

    /// Update the stored formation radii and animate all existing formations to the new positions.
    pub fn update_formation_radii(&mut self, min_radius: f32, max_radius: f32, now: Instant) {
        self.min_formation_radius = min_radius;
        self.max_formation_radius = max_radius;
        for formation in self.formations.values_mut() {
            formation.reposition_elements(min_radius, max_radius, now);
        }
    }

    /// Reposition elements in only one voice's formation using the stored formation radii.
    pub fn update_voice_formation_radii(&mut self, voice_id: VoiceId, now: Instant) {
        let min = self.min_formation_radius;
        let max = self.max_formation_radius;
        if let Some(formation) = self.formations.get_mut(&voice_id) {
            formation.reposition_elements(min, max, now);
        }
    }

    /// Update the stored element radii and animate sizing on only one voice's formation.
    pub fn update_voice_element_radii(&mut self, voice_id: VoiceId, now: Instant) {
        let min = self.min_element_radius;
        let max = self.max_element_radius;
        if let Some(formation) = self.formations.get_mut(&voice_id) {
            formation.update_element_radii(min, max, now);
        }
    }

    /// Update the stored element radii and animate sizing on all existing elements.
    pub fn update_element_radii(&mut self, min_radius: f32, max_radius: f32, now: Instant) {
        self.min_element_radius = min_radius;
        self.max_element_radius = max_radius;
        for formation in self.formations.values_mut() {
            formation.update_element_radii(min_radius, max_radius, now);
        }
    }
}
