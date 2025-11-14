/// src/particle/emitter.rs
///
/// The thing that spits out particles
use crate::{groups::VoiceId, particle::Particle};
use nannou::prelude::*;
use rand::{rngs::ThreadRng, Rng};

/// The Emitter trait is implemented by different shapes of emitters.
pub trait Emitter {
    fn emit(
        &self,
        spawn_rate_factor: f32,
        speed: f32,
        size: f32,
        color: Rgba,
        rng: &mut ThreadRng,
    ) -> Vec<Particle>;
    fn is_enabled(&self) -> bool;
    fn set_enabled(&mut self, is_enabled: bool);
    fn parent_voice(&self) -> VoiceId;
    fn draw(&self, draw: &Draw, scale_x: f32, scale_y: f32);
}

pub struct FullScreenRandomEmitter {
    pub parent_voice: VoiceId,
    pub max_spawn_rate: f32,
    pub spawn_area: Rect,
    pub is_enabled: bool,
}

impl FullScreenRandomEmitter {
    pub fn new(parent_voice: VoiceId, spawn_area: Rect, max_spawn_rate: f32) -> Self {
        Self {
            parent_voice,
            spawn_area,
            max_spawn_rate,
            is_enabled: false,
        }
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
    ) -> Vec<Particle> {
        let adjusted_rate = self.max_spawn_rate * spawn_rate_factor;
        let mut particles = Vec::new();

        for _ in 0..adjusted_rate as usize {
            let spawn_pos = vec2(
                rng.random_range(self.spawn_area.left()..self.spawn_area.right()),
                rng.random_range(self.spawn_area.bottom()..self.spawn_area.top()),
            );

            // override nominal velocity
            let velocity = 2.0;
            let velocity = vec2(
                rng.random_range(-velocity..velocity),
                rng.random_range(-velocity..velocity),
            ) / velocity;

            particles.push(Particle::new(spawn_pos, size, color).with_velocity(velocity));
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

    fn draw(&self, draw: &Draw, scale_x: f32, scale_y: f32) {
        draw.rect()
            .xy(self.spawn_area.xy() * vec2(scale_x, scale_y))
            .wh(self.spawn_area.wh() * vec2(scale_x, scale_y))
            .stroke_color(rgba(1.0, 0.0, 0.0, 0.2))
            .stroke_weight(10.0)
            .no_fill();
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
    ) -> Vec<Particle> {
        let adjusted_rate = self.max_spawn_rate * spawn_rate_factor;
        let mut particles = Vec::with_capacity(adjusted_rate as usize);

        for _ in 0..adjusted_rate as usize {
            // Generate random angle for velocity direction
            let angle = rng.random_range(0.0..std::f32::consts::TAU);

            // Add speed variation to make particles less uniform
            let speed_variation = rng.random_range(0.7..1.3);
            let varied_speed = speed * speed_variation;
            let velocity = vec2(angle.cos() * varied_speed, angle.sin() * varied_speed);

            // Add random offset to spawn position to avoid exact point emission
            let spawn_offset_radius = 8.0;
            let offset_angle = rng.random_range(0.0..std::f32::consts::TAU);
            let offset_distance = rng.random_range(0.0..spawn_offset_radius);
            let spawn_position = self.origin
                + vec2(
                    offset_angle.cos() * offset_distance,
                    offset_angle.sin() * offset_distance,
                );

            particles.push(Particle::new(spawn_position, size, color).with_velocity(velocity));
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

    fn draw(&self, draw: &Draw, scale_x: f32, scale_y: f32) {
        draw.ellipse()
            .radius(2.0)
            .xy(self.origin * vec2(scale_x, scale_y))
            .color(rgba(1.0, 0.0, 0.0, 0.2))
            .stroke_weight(0.0);
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

pub enum EmitDirection {
    North,
    South,
    East,
    West,
}

#[allow(clippy::too_many_arguments)]
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
            midpoint: (start + end) / 2.0,
            start,
            end,
            direction,
            max_spawn_rate,
            is_enabled: false,
        }
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
    ) -> Vec<Particle> {
        let adjusted_rate = self.max_spawn_rate * spawn_rate_factor;

        let base_velocity = match self.direction {
            EmitDirection::North => vec2(0.0, speed),
            EmitDirection::South => vec2(0.0, -speed),
            EmitDirection::West => vec2(-speed, 0.0),
            EmitDirection::East => vec2(speed, 0.0),
        };
        let length = self.end.distance(self.start) - 4.0;
        let random_range = -length / 2.0..length / 2.0;

        let mut particles = Vec::with_capacity(adjusted_rate as usize);

        for _ in 0..adjusted_rate as usize {
            // Add speed variation to make particles less uniform
            let speed_variation = rng.random_range(0.95..1.05);
            let velocity = base_velocity * speed_variation;

            let var_pos = rng.random_range(random_range.clone());

            let base_position = match self.direction {
                EmitDirection::North => vec2(var_pos + self.midpoint.x, self.midpoint.y),
                EmitDirection::South => vec2(var_pos + self.midpoint.x, self.midpoint.y),
                EmitDirection::East => vec2(self.midpoint.x, var_pos + self.midpoint.y),
                EmitDirection::West => vec2(self.midpoint.x, var_pos + self.midpoint.y),
            };

            // Add small random offset perpendicular to emission direction
            let offset_magnitude = rng.random_range(-3.0..3.0);
            let position = match self.direction {
                EmitDirection::North | EmitDirection::South => {
                    base_position + vec2(offset_magnitude, 0.0)
                }
                EmitDirection::East | EmitDirection::West => {
                    base_position + vec2(0.0, offset_magnitude)
                }
            };

            particles.push(Particle::new(position, size, color).with_velocity(velocity));
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

    fn draw(&self, draw: &Draw, scale_x: f32, scale_y: f32) {
        draw.line()
            .start(self.start * vec2(scale_x, scale_y))
            .end(self.end * vec2(scale_x, scale_y))
            .color(rgba(1.0, 0.0, 0.0, 0.2))
            .stroke_weight(4.0);
    }
}
