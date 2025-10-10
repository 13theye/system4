// src/groups/rhythm.rs

use crossbeam_channel as channel;
//use nannou::rand::{rngs::ThreadRng, seq::SliceRandom, Rng};
use prat::BeatSubdivision;
use rand::{rngs::ThreadRng, seq::SliceRandom, Rng};
use rand_distr::{Distribution, SkewNormal};

use crate::{
    groups::{VoiceId, VoiceParams},
    particle::emitter::Emitter,
    services::sequencer::SequencerService,
    terminals::commands::rhythm::RangeSize,
};

const NUM_SLOTS: usize = 32;

#[derive(Debug, Clone)]
pub struct RhythmSlot {
    pub velocity: f32,
    pub length: f32,
    pub cutoff: f32,
}

#[derive(Debug, Clone)]
pub struct RhythmParams {
    pub capacity: usize,
    pub num_wings: usize,
    pub subdivision: BeatSubdivision,
    pub length_range: RangeSize,
    pub velocity_range: RangeSize,
    pub pitch_range: RangeSize,
    pub wings: Vec<usize>,
    pub wings_buffer: Vec<usize>,
    pub slots: Vec<RhythmSlot>,
}

impl Default for RhythmParams {
    fn default() -> Self {
        Self {
            capacity: 0,
            num_wings: 0,
            subdivision: BeatSubdivision::Eighth,
            length_range: RangeSize::default(),
            velocity_range: RangeSize::default(),
            pitch_range: RangeSize::default(),
            wings: Vec::new(),
            wings_buffer: Vec::new(),
            slots: Vec::new(),
        }
    }
}

impl RhythmParams {
    pub fn new(
        capacity: usize,
        num_wings: usize,
        subdivision: BeatSubdivision,
        length_range: RangeSize,
        velocity_range: RangeSize,
        pitch_range: RangeSize,
    ) -> Self {
        Self {
            capacity,
            num_wings,
            subdivision,
            length_range,
            velocity_range,
            pitch_range,
            wings: Vec::new(),
            wings_buffer: Vec::new(),
            slots: Vec::new(),
        }
    }
}

pub struct Rhythm {
    pub id: VoiceId,
    params: RhythmParams,
    pub voice_params: VoiceParams,
    pub emitters: Vec<Box<dyn Emitter>>,

    // Callback channel from the sequencer
    pub sequencer_data_rx: Option<channel::Receiver<usize>>,
}

impl Rhythm {
    pub fn new_with_id(id: VoiceId) -> Self {
        Self {
            id,
            params: RhythmParams::default(),
            voice_params: VoiceParams::default(),
            emitters: Vec::new(),
            sequencer_data_rx: None,
        }
    }

    pub fn new_with_params(id: VoiceId, params: RhythmParams) -> Self {
        Self {
            id,
            params,
            voice_params: VoiceParams::default(),
            emitters: Vec::new(),
            sequencer_data_rx: None,
        }
    }

    pub fn update(&mut self) -> (Option<usize>, Option<usize>) {
        let slot = self.receive_beat();
        let wing = slot.filter(|&beat| self.wing_has_beat(beat));
        (slot, wing)
    }

    fn receive_beat(&mut self) -> Option<usize> {
        let sequencer = self.sequencer_data_rx.as_ref()?;

        let result = sequencer.try_recv();
        result.ok()
    }

    /*************** Beat logic helpers *************************** */
    fn wing_has_beat(&self, beat: usize) -> bool {
        self.params.wings.contains(&beat)
    }

    /*************** Parameter setting ****************************** */

    pub fn set_sequencer_data_rx(&mut self, rx: channel::Receiver<usize>) {
        self.sequencer_data_rx = Some(rx);
    }

    pub fn clear_params(&mut self) {
        self.params = RhythmParams::default();
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

    pub fn get_capacity(&self) -> usize {
        self.params.capacity
    }

    pub fn set_num_wings(&mut self, num_wings: usize) {
        self.params.num_wings = num_wings;
    }

    pub fn get_num_wings(&self) -> usize {
        self.params.num_wings
    }

    pub fn set_subdivision(&mut self, subdivision: BeatSubdivision) {
        self.params.subdivision = subdivision;
    }

    pub fn get_subdivision(&self) -> &BeatSubdivision {
        &self.params.subdivision
    }

    pub fn set_length_range(&mut self, range: RangeSize) {
        self.params.length_range = range;
    }

    pub fn set_velocity_range(&mut self, range: RangeSize) {
        self.params.velocity_range = range;
    }

    pub fn set_cutoff_range(&mut self, range: RangeSize) {
        self.params.pitch_range = range; // Note: cutoff maps to pitch_range internally
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
        let removed_wings: Vec<usize> = wings.drain(wings.len() - actual_remove_count..).collect();

        // Add them to the buffer (they're already in LIFO order from drain)
        self.params.wings_buffer.extend(removed_wings);
    }

    pub fn randomize_wings(&mut self, rng: &mut ThreadRng) {
        self.params.wings = Rhythm::roll_wings(rng, self.params.capacity, self.params.num_wings);
    }

    pub fn initialize_slots(&mut self, rng: &mut ThreadRng) {
        for _ in 0..NUM_SLOTS {
            let slot = self.roll_slot(rng);
            self.params.slots.push(slot);
        }
    }

    fn roll_wings(rng: &mut ThreadRng, capacity: usize, num_wings: usize) -> Vec<usize> {
        let mut nums: Vec<usize> = (0..capacity).collect();
        nums.shuffle(rng);
        nums.truncate(num_wings);
        nums
    }

    pub fn roll_slot(&mut self, rng: &mut ThreadRng) -> RhythmSlot {
        let length = rng.random_range(self.params.length_range.to_range_inclusive());
        let velocity = rng.random_range(self.params.velocity_range.to_range_inclusive());
        let pitch = rng.random_range(self.params.pitch_range.to_range_inclusive());
        RhythmSlot {
            length,
            velocity,
            cutoff: pitch,
        }
    }

    pub fn set_all_slot_velocity(&mut self, val: f32) {
        for slot in &mut self.params.slots {
            slot.velocity = val;
        }
    }

    pub fn set_all_slot_length(&mut self, val: f32) {
        for slot in &mut self.params.slots {
            slot.length = val;
        }
    }

    pub fn set_all_slot_cutoff(&mut self, val: f32) {
        for slot in &mut self.params.slots {
            slot.cutoff = val;
        }
    }

    /// Randomize all slots' length within a range
    pub fn randomize_all_slots_length(&mut self, range: RangeSize, rng: &mut ThreadRng) {
        for slot in &mut self.params.slots {
            slot.length = rng.random_range(range.to_range_inclusive());
        }
    }

    pub fn generate_skew_normal_value(
        &mut self,
        skew_normal_params: (f32, f32, f32),
        rng: &mut ThreadRng,
    ) -> Option<f32> {
        let skew_normal = SkewNormal::new(
            skew_normal_params.0,
            skew_normal_params.1,
            skew_normal_params.2,
        )
        .ok()?;
        Some(skew_normal.sample(rng))
    }

    /// Randomize all slots' velocity within a range
    pub fn randomize_all_slots_velocity(&mut self, range: RangeSize, rng: &mut ThreadRng) {
        for slot in &mut self.params.slots {
            slot.velocity = rng.random_range(range.to_range_inclusive());
        }
    }

    /// Randomize all slots' cutoff within a range
    pub fn randomize_all_slots_cutoff(&mut self, range: RangeSize, rng: &mut ThreadRng) {
        for slot in &mut self.params.slots {
            slot.cutoff = rng.random_range(range.to_range_inclusive());
        }
    }

    /// Modify all slots' length based on ParameterModification
    pub fn modify_all_slots_length(
        &mut self,
        modification: crate::terminals::commands::rhythm::ParameterModification,
        rng: &mut ThreadRng,
    ) {
        use crate::terminals::commands::rhythm::ParameterModification;
        match modification {
            ParameterModification::Absolute(value) => {
                self.set_all_slot_length(value.clamp(0.0, 1.0));
            }
            ParameterModification::Relative(delta) => {
                for slot in &mut self.params.slots {
                    slot.length = (slot.length + delta).clamp(0.0, 1.0);
                }
            }
            ParameterModification::Randomize(range) => {
                self.randomize_all_slots_length(range, rng);
            }
        }
    }

    /// Modify all slots' velocity based on ParameterModification
    pub fn modify_all_slots_velocity(
        &mut self,
        modification: crate::terminals::commands::rhythm::ParameterModification,
        rng: &mut ThreadRng,
    ) {
        use crate::terminals::commands::rhythm::ParameterModification;
        match modification {
            ParameterModification::Absolute(value) => {
                self.set_all_slot_velocity(value.clamp(0.0, 1.0));
            }
            ParameterModification::Relative(delta) => {
                for slot in &mut self.params.slots {
                    slot.velocity = (slot.velocity + delta).clamp(0.0, 1.0);
                }
            }
            ParameterModification::Randomize(range) => {
                self.randomize_all_slots_velocity(range, rng);
            }
        }
    }

    /// Modify all slots' cutoff based on ParameterModification
    pub fn modify_all_slots_cutoff(
        &mut self,
        modification: crate::terminals::commands::rhythm::ParameterModification,
        rng: &mut ThreadRng,
    ) {
        use crate::terminals::commands::rhythm::ParameterModification;
        match modification {
            ParameterModification::Absolute(value) => {
                self.set_all_slot_cutoff(value.clamp(0.0, 1.0));
            }
            ParameterModification::Relative(delta) => {
                for slot in &mut self.params.slots {
                    slot.cutoff = (slot.cutoff + delta).clamp(0.0, 1.0);
                }
            }
            ParameterModification::Randomize(range) => {
                self.randomize_all_slots_cutoff(range, rng);
            }
        }
    }

    /********Gateway methods for SequencerService communication *********/

    /// Start the sequencer for this rhythm via SequencerService
    pub fn add_sequencer(&self, sequencer_service: &mut SequencerService) {
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
