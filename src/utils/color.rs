//! src/utils/color/rs
//!
//! Color conversion utilities

use nannou::prelude::*;

/// Convert RGB color to HSV
/// Returns (hue in degrees 0-360, saturation 0-1, value 0-1)
pub fn rgb_to_hsv(color: Rgb) -> (f32, f32, f32) {
    let r = color.red;
    let g = color.green;
    let b = color.blue;

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    // Value
    let v = max;

    // Saturation
    let s = if max == 0.0 { 0.0 } else { delta / max };

    // Hue
    let h = if delta == 0.0 {
        0.0 // Undefined, grayscale
    } else if max == r {
        60.0 * (((g - b) / delta) % 6.0)
    } else if max == g {
        60.0 * (((b - r) / delta) + 2.0)
    } else {
        60.0 * (((r - g) / delta) + 4.0)
    };

    let h = if h < 0.0 { h + 360.0 } else { h };

    (h, s, v)
}

/// Convert HSV to RGB color
/// Takes (hue in degrees 0-360, saturation 0-1, value 0-1)
/// Returns Rgba with alpha = 1.0
pub fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Rgb {
    if s == 0.0 {
        // Grayscale
        return rgb(v, v, v);
    }

    let h = (h % 360.0) / 60.0; // Convert to 0-6 range
    let i = h.floor() as i32;
    let f = h - i as f32;

    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));

    let (r, g, b) = match i {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q), // 5 or wrapping back to 0
    };

    rgb(r, g, b)
}
