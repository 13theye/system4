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
const RECT_HIGH_A: f32 = 1.0;
const RECT_DEFAULT_WIDTH: f32 = 10.0;
const RECT_DEFAULT_HEIGHT: f32 = 10.0;
const RECT_MAX_WIDTH: f32 = 50.0;
const RECT_MAX_HEIGHT: f32 = 50.0;

// Animation timing constants
const RAMP_UP_PERCENT: f32 = 0.1;
const DWELL_PERCENT: f32 = 0.3;
const RAMP_CURVE_EXPONENT: f32 = 3.0;
const FADE_CURVE_EXPONENT: f32 = 1.5;

// Formation state animation durations (in seconds)
const INIT_ANIMATION_DURATION: f32 = 1.0;
const REINIT_ANIMATION_DURATION: f32 = 0.8;
const CLEAR_ANIMATION_DURATION: f32 = 1.0;

#[derive(Debug)]
pub struct RhythmCircleFormation {
    pub center: Vec2,
    pub capacity: usize,
    pub radius: f32,
    pub elements: HashMap<usize, RhythmRect>, // HashMap<wing, <RhythmRect>,
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
            // Elements start at center and animate to their circle positions
            self.elements
                .insert(i, RhythmRect::new_with_animation(self.center, *target_pos));
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

        if new_capacity > old_capacity {
            // Add new elements: they start at center and move to circle positions
            for i in old_capacity..new_capacity {
                if let Some(&target_pos) = new_positions.get(&i) {
                    self.elements
                        .insert(i, RhythmRect::new_with_animation(self.center, target_pos));
                }
            }
        } else if new_capacity < old_capacity {
            // Remove excess elements
            for i in new_capacity..old_capacity {
                self.elements.remove(&i);
            }
        }

        // For existing elements, update their target positions and set start to current position
        for i in 0..new_capacity.min(old_capacity) {
            if let Some(rect) = self.elements.get_mut(&i) {
                if let Some(&new_target) = new_positions.get(&i) {
                    let current_pos = rect.position();
                    rect.set_animation_positions(current_pos, new_target);
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
                for rect in self.elements.values_mut() {
                    let start = rect.start_position();
                    let target = rect.target_position();
                    // Use cubic ease-out for smooth deceleration
                    let eased_progress = 1.0 - (1.0 - progress).powi(3);
                    let new_pos = start + (target - start) * eased_progress;
                    rect.set_position(new_pos);
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
                for rect in self.elements.values_mut() {
                    let start = rect.start_position();
                    let target = rect.target_position();
                    // Use cubic ease-out for smooth deceleration
                    let eased_progress = 1.0 - (1.0 - progress).powi(3);
                    let new_pos = start + (target - start) * eased_progress;
                    rect.set_position(new_pos);
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
                for rect in self.elements.values_mut() {
                    let start = rect.start_position();
                    let target = rect.target_position(); // center
                                                         // Use cubic ease-out for smooth deceleration
                    let eased_progress = 1.0 - (1.0 - progress).powi(3);
                    let new_pos = start + (target - start) * eased_progress;
                    rect.set_position(new_pos);
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
        // Only update if in Active state
        if !matches!(self.state, RhythmFormationState::Active) {
            return;
        }

        // Active state: standard beat-based animation
        for (wing, rect) in self.elements.iter_mut() {
            // Update last update time of the current wing's Rect
            if let Some(current_wing) = update_params.current_wing {
                if *wing == current_wing {
                    rect.last_update_time = time;
                }
            }

            // Calculate the interpolation duration
            let wing_duration = ((update_params.subdivision.multiplier() / 2.0)
                * (60.0 / update_params.tempo)) as f32;

            rect.color = tween::interpolate_color(
                rgb(RECT_DEFAULT_R, RECT_DEFAULT_G, RECT_DEFAULT_B),
                rgb(RECT_HIGH_R, RECT_HIGH_G, RECT_HIGH_B),
                wing_duration,
                RAMP_UP_PERCENT,
                DWELL_PERCENT,
                RAMP_CURVE_EXPONENT,
                FADE_CURVE_EXPONENT,
                time,
                rect.last_update_time,
            );

            // Get slot parameters for scaling
            let slot = rhythm_params.slots.get(*wing);
            let length_scale = slot.map(|s| s.length).unwrap_or(1.0);
            let velocity_scale = slot.map(|s| s.velocity).unwrap_or(1.0);

            let (base_width, base_height) = if rhythm_params.wings.contains(wing) {
                (RECT_MAX_WIDTH, RECT_MAX_HEIGHT)
            } else {
                (RECT_DEFAULT_WIDTH, RECT_DEFAULT_HEIGHT)
            };

            // Interpolate dims with slot parameter scaling
            let width = tween::interpolate_dimension(
                base_width * (2.0 * length_scale),
                base_width,
                wing_duration,
                RAMP_UP_PERCENT,
                DWELL_PERCENT,
                RAMP_CURVE_EXPONENT,
                FADE_CURVE_EXPONENT,
                time,
                rect.last_update_time,
            );

            let height = tween::interpolate_dimension(
                base_height * (2.0 * velocity_scale),
                base_height,
                wing_duration,
                RAMP_UP_PERCENT,
                DWELL_PERCENT,
                RAMP_CURVE_EXPONENT,
                FADE_CURVE_EXPONENT,
                time,
                rect.last_update_time,
            );

            rect.dims = Vec2::new(width, height);
        }
    }

    fn draw(&self, draw: &Draw) {
        // Don't draw if inactive
        if matches!(self.state, RhythmFormationState::Inactive) {
            return;
        }

        for (_, rect) in self.elements.iter() {
            let pos = rect.pos;
            draw.rect()
                .x_y(pos.x, pos.y)
                .w_h(rect.dims.x, rect.dims.y)
                .color(rgba(
                    rect.color.red,
                    rect.color.green,
                    rect.color.blue,
                    rect.alpha,
                ));
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
            let angle = (i as f32) * 2.0 * std::f32::consts::PI / (capacity as f32);
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
    pub(crate) last_update_time: f32,
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

    fn dims(&self) -> Vec2 {
        self.dims
    }

    fn color(&self) -> Rgb {
        self.color
    }

    fn alpha(&self) -> f32 {
        self.alpha
    }

    fn last_update_time(&self) -> f32 {
        self.last_update_time
    }
}

impl RhythmRect {
    pub fn new_with_animation(start: Vec2, target: Vec2) -> Self {
        Self {
            pos: start,
            start_pos: start,
            target_pos: target,
            ..Default::default()
        }
    }

    #[allow(dead_code)]
    pub fn new_with_pos(pos: Vec2) -> Self {
        Self {
            pos,
            start_pos: pos,
            target_pos: pos,
            ..Default::default()
        }
    }
}

impl Default for RhythmRect {
    fn default() -> Self {
        Self {
            pos: Vec2::new(0.0, 0.0),
            target_pos: Vec2::new(0.0, 0.0),
            start_pos: Vec2::new(0.0, 0.0),
            dims: Vec2::new(RECT_DEFAULT_WIDTH, RECT_DEFAULT_HEIGHT),
            color: Rgb::new(RECT_DEFAULT_R, RECT_DEFAULT_G, RECT_DEFAULT_B),
            alpha: RECT_DEFAULT_A,
            last_update_time: 0.0,
        }
    }
}

fn scale_dims(dims: Vec2, length: f32, velocity: f32) -> Vec2 {
    vec2(dims.x * length, dims.y * velocity)
}
