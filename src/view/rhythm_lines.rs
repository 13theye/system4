use nannou::prelude::*;
use std::collections::HashMap;
use std::time::Instant;

use crate::{
    groups::RhythmParams,
    utils::tween,
    view::{RhythmElement, RhythmFormation, RhythmViewUpdateParams},
};

const LINE_DEFAULT_R: f32 = 0.7;
const LINE_DEFAULT_G: f32 = 0.7;
const LINE_DEFAULT_B: f32 = 0.7;
const LINE_DEFAULT_A: f32 = 1.0;
const LINE_HIGH_R: f32 = 1.0;
const LINE_HIGH_G: f32 = 0.0;
const LINE_HIGH_B: f32 = 0.0;
const LINE_DEFAULT_WIDTH: f32 = 5.0;
const LINE_DEFAULT_HEIGHT: f32 = 50.0;
const LINE_MAX_WIDTH: f32 = 5.0;
const LINE_MAX_HEIGHT: f32 = 1000.0;

// Animation timing constants
const RAMP_UP_PERCENT: f32 = 0.1;
const DWELL_PERCENT: f32 = 0.3;
const RAMP_CURVE_EXPONENT: f32 = 3.0;
const FADE_CURVE_EXPONENT: f32 = 1.5;

#[derive(Debug)]
pub struct RhythmLinesFormation {
    pub center: Vec2,
    pub capacity: usize,
    pub width: f32,
    pub length: f32,
    pub elements: HashMap<usize, RhythmLine>,
}

impl RhythmFormation for RhythmLinesFormation {
    fn center(&self) -> Vec2 {
        self.center
    }

    fn capacity(&self) -> usize {
        self.capacity
    }

    fn initialize_rhythm(&mut self, rhythm_params: &RhythmParams, _time: Instant) {
        self.capacity = rhythm_params.capacity;
        let positions = Self::initialize_positions(self.center, self.width, self.capacity);

        for i in 0..self.capacity {
            let Some(pos) = positions.get(&i) else {
                continue;
            };
            self.elements.insert(i, RhythmLine::new_with_pos(*pos));
        }
    }

    fn reinitialize_rhythm(&mut self, rhythm_params: &RhythmParams, now: Instant) {
        // TODO: Implement state-based animation for RhythmLinesFormation
        // For now, just re-initialize
        self.initialize_rhythm(rhythm_params, now);
    }

    fn clear_rhythm(&mut self, _now: Instant) {
        // TODO: Implement clearing animation for RhythmLinesFormation
        // For now, just clear elements immediately
        self.elements.clear();
    }

    fn update_transitions(&mut self, _now: Instant) {
        // TODO: Implement state-based animations for RhythmLinesFormation
        // Currently this formation has no state transitions
    }

    fn update_active(
        &mut self,
        rhythm_params: &RhythmParams,
        update_params: &RhythmViewUpdateParams,
        now: Instant,
    ) {
        for (wing, rect) in self.elements.iter_mut() {
            // Update last update time of the current wing's Rect

            let mut active_wing = false;
            if let Some(current_wing) = update_params.current_slot {
                if *wing == current_wing {
                    rect.last_update_instant = now;
                    active_wing = true;
                }
            }

            // Calculate the interpolation duration
            let wing_duration = ((update_params.subdivision.multiplier() / 2.0)
                * (60.0 / update_params.tempo)) as f32;

            rect.color = tween::interpolate_color(
                rgb(LINE_DEFAULT_R, LINE_DEFAULT_G, LINE_DEFAULT_B),
                rgb(LINE_HIGH_R, LINE_HIGH_G, LINE_HIGH_B),
                wing_duration,
                RAMP_UP_PERCENT,
                DWELL_PERCENT,
                RAMP_CURVE_EXPONENT,
                FADE_CURVE_EXPONENT,
                now,
                rect.last_update_instant,
            );

            // Get slot parameters for scaling
            let slot = rhythm_params.slot_params.get(*wing);
            let length_scale = slot.map(|s| s.length).unwrap_or(1.0);
            let velocity_scale = slot.map(|s| s.velocity).unwrap_or(1.0);

            if active_wing {
                println!(
                    "Wing: {}, Length Scale: {}, Velocity Scale: {}",
                    wing, length_scale, velocity_scale
                );
            }

            let (base_width, base_height) = if rhythm_params.wings.contains(wing) {
                (LINE_MAX_WIDTH, LINE_MAX_HEIGHT)
            } else {
                (LINE_DEFAULT_WIDTH, LINE_DEFAULT_HEIGHT)
            };

            // Interpolate dims with slot parameter scaling
            let width = tween::interpolate_dimension(
                base_width * (5.0 * length_scale),
                base_width,
                wing_duration,
                RAMP_UP_PERCENT,
                DWELL_PERCENT,
                RAMP_CURVE_EXPONENT,
                FADE_CURVE_EXPONENT,
                now,
                rect.last_update_instant,
            );

            let height = tween::interpolate_dimension(
                base_height * (2.0 * length_scale),
                base_height,
                wing_duration,
                RAMP_UP_PERCENT,
                DWELL_PERCENT,
                RAMP_CURVE_EXPONENT,
                FADE_CURVE_EXPONENT,
                now,
                rect.last_update_instant,
            );

            rect.dims = Vec2::new(width, height);
        }
    }

    fn draw(&self, draw: &Draw) {
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

impl RhythmLinesFormation {
    pub fn new(center: Vec2, width: f32, length: f32, capacity: usize) -> Self {
        Self {
            center,
            width,
            length,
            capacity,
            elements: HashMap::new(),
        }
    }

    fn initialize_positions(center: Vec2, width: f32, capacity: usize) -> HashMap<usize, Vec2> {
        let start = center.x - width / 2.0;
        let delta = width / ((capacity - 1) as f32);

        let mut positions = HashMap::new();
        for i in 0..capacity {
            let x = start + i as f32 * delta;
            let y = center.y;
            positions.insert(i, Vec2::new(x, y));
        }
        positions
    }
}

#[derive(Debug)]
pub struct RhythmLine {
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
    pub(crate) last_update_instant: Instant,
}

impl RhythmElement for RhythmLine {
    fn set_position(&mut self, pos: Vec2) {
        self.pos = pos;
    }

    fn position(&self) -> Vec2 {
        self.pos
    }

    fn target_position(&self) -> Vec2 {
        self.target_pos
    }

    fn start_position(&self) -> Vec2 {
        self.start_pos
    }

    fn set_animation_positions(&mut self, start: Vec2, target: Vec2) {
        self.start_pos = start;
        self.target_pos = target;
    }

    fn set_last_active_instant(&mut self, instant: Instant) {
        self.last_update_instant = instant;
    }

    fn last_update_instant(&self) -> Instant {
        self.last_update_instant
    }

    fn set_is_wing(&mut self, _is_wing: bool) {
        // RhythmLine doesn't store wing status, no-op
    }

    fn update(
        &mut self,
        _rhythm_params: &RhythmParams,
        _update_params: &RhythmViewUpdateParams,
        _wing: usize,
        now: Instant,
    ) {
        self.last_update_instant = now;
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

impl RhythmLine {
    pub fn new_with_pos(pos: Vec2) -> Self {
        Self {
            pos,
            start_pos: pos,
            target_pos: pos,
            ..Default::default()
        }
    }
}

impl Default for RhythmLine {
    fn default() -> Self {
        Self {
            pos: Vec2::new(0.0, 0.0),
            target_pos: Vec2::new(0.0, 0.0),
            start_pos: Vec2::new(0.0, 0.0),
            dims: Vec2::new(LINE_DEFAULT_WIDTH, LINE_DEFAULT_HEIGHT),
            color: Rgb::new(LINE_DEFAULT_R, LINE_DEFAULT_G, LINE_DEFAULT_B),
            alpha: LINE_DEFAULT_A,
            last_update_instant: Instant::now(),
        }
    }
}

pub fn scale_dims(dims: Vec2, length: f32, velocity: f32) -> Vec2 {
    vec2(dims.x * length, dims.y * velocity)
}
