use nannou::noise::{NoiseFn, Perlin};
use nannou::prelude::*;

use crate::{
    forces::wind::{CircleFormation, Wind},
    groups::VoiceId,
    particle::ParticleCore,
};

/// Given a particle, computes the forces present at the particle's location.
/// Updates the particle based on these forces.
pub fn apply_combined_winds_to_particle(
    particle: &mut ParticleCore,
    circles: &[&dyn CircleFormation],
    mass_variation_factor: f32,
    noise: f64,
) {
    let Some(combined_wind) = combined_wind_at_pos(circles, particle.position, noise) else {
        return; // No wind = no force applied
    };

    combined_wind.apply(particle, mass_variation_factor);
}

/// Get combined `Wind` at a position in ParticleSystem coordinates
/// Returns `None` if position is out of bounds or if there is no wind at that position
pub fn combined_wind_at_pos(
    circles: &[&dyn CircleFormation],
    position: Vec2,
    noise: f64,
) -> Option<Wind> {
    let total_velocity = circles
        .iter()
        .filter_map(|circle| circle.wind_for_position(position, noise))
        .map(|wind| wind.velocity)
        .reduce(|a, b| a + b)?;

    // Very low strength are clamped to None for aesthetics
    if total_velocity.length() < 0.05 {
        return None;
    }

    Some(Wind {
        velocity: total_velocity,
    })
}

pub fn apply_voice_wind_to_particle(
    particle: &mut ParticleCore,
    circles: &[&dyn CircleFormation],
    mass_variation_factor: f32,
    noise: f64,
    current_voice: &VoiceId,
) {
    let Some(combined_wind) = voice_wind_at_pos(circles, particle.position, noise, current_voice)
    else {
        return; // No wind = no force applied
    };

    combined_wind.apply(particle, mass_variation_factor);
}

/// Get a combined `Wind` at a position in ParticleSystem coordinates, only including
/// `Winds` from the current voice
/// Returns `None` if position is out of bounds or if there is no wind at that position
pub fn voice_wind_at_pos(
    circles: &[&dyn CircleFormation],
    position: Vec2,
    noise: f64,
    current_voice: &VoiceId,
) -> Option<Wind> {
    let total_velocity = circles
        .iter()
        .filter(|circle| circle.parent_voice() == *current_voice)
        .filter_map(|circle| circle.wind_for_position(position, noise))
        .map(|wind| wind.velocity)
        .reduce(|a, b| a + b)?;

    Some(Wind {
        velocity: total_velocity,
    })
}

/// Parameters for the WindField
#[derive(Default)]
pub struct WindFieldParams {
    pub origin: Vec2,
    pub bounds_size: Vec2,
}

/// The WindField defines the bounds and location of the force field overlaid on the
/// ParticleSystem, and provides methods for visualizing Wind forces.
/// The actual force objects are stored in Voices.
pub struct WindField {
    pub params: WindFieldParams,
}

impl WindField {
    /// Create a new WindField, with a center origin and x&y size
    pub fn new(origin: Vec2, bounds_size: Vec2) -> Self {
        let params = WindFieldParams {
            origin,
            bounds_size,
        };
        Self { params }
    }

    pub fn size(&self) -> Vec2 {
        self.params.bounds_size
    }

    /******************* Drawing methods ****************************************/

    /// Draw the WindField with force vectors and origin
    pub fn draw(
        &self,
        circles: &[&dyn CircleFormation],
        draw: &Draw,
        scale_x: f32,
        scale_y: f32,
        perlin_gen: Perlin,
        should_combine: bool,
    ) {
        self.draw_origin(draw, scale_x, scale_y);
        self.draw_vectors(circles, draw, scale_x, scale_y, perlin_gen, should_combine);
    }

    /// Draw all the Wind vectors
    fn draw_vectors(
        &self,
        circles: &[&dyn CircleFormation],
        draw: &Draw,
        scale_x: f32,
        scale_y: f32,
        perlin_gen: Perlin,
        should_combine: bool,
    ) {
        let step_size = 50;
        let vector_scale = 3.0; // Increased scale for better visibility

        let params = &self.params;

        // Calculate bounds
        let x_min = params.origin.x - params.bounds_size.x / 2.0;
        let x_max = params.origin.x + params.bounds_size.x / 2.0;
        let y_min = params.origin.y - params.bounds_size.y / 2.0;
        let y_max = params.origin.y + params.bounds_size.y / 2.0;

        // Calculate how many steps we can fit in each direction from origin
        let steps_left = ((params.origin.x - x_min) / step_size as f32).floor() as isize;
        let steps_right = ((x_max - params.origin.x) / step_size as f32).floor() as isize;
        let steps_down = ((params.origin.y - y_min) / step_size as f32).floor() as isize;
        let steps_up = ((y_max - params.origin.y) / step_size as f32).floor() as isize;

        // Generate grid centered on origin
        for x_step in -steps_left..=steps_right {
            for y_step in -steps_down..=steps_up {
                let cell_origin = params.origin
                    + vec2(
                        x_step as f32 * step_size as f32,
                        y_step as f32 * step_size as f32,
                    );

                // Sample noise factor at slight offset from whole-number cell_origin
                // because Perlin at whole-number coordinates gives noise factors of 0.
                // So, the visualization is for an idea of the amount of noise present
                // but not necessarily accurate for the actual noise applied on a
                // per-particle basis.
                let noise_factor =
                    perlin_gen.get([(cell_origin.x + 0.1) as f64, (cell_origin.y + 0.1) as f64]);

                if should_combine {
                    // Calculate wind at cell origin
                    if let Some(wind) = combined_wind_at_pos(circles, cell_origin, noise_factor) {
                        self.draw_wind_vector(
                            wind,
                            cell_origin,
                            draw,
                            vector_scale,
                            scale_x,
                            scale_y,
                        );
                    }
                } else {
                    // Calculate and draw wind vectors for each voice
                    for voice_id in VoiceId::all_drones() {
                        if let Some(wind) =
                            voice_wind_at_pos(circles, cell_origin, noise_factor, &voice_id)
                        {
                            self.draw_wind_vector(
                                wind,
                                cell_origin,
                                draw,
                                vector_scale,
                                scale_x,
                                scale_y,
                            );
                        }
                    }
                }
            }
        }
    }

    /// Draw a single Wind vector
    fn draw_wind_vector(
        &self,
        wind: Wind,
        cell_origin: Vec2,
        draw: &Draw,
        vector_scale: f32,
        scale_x: f32,
        scale_y: f32,
    ) {
        // Draw wind vector from cell origin
        let direction = wind.direction();
        let strength = wind.strength();
        let vector_end = cell_origin + direction * strength * vector_scale;

        // Draw vector as line
        draw.line()
            .start(cell_origin * vec2(scale_x, scale_y))
            .end(vector_end * vec2(scale_x, scale_y))
            .color(rgba(0.0, 0.8, 1.0, 0.2))
            .stroke_weight(2.0);

        // Draw "arrowhead"
        draw.rect()
            .xy(vector_end * vec2(scale_x, scale_y))
            .wh(vec2(10.0 * scale_x, 10.0 * scale_y))
            .color(rgba(1.0, 0.0, 0.0, 1.0))
            .stroke_weight(0.0);
    }

    /// Draw the origin of this WindField
    fn draw_origin(&self, draw: &Draw, scale_x: f32, scale_y: f32) {
        draw.ellipse()
            .xy(self.params.origin * vec2(scale_x, scale_y))
            .w_h(20.0 * scale_x, 20.0 * scale_y)
            .color(rgba(0.0, 1.0, 0.0, 0.2));
    }
}
