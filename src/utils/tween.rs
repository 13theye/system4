// src/utils/tween.rs
//
// Utility functions for interpolation

use nannou::prelude::*;

pub enum InterpolationPhase {
    RampUp(f32),   // curved t value for ramping up
    Dwell,         // holding at max
    FadeDown(f32), // curved t value for fading down
}

/// Linear interpolation.
/// - `Progress` must be between 0.0 and 1.0
pub fn lerp<T>(start: T, end: T, progress: f32) -> T
where
    T: std::ops::Mul<f32, Output = T> + std::ops::Add<Output = T> + Copy,
{
    start * (1.0 - progress) + end * progress
}

/// Linear interpolation for color
/// - `Progress` must be between 0.0 and 1.0
pub fn lerp_rgba(start: Rgba, end: Rgba, progress: f32) -> Rgba {
    let r = lerp(start.red, end.red, progress);
    let g = lerp(start.green, end.green, progress);
    let b = lerp(start.blue, end.blue, progress);
    let a = lerp(start.alpha, end.alpha, progress);
    rgba(r, g, b, a)
}

/// Constant-hue linear interpolation for color
/// - internally converts to HSLA and maintains hue while interpolating saturation and lightness
/// - `Progress` must be between 0.0 and 1.0
pub fn lerp_hue_rgba(start: Rgba, end: Rgba, progress: f32) -> Rgba {
    let start_hsla = Hsla::from(start);
    let end_hsla = Hsla::from(end);

    let mut hue_diff = end_hsla.hue.to_degrees() - start_hsla.hue.to_degrees();
    if hue_diff.abs() > 180.0 {
        hue_diff -= 360.0 * hue_diff.signum();
    }

    let hue = start_hsla.hue.to_degrees() + hue_diff * progress;

    let lerped_hsla = Hsla::new(
        hue,
        start_hsla.saturation + (end_hsla.saturation - start_hsla.saturation) * progress,
        start_hsla.lightness + (end_hsla.lightness - start_hsla.lightness) * progress,
        start_hsla.alpha + (end_hsla.alpha - start_hsla.alpha) * progress,
    );

    Rgba::from(lerped_hsla)
}

/// Helper function to use one of the easing functions with Vec2.
/// - Pass in an easing function with type parameter bound to f32.
/// - For example `ease_out::<f32>`
pub fn ease_vec2<F: Fn(f32, f32, f32, f32) -> f32>(
    f: F,
    t: f32,
    start: Vec2,
    end: Vec2,
    d: f32,
) -> Vec2 {
    let x = f(t, start.x, end.x - start.x, d);
    let y = f(t, start.y, end.y - start.y, d);
    vec2(x, y)
}
