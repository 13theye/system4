use nannou::prelude::*;
use std::collections::HashMap;

use crate::{
    groups::rhythm::RhythmParams,
    utils::tween,
    view::{RhythmElement, RhythmFormation, RhythmFormationState, RhythmViewUpdateParams},
};

const RECT_DEFAULT_R: f32 = 0.7;
const RECT_DEFAULT_G: f32 = 0.7;
const RECT_DEFAULT_B: f32 = 0.7;
const RECT_DEFAULT_A: f32 = 1.0;
const RECT_HIGH_R: f32 = 1.0;
const RECT_HIGH_G: f32 = 0.0;
const RECT_HIGH_B: f32 = 0.0;
const RECT_LOW_R: f32 = 0.2;
const RECT_LOW_G: f32 = 0.2;
const RECT_LOW_B: f32 = 0.2;
const RECT_DEFAULT_WIDTH: f32 = 10.0;
const RECT_DEFAULT_HEIGHT: f32 = 10.0;
const RECT_MAX_WIDTH: f32 = 200.0;
const RECT_MAX_HEIGHT: f32 = 100.0;

// Animation timing constants
const RAMP_UP_PERCENT: f32 = 0.1;
const DWELL_PERCENT: f32 = 0.3;
const RAMP_CURVE_EXPONENT: f32 = 3.0;
const FADE_CURVE_EXPONENT: f32 = 1.5;

// Formation state animation durations (in seconds)
const INIT_ANIMATION_DURATION: f32 = 1.0;
const REINIT_ANIMATION_DURATION: f32 = 0.8;
const CLEAR_ANIMATION_DURATION: f32 = 1.0;

pub struct RhythmCircleFormation {
    pub center: Vec2,
    pub capacity: usize,
    pub radius: f32,
    pub elements: HashMap<usize, Box<dyn RhythmElement>>, // HashMap<wing, <RhythmRect>,
    state: RhythmFormationState,
    last_state_change: f32,
}

impl RhythmFormation for RhythmCircleFormation {
    fn center(&self) -> Vec2 {
        self.center
    }

    fn capacity(&self) -> usize {
        self.capacity
    }

    fn initialize_rhythm(&mut self, rhythm_params: &RhythmParams, time: f32) {
        self.capacity = rhythm_params.capacity;
        let positions = Self::initialize_positions(self.radius, self.capacity);

        for i in 0..self.capacity {
            let Some(target_pos) = positions.get(&i) else {
                continue;
            };
            let mut new_element = RhythmRect::new();
            // Elements start at center and animate to their circle positions
            new_element.set_animation_positions(self.center, *target_pos);
            new_element.set_is_wing(rhythm_params.wings.contains(&i));
            // Add element to the RhythmCircleFormation
            self.elements.insert(i, Box::new(new_element));
        }

        self.change_state(RhythmFormationState::Initializing, time);
    }

    fn reinitialize_rhythm(&mut self, rhythm_params: &RhythmParams, time: f32) {
        let new_capacity = rhythm_params.capacity;
        let old_capacity = self.capacity;

        // Calculate new target positions for the new capacity
        let new_positions = Self::initialize_positions(self.radius, new_capacity);

        // Update capacity
        self.capacity = new_capacity;

        // Add or remove elements
        if new_capacity > old_capacity {
            // Add new elements
            for i in old_capacity..new_capacity {
                self.elements.insert(i, Box::new(RhythmRect::new()));
            }
        } else if new_capacity < old_capacity {
            // Remove excess elements
            for i in new_capacity..old_capacity {
                self.elements.remove(&i);
            }
        }

        // For all elements, update their target positions and set start to current position
        // and update wing status
        for i in 0..new_capacity {
            if let Some(element) = self.elements.get_mut(&i) {
                element.set_is_wing(rhythm_params.wings.contains(&i));

                if let Some(&new_target) = new_positions.get(&i) {
                    let current_pos = element.position();
                    element.set_animation_positions(current_pos, new_target);
                }
            }
        }

        // Transition to Reinitializing state
        self.change_state(RhythmFormationState::Reinitializing, time);
    }

    fn clear_rhythm(&mut self, time: f32) {
        // For all existing elements, set their target position to center
        // and their start position to current position
        for rect in self.elements.values_mut() {
            let current_pos = rect.position();
            rect.set_animation_positions(current_pos, self.center);
        }

        // Transition to Clearing state
        self.change_state(RhythmFormationState::Clearing, time);
    }

    fn update_transitions(&mut self, time: f32) {
        // Handle state transition animations (Initializing, Reinitializing, Clearing)
        match self.state {
            RhythmFormationState::Inactive | RhythmFormationState::Active => {
                // No transitions to update
            }
            RhythmFormationState::Initializing => {
                let progress = ((time - self.last_state_change) / INIT_ANIMATION_DURATION).min(1.0);

                // Animate elements from center to their target positions
                for element in self.elements.values_mut() {
                    let start = element.start_position();
                    let target = element.target_position();
                    // Use cubic ease-out for smooth deceleration
                    let eased_progress = 1.0 - (1.0 - progress).powi(3);
                    let new_pos = start + (target - start) * eased_progress;
                    element.set_position(new_pos);
                }

                // Transition to Active when animation completes
                if progress >= 1.0 {
                    self.change_state(RhythmFormationState::Active, time);
                }
            }
            RhythmFormationState::Reinitializing => {
                let progress =
                    ((time - self.last_state_change) / REINIT_ANIMATION_DURATION).min(1.0);

                // Animate elements to their new target positions
                for element in self.elements.values_mut() {
                    let start = element.start_position();
                    let target = element.target_position();
                    // Use cubic ease-out for smooth deceleration
                    let eased_progress = 1.0 - (1.0 - progress).powi(3);
                    let new_pos = start + (target - start) * eased_progress;
                    element.set_position(new_pos);
                }

                // Transition to Active when animation completes
                if progress >= 1.0 {
                    self.change_state(RhythmFormationState::Active, time);
                }
            }
            RhythmFormationState::Clearing => {
                let progress =
                    ((time - self.last_state_change) / CLEAR_ANIMATION_DURATION).min(1.0);

                // Animate elements from current position back to center
                for element in self.elements.values_mut() {
                    let start = element.start_position();
                    let target = element.target_position(); // center
                                                            // Use cubic ease-out for smooth deceleration
                    let eased_progress = 1.0 - (1.0 - progress).powi(3);
                    let new_pos = start + (target - start) * eased_progress;
                    element.set_position(new_pos);
                }

                // Transition to Inactive when animation completes
                if progress >= 1.0 {
                    self.change_state(RhythmFormationState::Inactive, time);
                }
            }
        }
    }

    fn update_active(
        &mut self,
        rhythm_params: &RhythmParams,
        update_params: &RhythmViewUpdateParams,
        time: f32,
    ) {
        // Don't run this if Inactive
        if matches!(self.state, RhythmFormationState::Inactive) {
            return;
        }

        // Update elements
        for (slot, element) in self.elements.iter_mut() {
            element.update(rhythm_params, update_params, *slot, time);
        }
    }

    fn draw(&self, draw: &Draw) {
        // Don't draw if inactive
        if matches!(self.state, RhythmFormationState::Inactive) {
            return;
        }

        // Draw elements
        for element in self.elements.values() {
            element.draw(draw);
        }
    }
}

impl RhythmCircleFormation {
    pub fn new(center: Vec2, radius: f32, capacity: usize, time: f32) -> Self {
        Self {
            center,
            radius,
            capacity,
            elements: HashMap::new(),
            state: RhythmFormationState::Inactive,
            last_state_change: time,
        }
    }

    fn initialize_positions(radius: f32, capacity: usize) -> HashMap<usize, Vec2> {
        let mut positions = HashMap::new();
        for i in 0..capacity {
            let angle = std::f32::consts::FRAC_PI_2
                - (i as f32) * 2.0 * std::f32::consts::PI / (capacity as f32);
            let x = radius * angle.cos();
            let y = radius * angle.sin();
            positions.insert(i, Vec2::new(x, y));
        }
        positions
    }

    fn change_state(&mut self, state: RhythmFormationState, time: f32) {
        self.state = state;
        self.last_state_change = time;
    }
}

#[derive(Debug)]
pub struct RhythmRect {
    /// Whether the element is an activated wing
    pub(crate) is_wing: bool,
    /// Current animated screen position
    pub(crate) pos: Vec2,
    /// Target position for animation
    pub(crate) target_pos: Vec2,
    /// Start position for animation
    pub(crate) start_pos: Vec2,
    /// Width and Length
    pub(crate) dims: Vec2,
    /// Color in Nannou Rgb
    pub(crate) color: Rgb,
    /// Alpha value
    pub(crate) alpha: f32,
    /// Last update time
    pub(crate) last_active_time: f32,
}

impl RhythmElement for RhythmRect {
    fn position(&self) -> Vec2 {
        self.pos
    }

    fn target_position(&self) -> Vec2 {
        self.target_pos
    }

    fn start_position(&self) -> Vec2 {
        self.start_pos
    }

    fn set_position(&mut self, pos: Vec2) {
        self.pos = pos;
    }

    fn set_animation_positions(&mut self, start: Vec2, target: Vec2) {
        self.start_pos = start;
        self.target_pos = target;
    }

    fn set_last_active_time(&mut self, time: f32) {
        self.last_active_time = time;
    }

    fn last_update_time(&self) -> f32 {
        self.last_active_time
    }

    fn set_is_wing(&mut self, is_wing: bool) {
        self.is_wing = is_wing;
    }

    fn update(
        &mut self,
        rhythm_params: &RhythmParams,
        update_params: &RhythmViewUpdateParams,
        slot: usize,
        time: f32,
    ) {
        // Update last active time of the current wing's Rect
        if let Some(current_slot) = update_params.current_slot {
            if slot == current_slot {
                self.last_active_time = time;
            }
        }

        // Calculate the interpolation duration
        let wing_duration =
            ((60.0 / update_params.tempo) / update_params.subdivision.multiplier()) as f32;

        // Determine color range based on whether this is a wing or not
        let (start_color, end_color) = if self.is_wing {
            // Wings: flash from default to high (medium gray to red)
            (
                rgb(RECT_DEFAULT_R, RECT_DEFAULT_G, RECT_DEFAULT_B),
                rgb(RECT_HIGH_R, RECT_HIGH_G, RECT_HIGH_B),
            )
        } else {
            // Non-wings: flash from low to default (dark gray to medium gray)
            (
                rgb(RECT_LOW_R, RECT_LOW_G, RECT_LOW_B),
                rgb(RECT_DEFAULT_R, RECT_DEFAULT_G, RECT_DEFAULT_B),
            )
        };

        self.color = tween::interpolate_color(
            start_color,
            end_color,
            wing_duration,
            RAMP_UP_PERCENT,
            DWELL_PERCENT,
            RAMP_CURVE_EXPONENT,
            FADE_CURVE_EXPONENT,
            time,
            self.last_active_time,
        );

        // Get slot parameters for scaling
        let rhythm_slot = rhythm_params.slots.get(slot);
        let length_scale = rhythm_slot.map(|s| s.length).unwrap_or(1.0);
        let velocity_scale = rhythm_slot.map(|s| s.velocity).unwrap_or(1.0);

        let (base_width, base_height) = if self.is_wing {
            (
                RECT_MAX_WIDTH * length_scale,
                RECT_MAX_HEIGHT * velocity_scale,
            )
        } else {
            (RECT_DEFAULT_WIDTH, RECT_DEFAULT_HEIGHT)
        };

        let (mod_width, mod_height) = if self.is_wing {
            ((base_width * 2.0), (base_height * 2.0))
        } else {
            (base_width, base_height)
        };

        // Interpolate dims with slot parameter scaling
        let width = tween::interpolate_dimension(
            mod_width,
            base_width,
            wing_duration,
            RAMP_UP_PERCENT,
            DWELL_PERCENT,
            RAMP_CURVE_EXPONENT,
            FADE_CURVE_EXPONENT,
            time,
            self.last_active_time,
        );

        let height = tween::interpolate_dimension(
            mod_height,
            base_height,
            wing_duration,
            RAMP_UP_PERCENT,
            DWELL_PERCENT,
            RAMP_CURVE_EXPONENT,
            FADE_CURVE_EXPONENT,
            time,
            self.last_active_time,
        );

        self.dims = Vec2::new(width, height);
    }

    fn draw(&self, draw: &Draw) {
        draw.rect()
            .x_y(self.pos.x, self.pos.y)
            .w_h(self.dims.x, self.dims.y)
            .color(rgba(
                self.color.red,
                self.color.green,
                self.color.blue,
                self.alpha,
            ));
    }
}

impl RhythmRect {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }
}

impl Default for RhythmRect {
    fn default() -> Self {
        Self {
            is_wing: false,
            pos: Vec2::new(0.0, 0.0),
            target_pos: Vec2::new(0.0, 0.0),
            start_pos: Vec2::new(0.0, 0.0),
            dims: Vec2::new(RECT_DEFAULT_WIDTH, RECT_DEFAULT_HEIGHT),
            color: Rgb::new(RECT_LOW_R, RECT_LOW_G, RECT_LOW_B),
            alpha: RECT_DEFAULT_A,
            last_active_time: 0.0,
        }
    }
}

fn scale_dims(dims: Vec2, length: f32, velocity: f32) -> Vec2 {
    vec2(dims.x * length, dims.y * velocity)
}
