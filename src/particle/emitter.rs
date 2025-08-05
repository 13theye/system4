/// src/particle/emitter.rs
///
/// The thing that spits out particles, refactored to spit out particle data
use crate::{particle::Particles, view::Voice};
use nannou::prelude::*;
use nannou::rand::{rngs::ThreadRng, Rng};

/// The Emitter trait is implemented by different shapes of emitters.
pub trait Emitter {
    fn emit(&self, speed: f32, size: f32, rgba: Rgba, rng: &mut ThreadRng) -> Particles;
    fn should_emit_particle(&self, rng: &mut ThreadRng) -> bool;
    fn is_spawning(&self) -> bool;
    fn set_is_spawning(&mut self, is_spawning: bool);
    fn set_spawn_rate_factor(&mut self, spawn_rate_factor: f32);
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
    pub is_spawning: bool,
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
            is_spawning: false,
        }
    }
}

impl Emitter for PointEmitter {
    /// Generate a Particles struct containing all particles emitted based on the emitter's parameters
    fn emit(&self, speed: f32, size: f32, rgba: Rgba, rng: &mut ThreadRng) -> Particles {
        let mut new_particles = Particles::new();

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

            new_particles.add_new_particle_with_motion(
                self.id,
                self.parent_voice,
                spawn_position,
                size,
                rgba,
                vec2(0.0, 0.0),
                velocity,
            );
        }

        new_particles
    }

    fn should_emit_particle(&self, rng: &mut ThreadRng) -> bool {
        let emission_probability = (self.max_spawn_rate * self.spawn_rate_factor) / 30.0; // Normalize to reasonable probability
        rng.gen::<f32>() < emission_probability.min(1.0)
    }

    fn is_spawning(&self) -> bool {
        self.is_spawning
    }

    fn set_is_spawning(&mut self, is_spawning: bool) {
        self.is_spawning = is_spawning;
    }

    fn set_spawn_rate_factor(&mut self, spawn_rate_factor: f32) {
        self.spawn_rate_factor = spawn_rate_factor;
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
    pub origin: Vec2,        // Center point
    pub start: Vec2,         // Start point
    pub end: Vec2,           // End point
    pub direction: EmitDirection,
    pub max_spawn_rate: f32,
    pub spawn_rate_factor: f32,
    pub is_spawning: bool,
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
        origin: Vec2,
        start: Vec2,
        end: Vec2,
        direction: EmitDirection,
        max_spawn_rate: f32,
        spawn_rate_factor: f32,
    ) -> Self {
        Self {
            id,
            parent_voice,
            origin,
            start,
            end,
            direction,
            max_spawn_rate,
            spawn_rate_factor,
            is_spawning: false,
        }
    }
}

impl Emitter for LinearEmitter {
    /// Generate a Particles struct containing all particles emitted based on the emitter's parameters
    fn emit(&self, speed: f32, size: f32, rgba: Rgba, rng: &mut ThreadRng) -> Particles {
        let mut new_particles = Particles::new();

        // Use probabilistic emission instead of fixed rate for timing variation
        let base_rate = self.max_spawn_rate * self.spawn_rate_factor;
        let max_particles = (base_rate * 1.5) as usize; // Allow some variation above base rate

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
            let speed_variation = rng.gen_range(0.8..1.2);
            let velocity = base_velocity * speed_variation;

            let var_pos = rng.gen_range(gen_range.clone());

            let base_position = match self.direction {
                EmitDirection::North => vec2(var_pos + self.origin.x, self.origin.y),
                EmitDirection::South => vec2(var_pos + self.origin.x, self.origin.y),
                EmitDirection::East => vec2(self.origin.x, var_pos + self.origin.y),
                EmitDirection::West => vec2(self.origin.x, var_pos + self.origin.y),
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

            new_particles.add_new_particle_with_motion(
                self.id,
                self.parent_voice,
                position,
                size,
                rgba,
                vec2(0.0, 0.0),
                velocity,
            );
        }

        new_particles
    }

    fn should_emit_particle(&self, rng: &mut ThreadRng) -> bool {
        let emission_probability = (self.max_spawn_rate * self.spawn_rate_factor) / 30.0; // Normalize to reasonable probability
        rng.gen::<f32>() < emission_probability.min(1.0)
    }

    fn is_spawning(&self) -> bool {
        self.is_spawning
    }

    fn set_is_spawning(&mut self, is_spawning: bool) {
        self.is_spawning = is_spawning;
    }

    fn set_spawn_rate_factor(&mut self, spawn_rate_factor: f32) {
        self.spawn_rate_factor = spawn_rate_factor;
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
