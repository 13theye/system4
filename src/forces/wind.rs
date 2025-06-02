// src/forces/wind.rs
//
// Grid-based wind force for particle system

use crate::{forces::CellIdx, particle::Particle};
use nannou::prelude::*;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

#[derive(Clone, Copy, Debug, Default)]
pub struct Wind {
    direction: Vec2,
    strength: f32,
}

// A wind is a simple vector force that is applied to a particle.
// It has a direction and a strength.
// The direction is a unit vector that points in the direction of the wind.
// The strength is a scalar that multiplies the direction to get the actual force.
// The force is calculated as the difference between the wind's target velocity
// and the particle's current velocity. Particle intertia is also considered.
// The force is then added to the particle's acceleration.

impl Wind {
    pub fn new() -> Self {
        Self {
            direction: vec2(0.0, 0.0),
            strength: 0.0,
        }
    }

    pub fn new_with(direction: Vec2, strength: f32) -> Self {
        Self {
            direction,
            strength,
        }
    }

    pub fn apply(&self, particle: &mut Particle) {
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

pub struct WindCell {
    winds: HashMap<usize, Wind>,
    combined_wind: Option<Wind>,
    origin: Vec2,
    rect: Rect,
    needs_update: bool,
}

// A WindCell is a basic unit of a WindField.
// It contains a list of winds that are acting on it,
// and a combined wind cache that is the sum of all the winds.
// The combined wind is recalculated when the cell or wind is updated.

impl WindCell {
    pub fn new_from_origin(origin: Vec2, size: Vec2) -> Self {
        let rect = Rect::from_x_y_w_h(origin.x, origin.y, size.x, size.y);

        Self {
            winds: HashMap::new(),
            combined_wind: None,
            origin,
            rect,
            needs_update: false,
        }
    }

    pub fn add_wind(&mut self, source_id: usize, wind: Wind) {
        self.winds.insert(source_id, wind);
        self.needs_update = true;
    }

    pub fn remove_wind(&mut self, id: usize) {
        self.winds.remove(&id);
        self.needs_update = true;
    }

    pub fn get_combined_wind(&mut self) -> Option<Wind> {
        if self.needs_update {
            self.calculate_combined_wind();
        }
        self.combined_wind
    }

    fn calculate_combined_wind(&mut self) {
        if self.winds.is_empty() {
            self.combined_wind = None;
            return;
        }

        let mut total_force = vec2(0.0, 0.0);
        for wind in self.winds.values() {
            total_force += wind.direction * wind.strength;
        }

        let combined_strength = total_force.length();
        let combined_direction = if combined_strength > 0.0 {
            total_force.normalize()
        } else {
            vec2(0.0, 0.0) // if no force, direction doesn't matter.
        };

        self.combined_wind = Some(Wind::new_with(combined_direction, combined_strength));
        self.needs_update = false;
    }
}

pub struct WindField {
    // Grid of winds in x,y order. (0,0) is top left.
    cells: Vec<Vec<WindCell>>,

    // Origin should align with ParticleSystem origin
    origin: Vec2,
    bounds_size: Vec2,
    grid_cols: usize,
    grid_rows: usize,
    cell_size: Vec2,
}

// The WindField is a grid of WindCells.
// It is used to apply wind forces to particles.
// The grid is used to quickly find the wind force at a given position.

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
        }
    }

    pub fn apply(&mut self, particle: &mut Particle) {
        let Some(wind) = self.get_wind_at_pos(particle.position) else {
            return;
        };
        wind.apply(particle);
    }

    pub fn force_update_all(&mut self) {
        for col in 0..self.grid_cols {
            for row in 0..self.grid_rows {
                if let Some(cell) = self.get_mut_cell(col, row) {
                    let _ = cell.get_combined_wind();
                }
            }
        }
    }

    /******************* Grid accessors *******************/

    // Get combined wind at a position in ParticleSystem coordinates
    pub fn get_wind_at_pos(&mut self, position: Vec2) -> Option<Wind> {
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

    pub fn get_wind(&mut self, x: usize, y: usize) -> Option<Wind> {
        let col = self.cells.get_mut(x)?;
        let cell = col.get_mut(y)?;
        cell.get_combined_wind()
    }

    pub fn clear_cell(&mut self, x: usize, y: usize) {
        let Some(cell) = self.get_mut_cell(x, y) else {
            return;
        };
        cell.combined_wind = None;
    }

    // Take a center-origin position and convert it to a index with 0,0 at top left
    fn position_to_idx(&self, pos: Vec2) -> Option<(usize, usize)> {
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

    pub fn draw(&self, draw: &Draw, scale_x: f32, scale_y: f32) {
        self.draw_origin(draw, scale_x, scale_y);
        self.draw_grid(draw, scale_x, scale_y);
        self.draw_vectors(draw, scale_x, scale_y);
    }

    fn draw_vectors(&self, draw: &Draw, scale_x: f32, scale_y: f32) {
        // Draw wind vectors from each cell's origin
        for col in 0..self.grid_cols {
            for row in 0..self.grid_rows {
                let cell = &self.cells[col][row];
                let Some(wind) = &cell.combined_wind else {
                    continue;
                };

                // Draw wind vector from cell origin
                let vector_scale = 5.0; // Increased scale for better visibility
                let vector_end = cell.origin + wind.direction * wind.strength * vector_scale;

                // Draw the main vector line
                draw.line()
                    .start(cell.origin * vec2(scale_x, scale_y))
                    .end(vector_end * vec2(scale_x, scale_y))
                    .color(rgba(0.0, 0.8, 1.0, 0.2)) // Bright yellow for better visibility
                    .stroke_weight(1.0);

                // Draw arrowhead
                let arrow_size = 6.0;
                let arrow_back = vector_end - wind.direction * arrow_size;
                let perpendicular = vec2(-wind.direction.y, wind.direction.x) * arrow_size * 0.5;

                // Draw arrow triangle
                draw.tri()
                    .points(
                        vector_end * vec2(scale_x, scale_y),
                        (arrow_back + perpendicular) * vec2(scale_x, scale_y),
                        (arrow_back - perpendicular) * vec2(scale_x, scale_y),
                    )
                    .color(rgba(1.0, 0.0, 0.0, 0.3)); // Red arrowhead
            }
        }
    }

    pub fn draw_grid(&self, draw: &Draw, scale_x: f32, scale_y: f32) {
        let rect = Rect::from_x_y_w_h(
            self.origin.x * scale_x,
            self.origin.y * scale_y,
            self.bounds_size.x * scale_x,
            self.bounds_size.y * scale_y,
        );

        for col in 0..self.grid_cols {
            for row in 0..self.grid_rows {
                let Some(cell) = self.get_cell(col, row) else {
                    return;
                };
                draw.rect()
                    .xy(cell.origin * vec2(scale_x, scale_y))
                    .w_h(self.cell_size.x * scale_x, self.cell_size.y * scale_y)
                    .stroke_color(rgba(0.3, 0.3, 0.3, 0.3))
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

    fn draw_origin(&self, draw: &Draw, scale_x: f32, scale_y: f32) {
        draw.ellipse()
            .xy(self.origin * vec2(scale_x, scale_y))
            .w_h(20.0 * scale_x, 20.0 * scale_y)
            .color(rgba(0.0, 1.0, 0.0, 0.2));
    }
}

/******************* WindCircle ******************************* */

#[derive(Clone)]
pub struct WindCircle {
    pub id: usize,
    cell_idxs: Vec<CellIdx>, // Indices of cells that are affected by the circle
    params: Arc<RwLock<WindCircleParams>>, // Params of the circle
}

// A WindCircle is a circular area of wind that is applied to the WindField.
// It is used to create a circular wind effect.
// The circle is defined by a center, radius, width, strength, and center bias.
// The center bias is a value between 0.0 and 1.0 that controls the balance between tangential and radial inward wind.
// The wind is applied to the WindField by calculating the bounding box of the circle and applying the wind to the cells within the box.
// The wind is then removed from the WindField by removing the wind from the cells that are within the circle.

impl WindCircle {
    pub fn new(
        id: usize,
        center: Vec2,
        radius: f32,
        width: f32,
        strength: f32,
        center_bias: f32,
    ) -> Self {
        let config = WindCircleParams {
            center,
            radius,
            width,
            strength,
            center_bias,
            needs_recalculation: true,
        };
        Self {
            id,
            cell_idxs: Vec::new(),
            params: Arc::new(RwLock::new(config)),
        }
    }

    pub fn update(&mut self, field: &mut WindField, show_forces: bool) {
        if self.has_changes() {
            self.remove_from_field(field, show_forces);
            self.cell_idxs = self.apply_to_field(field, show_forces);
            self.clear_changes();
        }
    }

    // Remove the circle's wind from the field
    pub fn remove_from_field(&mut self, field: &mut WindField, show_forces: bool) {
        for cell_idx in self.cell_idxs.iter() {
            let Some(cell) = field.get_mut_cell(cell_idx.x, cell_idx.y) else {
                continue;
            };
            cell.remove_wind(self.id);

            if show_forces {
                let _ = cell.get_combined_wind();
            }
        }
        self.cell_idxs.clear();
    }

    // Apply the circle's wind to the field, return the cells that were affected
    pub fn apply_to_field(&self, field: &mut WindField, show_forces: bool) -> Vec<CellIdx> {
        // Get a copy of the config
        let params = self.params.read().unwrap_or_else(|poisoned| {
            eprintln!("Warning: RwLock was poisoned. Recovering...");
            poisoned.into_inner() // You still get access to the data
        });

        // Calculate bounding box
        let (min_col, max_col, min_row, max_row) = self.calculate_bounding_box(field, &params);

        let mut affected_cells = Vec::new();

        for col in min_col..max_col {
            for row in min_row..max_row {
                let Some(cell) = field.get_mut_cell(col, row) else {
                    continue;
                };
                let Some(wind) = self.calculate_wind_for_cell(cell, &params) else {
                    continue;
                };
                cell.add_wind(self.id, wind);

                // In debug mode, pre-calculate combined wind so we can draw the field
                if show_forces {
                    let _ = cell.get_combined_wind();
                }

                affected_cells.push(CellIdx { x: col, y: row });
            }
        }

        affected_cells
    }

    // Get the bounding box containing all cells that are affected by the circle
    fn calculate_bounding_box(
        &self,
        field: &WindField,
        params: &WindCircleParams,
    ) -> (usize, usize, usize, usize) {
        // Calculate the outer radius for bounding box
        let outer_radius = params.radius + params.width / 2.0;

        // Calculate bounding box in grid coordinates
        let center_x_in_grid = (params.center.x + field.bounds_size.x / 2.0) / field.cell_size.x;
        let center_y_in_grid = (-params.center.y + field.bounds_size.y / 2.0) / field.cell_size.y;

        let radius_in_cells_x = outer_radius / field.cell_size.x;
        let radius_in_cells_y = outer_radius / field.cell_size.y;

        let min_col = (center_x_in_grid - radius_in_cells_x).floor().max(0.0) as usize;
        let max_col = (center_x_in_grid + radius_in_cells_x)
            .ceil()
            .min(field.grid_cols as f32) as usize;
        let min_row = (center_y_in_grid - radius_in_cells_y).floor().max(0.0) as usize;
        let max_row = (center_y_in_grid + radius_in_cells_y)
            .ceil()
            .min(field.grid_rows as f32) as usize;

        (min_col, max_col, min_row, max_row)
    }

    fn calculate_wind_for_cell(&self, cell: &WindCell, params: &WindCircleParams) -> Option<Wind> {
        let distance_to_center = (cell.origin - params.center).length();
        let inner_radius = params.radius - params.width / 2.0;
        let outer_radius = params.radius + params.width / 2.0;

        if distance_to_center >= inner_radius && distance_to_center <= outer_radius {
            // Wind generation logic specific to circular fields
            let radius_vector = cell.origin - params.center;
            let tangential_direction = vec2(radius_vector.y, -radius_vector.x).normalize();
            let radial_inward_direction = -radius_vector.normalize();

            let blended_direction = (tangential_direction * (1.0 - params.center_bias)
                + radial_inward_direction * params.center_bias)
                .normalize();

            Some(Wind::new_with(blended_direction, params.strength))
        } else {
            None
        }
    }

    /******************* Methods to change circle properties *******************/

    pub fn has_changes(&self) -> bool {
        self.with_params_read(|params| params.needs_recalculation)
    }

    pub fn clear_changes(&mut self) {
        self.with_params_write(|params| params.needs_recalculation = false);
    }

    pub fn params_arc(&self) -> Arc<RwLock<WindCircleParams>> {
        self.params.clone()
    }

    pub fn with_params_read<R>(&self, f: impl FnOnce(&WindCircleParams) -> R) -> R {
        let params = self.params.read().unwrap_or_else(|poisoned| {
            eprintln!("Warning: RwLock was poisoned. Recovering...");
            poisoned.into_inner() // You still get access to the data
        });
        f(&params)
    }

    pub fn with_params_write<R>(&self, f: impl FnOnce(&mut WindCircleParams) -> R) -> R {
        let mut params = self.params.write().unwrap_or_else(|poisoned| {
            eprintln!("Warning: RwLock was poisoned. Recovering...");
            poisoned.into_inner() // You still get access to the data
        });
        f(&mut params)
    }
}

#[derive(Clone)]
pub struct WindCircleParams {
    pub center: Vec2, // center of the circle in the ParticleSystem space
    pub radius: f32,
    pub width: f32,                // width of the wind band (for hollow circles)
    pub strength: f32,             // strength of the wind
    pub center_bias: f32,          // 0.0 = purely tangential, 1.0 = purely radial inward
    pub needs_recalculation: bool, // if settings changed, we need to recalculate the cells
}

impl WindCircleParams {
    pub fn center(&mut self, center: Vec2) {
        if self.center != center {
            self.center = center;
            self.needs_recalculation = true;
        }
    }

    pub fn radius(&mut self, radius: f32) {
        if self.radius != radius {
            self.radius = radius;
            self.needs_recalculation = true;
        }
    }

    pub fn width(&mut self, width: f32) {
        if self.width != width {
            self.width = width;
            self.needs_recalculation = true;
        }
    }

    pub fn strength(&mut self, strength: f32) {
        if self.strength != strength {
            self.strength = strength;
            self.needs_recalculation = true;
        }
    }

    pub fn center_bias(&mut self, center_bias: f32) {
        if self.center_bias != center_bias {
            self.center_bias = center_bias;
            self.needs_recalculation = true;
        }
    }
}
