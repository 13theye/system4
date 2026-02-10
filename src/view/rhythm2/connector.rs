//! src/view/rhythm2/connector.rs
//!
//! Bezier-based connector drawing for rhythm2 formations.
//! Connectors are drawn between adjacent elements using cubic Bezier curves
//! to create smooth, organic-looking metaball-style connections.

use nannou::prelude::*;

use super::{
    element::{RhythmElement, RhythmElementMovement},
    formation::ElementMap,
};

// Bezier connector constants
const BEZIER_SEGMENTS: usize = 24;

// Tangent departure angles
const THETA_NEAR: f32 = std::f32::consts::FRAC_PI_4; // for close circles
const THETA_FAR: f32 = std::f32::consts::FRAC_PI_3; // for distant circles

// Handle scale controls waist tightness
const HANDLE_SCALE_MIN: f32 = 0.35;
const HANDLE_SCALE_MAX: f32 = 0.45;

// Bend-adaptive theta: when two connectors share a circle at a tight bend,
// the departure points on the inside of the bend can overlap, creating a
// sharp pinch. To fix this, theta is reduced on the inside of the bend so
// the departure point slides along the circle surface toward the forward
// direction, away from the conflict zone.
//
// BEND_MARGIN is the angular buffer left between adjacent departure points.
// MIN_BEND_THETA is the floor — theta won't be reduced below this value.
const BEND_MARGIN: f32 = 0.175; // ~10 degrees
const MIN_BEND_THETA: f32 = 0.35; // ~20 degrees

// Debug visualization
const SHOW_DEBUG_GEOMETRY: bool = false;

/// Bend information at a shared circle between two consecutive connectors.
#[derive(Copy, Clone, Debug)]
struct BendInfo {
    /// Absolute bend angle (0 = straight, PI = U-turn).
    angle: f32,
    /// 2D cross product of the incoming and outgoing directions.
    /// Positive = left turn (counterclockwise), negative = right turn (clockwise).
    cross: f32,
}

/// Compute a reduced theta for a departure point on the inside of a tight bend.
///
/// The angular gap between departure points from adjacent connectors on the
/// inside of a bend is approximately: `gap = PI - bend_angle - 2*theta`.
/// When this gap is too small, we reduce theta so that
/// `theta_max = (PI - bend_angle - BEND_MARGIN) / 2`.
///
/// `is_departure`: true at c1 (connector leaves), false at c2 (connector arrives).
/// The inside/outside determination differs between the two endpoints.
fn bend_adjusted_theta(
    base_theta: f32,
    side_sign: f32,
    bend: Option<BendInfo>,
    is_departure: bool,
) -> f32 {
    let Some(bend) = bend else {
        return base_theta;
    };

    let is_inside = if is_departure {
        bend.cross * side_sign > 0.0
    } else {
        bend.cross * side_sign < 0.0
    };

    if !is_inside {
        return base_theta;
    }

    // Maximum theta that keeps departure points from overlapping.
    // Dividing by 2 splits the available angular space equally between
    // the two connectors sharing this circle (assumes similar theta).
    let theta_max = ((std::f32::consts::PI - bend.angle - BEND_MARGIN) / 2.0).max(MIN_BEND_THETA);

    base_theta.min(theta_max)
}

/// Draw all connectors between adjacent elements in the formation.
/// Only elements with indices less than `capacity` are considered.
pub fn draw_connectors(
    draw: &Draw,
    elements: &ElementMap,
    capacity: usize,
    show_debug_geometry: bool,
) {
    // Collect valid (non-clearing) element indices
    let mut indices: Vec<usize> = elements
        .keys()
        .copied()
        .filter(|i| {
            *i < capacity && !matches!(elements[i].movement, RhythmElementMovement::Clearing { .. })
        })
        .collect();
    indices.sort();

    if indices.len() < 2 {
        return;
    }

    // Compute direction vectors for each consecutive pair
    let dirs: Vec<Vec2> = indices
        .windows(2)
        .map(|pair| {
            let c1 = elements[&pair[0]].params.current_position;
            let c2 = elements[&pair[1]].params.current_position;
            let d = c2 - c1;
            let len = d.length();
            if len < 1e-6 {
                Vec2::ZERO
            } else {
                d / len
            }
        })
        .collect();

    // Draw each connector with bend info from neighboring connectors
    let num_connectors = dirs.len();
    for (i, pair) in indices.windows(2).enumerate() {
        let e1 = &elements[&pair[0]];
        let e2 = &elements[&pair[1]];

        // Bend info at c1: angle between previous connector and this one
        let bend_c1 = if i > 0 {
            let prev_dir = dirs[i - 1];
            let curr_dir = dirs[i];
            let dot = prev_dir.dot(curr_dir).clamp(-1.0, 1.0);
            Some(BendInfo {
                angle: dot.acos(),
                cross: prev_dir.x * curr_dir.y - prev_dir.y * curr_dir.x,
            })
        } else {
            None
        };

        // Bend info at c2: angle between this connector and the next one
        let bend_c2 = if i + 1 < num_connectors {
            let curr_dir = dirs[i];
            let next_dir = dirs[i + 1];
            let dot = curr_dir.dot(next_dir).clamp(-1.0, 1.0);
            Some(BendInfo {
                angle: dot.acos(),
                cross: curr_dir.x * next_dir.y - curr_dir.y * next_dir.x,
            })
        } else {
            None
        };

        draw_connector(draw, e1, e2, bend_c1, bend_c2, show_debug_geometry);
    }
}

/// Draw a single Bezier-based connector between two adjacent elements.
fn draw_connector(
    draw: &Draw,
    e1: &RhythmElement,
    e2: &RhythmElement,
    bend_c1: Option<BendInfo>,
    bend_c2: Option<BendInfo>,
    show_debug_geometry: bool,
) {
    let c1 = e1.params.current_position;
    let c2 = e2.params.current_position;
    let r1 = e1.params.radius;
    let r2 = e2.params.radius;
    let r_min = r1.min(r2);
    let r_max = r1.max(r2);

    // Step 1: Frame setup
    let d = c2 - c1;
    let dist = d.length();

    // Early exit if circles are too close or overlapping
    if dist < r_min * 0.5 || dist < (r1 - r2).abs() {
        return;
    }

    let dir = d / dist;
    let perp = vec2(-dir.y, dir.x);

    // Step 2: Tangent departure angle (theta)
    let gap = (dist - r1 - r2).max(0.0);
    let blend = (gap / (r1 + r2)).clamp(0.0, 1.0);
    let theta = lerp(THETA_NEAR, THETA_FAR, blend);

    // Step 3: Handle scale (controls waist tightness)
    let ratio = r_min / r_max;
    let handle_scale = lerp(HANDLE_SCALE_MIN, HANDLE_SCALE_MAX, ratio);

    // Step 4: Compute Bezier curves for both sides
    let side_a = bezier_side_points(
        c1,
        c2,
        r1,
        r2,
        dir,
        perp,
        theta,
        handle_scale,
        bend_c1,
        bend_c2,
    );
    let side_b = bezier_side_points(
        c1,
        c2,
        r1,
        r2,
        dir,
        -perp,
        theta,
        handle_scale,
        bend_c1,
        bend_c2,
    );

    // Step 5: Build final polygon
    let mut points: Vec<Vec2> = side_a.clone();
    points.extend(side_b.iter().rev());

    // Draw the connector polygon with full opacity - alpha will be applied during composite
    draw.polygon().points(points).color(e1.params.color);

    // Debug drawing
    if show_debug_geometry || SHOW_DEBUG_GEOMETRY {
        draw_debug_info(
            draw,
            c1,
            c2,
            r1,
            r2,
            dir,
            perp,
            theta,
            handle_scale,
            bend_c1,
            bend_c2,
            &side_a,
            &side_b,
        );
    }
}

/// Compute Bezier curve points for one side of the connector.
#[allow(clippy::too_many_arguments)]
fn bezier_side_points(
    c1: Vec2,
    c2: Vec2,
    r1: f32,
    r2: f32,
    dir: Vec2,
    side: Vec2,
    theta: f32,
    handle_scale: f32,
    bend_c1: Option<BendInfo>,
    bend_c2: Option<BendInfo>,
) -> Vec<Vec2> {
    // Compute per-endpoint theta, reducing on the inside of tight bends.
    // This slides the departure point along the circle surface toward the
    // forward direction, avoiding overlap with the adjacent connector's
    // departure point while keeping the point ON the circle.
    let perp_ref = vec2(-dir.y, dir.x);
    let side_sign = side.dot(perp_ref); // +1.0 for perp, -1.0 for -perp

    let theta1 = bend_adjusted_theta(theta, side_sign, bend_c1, true);
    let theta2 = bend_adjusted_theta(theta, side_sign, bend_c2, false);

    // Radial direction vectors for departure points
    let rad_dir1 = dir * theta1.cos() + side * theta1.sin();
    let rad_dir2 = -dir * theta2.cos() + side * theta2.sin();

    // Tangent directions (perpendicular to radial direction)
    let mut tan1 = vec2(-rad_dir1.y, rad_dir1.x);
    if tan1.dot(dir) < 0.0 {
        tan1 = -tan1;
    }

    let mut tan2 = vec2(-rad_dir2.y, rad_dir2.x);
    if tan2.dot(-dir) < 0.0 {
        tan2 = -tan2;
    }

    // Tangent points on circle surface
    let p1 = c1 + r1 * rad_dir1;
    let p2 = c2 + r2 * rad_dir2;

    // Control points — cap handle length so cp doesn't cross to the wrong
    // side of the center line (the line from c1 to c2 along `dir`).
    // A point's signed distance from this line is dot(point - c1, side).
    let chord = (p2 - p1).length();
    let base_handle = chord * handle_scale;

    let p1_side_dist = (p1 - c1).dot(side);
    let tan1_side = tan1.dot(side);
    let h1 = if tan1_side < 0.0 {
        let max_h = p1_side_dist / (-tan1_side) * 0.85;
        base_handle.min(max_h.max(0.0))
    } else {
        base_handle
    };

    let p2_side_dist = (p2 - c2).dot(side);
    let tan2_side = tan2.dot(side);
    let h2 = if tan2_side < 0.0 {
        let max_h = p2_side_dist / (-tan2_side) * 0.85;
        base_handle.min(max_h.max(0.0))
    } else {
        base_handle
    };

    let cp1 = p1 + tan1 * h1;
    let cp2 = p2 + tan2 * h2;

    // Sample the Bezier curve
    (0..=BEZIER_SEGMENTS)
        .map(|i| {
            let t = i as f32 / BEZIER_SEGMENTS as f32;
            cubic_bezier(p1, cp1, cp2, p2, t)
        })
        .collect()
}

/// Evaluate a cubic Bezier curve at parameter t.
fn cubic_bezier(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, t: f32) -> Vec2 {
    let one_minus_t = 1.0 - t;
    let one_minus_t_sq = one_minus_t * one_minus_t;
    let one_minus_t_cb = one_minus_t_sq * one_minus_t;
    let t_sq = t * t;
    let t_cb = t_sq * t;

    one_minus_t_cb * p0 + 3.0 * one_minus_t_sq * t * p1 + 3.0 * one_minus_t * t_sq * p2 + t_cb * p3
}

/// Linear interpolation helper.
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Draw debug visualization showing the Bezier construction.
/// - Orange: Points where theta was reduced due to tight bends
#[allow(clippy::too_many_arguments)]
fn draw_debug_info(
    draw: &Draw,
    c1: Vec2,
    c2: Vec2,
    r1: f32,
    r2: f32,
    dir: Vec2,
    perp: Vec2,
    theta: f32,
    handle_scale: f32,
    bend_c1: Option<BendInfo>,
    bend_c2: Option<BendInfo>,
    side_a: &[Vec2],
    side_b: &[Vec2],
) {
    let color_red = rgba(1.0, 0.0, 0.0, 0.8);
    let color_orange = rgba(1.0, 0.6, 0.0, 0.9);
    let color_green = rgba(0.0, 1.0, 0.0, 0.8);
    let color_yellow = rgba(1.0, 1.0, 0.0, 0.6);
    let color_cyan = rgba(0.0, 1.0, 1.0, 0.4);
    let color_magenta = rgba(1.0, 0.0, 1.0, 0.5);

    let perp_ref = vec2(-dir.y, dir.x);

    for (i, side) in [perp, -perp].iter().enumerate() {
        let side_sign = side.dot(perp_ref);

        let theta1 = bend_adjusted_theta(theta, side_sign, bend_c1, true);
        let theta2 = bend_adjusted_theta(theta, side_sign, bend_c2, false);
        let p1_was_adjusted = theta1 < theta;
        let p2_was_adjusted = theta2 < theta;

        // Radial direction vectors
        let rad_dir1 = dir * theta1.cos() + *side * theta1.sin();
        let rad_dir2 = -dir * theta2.cos() + *side * theta2.sin();

        // Tangent directions
        let mut tan1 = vec2(-rad_dir1.y, rad_dir1.x);
        if tan1.dot(dir) < 0.0 {
            tan1 = -tan1;
        }
        let mut tan2 = vec2(-rad_dir2.y, rad_dir2.x);
        if tan2.dot(-dir) < 0.0 {
            tan2 = -tan2;
        }

        // Original departure points (base theta, before bend adjustment)
        let p1_orig = c1 + r1 * (dir * theta.cos() + *side * theta.sin());
        let p2_orig = c2 + r2 * (-dir * theta.cos() + *side * theta.sin());

        // Departure points on circle surface (with bend-adjusted theta)
        let p1 = c1 + r1 * rad_dir1;
        let p2 = c2 + r2 * rad_dir2;

        // Apply handle capping
        let chord = (p2 - p1).length();
        let base_handle = chord * handle_scale;

        let p1_side_dist = (p1 - c1).dot(*side);
        let tan1_side = tan1.dot(*side);
        let h1 = if tan1_side < 0.0 {
            let max_h = p1_side_dist / (-tan1_side) * 0.85;
            base_handle.min(max_h.max(0.0))
        } else {
            base_handle
        };

        let p2_side_dist = (p2 - c2).dot(*side);
        let tan2_side = tan2.dot(*side);
        let h2 = if tan2_side < 0.0 {
            let max_h = p2_side_dist / (-tan2_side) * 0.85;
            base_handle.min(max_h.max(0.0))
        } else {
            base_handle
        };

        let cp1 = p1 + tan1 * h1;
        let cp2 = p2 + tan2 * h2;

        // --- Bend-adjustment indicators (orange) ---
        if p1_was_adjusted {
            draw.ellipse()
                .xy(p1_orig)
                .radius(6.0)
                .no_fill()
                .stroke_weight(1.5)
                .stroke_color(color_orange);
            draw.line()
                .start(p1_orig)
                .end(p1)
                .weight(2.0)
                .color(color_orange);
        }
        if p2_was_adjusted {
            draw.ellipse()
                .xy(p2_orig)
                .radius(6.0)
                .no_fill()
                .stroke_weight(1.5)
                .stroke_color(color_orange);
            draw.line()
                .start(p2_orig)
                .end(p2)
                .weight(2.0)
                .color(color_orange);
        }

        // --- Tangent departure points ---
        // Color: red (normal), orange (bend-adjusted)
        let p1_color = if p1_was_adjusted {
            color_orange
        } else {
            color_red
        };
        let p2_color = if p2_was_adjusted {
            color_orange
        } else {
            color_red
        };
        draw.ellipse().xy(p1).radius(4.0).color(p1_color);
        draw.ellipse().xy(p2).radius(4.0).color(p2_color);

        // --- Control points (green) ---
        draw.ellipse().xy(cp1).radius(4.0).color(color_green);
        draw.ellipse().xy(cp2).radius(4.0).color(color_green);

        // --- Handle lines (yellow) ---
        draw.line()
            .start(p1)
            .end(cp1)
            .weight(1.0)
            .color(color_yellow);
        draw.line()
            .start(p2)
            .end(cp2)
            .weight(1.0)
            .color(color_yellow);

        // --- Radii to tangent points (cyan) ---
        draw.line().start(c1).end(p1).weight(1.0).color(color_cyan);
        draw.line().start(c2).end(p2).weight(1.0).color(color_cyan);

        // --- Bezier curve skeleton (magenta) ---
        let points = if i == 0 { side_a } else { side_b };
        for window in points.windows(2) {
            draw.line()
                .start(window[0])
                .end(window[1])
                .weight(2.0)
                .color(color_magenta);
        }
    }
}
