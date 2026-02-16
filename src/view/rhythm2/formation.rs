use nannou::prelude::*;
use rand::rngs::ThreadRng;
use std::{collections::HashMap, time::Instant};

use super::{
    activation::ActivationElement,
    animation::*,
    element::{RhythmElement, RhythmElementMovement},
};
use crate::{
    groups::{RhythmParams, VoiceId},
    view::rhythm::RhythmViewUpdateParams,
};

/// The state of the `RhythmFormation`
#[derive(Copy, Clone, Debug)]
pub enum RhythmFormationState {
    Inactive,
    Active { start_time: Instant },
    Initializing { start_time: Instant },
    Reinitializing { start_time: Instant },
    Clearing { start_time: Instant },
}

/// Which side of the screen the formation is on
#[derive(Copy, Clone, Debug)]
pub enum RhythmFormationSide {
    Left,
    Right,
}

/// Parameters for the `RhythmFormation` used for drawing
#[derive(Debug)]
pub struct RhythmFormationParams {
    // The center of the formation
    pub center: Vec2,
    // The radius of the circle that contains the centers of inactive elements
    pub min_radius: f32,
    // The radius of the circle that contains the centers of active elements with velocity = 1.0
    pub max_radius: f32,
}

impl Default for RhythmFormationParams {
    fn default() -> Self {
        Self {
            center: Vec2::new(0.0, 0.0),
            min_radius: MIN_FORMATION_RADIUS,
            max_radius: MAX_FORMATION_RADIUS,
        }
    }
}

/// A `HashMap` of `RhythmElement`s where:
/// - Key is the rhythm slot index.
/// - Value is the `RhythmElement`
pub type ElementMap = HashMap<usize, RhythmElement>;

/// A `HashMap` of `RhythmElement`s where:
/// - Key is the rhythm slot index.
/// - Value is the `RhythmElementActivation`
pub type ActivationMap = HashMap<usize, ActivationElement>;

/// A structure representing a Rhythm, comprised of `RhythmElement`s and `ActivationElement`s
#[derive(Debug)]
pub struct RhythmFormation {
    pub voice_id: VoiceId,
    pub side: RhythmFormationSide,
    pub params: RhythmFormationParams,
    pub capacity: usize,
    pub elements: ElementMap,
    pub activations: ActivationMap,
    pub state: RhythmFormationState,
}

impl RhythmFormation {
    /// Initialize a new rhythm formation from given `RhythmParams`
    pub fn init_rhythm(
        voice_id: VoiceId,
        rhythm_params: &RhythmParams,
        now: Instant,
    ) -> Option<Self> {
        let Some(side) = Self::get_side(voice_id) else {
            eprintln!(
                "RhythmFormation: VoiceId: {} shouldn't be a rhythm. No formation will be created",
                voice_id
            );
            return None;
        };

        let params = RhythmFormationParams::default();
        let elements = Self::init_elements(side, rhythm_params, &params, now);

        Some(Self {
            voice_id,
            side,
            params,
            capacity: rhythm_params.capacity,
            elements,
            activations: HashMap::new(),
            state: RhythmFormationState::Initializing { start_time: now },
        })
    }

    /// Initialize all `RhythmElements` for the given `RhythmParams`
    fn init_elements(
        side: RhythmFormationSide,
        rhythm_params: &RhythmParams,
        formation_params: &RhythmFormationParams,
        now: Instant,
    ) -> ElementMap {
        let mut element_map: ElementMap = HashMap::new();

        // Calculate element positions
        let min_positions = Self::calculate_formation_positions(
            side,
            rhythm_params.capacity,
            formation_params.min_radius,
        );

        let initial_position = vec2(0.0, 0.0);

        let mut rng = ThreadRng::default();

        // Iterate through the positions and create elements for each.
        min_positions.iter().for_each(|(i, min_position)| {
            let slot_params = if let Some(&p) = rhythm_params.slot_params.get(*i) {
                p
            } else {
                // if for some reason slot params are missing, fallback to default
                crate::groups::RhythmSlotParams::default()
            };

            let max_position = Self::calculate_formation_position(
                *i,
                side,
                rhythm_params.capacity,
                formation_params.max_radius,
            );

            let movement_duration = adjusted_duration(INIT_ANIMATION_DURATION, &mut rng);

            let mut element = RhythmElement::new(
                initial_position,
                *min_position,
                max_position,
                movement_duration,
                slot_params,
                now,
            );
            if rhythm_params.wings.contains(i) {
                element.set_is_wing(movement_duration, now);
            }
            element_map.insert(*i, element);
        });

        element_map
    }

    /// Helper function to get the side of the screen that this `RhythmFormation` is on
    fn get_side(voice_id: VoiceId) -> Option<RhythmFormationSide> {
        match voice_id {
            VoiceId::Voice1 => Some(RhythmFormationSide::Left),
            VoiceId::Voice2 => Some(RhythmFormationSide::Right),
            _ => None,
        }
    }

    /// Regenerate this `RhythmFormation` from given `RhythmParams`
    pub fn reinit_rhythm(&mut self, rhythm_params: &RhythmParams, now: Instant) {
        let (new_capacity, old_capacity) = (rhythm_params.capacity, self.capacity);

        // Calculate new target positions for the new capacity
        let new_min_positions = Self::calculate_formation_positions(
            self.side,
            rhythm_params.capacity,
            self.params.min_radius,
        );

        // Update capacity
        self.capacity = new_capacity;

        let mut rng = ThreadRng::default();

        // Add new elements
        if new_capacity > old_capacity {
            let initial_position = vec2(0.0, 0.0);

            for i in old_capacity..new_capacity {
                let Some(min_position) = new_min_positions.get(&i) else {
                    continue;
                };
                let max_position = Self::calculate_formation_position(
                    i,
                    self.side,
                    rhythm_params.capacity,
                    self.params.max_radius,
                );
                let Some(&slot_params) = rhythm_params.slot_params.get(i) else {
                    continue;
                };

                let movement_duration: f32 = adjusted_duration(INIT_ANIMATION_DURATION, &mut rng);

                self.elements.insert(
                    i,
                    RhythmElement::new(
                        initial_position,
                        *min_position,
                        max_position,
                        movement_duration,
                        slot_params,
                        now,
                    ),
                );
            }
        } else if new_capacity < old_capacity {
            // Remove excess elements
            for i in new_capacity..old_capacity {
                let Some(element) = self.elements.get_mut(&i) else {
                    continue;
                };

                element.set_clearing(self.params.center, CLEAR_ANIMATION_DURATION, now);
            }
        }

        // Move all elements to the new positions and update formation_position and wing_position
        for (i, element) in self.elements.iter_mut() {
            let Some(new_position) = new_min_positions.get(i) else {
                // Elements that don't have a new position were marked for clearing above, and are skipped here.
                continue;
            };

            // Update slot params
            if let Some(p) = rhythm_params.slot_params.get(*i) {
                element.params.slot = *p;
            } else {
                // if for some reason the slot params are missing, fallback to default
                element.params.slot = crate::groups::RhythmSlotParams::default();
            }

            let max_position = Self::calculate_formation_position(
                *i,
                self.side,
                rhythm_params.capacity,
                self.params.max_radius,
            );
            element.params.min_position = *new_position;
            element.params.max_position = max_position;

            let movement_duration: f32 = adjusted_duration(INIT_ANIMATION_DURATION, &mut rng);

            element.movement = RhythmElementMovement::Moving {
                start_pos: element.params.current_position,
                target_pos: *new_position,
                start_time: now,
                duration: movement_duration,
            };
        }

        // Update wing status of elements within the new capacity
        self.elements.iter_mut().for_each(|(i, element)| {
            // Skip elements that are marked for clearing
            if *i >= self.capacity {
                return;
            }
            if rhythm_params.wings.contains(i) {
                let movement_duration: f32 = adjusted_duration(INIT_ANIMATION_DURATION, &mut rng);

                element.set_is_wing(movement_duration, now);
            } else {
                let movement_duration: f32 = adjusted_duration(INIT_ANIMATION_DURATION, &mut rng);

                element.set_is_not_wing(movement_duration, now);
            }
        });

        // Set own state flag
        self.state = RhythmFormationState::Reinitializing { start_time: now };
    }

    /// Initiate a clearing animation
    pub fn clear_rhythm(&mut self, now: Instant) {
        let target_pos = self.params.center;

        self.elements.values_mut().for_each(|element| {
            element.set_clearing(target_pos, CLEAR_ANIMATION_DURATION, now);
        });

        self.state = RhythmFormationState::Clearing { start_time: now };
    }

    /// Update RhythmElements' positions and sizes based on updated RhythmParams
    /// This should be called when slot parameters (length, velocity, cutoff) are modified
    pub fn update_element_params(&mut self, rhythm_params: &RhythmParams, now: Instant) {
        let mut rng = ThreadRng::default();

        for (i, element) in self.elements.iter_mut() {
            // Skip elements beyond capacity
            if *i >= rhythm_params.capacity {
                continue;
            }

            // Update slot params from RhythmParams
            if let Some(new_slot_params) = rhythm_params.slot_params.get(*i) {
                element.params.slot = *new_slot_params;
            }

            if element.is_wing {
                // Move to updated wing position
                let movement_duration =
                    adjusted_duration(WINGS_REINIT_ANIMATION_DURATION, &mut rng);
                element.set_is_wing(movement_duration, now);
            }
        }
    }

    /// Helper function to calculate all center positions of a rhythm along the formation semicircle of a given radius
    fn calculate_formation_positions(
        side: RhythmFormationSide,
        capacity: usize,
        radius: f32,
    ) -> HashMap<usize, Vec2> {
        let mut formation_positions = HashMap::new();
        for i in 0..capacity {
            let pos = Self::calculate_formation_position(i, side, capacity, radius);
            formation_positions.insert(i, pos);
        }
        formation_positions
    }

    /// Helper function to calculate the center position of a slot along the formation semicircle of a given radius
    fn calculate_formation_position(
        slot: usize,
        side: RhythmFormationSide,
        capacity: usize,
        radius: f32,
    ) -> Vec2 {
        let unit_angle = std::f32::consts::PI / (capacity as f32);

        let angle = match side {
            RhythmFormationSide::Left => {
                std::f32::consts::FRAC_PI_2 + unit_angle / 2.0 + (slot as f32) * unit_angle
            }
            RhythmFormationSide::Right => {
                std::f32::consts::FRAC_PI_2 - unit_angle / 2.0 - (slot as f32) * unit_angle
            }
        };

        let x = radius * angle.cos();
        let y = radius * angle.sin();

        vec2(x, y)
    }

    pub fn update(&mut self, update_params: &RhythmViewUpdateParams, now: Instant) {
        // Update elements
        self.elements.iter_mut().for_each(|(i, element)| {
            element.update_animations(now);

            if matches!(self.state, RhythmFormationState::Inactive) {
                return;
            }

            let Some(active_slot) = update_params.current_slot else {
                return;
            };

            if *i == active_slot && element.is_wing {
                let activation = ActivationElement::new(
                    update_params.tempo,
                    self.side,
                    &element.params,
                    *i,
                    now,
                );
                self.activations.insert(active_slot, activation);
            }
        });

        // Update activations
        let mut to_clear: Vec<usize> = Vec::with_capacity(self.activations.len());
        self.activations.iter_mut().for_each(|(i, activation)| {
            activation.update(now);
            if activation.is_done() {
                to_clear.push(*i);
            }
        });
        to_clear.iter().for_each(|i| {
            self.activations.remove(i);
        });
    }

    /// Update animations when RhythmFormation's parent rhythm doesn't exist because of a clearing or
    /// initialization operation in progress
    pub fn update_transitions(&mut self, now: Instant) {
        // Clear any elements marked for removal
        let mut to_clear: Vec<usize> = Vec::with_capacity(self.elements.len());
        self.elements.iter().for_each(|(i, element)| {
            if element.is_ready_to_clear() {
                to_clear.push(*i);
            }
        });
        to_clear.iter().for_each(|i| {
            self.elements.remove(i);
        });

        // Do nothing if formation is inactive.
        // If formation is active, animations are updated in `update()`
        if matches!(self.state, RhythmFormationState::Inactive)
            || matches!(self.state, RhythmFormationState::Active { .. })
        {
            return;
        }

        // Update animations
        self.elements.values_mut().for_each(|element| {
            element.update_animations(now);
        });

        match self.state {
            RhythmFormationState::Initializing { start_time }
            | RhythmFormationState::Reinitializing { start_time } => {
                let progress =
                    ((now - start_time).as_secs_f32() / INIT_ANIMATION_DURATION).min(1.0);

                if progress >= 1.0 {
                    self.state = RhythmFormationState::Active { start_time: now };
                }
            }

            RhythmFormationState::Clearing { start_time } => {
                let progress =
                    ((now - start_time).as_secs_f32() / CLEAR_ANIMATION_DURATION).min(1.0);

                if progress >= 1.0 {
                    self.state = RhythmFormationState::Inactive;
                }
            }
            _ => {}
        }
    }

    pub fn draw_elements(&self, draw: &Draw, show_debug_geometry: bool) {
        // Draw connectors first so elements are layered on top
        super::connector::draw_connectors(draw, &self.elements, self.capacity, show_debug_geometry);

        for element in self.elements.values() {
            element.draw(draw);
        }
    }

    pub fn draw_activations(&self, draw: &Draw) {
        self.activations.iter().for_each(|(i, activation)| {
            let Some(element) = self.elements.get(i) else {
                return;
            };
            activation.draw(draw, &element.params);
        });
    }
}
