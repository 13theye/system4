// system4-app/src/utils_ext.rs
//
// Extensions to system4-core::utils that depend on Nannou

use nannou::prelude::*;
use system4_core::utils::tween::{get_interpolation_phase, lerp, InterpolationPhase};

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
