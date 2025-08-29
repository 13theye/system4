/// src/particle/emitter.rs
///
/// The thing that spits out particles
use crate::{particle::Particle, voice::Voice};
use nannou::prelude::*;
use nannou::rand::{rngs::ThreadRng, Rng};

/// The Emitter trait is implemented by different shapes of emitters.
pub trait Emitter {
    fn emit(&self, speed: f32, size: f32, color: Rgba, rng: &mut ThreadRng) -> Vec<Particle>;
    fn should_emit_particle(&self, rng: &mut ThreadRng) -> bool;
    fn is_enabled(&self) -> bool;
    fn set_enabled(&mut self, is_enabled: bool);
    fn set_spawn_rate_factor(&mut self, spawn_rate_factor: f32);
    fn get_spawn_rate_factor(&self) -> f32;
    fn parent_voice(&self) -> Voice;
    fn draw(&self, draw: &Draw, scale_x: f32, scale_y: f32);
}

/// An emitter defined by a center point
/// Radiates particles outward from that point in any direction
pub struct PointEmitter {
    pub id: usize,
    pub parent_voice: Voice,
    pub origin: Vec2,
    pub max_spawn_rate: f32,
    pub spawn_rate_factor: f32,
    pub is_enabled: bool,
}

impl PointEmitter {
    pub fn new(
        id: usize,
        parent_voice: Voice,
        origin: Vec2,
        max_spawn_rate: f32,
        spawn_rate_factor: f32,
    ) -> Self {
        Self {
            id,
            parent_voice,
            origin,
            max_spawn_rate,
            spawn_rate_factor,
            is_enabled: false,
        }
    }
}

impl Emitter for PointEmitter {
    /// Generate a Vec of particles based on the emitter's parameters
    fn emit(&self, speed: f32, size: f32, color: Rgba, rng: &mut ThreadRng) -> Vec<Particle> {
        let mut particles = Vec::new();

        // Use probabilistic emission instead of fixed rate for timing variation
        let adjusted_rate = self.max_spawn_rate * self.spawn_rate_factor;

        for _ in 0..adjusted_rate as usize {
            // Only emit if random chance based on spawn rate
            if !self.should_emit_particle(rng) {
                continue;
            }

            // Generate random angle for velocity direction
            let angle = rng.gen_range(0.0..std::f32::consts::TAU);

            // Add speed variation to make particles less uniform
            let speed_variation = rng.gen_range(0.7..1.3);
            let varied_speed = speed * speed_variation;
            let velocity = vec2(angle.cos() * varied_speed, angle.sin() * varied_speed);

            // Add random offset to spawn position to avoid exact point emission
            let spawn_offset_radius = 8.0;
            let offset_angle = rng.gen_range(0.0..std::f32::consts::TAU);
            let offset_distance = rng.gen_range(0.0..spawn_offset_radius);
            let spawn_position = self.origin
                + vec2(
                    offset_angle.cos() * offset_distance,
                    offset_angle.sin() * offset_distance,
                );

            particles.push(Particle::new_with_motion(
                self.id,
                spawn_position,
                size,
                color,
                vec2(0.0, 0.0),
                velocity,
            ));
        }

        particles
    }

    fn should_emit_particle(&self, rng: &mut ThreadRng) -> bool {
        let emission_probability = (self.max_spawn_rate * self.spawn_rate_factor) / 30.0; // Normalize to reasonable probability
        rng.gen::<f32>() < emission_probability.min(1.0)
    }

    fn is_enabled(&self) -> bool {
        self.is_enabled
    }

    fn set_enabled(&mut self, is_spawning: bool) {
        self.is_enabled = is_spawning;
    }

    fn set_spawn_rate_factor(&mut self, spawn_rate_factor: f32) {
        self.spawn_rate_factor = spawn_rate_factor;
    }

    fn get_spawn_rate_factor(&self) -> f32 {
        self.spawn_rate_factor
    }

    fn parent_voice(&self) -> Voice {
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
    pub id: usize,           // unique id for this emitter
    pub parent_voice: Voice, // voice that this emitter belongs to
    pub midpoint: Vec2,      // Center point
    pub start: Vec2,         // Start point
    pub end: Vec2,           // End point
    pub direction: EmitDirection,
    pub max_spawn_rate: f32,
    pub spawn_rate_factor: f32,
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
        id: usize,
        parent_voice: Voice,
        start: Vec2,
        end: Vec2,
        direction: EmitDirection,
        max_spawn_rate: f32,
        spawn_rate_factor: f32,
    ) -> Self {
        Self {
            id,
            parent_voice,
            midpoint: (start + end) / 2.0,
            start,
            end,
            direction,
            max_spawn_rate,
            spawn_rate_factor,
            is_enabled: false,
        }
    }
}

impl Emitter for LinearEmitter {
    // Generate a Vec of particles based on the emitter's parameters
    fn emit(&self, speed: f32, size: f32, color: Rgba, rng: &mut ThreadRng) -> Vec<Particle> {
        let mut particles = Vec::new();

        // Use probabilistic emission instead of fixed rate for timing variation
        let base_rate = self.max_spawn_rate * self.spawn_rate_factor;
        let max_particles = base_rate as usize; // Allow some variation above base rate

        let base_velocity = match self.direction {
            EmitDirection::North => vec2(0.0, speed),
            EmitDirection::South => vec2(0.0, -speed),
            EmitDirection::West => vec2(-speed, 0.0),
            EmitDirection::East => vec2(speed, 0.0),
        };
        let length = self.end.distance(self.start) - 4.0;
        let gen_range = -length / 2.0..length / 2.0;

        for _ in 0..max_particles {
            // Only emit if random chance based on spawn rate
            if !self.should_emit_particle(rng) {
                continue;
            }

            // Add speed variation to make particles less uniform
            let speed_variation = rng.gen_range(0.95..1.05);
            let velocity = base_velocity * speed_variation;

            let var_pos = rng.gen_range(gen_range.clone());

            let base_position = match self.direction {
                EmitDirection::North => vec2(var_pos + self.midpoint.x, self.midpoint.y),
                EmitDirection::South => vec2(var_pos + self.midpoint.x, self.midpoint.y),
                EmitDirection::East => vec2(self.midpoint.x, var_pos + self.midpoint.y),
                EmitDirection::West => vec2(self.midpoint.x, var_pos + self.midpoint.y),
            };

            // Add small random offset perpendicular to emission direction
            let offset_magnitude = rng.gen_range(-3.0..3.0);
            let position = match self.direction {
                EmitDirection::North | EmitDirection::South => {
                    base_position + vec2(offset_magnitude, 0.0)
                }
                EmitDirection::East | EmitDirection::West => {
                    base_position + vec2(0.0, offset_magnitude)
                }
            };

            particles.push(Particle::new_with_motion(
                self.id,
                position,
                size,
                color,
                vec2(0.0, 0.0),
                velocity,
            ));
        }

        particles
    }

    fn should_emit_particle(&self, rng: &mut ThreadRng) -> bool {
        let emission_probability = (self.max_spawn_rate * self.spawn_rate_factor) / 30.0; // Normalize to reasonable probability
        rng.gen::<f32>() < emission_probability.min(1.0)
    }

    fn is_enabled(&self) -> bool {
        self.is_enabled
    }

    fn set_enabled(&mut self, is_spawning: bool) {
        self.is_enabled = is_spawning;
    }

    fn set_spawn_rate_factor(&mut self, spawn_rate_factor: f32) {
        self.spawn_rate_factor = spawn_rate_factor;
    }

    fn get_spawn_rate_factor(&self) -> f32 {
        self.spawn_rate_factor
    }

    fn parent_voice(&self) -> Voice {
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
