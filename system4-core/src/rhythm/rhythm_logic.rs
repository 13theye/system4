// system4-core/src/rhythm/rhythm_logic.rs
// Core rhythm logic - pure algorithms without app dependencies

use crate::commands::rhythm::{ParameterModification, RangeSize};
use prat::BeatSubdivision;
use rand::{seq::SliceRandom, Rng};
use rand_distr::{Distribution, SkewNormal};

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

/// Core rhythm logic - pure algorithms without app dependencies
pub struct RhythmLogic {
    params: RhythmParams,
}

impl RhythmLogic {
    pub fn new(params: RhythmParams) -> Self {
        Self { params }
    }

    pub fn new_default() -> Self {
        Self {
            params: RhythmParams::default(),
        }
    }

    /// Get a reference to the parameters
    pub fn params(&self) -> &RhythmParams {
        &self.params
    }

    /// Get a mutable reference to the parameters
    pub fn params_mut(&mut self) -> &mut RhythmParams {
        &mut self.params
    }

    /// Set the entire params structure
    pub fn set_params(&mut self, params: RhythmParams) {
        self.params = params;
    }

    /// Clear all parameters to default
    pub fn clear_params(&mut self) {
        self.params = RhythmParams::default();
    }

    /*************** Beat logic helpers *************************** */

    /// Check if a beat is in the active wings
    pub fn wing_has_beat(&self, beat: usize) -> bool {
        self.params.wings.contains(&beat)
    }

    /*************** Wing management *************************** */

    /// Add back wings from buffer, or generate additional wings as needed
    pub fn add_wings<R: Rng>(&mut self, number_to_add: usize, rng: &mut R) {
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

    /// Randomize wings positions
    pub fn randomize_wings<R: Rng>(&mut self, rng: &mut R) {
        self.params.wings = Self::roll_wings(rng, self.params.capacity, self.params.num_wings);
    }

    /// Generate random wing positions
    fn roll_wings<R: Rng>(rng: &mut R, capacity: usize, num_wings: usize) -> Vec<usize> {
        let mut nums: Vec<usize> = (0..capacity).collect();
        nums.shuffle(rng);
        nums.truncate(num_wings);
        nums
    }

    /*************** Slot management *************************** */

    /// Initialize slots with random values based on current ranges
    pub fn initialize_slots<R: Rng>(&mut self, num_slots: usize, rng: &mut R) {
        self.params.slots.clear();
        for _ in 0..num_slots {
            let slot = self.roll_slot(rng);
            self.params.slots.push(slot);
        }
    }

    /// Generate a random slot based on current parameter ranges
    pub fn roll_slot<R: Rng>(&self, rng: &mut R) -> RhythmSlot {
        let length = rng.random_range(self.params.length_range.to_range_inclusive());
        let velocity = rng.random_range(self.params.velocity_range.to_range_inclusive());
        let pitch = rng.random_range(self.params.pitch_range.to_range_inclusive());
        RhythmSlot {
            length,
            velocity,
            cutoff: pitch,
        }
    }

    /// Set all slots' velocity to a specific value
    pub fn set_all_slot_velocity(&mut self, val: f32) {
        for slot in &mut self.params.slots {
            slot.velocity = val;
        }
    }

    /// Set all slots' length to a specific value
    pub fn set_all_slot_length(&mut self, val: f32) {
        for slot in &mut self.params.slots {
            slot.length = val;
        }
    }

    /// Set all slots' cutoff to a specific value
    pub fn set_all_slot_cutoff(&mut self, val: f32) {
        for slot in &mut self.params.slots {
            slot.cutoff = val;
        }
    }

    /// Randomize all slots' length within a range using SkewNormal distribution
    pub fn randomize_all_slots_length<R: Rng>(&mut self, range: RangeSize, rng: &mut R) {
        let skew_params = range.to_skew_distribution_params();
        if let Ok(skew_normal) = SkewNormal::new(skew_params.0, skew_params.1, skew_params.2) {
            for slot in &mut self.params.slots {
                slot.length = skew_normal.sample(rng);
            }
        } else {
            // Fallback to uniform distribution if SkewNormal fails
            for slot in &mut self.params.slots {
                slot.length = rng.random_range(range.to_range_inclusive());
            }
        }
    }

    /// Randomize all slots' velocity within a range using SkewNormal distribution
    pub fn randomize_all_slots_velocity<R: Rng>(&mut self, range: RangeSize, rng: &mut R) {
        let skew_params = range.to_skew_distribution_params();
        if let Ok(skew_normal) = SkewNormal::new(skew_params.0, skew_params.1, skew_params.2) {
            for slot in &mut self.params.slots {
                slot.velocity = skew_normal.sample(rng);
            }
        } else {
            // Fallback to uniform distribution if SkewNormal fails
            for slot in &mut self.params.slots {
                slot.velocity = rng.random_range(range.to_range_inclusive());
            }
        }
    }

    /// Randomize all slots' cutoff within a range using SkewNormal distribution
    pub fn randomize_all_slots_cutoff<R: Rng>(&mut self, range: RangeSize, rng: &mut R) {
        let skew_params = range.to_skew_distribution_params();
        if let Ok(skew_normal) = SkewNormal::new(skew_params.0, skew_params.1, skew_params.2) {
            for slot in &mut self.params.slots {
                slot.cutoff = skew_normal.sample(rng);
            }
        } else {
            // Fallback to uniform distribution if SkewNormal fails
            for slot in &mut self.params.slots {
                slot.cutoff = rng.random_range(range.to_range_inclusive());
            }
        }
    }

    /// Modify all slots' length based on ParameterModification
    pub fn modify_all_slots_length<R: Rng>(&mut self, modification: ParameterModification, rng: &mut R) {
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
    pub fn modify_all_slots_velocity<R: Rng>(&mut self, modification: ParameterModification, rng: &mut R) {
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
    pub fn modify_all_slots_cutoff<R: Rng>(&mut self, modification: ParameterModification, rng: &mut R) {
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

    /*************** Parameter accessors ****************************** */

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
}
