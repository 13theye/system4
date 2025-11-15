// system4-core/src/physics/particles/emitter.rs
// Core emitter trait and implementations for particle emission

use crate::physics::{particles::ParticleCore, Rgba, VoiceId};
use glam::Vec2;
use rand::{rngs::ThreadRng, Rng};

/// The Emitter trait is implemented by different shapes of emitters.
/// This is the core trait with only physics/emission logic - no rendering.
pub trait Emitter: std::any::Any {
    fn emit(
        &self,
        spawn_rate_factor: f32,
        speed: f32,
        size: f32,
        color: Rgba,
        rng: &mut ThreadRng,
    ) -> Vec<ParticleCore>;
    fn is_enabled(&self) -> bool;
    fn set_enabled(&mut self, is_enabled: bool);
    fn parent_voice(&self) -> VoiceId;
}

/// An emitter that spawns particles randomly across a rectangular area
pub struct FullScreenRandomEmitter {
    pub parent_voice: VoiceId,
    pub max_spawn_rate: f32,
    pub spawn_area_min: Vec2,
    pub spawn_area_max: Vec2,
    pub is_enabled: bool,
}

impl FullScreenRandomEmitter {
    pub fn new(parent_voice: VoiceId, spawn_area_min: Vec2, spawn_area_max: Vec2, max_spawn_rate: f32) -> Self {
        Self {
            parent_voice,
            spawn_area_min,
            spawn_area_max,
            max_spawn_rate,
            is_enabled: false,
        }
    }

    pub fn spawn_area_min(&self) -> Vec2 {
        self.spawn_area_min
    }

    pub fn spawn_area_max(&self) -> Vec2 {
        self.spawn_area_max
    }

    pub fn spawn_area_center(&self) -> Vec2 {
        (self.spawn_area_min + self.spawn_area_max) * 0.5
    }

    pub fn spawn_area_size(&self) -> Vec2 {
        self.spawn_area_max - self.spawn_area_min
    }
}

impl Emitter for FullScreenRandomEmitter {
    fn emit(
        &self,
        spawn_rate_factor: f32,
        _velocity: f32,
        size: f32,
        color: Rgba,
        rng: &mut ThreadRng,
    ) -> Vec<ParticleCore> {
        let adjusted_rate = self.max_spawn_rate * spawn_rate_factor;
        let mut particles = Vec::new();

        for _ in 0..adjusted_rate as usize {
            // Guard against empty ranges
            let x = if self.spawn_area_max.x > self.spawn_area_min.x {
                rng.random_range(self.spawn_area_min.x..self.spawn_area_max.x)
            } else {
                self.spawn_area_min.x
            };
            let y = if self.spawn_area_max.y > self.spawn_area_min.y {
                rng.random_range(self.spawn_area_min.y..self.spawn_area_max.y)
            } else {
                self.spawn_area_min.y
            };
            let spawn_pos = Vec2::new(x, y);

            // override nominal velocity
            let velocity = 2.0;
            let velocity = Vec2::new(
                rng.random_range(-velocity..velocity),
                rng.random_range(-velocity..velocity),
            ) / velocity;

            particles.push(ParticleCore::new(spawn_pos, size, color).with_velocity(velocity));
        }

        particles
    }

    fn is_enabled(&self) -> bool {
        self.is_enabled
    }

    fn set_enabled(&mut self, is_spawning: bool) {
        self.is_enabled = is_spawning;
    }

    fn parent_voice(&self) -> VoiceId {
        self.parent_voice
    }
}

/// An emitter defined by a center point
/// Radiates particles outward from that point in any direction
pub struct PointEmitter {
    pub parent_voice: VoiceId,
    pub origin: Vec2,
    pub max_spawn_rate: f32,
    pub is_enabled: bool,
}

impl PointEmitter {
    pub fn new(parent_voice: VoiceId, origin: Vec2, max_spawn_rate: f32) -> Self {
        Self {
            parent_voice,
            origin,
            max_spawn_rate,
            is_enabled: false,
        }
    }

    pub fn origin(&self) -> Vec2 {
        self.origin
    }
}

impl Emitter for PointEmitter {
    /// Generate a Vec of particles based on the emitter's parameters
    fn emit(
        &self,
        spawn_rate_factor: f32,
        speed: f32,
        size: f32,
        color: Rgba,
        rng: &mut ThreadRng,
    ) -> Vec<ParticleCore> {
        let adjusted_rate = self.max_spawn_rate * spawn_rate_factor;
        let mut particles = Vec::with_capacity(adjusted_rate as usize);

        for _ in 0..adjusted_rate as usize {
            // Generate random angle for velocity direction
            let angle = rng.random_range(0.0..std::f32::consts::TAU);

            // Add speed variation to make particles less uniform
            let speed_variation = rng.random_range(0.7..1.3);
            let varied_speed = speed * speed_variation;
            let velocity = Vec2::new(angle.cos() * varied_speed, angle.sin() * varied_speed);

            // Add random offset to spawn position to avoid exact point emission
            let spawn_offset_radius = 8.0;
            let offset_angle = rng.random_range(0.0..std::f32::consts::TAU);
            let offset_distance = rng.random_range(0.0..spawn_offset_radius);
            let spawn_position = self.origin
                + Vec2::new(
                    offset_angle.cos() * offset_distance,
                    offset_angle.sin() * offset_distance,
                );

            particles.push(ParticleCore::new(spawn_position, size, color).with_velocity(velocity));
        }

        particles
    }

    fn is_enabled(&self) -> bool {
        self.is_enabled
    }

    fn set_enabled(&mut self, is_spawning: bool) {
        self.is_enabled = is_spawning;
    }

    fn parent_voice(&self) -> VoiceId {
        self.parent_voice
    }
}

/// An emitter defined by a center point, a start point, and an end point
/// Emits particles anywhere along the line
pub struct LinearEmitter {
    pub parent_voice: VoiceId, // voice that this emitter belongs to
    pub midpoint: Vec2,        // Center point
    pub start: Vec2,           // Start point
    pub end: Vec2,             // End point
    pub direction: EmitDirection,
    pub max_spawn_rate: f32,
    pub is_enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmitDirection {
    North,
    South,
    East,
    West,
}

impl LinearEmitter {
    pub fn new(
        parent_voice: VoiceId,
        start: Vec2,
        end: Vec2,
        direction: EmitDirection,
        max_spawn_rate: f32,
    ) -> Self {
        Self {
            parent_voice,
            midpoint: (start + end) * 0.5,
            start,
            end,
            direction,
            max_spawn_rate,
            is_enabled: false,
        }
    }

    pub fn midpoint(&self) -> Vec2 {
        self.midpoint
    }

    pub fn start(&self) -> Vec2 {
        self.start
    }

    pub fn end(&self) -> Vec2 {
        self.end
    }

    pub fn direction(&self) -> EmitDirection {
        self.direction
    }
}

impl Emitter for LinearEmitter {
    // Generate a Vec of particles based on the emitter's parameters
    fn emit(
        &self,
        spawn_rate_factor: f32,
        speed: f32,
        size: f32,
        color: Rgba,
        rng: &mut ThreadRng,
    ) -> Vec<ParticleCore> {
        let adjusted_rate = self.max_spawn_rate * spawn_rate_factor;

        let base_velocity = match self.direction {
            EmitDirection::North => Vec2::new(0.0, speed),
            EmitDirection::South => Vec2::new(0.0, -speed),
            EmitDirection::West => Vec2::new(-speed, 0.0),
            EmitDirection::East => Vec2::new(speed, 0.0),
        };
        let length = (self.end.distance(self.start) - 4.0).max(0.0);
        let random_range = -length / 2.0..length / 2.0;

        let mut particles = Vec::with_capacity(adjusted_rate as usize);

        for _ in 0..adjusted_rate as usize {
            // Add speed variation to make particles less uniform
            let speed_variation = rng.random_range(0.95..1.05);
            let velocity = base_velocity * speed_variation;

            let var_pos = if length > 0.0 {
                rng.random_range(random_range.clone())
            } else {
                0.0
            };

            let base_position = match self.direction {
                EmitDirection::North => Vec2::new(var_pos + self.midpoint.x, self.midpoint.y),
                EmitDirection::South => Vec2::new(var_pos + self.midpoint.x, self.midpoint.y),
                EmitDirection::East => Vec2::new(self.midpoint.x, var_pos + self.midpoint.y),
                EmitDirection::West => Vec2::new(self.midpoint.x, var_pos + self.midpoint.y),
            };

            // Add small random offset perpendicular to emission direction
            let offset_magnitude = rng.random_range(-3.0..3.0);
            let position = match self.direction {
                EmitDirection::North | EmitDirection::South => {
                    base_position + Vec2::new(offset_magnitude, 0.0)
                }
                EmitDirection::East | EmitDirection::West => {
                    base_position + Vec2::new(0.0, offset_magnitude)
                }
            };

            particles.push(ParticleCore::new(position, size, color).with_velocity(velocity));
        }

        particles
    }

    fn is_enabled(&self) -> bool {
        self.is_enabled
    }

    fn set_enabled(&mut self, is_spawning: bool) {
        self.is_enabled = is_spawning;
    }

    fn parent_voice(&self) -> VoiceId {
        self.parent_voice
    }
}
