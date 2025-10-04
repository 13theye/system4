use nannou::prelude::*;
use prat::BeatSubdivision;
use std::collections::HashMap;

use crate::{groups::rhythm::Rhythm, view::rhythm_rect::RhythmRect};

const RECT_DEFAULT_R: f32 = 0.7;
const RECT_DEFAULT_G: f32 = 0.7;
const RECT_DEFAULT_B: f32 = 0.7;
const RECT_DEFAULT_A: f32 = 1.0;
const RECT_DEFAULT_WIDTH: f32 = 20.0;
const RECT_DEFAULT_HEIGHT: f32 = 20.0;
const RECT_MAX_WIDTH: f32 = 100.0;
const RECT_MAX_HEIGHT: f32 = 100.0;

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

    pub fn update(&mut self, update_params: RhythmViewUpdateParams, time: f32) {
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
            let color = interpolate_color(time, *last_update_time, factor as f32);
            rect.color = color;

            // Interpolate dims
            let width = interpolate_dimension(
                RECT_MAX_WIDTH,
                RECT_DEFAULT_WIDTH,
                time,
                *last_update_time,
                factor,
            );

            let height = interpolate_dimension(
                RECT_MAX_HEIGHT,
                RECT_DEFAULT_HEIGHT,
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
                .color(rgba(
                    rect.color.red,
                    rect.color.green,
                    rect.color.blue,
                    rect.alpha,
                ));
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

fn interpolate_color(current_time: f32, last_update_time: f32, fade_duration: f32) -> Rgb {
    // Convert RGB constants to HSV
    let max_hsv = Hsv::from(rgb(1.0, 0.0, 0.0)); // Red
    let default_hsv = Hsv::from(rgb(RECT_DEFAULT_R, RECT_DEFAULT_G, RECT_DEFAULT_B)); // Gray

    let elapsed = (current_time - last_update_time).max(0.0);
    let ramp_up_duration = fade_duration * 0.15;

    let (h, s, v) = if elapsed < ramp_up_duration {
        // Exponential ramp up from default to max in HSV
        let t = (elapsed / ramp_up_duration).clamp(0.0, 1.0);
        let t_curved = t.powf(1.0);
        let h = default_hsv.hue.to_positive_radians() * (1.0 - t_curved)
            + max_hsv.hue.to_positive_radians() * t_curved;
        let s = default_hsv.saturation * (1.0 - t_curved) + max_hsv.saturation * t_curved;
        let v = default_hsv.value * (1.0 - t_curved) + max_hsv.value * t_curved;
        (h, s, v)
    } else {
        // Exponential fade down from max to default in HSV
        let t = ((elapsed - ramp_up_duration) / (fade_duration - ramp_up_duration)).clamp(0.0, 1.0);
        let t_curved = t.powf(0.5);
        let h = max_hsv.hue.to_positive_radians() * (1.0 - t_curved)
            + default_hsv.hue.to_positive_radians() * t_curved;
        let s = max_hsv.saturation * (1.0 - t_curved) + default_hsv.saturation * t_curved;
        let v = max_hsv.value * (1.0 - t_curved) + default_hsv.value * t_curved;
        (h, s, v)
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
    let elapsed = (current_time - last_update_time).max(0.0);
    let ramp_up_duration = fade_duration * 0.15;

    if elapsed < ramp_up_duration {
        // Exponential ramp up from min to max
        let t = (elapsed / ramp_up_duration).clamp(0.0, 1.0);
        let t_curved = t.powf(3.0);
        min * (1.0 - t_curved) + max * t_curved
    } else {
        // Exponential fade down from max to min
        let t = ((elapsed - ramp_up_duration) / (fade_duration - ramp_up_duration)).clamp(0.0, 1.0);
        let t_curved = t.powf(0.5);
        max * (1.0 - t_curved) + min * t_curved
    }
}
