use nannou::prelude::*;
use prat::BeatSubdivision;
use std::collections::HashMap;

use crate::{groups::rhythm::RhythmParams, view::rhythm_rect::RhythmRect};

const RECT_DEFAULT_R: f32 = 0.7;
const RECT_DEFAULT_G: f32 = 0.7;
const RECT_DEFAULT_B: f32 = 0.7;
const RECT_DEFAULT_A: f32 = 1.0;
const RECT_DEFAULT_WIDTH: f32 = 16.0;
const RECT_DEFAULT_HEIGHT: f32 = 20.0;
const RECT_MAX_WIDTH: f32 = 80.0;
const RECT_MAX_HEIGHT: f32 = 100.0;

// Animation timing constants
const RAMP_UP_PERCENT: f32 = 0.1;
const DWELL_PERCENT: f32 = 0.3;
const RAMP_CURVE_EXPONENT: f32 = 3.0;
const FADE_CURVE_EXPONENT: f32 = 1.5;

#[derive(Debug, Clone, Copy)]
pub struct RhythmViewUpdateParams {
    pub current_wing: Option<usize>,
    pub tempo: f64,
    pub subdivision: BeatSubdivision,
}

#[derive(Debug, Default)]
pub struct RhythmView {
    center: Vec2,
    capacity: usize,
    radius: f32,
    // Positions of each slot
    positions: HashMap<usize, Vec2>,
    rects: HashMap<usize, RhythmRect>,
    last_update_times: HashMap<usize, f32>,
}

impl RhythmView {
    pub fn new(center: Vec2, radius: f32) -> Self {
        Self {
            center,
            radius,
            ..Default::default()
        }
    }

    pub fn update(
        &mut self,
        rhythm_params: &RhythmParams,
        update_params: RhythmViewUpdateParams,
        time: f32,
    ) {
        if let Some(wing) = update_params.current_wing {
            self.last_update_times.insert(wing, time);
        }

        for i in (0..self.capacity) {
            let Some(rect) = self.rects.get_mut(&i) else {
                continue;
            };

            let Some(last_update_time) = self.last_update_times.get(&i) else {
                continue;
            };

            let factor = ((update_params.subdivision.multiplier() / 2.0)
                * (60.0 / update_params.tempo)) as f32;

            // Interpolate color
            let color = interpolate_color(time, *last_update_time, factor);
            rect.color = color;

            // Get slot parameters for scaling
            let slot = rhythm_params.slots.get(i);
            let length_scale = slot.map(|s| s.length).unwrap_or(1.0);
            let velocity_scale = slot.map(|s| s.velocity).unwrap_or(1.0);

            let (base_width, base_height) = if rhythm_params.wings.contains(&i) {
                (RECT_MAX_WIDTH, RECT_MAX_HEIGHT)
            } else {
                (RECT_DEFAULT_WIDTH, RECT_MAX_HEIGHT)
            };

            // Interpolate dims with slot parameter scaling
            let width = interpolate_dimension(
                base_width * (2.0 + length_scale),
                base_width,
                time,
                *last_update_time,
                factor,
            );

            let height = interpolate_dimension(
                base_height + (2.0 + velocity_scale),
                base_height,
                time,
                *last_update_time,
                factor,
            );

            rect.dims = Vec2::new(width, height);
        }
    }

    pub fn draw(&self, draw: &Draw) {
        for (slot, rect) in self.rects.iter() {
            let pos = self.positions[slot];
            draw.rect()
                .x_y(pos.x, pos.y)
                .w_h(rect.dims.x, rect.dims.y)
                .color(rgb(rect.color.red, rect.color.green, rect.color.blue));
        }
    }

    pub fn set_capacity(&mut self, capacity: usize) {
        self.capacity = capacity;
        self.positions = initialize_positions(self.radius, capacity);

        // Initialize rects
        for i in 0..capacity {
            self.rects.insert(i, RhythmRect::new());
        }
    }

    pub fn add_rect(&mut self, wing: usize) {
        let rect = RhythmRect::new();
        self.rects.insert(wing, rect);
    }
}

impl Default for RhythmRect {
    fn default() -> Self {
        Self {
            dims: Vec2::new(RECT_DEFAULT_WIDTH, RECT_DEFAULT_HEIGHT),
            color: Rgb::new(RECT_DEFAULT_R, RECT_DEFAULT_G, RECT_DEFAULT_B),
            alpha: RECT_DEFAULT_A,
        }
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

enum InterpolationPhase {
    RampUp(f32),   // curved t value for ramping up
    Dwell,         // holding at max
    FadeDown(f32), // curved t value for fading down
}

fn get_interpolation_phase(
    current_time: f32,
    last_update_time: f32,
    fade_duration: f32,
    ramp_curve: f32,
    fade_curve: f32,
) -> InterpolationPhase {
    let elapsed = (current_time - last_update_time).max(0.0);
    let ramp_up_duration = fade_duration * RAMP_UP_PERCENT;
    let dwell_duration = fade_duration * DWELL_PERCENT;
    let dwell_end = ramp_up_duration + dwell_duration;

    if elapsed < ramp_up_duration {
        let t = (elapsed / ramp_up_duration).clamp(0.0, 1.0);
        let t_curved = t.powf(ramp_curve);
        InterpolationPhase::RampUp(t_curved)
    } else if elapsed < dwell_end {
        InterpolationPhase::Dwell
    } else {
        let t = ((elapsed - dwell_end) / (fade_duration - dwell_end)).clamp(0.0, 1.0);
        let t_curved = t.powf(fade_curve);
        InterpolationPhase::FadeDown(t_curved)
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a * (1.0 - t) + b * t
}

fn interpolate_color(current_time: f32, last_update_time: f32, fade_duration: f32) -> Rgb {
    // Convert RGB constants to HSV
    let max_hsv = Hsv::from(rgb(1.0, 0.0, 0.0)); // Red
    let default_hsv = Hsv::from(rgb(RECT_DEFAULT_R, RECT_DEFAULT_G, RECT_DEFAULT_B)); // Gray

    let phase = get_interpolation_phase(
        current_time,
        last_update_time,
        fade_duration,
        RAMP_CURVE_EXPONENT,
        FADE_CURVE_EXPONENT,
    );

    let (h, s, v) = match phase {
        InterpolationPhase::RampUp(t) => {
            let h = lerp(
                default_hsv.hue.to_positive_radians(),
                max_hsv.hue.to_positive_radians(),
                t,
            );
            let s = lerp(default_hsv.saturation, max_hsv.saturation, t);
            let v = lerp(default_hsv.value, max_hsv.value, t);
            (h, s, v)
        }
        InterpolationPhase::Dwell => (
            max_hsv.hue.to_positive_radians(),
            max_hsv.saturation,
            max_hsv.value,
        ),
        InterpolationPhase::FadeDown(t) => {
            let h = lerp(
                max_hsv.hue.to_positive_radians(),
                default_hsv.hue.to_positive_radians(),
                t,
            );
            let s = lerp(max_hsv.saturation, default_hsv.saturation, t);
            let v = lerp(max_hsv.value, default_hsv.value, t);
            (h, s, v)
        }
    };

    Rgb::from(hsv(h, s, v))
}

fn interpolate_dimension(
    max: f32,
    min: f32,
    current_time: f32,
    last_update_time: f32,
    fade_duration: f32,
) -> f32 {
    let phase = get_interpolation_phase(
        current_time,
        last_update_time,
        fade_duration,
        RAMP_CURVE_EXPONENT,
        FADE_CURVE_EXPONENT,
    );

    match phase {
        InterpolationPhase::RampUp(t) => lerp(min, max, t),
        InterpolationPhase::Dwell => max,
        InterpolationPhase::FadeDown(t) => lerp(max, min, t),
    }
}
