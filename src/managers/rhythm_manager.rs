use crate::{
    groups::{Rhythm, VoiceId},
    services::sequencer::SequencerService,
    terminals::commands::rhythm::RhythmParamModification,
};
use rand::rngs::ThreadRng;
use std::collections::HashMap;

/// RhythmManager handles all rhythm-related state and operations.
/// This includes:
/// - Rhythm lifecycle (creation, removal)
/// - Rhythm state access (mutable and immutable)
/// - Composite operations requiring coordinated access to sequencer service and RNG
pub struct RhythmManager {
    rhythms: HashMap<VoiceId, Rhythm>,
}

impl RhythmManager {
    pub fn new() -> Self {
        Self {
            rhythms: HashMap::new(),
        }
    }

    // Rhythm state access
    pub fn has_rhythm(&self, voice_id: VoiceId) -> bool {
        self.rhythms.contains_key(&voice_id)
    }

    pub fn get_rhythm(&self, voice_id: VoiceId) -> Option<&Rhythm> {
        self.rhythms.get(&voice_id)
    }

    pub fn get_rhythm_mut(&mut self, voice_id: VoiceId) -> Option<&mut Rhythm> {
        self.rhythms.get_mut(&voice_id)
    }

    pub fn insert_rhythm(&mut self, voice_id: VoiceId, rhythm: Rhythm) {
        self.rhythms.insert(voice_id, rhythm);
    }

    pub fn remove_rhythm(&mut self, voice_id: VoiceId) -> Option<Rhythm> {
        self.rhythms.remove(&voice_id)
    }

    pub fn rhythms(&self) -> &HashMap<VoiceId, Rhythm> {
        &self.rhythms
    }

    pub fn rhythms_mut(&mut self) -> &mut HashMap<VoiceId, Rhythm> {
        &mut self.rhythms
    }

    // Validation
    pub fn validate_rhythm_exists(&self, voice_id: VoiceId) -> bool {
        self.rhythms.contains_key(&voice_id)
    }

    // Composite operations requiring coordinated access to services
    /// Update the sequencer for a rhythm
    /// Requires coordinated access to rhythm and sequencer service
    pub fn update_rhythm_sequencer(
        &self,
        voice_id: VoiceId,
        sequencer_service: &mut SequencerService,
    ) {
        if let Some(rhythm) = self.rhythms.get(&voice_id) {
            rhythm.update_sequencer(sequencer_service);
        }
    }

    /// Reroll wings for a rhythm
    /// Requires coordinated access to rhythm, RNG, and sequencer service
    pub fn rhythm_reroll_wings(
        &mut self,
        voice_id: VoiceId,
        rng: &mut ThreadRng,
        sequencer_service: &mut SequencerService,
    ) {
        if let Some(rhythm) = self.rhythms.get_mut(&voice_id) {
            rhythm.reroll_wings(rng, sequencer_service);
        }
    }

    /// Add wings to a rhythm
    /// Requires coordinated access to rhythm and RNG
    pub fn rhythm_add_wings(&mut self, voice_id: VoiceId, count: usize, rng: &mut ThreadRng) {
        if let Some(rhythm) = self.rhythms.get_mut(&voice_id) {
            rhythm.add_wings(count, rng);
        }
    }

    /// Stop the sequencer for a rhythm
    /// Requires coordinated access to rhythm and sequencer service
    pub fn rhythm_stop_sequencer(
        &mut self,
        voice_id: VoiceId,
        sequencer_service: &mut SequencerService,
    ) {
        if let Some(rhythm) = self.rhythms.get_mut(&voice_id) {
            rhythm.stop_sequencer(sequencer_service);
        }
    }

    /// Modify all slot lengths for a rhythm
    /// Requires coordinated access to rhythm and RNG
    pub fn rhythm_modify_all_slots_length(
        &mut self,
        voice_id: VoiceId,
        modification: RhythmParamModification,
        rng: &mut ThreadRng,
    ) {
        if let Some(rhythm) = self.rhythms.get_mut(&voice_id) {
            rhythm.modify_all_slots_length(modification, rng);
        }
    }

    /// Modify all slot velocities for a rhythm
    /// Requires coordinated access to rhythm and RNG
    pub fn rhythm_modify_all_slots_velocity(
        &mut self,
        voice_id: VoiceId,
        modification: RhythmParamModification,
        rng: &mut ThreadRng,
    ) {
        if let Some(rhythm) = self.rhythms.get_mut(&voice_id) {
            rhythm.modify_all_slots_velocity(modification, rng);
        }
    }

    /// Modify all slot cutoffs for a rhythm
    /// Requires coordinated access to rhythm and RNG
    pub fn rhythm_modify_all_slots_cutoff(
        &mut self,
        voice_id: VoiceId,
        modification: RhythmParamModification,
        rng: &mut ThreadRng,
    ) {
        if let Some(rhythm) = self.rhythms.get_mut(&voice_id) {
            rhythm.modify_all_slots_cutoff(modification, rng);
        }
    }
}

impl Default for RhythmManager {
    fn default() -> Self {
        Self::new()
    }
}
