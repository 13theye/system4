// src/groups/rhythm.rs
// App wrapper for rhythm logic with Sequencer integration

use crossbeam_channel as channel;
use rand::rngs::ThreadRng;
pub use system4_core::commands::rhythm::{ParameterModification, RangeSize};
pub use system4_core::rhythm::{RhythmLogic, RhythmParams, RhythmSlot};

use crate::{
    groups::{VoiceId, VoiceParams},
    particle::emitter::Emitter,
    services::sequencer::SequencerService,
};

const NUM_SLOTS: usize = 32;

/// App wrapper for Rhythm that integrates core logic with Sequencer services
pub struct Rhythm {
    pub id: VoiceId,
    logic: RhythmLogic,
    pub voice_params: VoiceParams,
    pub emitters: Vec<Box<dyn Emitter>>,

    // Callback channel from the sequencer
    pub sequencer_data_rx: Option<channel::Receiver<usize>>,
}

impl Rhythm {
    pub fn new_with_id(id: VoiceId) -> Self {
        Self {
            id,
            logic: RhythmLogic::new_default(),
            voice_params: VoiceParams::default(),
            emitters: Vec::new(),
            sequencer_data_rx: None,
        }
    }

    pub fn new_with_params(id: VoiceId, params: RhythmParams) -> Self {
        Self {
            id,
            logic: RhythmLogic::new(params),
            voice_params: VoiceParams::default(),
            emitters: Vec::new(),
            sequencer_data_rx: None,
        }
    }

    /// Update receives beats from channel and filters by active wings
    pub fn update(&mut self) -> (Option<usize>, Option<usize>) {
        let slot = self.receive_beat();
        let wing = slot.filter(|&beat| self.logic.wing_has_beat(beat));
        (slot, wing)
    }

    fn receive_beat(&mut self) -> Option<usize> {
        let sequencer = self.sequencer_data_rx.as_ref()?;
        let result = sequencer.try_recv();
        result.ok()
    }

    /*************** Delegation to core logic *************************** */

    pub fn set_sequencer_data_rx(&mut self, rx: channel::Receiver<usize>) {
        self.sequencer_data_rx = Some(rx);
    }

    pub fn clear_params(&mut self) {
        self.logic.clear_params();
    }

    pub fn get_params(&self) -> &RhythmParams {
        self.logic.params()
    }

    pub fn set_params(&mut self, params: RhythmParams) {
        self.logic.set_params(params);
    }

    pub fn set_capacity(&mut self, capacity: usize) {
        self.logic.set_capacity(capacity);
    }

    pub fn get_capacity(&self) -> usize {
        self.logic.get_capacity()
    }

    pub fn set_num_wings(&mut self, num_wings: usize) {
        self.logic.set_num_wings(num_wings);
    }

    pub fn get_num_wings(&self) -> usize {
        self.logic.get_num_wings()
    }

    pub fn set_subdivision(&mut self, subdivision: prat::BeatSubdivision) {
        self.logic.set_subdivision(subdivision);
    }

    pub fn get_subdivision(&self) -> &prat::BeatSubdivision {
        self.logic.get_subdivision()
    }

    pub fn set_length_range(&mut self, range: RangeSize) {
        self.logic.set_length_range(range);
    }

    pub fn set_velocity_range(&mut self, range: RangeSize) {
        self.logic.set_velocity_range(range);
    }

    pub fn set_cutoff_range(&mut self, range: RangeSize) {
        self.logic.set_cutoff_range(range);
    }

    pub fn add_wings(&mut self, number_to_add: usize, rng: &mut ThreadRng) {
        self.logic.add_wings(number_to_add, rng);
    }

    pub fn remove_wings(&mut self, number_to_remove: usize) {
        self.logic.remove_wings(number_to_remove);
    }

    pub fn randomize_wings(&mut self, rng: &mut ThreadRng) {
        self.logic.randomize_wings(rng);
    }

    pub fn initialize_slots(&mut self, rng: &mut ThreadRng) {
        self.logic.initialize_slots(NUM_SLOTS, rng);
    }

    pub fn roll_slot(&mut self, rng: &mut ThreadRng) -> RhythmSlot {
        self.logic.roll_slot(rng)
    }

    pub fn set_all_slot_velocity(&mut self, val: f32) {
        self.logic.set_all_slot_velocity(val);
    }

    pub fn set_all_slot_length(&mut self, val: f32) {
        self.logic.set_all_slot_length(val);
    }

    pub fn set_all_slot_cutoff(&mut self, val: f32) {
        self.logic.set_all_slot_cutoff(val);
    }

    pub fn randomize_all_slots_length(&mut self, range: RangeSize, rng: &mut ThreadRng) {
        self.logic.randomize_all_slots_length(range, rng);
    }

    pub fn randomize_all_slots_velocity(&mut self, range: RangeSize, rng: &mut ThreadRng) {
        self.logic.randomize_all_slots_velocity(range, rng);
    }

    pub fn randomize_all_slots_cutoff(&mut self, range: RangeSize, rng: &mut ThreadRng) {
        self.logic.randomize_all_slots_cutoff(range, rng);
    }

    pub fn modify_all_slots_length(&mut self, modification: ParameterModification, rng: &mut ThreadRng) {
        self.logic.modify_all_slots_length(modification, rng);
    }

    pub fn modify_all_slots_velocity(&mut self, modification: ParameterModification, rng: &mut ThreadRng) {
        self.logic.modify_all_slots_velocity(modification, rng);
    }

    pub fn modify_all_slots_cutoff(&mut self, modification: ParameterModification, rng: &mut ThreadRng) {
        self.logic.modify_all_slots_cutoff(modification, rng);
    }

    /********Gateway methods for SequencerService communication *********/

    /// Start the sequencer for this rhythm via SequencerService
    pub fn add_sequencer(&self, sequencer_service: &mut SequencerService) {
        let params = self.logic.params().clone();
        sequencer_service.add_sequencer(self.id, params);
    }

    /// Stop the sequencer for this rhythm via SequencerService
    pub fn stop_sequencer(&self, sequencer_service: &mut SequencerService) {
        sequencer_service.remove_sequencer(self.id);
    }

    /// Pause the sequencer for this rhythm via SequencerService
    pub fn pause_sequencer(&self, sequencer_service: &mut SequencerService) {
        sequencer_service.pause_sequencer(self.id);
    }

    /// Resume the sequencer for this rhythm via SequencerService
    pub fn resume_sequencer(&self, sequencer_service: &mut SequencerService) {
        sequencer_service.resume_sequencer(self.id);
    }

    /// Update sequencer parameters without re-rolling wings
    pub fn update_sequencer(&self, sequencer_service: &mut SequencerService) {
        let params = self.logic.params().clone();
        sequencer_service.update_sequencer_params(self.id, params);
    }

    /// Re-roll wings and update the running sequencer with new wings
    pub fn reroll_wings(&mut self, rng: &mut ThreadRng, sequencer_service: &mut SequencerService) {
        self.randomize_wings(rng);
        // Update the running sequencer with new wings
        self.update_sequencer(sequencer_service);
    }
}
