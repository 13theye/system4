use nannou::prelude::*;
use std::time::Instant;

use crate::{groups::RhythmSlotParams, utils::tween};

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
pub enum RhythmElementSizing {
    Idle,
    Changing {
        start_radius: f32,
        target_radius: f32,
        start_time: Instant,
        duration: f32,
    },
}

#[derive(Copy, Clone, Debug)]
pub enum RhythmElementClearState {
    Active,
    ToClear,
}

#[derive(Debug, Copy, Clone)]
pub struct RhythmElementParams {
    pub current_position: Vec2,
    // The position where the element belongs in formation when not a wing
    pub min_position: Vec2,
    // The position where the element is when velocity = 1.0
    pub max_position: Vec2,
    // The current radius of the element
    pub current_radius: f32,
    /// The base radius of the element
    pub min_radius: f32,
    /// The maximum radius of the element if length - 1.0
    pub max_radius: f32,
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
    pub sizing: RhythmElementSizing,
    pub clear_state: RhythmElementClearState,
}

impl Default for RhythmElementParams {
    fn default() -> Self {
        Self {
            current_position: vec2(0.0, 0.0),
            min_position: vec2(0.0, 0.0),
            max_position: vec2(0.0, 0.0),
            current_radius: 10.0,
            min_radius: MIN_ELEMENT_RADIUS,
            max_radius: MAX_ELEMENT_RADIUS,
            slot: RhythmSlotParams::default(),
            // Dark teal
            color: rgba(0.247, 0.349, 0.353, 1.0),
            // Red
            gradient_color_1: rgba(0.847, 0.137, 0.161, 1.0),
            // Light teal
            gradient_color_2: rgba(0.486, 0.706, 0.702, 1.0),
        }
    }
}

impl RhythmElement {
    pub fn new(
        initial_position: Vec2,
        min_position: Vec2,
        max_position: Vec2,
        movement_duration: f32,
        slot_params: RhythmSlotParams,
        now: Instant,
    ) -> Self {
        let params = RhythmElementParams {
            current_position: initial_position,
            min_position,
            max_position,
            slot: slot_params,
            ..Default::default()
        };

        Self {
            is_wing: false,
            params,
            movement: RhythmElementMovement::Moving {
                start_pos: initial_position,
                target_pos: min_position,
                start_time: now,
                duration: movement_duration,
            },
            sizing: RhythmElementSizing::Changing {
                start_radius: params.current_radius,
                target_radius: params.min_radius,
                start_time: now,
                duration: movement_duration,
            },
            clear_state: RhythmElementClearState::Active,
        }
    }

    /// Mark the element as a wing & set corresponding visual parameters & position
    pub fn set_is_wing(&mut self, movement_duration: f32, now: Instant) {
        self.is_wing = true;

        self.movement = RhythmElementMovement::Moving {
            start_pos: self.params.current_position,
            target_pos: self.wing_position(),
            start_time: now,
            duration: movement_duration,
        };

        self.sizing = RhythmElementSizing::Changing {
            start_radius: self.params.current_radius,
            target_radius: self.wing_radius(),
            start_time: now,
            duration: movement_duration,
        };
    }

    pub fn set_is_not_wing(&mut self, movement_duration: f32, now: Instant) {
        self.is_wing = false;

        self.movement = RhythmElementMovement::Moving {
            start_pos: self.params.current_position,
            target_pos: self.params.min_position,
            start_time: now,
            duration: movement_duration,
        };

        self.sizing = RhythmElementSizing::Changing {
            start_radius: self.params.current_radius,
            target_radius: self.params.min_radius,
            start_time: now,
            duration: movement_duration,
        }
    }

    pub fn set_clearing(&mut self, target_pos: Vec2, movement_duration: f32, now: Instant) {
        self.movement = RhythmElementMovement::Clearing {
            start_pos: self.params.current_position,
            target_pos,
            start_time: now,
        };

        self.sizing = RhythmElementSizing::Changing {
            start_radius: self.params.current_radius,
            target_radius: 1.0,
            start_time: now,
            duration: movement_duration,
        }
    }

    pub fn is_ready_to_clear(&self) -> bool {
        matches!(self.clear_state, RhythmElementClearState::ToClear)
    }

    pub fn wing_position(&self) -> Vec2 {
        self.params
            .min_position
            .lerp(self.params.max_position, self.params.slot.velocity)
    }

    pub fn wing_radius(&self) -> f32 {
        (self.params.slot.length * (self.params.max_radius - self.params.min_radius))
            + self.params.min_radius
    }

    /// Called by RhythmFormation irrespective of there being an active rhythm.
    /// This allows for elements to finish animations even if a rhythm is not playing.
    pub fn update_animations(&mut self, now: Instant) {
        self.update_movement(now);
        self.update_sizing(now);
    }

    fn update_movement(&mut self, now: Instant) {
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
                    self.clear_state = RhythmElementClearState::ToClear;
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

    fn update_sizing(&mut self, now: Instant) {
        if let RhythmElementSizing::Changing {
            start_radius,
            target_radius,
            start_time,
            duration,
        } = self.sizing
        {
            let t = now.duration_since(start_time).as_secs_f32();

            if t >= duration {
                self.params.current_radius = target_radius;
                self.sizing = RhythmElementSizing::Idle;
            } else {
                use nannou::ease::back::*;
                let f = ease_out::<f32>;

                self.params.current_radius =
                    f(t, start_radius, target_radius - start_radius, duration);
            }
        }
    }

    pub fn draw(&self, draw: &Draw) {
        draw.ellipse()
            .xy(self.params.current_position)
            .radius(self.params.current_radius)
            .color(self.params.color);
    }
}
