use nannou::noise::{NoiseFn, Perlin};
use nannou::prelude::*;

use crate::{
    forces::wind::{Wind, WindCircle},
    particle::ParticleCore,
};

/// Given a particle, computes the forces present at the particle's location.
/// Updates the particle based on these forces.
pub fn apply_winds_to_particle(
    particle: &mut ParticleCore,
    circles: &[&WindCircle],
    mass_variation_factor: f32,
    noise: f64,
) {
    let Some(combined_wind) = combined_wind_at_pos(circles, particle.position, noise) else {
        return; // No wind = no force applied
    };

    combined_wind.apply(particle, mass_variation_factor);
}

/// Get combined wind at a position in ParticleSystem coordinates
/// Returns None if position is out of bounds or if there is no wind at that position
pub fn combined_wind_at_pos(circles: &[&WindCircle], position: Vec2, noise: f64) -> Option<Wind> {
    let total_velocity = circles
        .iter()
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
    /// Create a new WindField, with a center origin, x&y size, number of columns and number of rows.
    /// Top-left is (0,0).
    pub fn new(origin: Vec2, bounds_size: Vec2) -> Self {
        let params = WindFieldParams {
            origin,
            bounds_size,
        };
        Self { params }
    }

    /******************* Drawing methods ****************************************/

    /// Draw the WindField with force vectors and origin
    pub fn draw(
        &self,
        circles: &[&WindCircle],
        draw: &Draw,
        scale_x: f32,
        scale_y: f32,
        perlin_gen: Perlin,
    ) {
        self.draw_origin(draw, scale_x, scale_y);
        self.draw_vectors(circles, draw, scale_x, scale_y, perlin_gen);
    }

    /// Draw all the Wind vectors
    fn draw_vectors(
        &self,
        circles: &[&WindCircle],
        draw: &Draw,
        scale_x: f32,
        scale_y: f32,
        perlin_gen: Perlin,
    ) {
        let step_size = 50;
        let vector_scale = 3.0; // Increased scale for better visibility

        let params = &self.params;
        let (x_min, x_max) = (
            (params.origin.x - params.bounds_size.x / 2.0) as isize,
            (params.origin.x + params.bounds_size.x / 2.0) as isize,
        );
        let (y_min, y_max) = (
            (params.origin.y - params.bounds_size.y / 2.0) as isize,
            (params.origin.y + params.bounds_size.y / 2.0) as isize,
        );

        let mut x = x_min;

        while x <= x_max {
            let mut y = y_min;
            while y <= y_max {
                let cell_origin = vec2(x as f32, y as f32);

                // Sample noise factor at slight offset from whole-number cell_origin
                // because Perlin at whole-number coordinates gives noise factors of 0.
                // So, the visualization is for an idea of the amount of noise present
                // but not necessarily accurate for the actual noise applied on a
                // per-particle basis.
                let noise_factor =
                    perlin_gen.get([(cell_origin.x + 0.1) as f64, (cell_origin.y + 0.1) as f64]);

                // Calculate wind at cell origin
                if let Some(wind) = combined_wind_at_pos(circles, cell_origin, noise_factor) {
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

                y += step_size;
            }
            x += step_size
        }
    }

    /// Draw the origin of this WindField
    fn draw_origin(&self, draw: &Draw, scale_x: f32, scale_y: f32) {
        draw.ellipse()
            .xy(self.params.origin * vec2(scale_x, scale_y))
            .w_h(20.0 * scale_x, 20.0 * scale_y)
            .color(rgba(0.0, 1.0, 0.0, 0.2));
    }
}
