// Constants for the Particle System

use crate::rendering::{render_state::GpuParticleBuffer, GpuSegmentBuffer};

// Constants for the Particle Core
pub const PARTICLE_MASS: f32 = 11.0;

// NOTE: These durations are now in FRAMES for CPU path compatibility.
// The GPU path will convert these to use actual dt (framerate_factor).
// At 60fps: 1800 frames = 30 seconds, 180 frames = 3 seconds, 100 frames = 1.67 seconds
pub const PARTICLE_LIFE_SPAN: f32 = 1800.0;        // frames (30 seconds at 60fps)
pub const PARTICLE_FADE_IN_DURATION: f32 = 180.0;  // frames (3 seconds at 60fps)
pub const PARTICLE_FADE_OUT_DURATION: f32 = 100.0; // frames (1.67 seconds at 60fps)
/// Buffer for out-of-bounds particles
pub const OOB_BUFFER: f32 = 1500.0;
pub const FEEDBACK_POSITIONS: usize = 128;

// Constant used on the system-level
pub const EMPTY_GPU_PARTICLE_BUFFER: GpuParticleBuffer = Vec::new();
pub const EMPTY_GPU_SEGMENT_BUFFER: GpuSegmentBuffer = Vec::new();
pub const PARTICLE_MAX_POSITION_OFFSET: f32 = 10.0; // Maximum screen distance for position offset in pixels
pub const PARTICLE_MAX_SPAWN_RATE: f32 = 80.0;

pub const PARTICLE_HIGH_R: f32 = 1.0;
pub const PARTICLE_HIGH_G: f32 = 1.0;
pub const PARTICLE_HIGH_B: f32 = 0.0;

// Animation timing constants
pub const FADE_DURATION: f32 = 1.0;
pub const RAMP_UP_PERCENT: f32 = 0.1;
pub const DWELL_PERCENT: f32 = 0.3;
pub const RAMP_CURVE_EXPONENT: f32 = 3.0;
pub const FADE_CURVE_EXPONENT: f32 = 1.5;
