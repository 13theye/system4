// src/utils/tween.rs
//
// Utility functions for interpolation

use nannou::prelude::*;

pub enum InterpolationPhase {
    RampUp(f32),   // curved t value for ramping up
    Dwell,         // holding at max
    FadeDown(f32), // curved t value for fading down
}

pub fn get_interpolation_phase(
    fade_duration: f32,
    ramp_up_percent: f32,
    dwell_percent: f32,
    ramp_curve: f32,
    fade_curve: f32,
    current_time: f32,
    last_update_time: f32,
) -> InterpolationPhase {
    let elapsed = (current_time - last_update_time).max(0.0);
    let ramp_up_duration = fade_duration * ramp_up_percent;
    let dwell_duration = fade_duration * dwell_percent;
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

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a * (1.0 - t) + b * t
}

#[allow(clippy::too_many_arguments)]
pub fn interpolate_color(
    start_rgb: Rgb,
    end_rgb: Rgb,
    fade_duration: f32,
    ramp_up_pct: f32,
    dwell_pct: f32,
    ramp_curve: f32,
    fade_curve: f32,
    current_time: f32,
    last_update_time: f32,
) -> Rgb {
    // Convert RGB constants to HSV
    let max_hsv = Hsv::from(end_rgb);
    let default_hsv = Hsv::from(start_rgb);

    let phase = get_interpolation_phase(
        fade_duration,
        ramp_up_pct,
        dwell_pct,
        ramp_curve,
        fade_curve,
        current_time,
        last_update_time,
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

#[allow(clippy::too_many_arguments)]
pub fn interpolate_dimension(
    max: f32,
    min: f32,
    fade_duration: f32,
    ramp_up_pct: f32,
    dwell_pct: f32,
    ramp_curve_expo: f32,
    fade_curve_expo: f32,
    current_time: f32,
    last_update_time: f32,
) -> f32 {
    let phase = get_interpolation_phase(
        fade_duration,
        ramp_up_pct,
        dwell_pct,
        ramp_curve_expo,
        fade_curve_expo,
        current_time,
        last_update_time,
    );

    match phase {
        InterpolationPhase::RampUp(t) => lerp(min, max, t),
        InterpolationPhase::Dwell => max,
        InterpolationPhase::FadeDown(t) => lerp(max, min, t),
    }
}
