// src/groups/rhythm.rs

use crossbeam_channel as channel;
//use nannou::rand::{rngs::ThreadRng, seq::SliceRandom, Rng};
use prat::BeatSubdivision;
use rand::{rngs::ThreadRng, seq::SliceRandom, Rng};
use rand_distr::{Distribution, SkewNormal};

use crate::{
    groups::{RhythmParams, RhythmSlotParams, VoiceId, VoiceParams},
    openai::schema::RhythmObject,
    particle::emitter::Emitter,
    sequencer::SequencerService,
    terminals::commands::rhythm::RangeSize,
};

const NUM_SLOTS: usize = 32;

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

    /*************** For AI Rhythm *************************** */
    pub fn as_serializable_object(&self) -> RhythmObject {
        self.params.to_serializable_object()
    }

    /// Simpler test function that gathers filled slots for LLM
    pub fn as_string_representation(&self) -> String {
        let mut output = String::from("[");
        for i in 0..self.params.capacity {
            if self.params.wings.contains(&i) {
                output.push('X');
            } else {
                output.push('O');
            }
        }
        output.push(']');

        output
    }

    /// Apply a rhythm pattern returned by the AI to this rhythm.
    ///
    /// The expected format is a bracketed string such as "[OXXOOXXOO]".
    /// Each 'X' becomes a filled slot (wing), each 'O' becomes empty.
    pub fn apply_ai_pattern(&mut self, pattern: &str) {
        let trimmed = pattern.trim();
        if !trimmed.starts_with('[') || !trimmed.ends_with(']') || trimmed.len() < 3 {
            println!(
                "Rhythm::apply_ai_pattern: invalid pattern format: {}",
                pattern
            );
            return;
        }

        let inner = &trimmed[1..trimmed.len() - 1];
        if inner.is_empty() {
            println!("Rhythm::apply_ai_pattern: empty pattern body: {}", pattern);
            return;
        }

        if !inner.chars().all(|c| c == 'O' || c == 'X') {
            println!(
                "Rhythm::apply_ai_pattern: pattern contains invalid characters: {}",
                pattern
            );
            return;
        }

        let capacity = inner.chars().count();
        let mut wings = Vec::new();
        for (idx, ch) in inner.chars().enumerate() {
            if ch == 'X' {
                wings.push(idx);
            }
        }

        self.params.capacity = capacity;
        self.params.wings = wings;
        self.params.num_wings = self.params.wings.len();
        self.params.wings_buffer.clear();
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
        self.params.cutoff_range = range; // Note: cutoff maps to pitch_range internally
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

    /// Slot parameters are pre-initialized up to NUM_SLOTS
    pub fn initialize_slots(&mut self, rng: &mut ThreadRng) {
        for _ in 0..NUM_SLOTS {
            let slot = self.roll_slot(rng);
            self.params.slot_params.push(slot);
        }
    }

    fn roll_wings(rng: &mut ThreadRng, capacity: usize, num_wings: usize) -> Vec<usize> {
        let mut nums: Vec<usize> = (0..capacity).collect();
        nums.shuffle(rng);
        nums.truncate(num_wings);
        nums
    }

    pub fn roll_slot(&mut self, rng: &mut ThreadRng) -> RhythmSlotParams {
        let length = rng.random_range(self.params.length_range.to_range_inclusive());
        let velocity = rng.random_range(self.params.velocity_range.to_range_inclusive());
        let cutoff = rng.random_range(self.params.cutoff_range.to_range_inclusive());
        RhythmSlotParams {
            length,
            velocity,
            cutoff,
        }
    }

    pub fn set_all_slot_velocity(&mut self, val: f32) {
        for slot in &mut self.params.slot_params {
            slot.velocity = val;
        }
    }

    pub fn set_all_slot_length(&mut self, val: f32) {
        for slot in &mut self.params.slot_params {
            slot.length = val;
        }
    }

    pub fn set_all_slot_cutoff(&mut self, val: f32) {
        for slot in &mut self.params.slot_params {
            slot.cutoff = val;
        }
    }

    /// Randomize all slots' length within a range
    pub fn randomize_all_slots_length(&mut self, range: RangeSize, rng: &mut ThreadRng) {
        let skew_params = range.to_skew_distribution_params();
        if let Ok(skew_normal) = SkewNormal::new(skew_params.0, skew_params.1, skew_params.2) {
            for slot in &mut self.params.slot_params {
                slot.length = skew_normal.sample(rng);
            }
        } else {
            for slot in &mut self.params.slot_params {
                println!("failed at skew");

                slot.length = rng.random_range(range.to_range_inclusive());
            }
        }
    }

    /// Randomize all slots' velocity within a range
    pub fn randomize_all_slots_velocity(&mut self, range: RangeSize, rng: &mut ThreadRng) {
        let skew_params = range.to_skew_distribution_params();
        if let Ok(skew_normal) = SkewNormal::new(skew_params.0, skew_params.1, skew_params.2) {
            for slot in &mut self.params.slot_params {
                slot.velocity = skew_normal.sample(rng);
            }
        } else {
            for slot in &mut self.params.slot_params {
                println!("failed at skew");

                slot.velocity = rng.random_range(range.to_range_inclusive());
            }
        }
    }

    /// Randomize all slots' cutoff within a range
    pub fn randomize_all_slots_cutoff(&mut self, range: RangeSize, rng: &mut ThreadRng) {
        let skew_params = range.to_skew_distribution_params();
        if let Ok(skew_normal) = SkewNormal::new(skew_params.0, skew_params.1, skew_params.2) {
            for slot in &mut self.params.slot_params {
                slot.cutoff = skew_normal.sample(rng);
            }
        } else {
            for slot in &mut self.params.slot_params {
                println!("failed at skew");
                slot.cutoff = rng.random_range(range.to_range_inclusive());
            }
        }
    }

    /// Modify all slots' length based on ParameterModification
    pub fn modify_all_slots_length(
        &mut self,
        modification: crate::terminals::commands::rhythm::RhythmParamModification,
        rng: &mut ThreadRng,
    ) {
        use crate::terminals::commands::rhythm::RhythmParamModification;
        match modification {
            RhythmParamModification::Absolute(value) => {
                self.set_all_slot_length(value.clamp(0.0, 1.0));
            }
            RhythmParamModification::Relative(delta) => {
                for slot in &mut self.params.slot_params {
                    slot.length = (slot.length + delta).clamp(0.0, 1.0);
                }
            }
            RhythmParamModification::Randomize(range) => {
                self.randomize_all_slots_length(range, rng);
            }
        }
    }

    /// Modify all slots' velocity based on ParameterModification
    pub fn modify_all_slots_velocity(
        &mut self,
        modification: crate::terminals::commands::rhythm::RhythmParamModification,
        rng: &mut ThreadRng,
    ) {
        use crate::terminals::commands::rhythm::RhythmParamModification;
        match modification {
            RhythmParamModification::Absolute(value) => {
                self.set_all_slot_velocity(value.clamp(0.0, 1.0));
            }
            RhythmParamModification::Relative(delta) => {
                for slot in &mut self.params.slot_params {
                    slot.velocity = (slot.velocity + delta).clamp(0.0, 1.0);
                }
            }
            RhythmParamModification::Randomize(range) => {
                self.randomize_all_slots_velocity(range, rng);
            }
        }
    }

    /// Modify all slots' cutoff based on ParameterModification
    pub fn modify_all_slots_cutoff(
        &mut self,
        modification: crate::terminals::commands::rhythm::RhythmParamModification,
        rng: &mut ThreadRng,
    ) {
        use crate::terminals::commands::rhythm::RhythmParamModification;
        match modification {
            RhythmParamModification::Absolute(value) => {
                self.set_all_slot_cutoff(value.clamp(0.0, 1.0));
            }
            RhythmParamModification::Relative(delta) => {
                for slot in &mut self.params.slot_params {
                    slot.cutoff = (slot.cutoff + delta).clamp(0.0, 1.0);
                }
            }
            RhythmParamModification::Randomize(range) => {
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
