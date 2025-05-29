// src/forces/wind.rs
//
// Grid-based wind force for particle system

use nannou::prelude::*;

use crate::{forces::Force, particle::Particle};

const DEBUG: bool = false;

#[derive(Clone, Debug, Default)]
pub struct Wind {
    direction: Vec2,
    strength: f32,
    turbulence: f32, // random variation 0.0-1.0
}

impl Force for Wind {
    fn apply(&self, particle: &mut Particle) {
        // Calculate the x and y components of particle's current velocity
        let particle_vx = particle.velocity.x;
        let particle_vy = particle.velocity.y;

        // Calculate the x and y components of wind's target velocity
        let wind_vx = self.direction.x * self.strength;
        let wind_vy = self.direction.y * self.strength;

        // Calculate the difference in each component
        let diff_x = wind_vx - particle_vx;
        let diff_y = wind_vy - particle_vy;

        // Calculate inertial resistance based on current momentum
        let current_speed = particle.velocity.length();
        let momentum_magnitude = particle.mass * current_speed;

        // Inertial resistance: particles with higher momentum resist changes more
        let inertia_coefficient = 0.1; // Adjust this to control resistance strength
        let inertia_factor = 1.0 / (1.0 + momentum_magnitude * inertia_coefficient);

        // Apply the force with inertial resistance
        let force = vec2(diff_x, diff_y) * inertia_factor;
        particle.acceleration += force / particle.mass;
    }
}

impl Wind {
    pub fn new() -> Self {
        Self {
            direction: vec2(0.0, 0.0),
            strength: 0.0,
            turbulence: 0.0,
        }
    }

    pub fn new_with(direction: Vec2, strength: f32, turbulence: f32) -> Self {
        Self {
            direction,
            strength,
            turbulence,
        }
    }
}

#[derive(Clone)]
pub struct WindCell {
    wind: Option<Wind>,
    origin: Vec2,
    rect: Rect,
}

impl WindCell {
    pub fn new_from_origin(origin: Vec2, size: Vec2) -> Self {
        let rect = Rect::from_x_y_w_h(origin.x, origin.y, size.x, size.y);

        Self {
            wind: None,
            origin,
            rect,
        }
    }
}

#[derive(Clone, Default)]
pub struct WindField {
    // Grid of winds in x,y order. (0,0) is top left.
    cells: Vec<Vec<WindCell>>,

    // Origin should align with ParticleSystem origin
    origin: Vec2,
    bounds_size: Vec2,
    grid_cols: usize,
    grid_rows: usize,
    cell_size: Vec2,

    // Debug
    pub debug: bool,
}

impl WindField {
    pub fn new(origin: Vec2, bounds_size: Vec2, grid_cols: usize, grid_rows: usize) -> Self {
        let cell_size = Vec2::new(
            bounds_size.x / grid_cols as f32,
            bounds_size.y / grid_rows as f32,
        );

        let top_left = Vec2::new(
            origin.x - bounds_size.x / 2.0,
            origin.y + bounds_size.y / 2.0, // Start from top (positive Y)
        );

        let mut cells = Vec::new();

        for col in 0..grid_cols {
            let mut row_of_cells = Vec::new();
            for row in 0..grid_rows {
                let cell_origin = top_left
                    + Vec2::new(
                        col as f32 * cell_size.x + cell_size.x / 2.0,
                        -(row as f32 * cell_size.y + cell_size.y / 2.0), // Negative Y to go downward
                    );
                let cell = WindCell::new_from_origin(cell_origin, cell_size);
                row_of_cells.push(cell);
            }
            cells.push(row_of_cells);
        }

        Self {
            cells,
            origin,
            bounds_size,
            grid_cols,
            grid_rows,
            cell_size,
            debug: DEBUG,
        }
    }

    pub fn apply(&self, particle: &mut Particle) {
        let Some(wind) = self.get_wind_at_pos(particle.position) else {
            return;
        };
        wind.apply(particle);
    }

    /******************* WindField Shapes *******************/
    pub fn make_circular_field(
        &mut self,
        center: Vec2, // center of the field, in the ParticleSystem space
        radius: f32,
        width: f32, // width of the wind band (for hollow circles)
        strength: f32,
        turbulence: f32,
    ) {
        // Add a parameter to control the bias toward center
        let center_bias = 0.8; // 0.0 = purely tangential, 1.0 = purely radial inward

        // Iterate through all grid cells
        for col in 0..self.grid_cols {
            for row in 0..self.grid_rows {
                let Some(cell) = self.get_mut_cell(col, row) else {
                    return;
                };

                // Calculate distance from cell center to circle center
                let distance_to_center = (cell.origin - center).length();

                // Check if this cell is within the circular wind field
                let inner_radius = radius - width / 2.0;
                let outer_radius = radius + width / 2.0;

                if distance_to_center >= inner_radius && distance_to_center <= outer_radius {
                    // Calculate radius vector (from center to cell)
                    let radius_vector = cell.origin - center;

                    // Create tangential vector by rotating radius vector 90 degrees
                    // For counter-clockwise rotation: (x, y) -> (-y, x)
                    let tangential_direction = vec2(radius_vector.y, -radius_vector.x).normalize();

                    // Create radial inward direction (toward center)
                    let radial_inward_direction = -radius_vector.normalize();

                    // Blend tangential and radial directions based on center_bias
                    let blended_direction = (tangential_direction * (1.0 - center_bias)
                        + radial_inward_direction * center_bias)
                        .normalize();

                    // Create the wind force for this cell
                    let wind = Wind::new_with(blended_direction, strength, turbulence);
                    cell.wind = Some(wind);
                }
            }
        }
        println!(
            "Wind field created with {} cells affected",
            self.cells
                .iter()
                .flatten()
                .filter(|cell| cell.wind.is_some())
                .count()
        );
    }

    /******************* Grid accessors *******************/

    // Get wind at a position in ParticleSystem coordinates
    pub fn get_wind_at_pos(&self, position: Vec2) -> Option<&Wind> {
        let (x, y) = self.position_to_idx(position)?;
        self.get_wind(x, y)
    }

    pub fn get_cell(&self, x: usize, y: usize) -> Option<&WindCell> {
        let col = self.cells.get(x)?;
        col.get(y)
    }

    pub fn get_mut_cell(&mut self, x: usize, y: usize) -> Option<&mut WindCell> {
        let col = self.cells.get_mut(x)?;
        col.get_mut(y)
    }

    pub fn get_wind(&self, x: usize, y: usize) -> Option<&Wind> {
        let col = self.cells.get(x)?;
        let cell = col.get(y)?;
        cell.wind.as_ref()
    }

    pub fn get_mut_wind(&mut self, x: usize, y: usize) -> Option<&mut Wind> {
        let col = self.cells.get_mut(x)?;
        let cell = col.get_mut(y)?;
        cell.wind.as_mut()
    }

    pub fn clear_cell(&mut self, x: usize, y: usize) {
        let Some(cell) = self.get_mut_cell(x, y) else {
            return;
        };
        cell.wind = None;
    }

    // Take a center-origin position and convert it to a index with 0,0 at top left
    fn position_to_idx(&self, pos: Vec2) -> Option<(usize, usize)> {
        let rect = Rect::from_x_y_w_h(
            self.origin.x,
            self.origin.y,
            self.bounds_size.x,
            self.bounds_size.y,
        );
        let x1 = pos.x + self.bounds_size.x / 2.0;
        let y1 = -pos.y + self.bounds_size.y / 2.0;

        let i = (x1 / self.cell_size.x).floor() as isize;
        let j = (y1 / self.cell_size.y).floor() as isize;

        if i >= 0 && j >= 0 && (i as usize) < self.grid_cols && (j as usize) < self.grid_rows {
            Some((i as usize, j as usize))
        } else {
            None // Out of bounds
        }
    }

    /******************* Draw for Debug *******************/

    pub fn draw(&self, draw: &Draw) {
        self.draw_origin(draw);
        self.draw_grid(draw);

        // Draw wind vectors from each cell's origin
        for col in 0..self.grid_cols {
            for row in 0..self.grid_rows {
                let cell = &self.cells[col][row];
                let Some(wind) = &cell.wind else {
                    continue;
                };

                // Draw wind vector from cell origin
                let vector_scale = 30.0; // Increased scale for better visibility
                let vector_end = cell.origin + wind.direction * wind.strength * vector_scale;

                // Draw the main vector line
                draw.line()
                    .start(cell.origin)
                    .end(vector_end)
                    .color(rgba(0.0, 0.8, 1.0, 0.5)) // Bright yellow for better visibility
                    .stroke_weight(1.0);

                // Draw arrowhead - make it more arrow-like
                let arrow_size = 6.0;
                let arrow_back = vector_end - wind.direction * arrow_size;
                let perpendicular = vec2(-wind.direction.y, wind.direction.x) * arrow_size * 0.5;

                // Draw arrow triangle
                draw.tri()
                    .points(
                        vector_end,
                        arrow_back + perpendicular,
                        arrow_back - perpendicular,
                    )
                    .color(rgba(1.0, 0.0, 0.0, 0.9)); // Red arrowhead
            }
        }
    }

    pub fn draw_grid(&self, draw: &Draw) {
        let rect = Rect::from_x_y_w_h(
            self.origin.x,
            self.origin.y,
            self.bounds_size.x,
            self.bounds_size.y,
        );

        draw.rect()
            .xy(self.origin)
            .w_h(rect.w(), rect.h())
            .stroke_color(rgba(0.0, 0.0, 1.0, 1.0))
            .stroke_weight(1.0)
            .no_fill();

        for col in 0..self.grid_cols {
            for row in 0..self.grid_rows {
                let Some(cell) = self.get_cell(col, row) else {
                    return;
                };
                draw.rect()
                    .xy(cell.origin)
                    .w_h(self.cell_size.x, self.cell_size.y)
                    .stroke_color(rgba(0.0, 0.0, 1.0, 0.3))
                    .stroke_weight(1.0)
                    .no_fill();
                /*
                let coord = format!("{},{}", col, row);
                draw.text(&coord)
                    .x_y(cell.origin.x, cell.origin.y)
                    .font_size(10)
                    .color(rgba(0.0, 0.5, 1.0, 0.8));
                 */
            }
        }
    }

    fn draw_origin(&self, draw: &Draw) {
        draw.ellipse()
            .xy(self.origin)
            .w_h(20.0, 20.0)
            .color(GREEN)
            .stroke(GREEN)
            .stroke_weight(2.0);
    }
}
