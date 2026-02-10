use nannou::prelude::*;
use std::time::Instant;

use crate::{groups::RhythmSlotParams, utils::tween, view::rhythm::RhythmViewUpdateParams};

use super::animation::*;

#[derive(Copy, Clone, Debug)]
pub enum RhythmElementMovement {
    Idle,
    Moving {
        start_pos: Vec2,
        target_pos: Vec2,
        start_time: Instant,
        duration: f32,
    },
    Clearing {
        start_pos: Vec2,
        target_pos: Vec2,
        start_time: Instant,
    },
}

#[derive(Copy, Clone, Debug)]
pub enum RhythmElementActivation {
    Idle,
    Active { start_time: Instant },
    ToClear,
}

#[derive(Debug)]
pub struct RhythmElementParams {
    pub current_position: Vec2,
    // The position where the element belongs in formation when not a wing
    pub formation_position: Vec2,
    // The position where the element belongs in formation when a wing
    pub wing_position: Vec2,
    // The current radius of the element
    pub radius: f32,
    // The rhythm parameters represented by this element
    pub slot: RhythmSlotParams,
    /// Default color
    pub color: Rgba,
    /// Gradient color 1
    pub gradient_color_1: Rgba,
    /// Gradient color 2
    pub gradient_color_2: Rgba,
}

#[derive(Debug)]
pub struct RhythmElement {
    pub is_wing: bool,
    pub params: RhythmElementParams,
    pub movement: RhythmElementMovement,
    pub activation: RhythmElementActivation,
}

impl Default for RhythmElementParams {
    fn default() -> Self {
        Self {
            current_position: vec2(0.0, 0.0),
            formation_position: vec2(0.0, 0.0),
            wing_position: vec2(0.0, 0.0),
            radius: MIN_ELEMENT_RADIUS,
            slot: RhythmSlotParams::default(),
            color: rgba(0.247, 0.349, 0.353, 0.5),
            gradient_color_1: rgba(0.847, 0.137, 0.161, 1.0),
            gradient_color_2: rgba(0.486, 0.706, 0.31, 1.0),
        }
    }
}

impl RhythmElement {
    pub fn new(
        initial_position: Vec2,
        formation_position: Vec2,
        wing_position: Vec2,
        movement_duration: f32,
        slot_params: RhythmSlotParams,
        now: Instant,
    ) -> Self {
        let params = RhythmElementParams {
            current_position: initial_position,
            formation_position,
            wing_position,
            slot: slot_params,
            ..Default::default()
        };

        Self {
            is_wing: false,
            params,
            movement: RhythmElementMovement::Moving {
                start_pos: initial_position,
                target_pos: formation_position,
                start_time: now,
                duration: movement_duration,
            },
            activation: RhythmElementActivation::Idle,
        }
    }

    /// Called by an active `RhythmFormation` to update the `RhythmElement`
    /// while a rhythm is playing.
    pub fn activate(&mut self, now: Instant) {
        if !self.is_wing {
            return;
        }
        self.activation = RhythmElementActivation::Active { start_time: now };
    }

    /// Mark the element as a wing & set corresponding visual parameters & position
    pub fn set_is_wing(&mut self, movement_duration: f32, now: Instant) {
        self.is_wing = true;
        self.params.radius = MIN_ELEMENT_RADIUS
            + self.params.slot.length * (MAX_ELEMENT_RADIUS - MIN_ELEMENT_RADIUS);

        self.movement = RhythmElementMovement::Moving {
            start_pos: self.params.current_position,
            target_pos: self.params.wing_position,
            start_time: now,
            duration: movement_duration,
        };
    }

    pub fn set_is_not_wing(&mut self, movement_duration: f32, now: Instant) {
        self.is_wing = false;
        self.params.radius = MIN_ELEMENT_RADIUS;

        self.movement = RhythmElementMovement::Moving {
            start_pos: self.params.current_position,
            target_pos: self.params.formation_position,
            start_time: now,
            duration: movement_duration,
        };
    }

    /// Updates the element based on activation state. Called once per frame.
    pub fn update_active(&mut self, update_params: &RhythmViewUpdateParams, now: Instant) {
        if let RhythmElementActivation::Active { start_time } = self.activation {
            if self.is_wing {
                let animation_length =
                    (60.0 / update_params.tempo) as f32 * self.params.slot.length;

                self.update_wing_animation(start_time, animation_length, now);
            }
        }
    }

    /// Creates a gradient animation that s\
    fn update_wing_animation(&mut self, start_time: Instant, animation_length: f32, now: Instant) {}

    /// Called by RhythmFormation irrespective of there being an active rhythm.
    /// This allows for elements to finish animations even if a rhythm is not playing.
    pub fn update_movement(&mut self, now: Instant) {
        match self.movement {
            RhythmElementMovement::Idle => {}
            RhythmElementMovement::Moving {
                start_pos,
                target_pos,
                start_time,
                duration,
            } => {
                let t = now.duration_since(start_time).as_secs_f32();

                if t >= duration {
                    self.params.current_position = target_pos;
                    self.movement = RhythmElementMovement::Idle;
                } else {
                    use nannou::ease::back::*;
                    let f = ease_out::<f32>;

                    self.params.current_position =
                        tween::ease_vec2(f, t, start_pos, target_pos, duration);
                }
            }
            RhythmElementMovement::Clearing {
                start_pos,
                target_pos,
                start_time,
            } => {
                let t = now.duration_since(start_time).as_secs_f32();

                if t >= CLEAR_ANIMATION_DURATION {
                    self.params.current_position = target_pos;
                    self.movement = RhythmElementMovement::Idle;
                    self.activation = RhythmElementActivation::ToClear;
                } else {
                    // Use cubic ease-out for smooth deceleration
                    use nannou::ease::cubic;
                    let f = cubic::ease_out::<f32>;

                    self.params.current_position =
                        tween::ease_vec2(f, t, start_pos, target_pos, CLEAR_ANIMATION_DURATION);
                }
            }
        }
    }

    pub fn draw(&self, draw: &Draw) {
        draw.ellipse()
            .xy(self.params.current_position)
            .radius(self.params.radius)
            .color(self.params.color);
    }
}
