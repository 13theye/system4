// system4-core/src/constants/physics.rs
// Physics simulation constants

// ========== Particle Constants ==========

/// Default mass for particles
pub const PARTICLE_MASS: f32 = 11.0;

/// Default particle lifespan in frames
pub const PARTICLE_LIFE_SPAN: f32 = 1800.0;

/// Duration of particle fade-in effect in frames
pub const FADE_IN_DURATION: f32 = 180.0;

/// Duration of particle fade-out effect in frames
pub const FADE_OUT_DURATION: f32 = 100.0;

/// Number of feedback/trail positions to store per particle
pub const FEEDBACK_POSITIONS: usize = 128;

// ========== Force Constants ==========

/// Maximum wind angle deviation in radians (PI = 180 degrees)
pub const MAX_WIND_ANGLE_DEVIATION: f32 = std::f32::consts::PI;

/// Inertia coefficient for particle momentum resistance
/// Higher values = more resistance to velocity changes
pub const INERTIA_COEFFICIENT: f32 = 0.1;
