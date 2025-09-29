// src/groups/rhythm.rs

use nannou::rand::{rngs::ThreadRng, seq::SliceRandom};
use prat::BeatSubdivision;

use crate::{
    groups::{VoiceId, VoiceParams},
    particle::emitter::Emitter,
    services::sequencer::SequencerService,
};

#[derive(Debug, Clone)]
pub struct RhythmParams {
    pub capacity: usize,
    pub num_wings: usize,
    pub subdivision: BeatSubdivision,
    pub wings: Vec<usize>,
    pub wings_buffer: Vec<usize>,
}

impl Default for RhythmParams {
    fn default() -> Self {
        Self {
            capacity: 0,
            num_wings: 0,
            subdivision: BeatSubdivision::Eighth,
            wings: Vec::new(),
            wings_buffer: Vec::new(),
        }
    }
}

impl RhythmParams {
    pub fn new(capacity: usize, num_wings: usize, subdivision: BeatSubdivision) -> Self {
        Self {
            capacity,
            num_wings,
            subdivision,
            wings: Vec::new(),
            wings_buffer: Vec::new(),
        }
    }
}

pub struct Rhythm {
    pub id: VoiceId,
    params: RhythmParams,
    pub voice_params: VoiceParams,
    pub emitters: Vec<Box<dyn Emitter>>,
}

impl Rhythm {
    pub fn new_with_id(id: VoiceId) -> Self {
        Self {
            id,
            params: RhythmParams::default(),
            voice_params: VoiceParams::default(),
            emitters: Vec::new(),
        }
    }

    pub fn new_with_params(id: VoiceId, params: RhythmParams) -> Self {
        Self {
            id,
            params,
            voice_params: VoiceParams::default(),
            emitters: Vec::new(),
        }
    }

    pub fn get_params(&self) -> &RhythmParams {
        &self.params
    }

    pub fn set_params(&mut self, params: RhythmParams) {
        self.params = params;
    }

    pub fn set_capacity(&mut self, capacity: usize) {
        self.params.capacity = capacity;
    }

    pub fn set_num_wings(&mut self, num_wings: usize) {
        self.params.num_wings = num_wings;
    }

    pub fn set_subdivision(&mut self, subdivision: BeatSubdivision) {
        self.params.subdivision = subdivision;
    }

    /// Add back wings from buffer, or generate additional wings as needed
    pub fn add_wings(&mut self, number_to_add: usize, rng: &mut ThreadRng) {
        let restore_count = number_to_add.min(self.params.wings_buffer.len());

        // restore from buffer
        let restored: Vec<_> = self
            .params
            .wings_buffer
            .drain(self.params.wings_buffer.len() - restore_count..)
            .rev()
            .collect();

        self.params.wings.extend(restored);

        let generate_count = number_to_add - restore_count;

        if generate_count > 0 {
            let mut available_positions: Vec<usize> = (0..self.params.capacity)
                .filter(|p| !self.params.wings.contains(p))
                .collect();

            available_positions.shuffle(rng);
            self.params
                .wings
                .extend(available_positions.into_iter().take(generate_count));
        }
    }

    /// Remove wings and save them to a LIFO buffer.
    pub fn remove_wings(&mut self, number_to_remove: usize) {
        let wings = &mut self.params.wings;
        let actual_remove_count = number_to_remove.min(wings.len());

        // Remove and collect the last N wings in one operation
        let removed_wings: Vec<usize> = wings
            .drain(wings.len() - actual_remove_count..)
            .collect();

        // Add them to the buffer (they're already in LIFO order from drain)
        self.params.wings_buffer.extend(removed_wings);
    }

    pub fn randomize_wings(&mut self, rng: &mut ThreadRng) {
        self.params.wings = Rhythm::roll_wings(rng, self.params.capacity, self.params.num_wings);
    }

    fn roll_wings(rng: &mut ThreadRng, capacity: usize, num_wings: usize) -> Vec<usize> {
        let mut nums: Vec<usize> = (0..capacity).collect();
        nums.shuffle(rng);
        nums.truncate(num_wings);
        nums
    }
    /********Gateway methods for SequencerService communication *********/

    /// Start the sequencer for this rhythm via SequencerService
    pub fn start_sequencer(&self, sequencer_service: &mut SequencerService) {
        let params = self.params.clone();
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
        let params = self.params.clone();
        sequencer_service.update_sequencer_params(self.id, params);
    }

    /// Re-roll wings and update the running sequencer with new wings
    pub fn reroll_wings(&mut self, rng: &mut ThreadRng, sequencer_service: &mut SequencerService) {
        self.randomize_wings(rng);
        // Update the running sequencer with new wings
        self.update_sequencer(sequencer_service);
    }
}
