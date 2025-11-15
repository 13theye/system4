// system4-app/src/physics_ext.rs
// Extensions and conversions for system4-core physics types

use nannou::prelude::*;
use nnpipe::renderers::{ParticleGpu, SegmentGpu};
use system4_core::physics::{
    particles::{ParticleCore, ParticleFeedback, FEEDBACK_POSITIONS},
    Rgb as CoreRgb,
    Rgba as CoreRgba,
};

// Import glam 0.29 (used by core) to distinguish from Nannou's glam 0.17
use glam as core_glam;

// Import app's particle types to distinguish from core
use crate::particle::particles::ParticleCore as AppParticleCore;

// ========== Type Conversions ==========

/// Convert Nannou Vec2/Point2 to core glam Vec2 (glam 0.29)
#[inline]
pub fn to_core_vec2(v: Vec2) -> core_glam::Vec2 {
    core_glam::vec2(v.x, v.y)
}

/// Convert core glam Vec2 (glam 0.29) to Nannou Point2
#[inline]
pub fn to_nannou_point2(v: core_glam::Vec2) -> Point2 {
    pt2(v.x, v.y)
}

/// Convert core glam Vec2 (glam 0.29) to Nannou Vec2
#[inline]
pub fn to_nannou_vec2(v: core_glam::Vec2) -> Vec2 {
    vec2(v.x, v.y)
}

/// Convert Nannou Rgb to core Rgb
#[inline]
pub fn to_core_rgb(color: Rgb) -> CoreRgb {
    CoreRgb::new(color.red, color.green, color.blue)
}

/// Convert core Rgb to Nannou Rgb
#[inline]
pub fn to_nannou_rgb(color: CoreRgb) -> Rgb {
    rgb(color.r, color.g, color.b)
}

/// Convert Nannou Rgba to core Rgba
#[inline]
pub fn to_core_rgba(color: Rgba) -> CoreRgba {
    CoreRgba::new(color.red, color.green, color.blue, color.alpha)
}

/// Convert core Rgba to Nannou Rgba
#[inline]
pub fn to_nannou_rgba(color: CoreRgba) -> Rgba {
    rgba(color.color.r, color.color.g, color.color.b, color.alpha)
}

/// Convert app ParticleCore to core ParticleCore
#[inline]
pub fn app_particle_to_core(app_particle: &AppParticleCore) -> ParticleCore {
    ParticleCore {
        position: to_core_vec2(app_particle.position),
        velocity: to_core_vec2(app_particle.velocity),
        acceleration: to_core_vec2(app_particle.acceleration),
        age: app_particle.age,
        remaining_life_span: app_particle.remaining_life_span,
        age_per_tick: app_particle.age_per_tick,
        is_alive: app_particle.is_alive,
        is_activated: app_particle.is_activated,
        size: app_particle.size,
        mass: app_particle.mass,
        rgba: to_core_rgba(app_particle.rgba),
    }
}

/// Convert core ParticleCore to app ParticleCore
#[inline]
pub fn core_particle_to_app(core_particle: &ParticleCore) -> AppParticleCore {
    AppParticleCore {
        position: to_nannou_point2(core_particle.position),
        velocity: to_nannou_vec2(core_particle.velocity),
        acceleration: to_nannou_vec2(core_particle.acceleration),
        age: core_particle.age,
        remaining_life_span: core_particle.remaining_life_span,
        age_per_tick: core_particle.age_per_tick,
        is_alive: core_particle.is_alive,
        is_activated: core_particle.is_activated,
        size: core_particle.size,
        mass: core_particle.mass,
        rgba: to_nannou_rgba(core_particle.rgba),
    }
}

/// Update app ParticleCore from core ParticleCore (for syncing after physics)
#[inline]
pub fn sync_app_particle_from_core(app_particle: &mut AppParticleCore, core_particle: &ParticleCore) {
    app_particle.position = to_nannou_point2(core_particle.position);
    app_particle.velocity = to_nannou_vec2(core_particle.velocity);
    app_particle.acceleration = to_nannou_vec2(core_particle.acceleration);
    app_particle.age = core_particle.age;
    app_particle.remaining_life_span = core_particle.remaining_life_span;
    app_particle.age_per_tick = core_particle.age_per_tick;
    app_particle.is_alive = core_particle.is_alive;
    app_particle.is_activated = core_particle.is_activated;
}

// ========== Re-export Core Types ==========

// Re-export commonly used core physics types for convenience
pub use system4_core::physics::forces::{CellIdx as CoreCellIdx, Wind as CoreWind, WindField as CoreWindField};
pub use system4_core::physics::particles::{ParticleCore as CoreParticleCore, ParticleFeedback as CoreParticleFeedback};

// ========== GPU Conversion Extensions ==========

/// Extension trait for ParticleCore to add GPU conversion
pub trait ParticleCoreGpuExt {
    fn to_gpu(&self, offset: glam::Vec2) -> ParticleGpu;
    fn is_out_of_bounds_rect(&self, bounds_rect: Rect) -> bool;
}

impl ParticleCoreGpuExt for ParticleCore {
    #[inline]
    fn to_gpu(&self, offset: glam::Vec2) -> ParticleGpu {
        ParticleGpu::new(
            [self.position.x + offset.x, self.position.y + offset.y],
            [self.rgba.color.r, self.rgba.color.g, self.rgba.color.b],
            self.rgba.alpha,
        )
    }

    #[inline]
    fn is_out_of_bounds_rect(&self, bounds_rect: Rect) -> bool {
        let buffer = 1500.0;
        self.position.x < bounds_rect.left() - buffer
            || self.position.x > bounds_rect.right() + buffer
            || self.position.y < bounds_rect.bottom() - buffer
            || self.position.y > bounds_rect.top() + buffer
    }
}

/// Helper function to generate SegmentGpu from core and feedback data
pub fn to_segment_gpu(
    core: &ParticleCore,
    feedback: &ParticleFeedback,
    offset: glam::Vec2,
    segment_length: f32,
    line_width: f32,
) -> SegmentGpu {
    let mut points = [[0.0f32; 2]; FEEDBACK_POSITIONS];
    let mut colors = [[0.0f32; 3]; FEEDBACK_POSITIONS];

    // First point is current position
    points[0] = [core.position.x + offset.x, core.position.y + offset.y];
    colors[0] = [
        core.rgba.color.r,
        core.rgba.color.g,
        core.rgba.color.b,
    ];

    // Fill remaining points and colors from feedback history (reading from ring buffer)
    let mut last_valid_pos = [core.position.x, core.position.y];
    for i in 0..(FEEDBACK_POSITIONS - 1) {
        // Calculate ring buffer index: read backwards from second most recent
        // (because the most recent is the current position)
        let ring_index = (feedback.current_index + FEEDBACK_POSITIONS - 2 - i) % FEEDBACK_POSITIONS;

        if let Some(feedback_pos) = feedback.positions[ring_index] {
            points[i + 1] = [feedback_pos.x, feedback_pos.y];
            last_valid_pos = [feedback_pos.x, feedback_pos.y];
        } else {
            // If no feedback position, use the last valid position to avoid ray artifacts
            points[i + 1] = last_valid_pos;
        }

        if let Some(feedback_color) = feedback.colors[ring_index] {
            colors[i + 1] = [feedback_color.r, feedback_color.g, feedback_color.b];
        } else {
            // If no feedback color, default to black
            colors[i + 1] = [0.0, 0.0, 0.0];
        }
    }

    // Calculate actual history length from particle age, ensuring it's at least 1
    // and doesn't exceed the maximum feedback positions
    let actual_history_length = (core.age as u32).clamp(1, FEEDBACK_POSITIONS as u32);

    SegmentGpu::new(
        points,
        colors,
        core.rgba.alpha,
        segment_length,
        line_width,
        actual_history_length,
    )
}
