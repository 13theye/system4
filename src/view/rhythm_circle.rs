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

// Beat indicator constants
const INDICATOR_ARC_SPAN: f32 = 0.15; // Angular span in radians (~8.5 degrees)
const INDICATOR_STROKE_WEIGHT: f32 = 4.0;

pub struct RhythmCircleFormation {
    pub center: Vec2,
    pub capacity: usize,
    pub radius: f32,
    pub elements: HashMap<usize, Box<dyn RhythmElement>>, // HashMap<wing, <RhythmRect>,
    state: RhythmFormationState,
    last_state_change: f32,
    // Beat indicator tracking
    next_beat_wing: Option<usize>,
    next_beat_time: f32,
    next_beat_angle: f32,
    sub_duration: f32,
    indicator_angle: f32,
    last_update_time: f32,
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
            self.elements.insert(
                i,
                Box::new(RhythmRect::new_with_animation(self.center, *target_pos)),
            );
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
                    self.elements.insert(
                        i,
                        Box::new(RhythmRect::new_with_animation(self.center, target_pos)),
                    );
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
        // Update indicator during transition states (rhythm continues playing)
        if !matches!(
            self.state,
            RhythmFormationState::Inactive | RhythmFormationState::Active
        ) {
            // Advance indicator using velocity-based approach
            let dt = time - self.last_update_time;
            if let Some(_next_wing) = self.next_beat_wing {
                let time_remaining = self.next_beat_time - time;
                if time_remaining > 0.001 {
                    // Calculate angle distance and ensure clockwise (negative) direction
                    let mut angle_distance = self.next_beat_angle - self.indicator_angle;
                    if angle_distance > 0.0 {
                        angle_distance -= 2.0 * std::f32::consts::PI;
                    }
                    let velocity = angle_distance / time_remaining;
                    self.indicator_angle += velocity * dt;
                } else {
                    // Fallback to constant velocity
                    let angular_velocity =
                        -2.0 * std::f32::consts::PI / (self.capacity as f32 * self.sub_duration);
                    self.indicator_angle += angular_velocity * dt;
                }
            } else {
                // No next beat - use constant velocity
                let angular_velocity =
                    -2.0 * std::f32::consts::PI / (self.capacity as f32 * self.sub_duration);
                self.indicator_angle += angular_velocity * dt;
            }

            // Normalize indicator_angle to stay in range (-2π, 0]
            let two_pi = 2.0 * std::f32::consts::PI;
            self.indicator_angle = self.indicator_angle % two_pi;
            if self.indicator_angle > 0.0 {
                self.indicator_angle -= two_pi;
            }

            self.last_update_time = time;
        }

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
        // Only update if in Active state
        if !matches!(self.state, RhythmFormationState::Active) {
            return;
        }

        // Update beat indicator
        self.update_indicator(update_params, time);

        // Update elements
        for (wing, element) in self.elements.iter_mut() {
            element.update(rhythm_params, update_params, *wing, time);
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

        // Draw beat indicator arc (always visible)
        let num_points = 20;
        let half_span = INDICATOR_ARC_SPAN / 2.0;

        let points: Vec<Vec2> = (0..num_points)
            .map(|i| {
                let t = i as f32 / (num_points - 1) as f32;
                let angle = self.indicator_angle - half_span + (INDICATOR_ARC_SPAN * t);
                let x = self.center.x + self.radius * angle.cos();
                let y = self.center.y + self.radius * angle.sin();
                pt2(x, y)
            })
            .collect();

        draw.polyline()
            .weight(INDICATOR_STROKE_WEIGHT)
            .points(points)
            .color(WHITE);
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
            next_beat_wing: None,
            next_beat_time: time,
            next_beat_angle: 0.0,
            sub_duration: 1.0,
            indicator_angle: 0.0,
            last_update_time: time,
        }
    }

    fn initialize_positions(radius: f32, capacity: usize) -> HashMap<usize, Vec2> {
        let mut positions = HashMap::new();
        for i in 0..capacity {
            let angle = -(i as f32) * 2.0 * std::f32::consts::PI / (capacity as f32);
            let x = radius * angle.cos();
            let y = radius * angle.sin();
            positions.insert(i, Vec2::new(x, y));
        }
        positions
    }

    fn update_indicator(&mut self, update_params: &RhythmViewUpdateParams, time: f32) {
        // Calculate beat duration
        self.sub_duration =
            ((60.0 / update_params.tempo) / update_params.subdivision.multiplier()) as f32;

        // Detect beat changes and update next beat target
        if update_params.current_wing != self.next_beat_wing {
            if let Some(current_wing) = update_params.current_wing {
                // Calculate next beat position
                let next_wing = (current_wing + 1) % self.capacity;
                self.next_beat_wing = Some(next_wing);
                self.next_beat_time = time + self.sub_duration;
                self.next_beat_angle =
                    -(next_wing as f32) * 2.0 * std::f32::consts::PI / (self.capacity as f32);
            }
        }

        // Calculate velocity to reach next beat on time
        let dt = time - self.last_update_time;
        if let Some(_next_wing) = self.next_beat_wing {
            let time_remaining = self.next_beat_time - time;
            if time_remaining > 0.001 {
                // Calculate angle distance and ensure clockwise (negative) direction
                let mut angle_distance = self.next_beat_angle - self.indicator_angle;
                if angle_distance > 0.0 {
                    angle_distance -= 2.0 * std::f32::consts::PI;
                }
                let velocity = angle_distance / time_remaining;
                self.indicator_angle += velocity * dt;
            } else {
                // Very close to or past beat time - just use constant velocity as fallback
                let angular_velocity =
                    -2.0 * std::f32::consts::PI / (self.capacity as f32 * self.sub_duration);
                self.indicator_angle += angular_velocity * dt;
            }
        } else {
            // No next beat yet - use constant velocity
            let angular_velocity =
                -2.0 * std::f32::consts::PI / (self.capacity as f32 * self.sub_duration);
            self.indicator_angle += angular_velocity * dt;
        }

        // Normalize indicator_angle to stay in range (-2π, 0]
        let two_pi = 2.0 * std::f32::consts::PI;
        self.indicator_angle = self.indicator_angle % two_pi;
        if self.indicator_angle > 0.0 {
            self.indicator_angle -= two_pi;
        }

        self.last_update_time = time;
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

    fn update(
        &mut self,
        rhythm_params: &RhythmParams,
        update_params: &RhythmViewUpdateParams,
        wing: usize,
        time: f32,
    ) {
        // Update last active time of the current wing's Rect
        if let Some(current_wing) = update_params.current_wing {
            if wing == current_wing {
                self.last_active_time = time;
            }
        }

        // Calculate the interpolation duration
        let wing_duration =
            ((60.0 / update_params.tempo) / update_params.subdivision.multiplier()) as f32;

        self.color = tween::interpolate_color(
            rgb(RECT_DEFAULT_R, RECT_DEFAULT_G, RECT_DEFAULT_B),
            rgb(RECT_HIGH_R, RECT_HIGH_G, RECT_HIGH_B),
            wing_duration,
            RAMP_UP_PERCENT,
            DWELL_PERCENT,
            RAMP_CURVE_EXPONENT,
            FADE_CURVE_EXPONENT,
            time,
            self.last_active_time,
        );

        // Get slot parameters for scaling
        let slot = rhythm_params.slots.get(wing);
        let length_scale = slot.map(|s| s.length).unwrap_or(1.0);
        let velocity_scale = slot.map(|s| s.velocity).unwrap_or(1.0);

        let (base_width, base_height) = if rhythm_params.wings.contains(&wing) {
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
            self.last_active_time,
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
            last_active_time: 0.0,
        }
    }
}

fn scale_dims(dims: Vec2, length: f32, velocity: f32) -> Vec2 {
    vec2(dims.x * length, dims.y * velocity)
}
