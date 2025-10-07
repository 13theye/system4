use nannou::prelude::*;
use std::collections::HashMap;

use crate::{
    groups::rhythm::RhythmParams,
    utils::tween,
    view::{RhythmElement, RhythmFormation, RhythmViewUpdateParams},
};

const RECT_DEFAULT_R: f32 = 0.7;
const RECT_DEFAULT_G: f32 = 0.7;
const RECT_DEFAULT_B: f32 = 0.7;
const RECT_DEFAULT_A: f32 = 1.0;
const RECT_HIGH_R: f32 = 1.0;
const RECT_HIGH_G: f32 = 0.0;
const RECT_HIGH_B: f32 = 0.0;
const RECT_HIGH_A: f32 = 1.0;
const RECT_DEFAULT_WIDTH: f32 = 16.0;
const RECT_DEFAULT_HEIGHT: f32 = 20.0;
const RECT_MAX_WIDTH: f32 = 80.0;
const RECT_MAX_HEIGHT: f32 = 100.0;

// Animation timing constants
const RAMP_UP_PERCENT: f32 = 0.1;
const DWELL_PERCENT: f32 = 0.3;
const RAMP_CURVE_EXPONENT: f32 = 3.0;
const FADE_CURVE_EXPONENT: f32 = 1.5;

#[derive(Debug)]
pub struct RhythmCircleFormation {
    pub center: Vec2,
    pub capacity: usize,
    pub radius: f32,
    pub elements: HashMap<usize, RhythmRect>, // HashMap<wing, <RhythmRect>,
}

impl RhythmFormation for RhythmCircleFormation {
    fn center(&self) -> Vec2 {
        self.center
    }

    fn capacity(&self) -> usize {
        self.capacity
    }

    fn initialize_rhythm(&mut self, rhythm_params: &RhythmParams) {
        self.capacity = rhythm_params.capacity;
        let positions = Self::initialize_positions(self.radius, self.capacity);

        for i in 0..self.capacity {
            let Some(pos) = positions.get(&i) else {
                continue;
            };
            self.elements.insert(i, RhythmRect::new_with_pos(*pos));
        }
    }

    fn update(
        &mut self,
        rhythm_params: &RhythmParams,
        update_params: &RhythmViewUpdateParams,
        time: f32,
    ) {
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
                (RECT_DEFAULT_WIDTH, RECT_MAX_HEIGHT)
            };

            // Interpolate dims with slot parameter scaling
            let width = tween::interpolate_dimension(
                base_width * (2.0 + length_scale),
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
                base_height + (2.0 + velocity_scale),
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
    pub fn new(center: Vec2, radius: f32, capacity: usize) -> Self {
        Self {
            center,
            radius,
            capacity,
            elements: HashMap::new(),
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
}

#[derive(Debug)]
pub struct RhythmRect {
    /// Screen position
    pub(crate) pos: Vec2,
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
    pub fn new_with_pos(pos: Vec2) -> Self {
        Self {
            pos,
            ..Default::default()
        }
    }
}

impl Default for RhythmRect {
    fn default() -> Self {
        Self {
            pos: Vec2::new(0.0, 0.0),
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
