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
        let Some(wind) = self.get_wind_at_pos(particle.position()) else {
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
        let transformed = self.world_to_grid_coords(pos);
        let i = transformed.x.floor() as isize;
        let j = transformed.y.floor() as isize;

        if i >= 0 && j >= 0 && (i as usize) < self.grid_cols && (j as usize) < self.grid_rows {
            Some((i as usize, j as usize))
        } else {
            None // Out of bounds
        }
    }

    // Helper method to transform world coordinates to grid coordinates (floating point)
    fn world_to_grid_coords(&self, pos: Vec2) -> Vec2 {
        let x1 = pos.x + self.bounds_size.x / 2.0;
        let y1 = -pos.y + self.bounds_size.y / 2.0;

        vec2(x1 / self.cell_size.x, y1 / self.cell_size.y)
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
                let vector_scale = 3.0; // Increased scale for better visibility
                let vector_end = cell.origin + wind.direction * wind.strength * vector_scale;

                draw.arrow()
                    .start(cell.origin * vec2(scale_x, scale_y))
                    .end(vector_end * vec2(scale_x, scale_y))
                    .color(rgba(0.0, 0.8, 1.0, 0.2))
                    .stroke_weight(0.5);
            }
        }
    }

    pub fn draw_grid(&self, draw: &Draw, scale_x: f32, scale_y: f32) {
        let cols = self.grid_cols;
        let rows = self.grid_rows;
        let cell_w = self.cell_size.x * scale_x;
        let cell_h = self.cell_size.y * scale_y;

        let total_w = cols as f32 * cell_w;
        let total_h = rows as f32 * cell_h;

        let offset_x = -total_w / 2.0;
        let offset_y = -total_h / 2.0;

        // Vertical lines
        for col in 0..=cols {
            let x = offset_x + col as f32 * cell_w;
            draw.line()
                .start(pt2(x, offset_y))
                .end(pt2(x, offset_y + total_h))
                .color(rgba(0.3, 0.3, 0.3, 0.3))
                .weight(1.0);
        }

        // Horizontal lines
        for row in 0..=rows {
            let y = offset_y + row as f32 * cell_h;
            draw.line()
                .start(pt2(offset_x, y))
                .end(pt2(offset_x + total_w, y))
                .color(rgba(0.3, 0.3, 0.3, 0.3))
                .weight(1.0);
        }
    }

    pub fn draw_grid_old(&self, draw: &Draw, scale_x: f32, scale_y: f32) {
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

        // Use the same coordinate transformation as position_to_idx for consistency
        let center_pos_transformed = field.world_to_grid_coords(params.center);

        let radius_in_cells_x = outer_radius / field.cell_size.x;
        let radius_in_cells_y = outer_radius / field.cell_size.y;

        let min_col = (center_pos_transformed.x - radius_in_cells_x)
            .floor()
            .max(0.0) as usize;
        let max_col = (center_pos_transformed.x + radius_in_cells_x)
            .ceil()
            .min(field.grid_cols as f32) as usize;
        let min_row = (center_pos_transformed.y - radius_in_cells_y)
            .floor()
            .max(0.0) as usize;
        let max_row = (center_pos_transformed.y + radius_in_cells_y)
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

    pub fn draw_center(&self, draw: &Draw, scale_x: f32, scale_y: f32) {
        let params = self.params.read().unwrap_or_else(|poisoned| {
            eprintln!("Warning: RwLock was poisoned. Recovering...");
            poisoned.into_inner() // You still get access to the data
        });

        let center = params.center;
        draw.ellipse()
            .xy(center * vec2(scale_x, scale_y))
            .w_h(40.0 * scale_x, 40.0 * scale_y)
            .color(rgba(1.0, 0.2, 0.0, 0.2));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_to_grid_coords() {
        // Set up a WindField with known parameters matching your debug output
        let origin = pt2(0.0, 0.0);
        let bounds_size = vec2(3840.0, 2160.0);
        let grid_cols = 96; // 3840 / 40
        let grid_rows = 54; // 2160 / 40

        let field = WindField::new(origin, bounds_size, grid_cols, grid_rows);

        // Test cases based on your debug output

        // Test case 1: Center of world should map to center of grid
        let world_center = vec2(0.0, 0.0);
        let grid_center = field.world_to_grid_coords(world_center);
        assert_eq!(grid_center, vec2(48.0, 27.0)); // (3840/2)/40 = 48, (2160/2)/40 = 27

        // Test case 2: Your specific debug example
        let pos1 = vec2(-400.0, 0.0);
        let result1 = field.world_to_grid_coords(pos1);
        assert_eq!(result1, vec2(38.0, 27.0)); // Matches your debug: Vec2(38.0, 27.0)

        // Test case 3: Another debug example with fractional result
        let pos2 = vec2(-340.0, 55.615112);
        let result2 = field.world_to_grid_coords(pos2);
        let expected2 = vec2(39.5, 25.609378); // Calculate: (-340 + 1920)/40 = 39.5, (-55.615112 + 1080)/40 = 25.609378
        assert!((result2.x - expected2.x).abs() < 0.001);
        assert!((result2.y - expected2.y).abs() < 0.001);

        // Test case 4: Top-left corner of world bounds
        let top_left = vec2(-1920.0, 1080.0);
        let result_tl = field.world_to_grid_coords(top_left);
        assert_eq!(result_tl, vec2(0.0, 0.0));

        // Test case 5: Bottom-right corner of world bounds
        let bottom_right = vec2(1920.0, -1080.0);
        let result_br = field.world_to_grid_coords(bottom_right);
        assert_eq!(result_br, vec2(96.0, 54.0));

        // Test case 6: Positive X, negative Y
        let pos3 = vec2(400.0, -200.0);
        let result3 = field.world_to_grid_coords(pos3);
        let expected3 = vec2(58.0, 32.0); // (400 + 1920)/40 = 58, (-(-200) + 1080)/40 = 32
        assert_eq!(result3, expected3);

        // Test case 7: Verify the coordinate system transformation logic
        let test_x = 100.0;
        let test_y = -50.0;
        let pos = vec2(test_x, test_y);
        let result = field.world_to_grid_coords(pos);

        // Manual calculation to verify the formula
        let expected_grid_x = (test_x + bounds_size.x / 2.0) / field.cell_size.x;
        let expected_grid_y = (-test_y + bounds_size.y / 2.0) / field.cell_size.y;

        assert_eq!(result.x, expected_grid_x);
        assert_eq!(result.y, expected_grid_y);
    }

    #[test]
    fn test_position_to_idx() {
        let origin = pt2(0.0, 0.0);
        let bounds_size = vec2(3840.0, 2160.0);
        let grid_cols = 96;
        let grid_rows = 54;

        let field = WindField::new(origin, bounds_size, grid_cols, grid_rows);

        // Test valid positions
        let pos1 = vec2(-400.0, 0.0);
        let idx1 = field.position_to_idx(pos1);
        assert_eq!(idx1, Some((38, 27))); // floor(38.0, 27.0)

        // Test fractional grid coordinates
        let pos2 = vec2(-340.0, 55.615112);
        let idx2 = field.position_to_idx(pos2);
        assert_eq!(idx2, Some((39, 25))); // floor(39.5, 25.609...)

        // Test boundary cases - just inside bounds
        let pos3 = vec2(-1919.0, 1079.0);
        let idx3 = field.position_to_idx(pos3);
        assert_eq!(idx3, Some((0, 0)));

        // Test out of bounds - should return None
        let pos_oob = vec2(-2000.0, 0.0);
        let idx_oob = field.position_to_idx(pos_oob);
        assert_eq!(idx_oob, None);

        let pos_oob2 = vec2(0.0, 1200.0);
        let idx_oob2 = field.position_to_idx(pos_oob2);
        assert_eq!(idx_oob2, None);
    }

    #[test]
    fn test_coordinate_system_properties() {
        let origin = pt2(0.0, 0.0);
        let bounds_size = vec2(800.0, 600.0);
        let grid_cols = 20; // 40px cells
        let grid_rows = 15; // 40px cells

        let field = WindField::new(origin, bounds_size, grid_cols, grid_rows);

        // Property 1: Moving right in world space increases grid X
        let left_pos = vec2(-100.0, 0.0);
        let right_pos = vec2(100.0, 0.0);
        let left_grid = field.world_to_grid_coords(left_pos);
        let right_grid = field.world_to_grid_coords(right_pos);
        assert!(right_grid.x > left_grid.x);

        // Property 2: Moving up in world space decreases grid Y (Y is flipped)
        let bottom_pos = vec2(0.0, -100.0);
        let top_pos = vec2(0.0, 100.0);
        let bottom_grid = field.world_to_grid_coords(bottom_pos);
        let top_grid = field.world_to_grid_coords(top_pos);
        assert!(top_grid.y < bottom_grid.y);

        // Property 3: Grid center should correspond to world center
        let world_center = vec2(0.0, 0.0);
        let grid_center = field.world_to_grid_coords(world_center);
        let expected_center = vec2(10.0, 7.5); // 800/2/40 = 10, 600/2/40 = 7.5
        assert_eq!(grid_center, expected_center);
    }

    #[test]
    fn test_get_wind_at_pos_returns_correct_cell_wind() {
        // Create a 4x3 grid for easier testing
        let origin = pt2(0.0, 0.0);
        let bounds_size = vec2(400.0, 300.0); // 4x3 grid with 100x100 cells
        let grid_cols = 4;
        let grid_rows = 3;

        let mut field = WindField::new(origin, bounds_size, grid_cols, grid_rows);

        // Add distinct winds to specific cells for identification
        // Cell (0,0) - top-left
        if let Some(cell) = field.get_mut_cell(0, 0) {
            cell.add_wind(1, Wind::new_with(vec2(1.0, 0.0), 10.0)); // Right wind, strength 10
        }

        // Cell (1,1) - center-left
        if let Some(cell) = field.get_mut_cell(1, 1) {
            cell.add_wind(2, Wind::new_with(vec2(0.0, 1.0), 20.0)); // Up wind, strength 20
        }

        // Cell (3,2) - bottom-right
        if let Some(cell) = field.get_mut_cell(3, 2) {
            cell.add_wind(3, Wind::new_with(vec2(-1.0, 0.0), 30.0)); // Left wind, strength 30
        }

        // Cell (2,0) - top-right area
        if let Some(cell) = field.get_mut_cell(2, 0) {
            cell.add_wind(4, Wind::new_with(vec2(0.0, -1.0), 40.0)); // Down wind, strength 40
        }

        // Test 1: Position that maps to cell (0,0)
        // World coordinate (-150, 100) should map to grid (1.0, 0.0) -> cell (1,0)
        // Actually let me recalculate: (-150 + 200)/100 = 0.5, (-100 + 150)/100 = 0.5 -> cell (0,0)
        let pos1 = vec2(-150.0, 100.0); // Should map to cell (0,0)
        let wind1 = field.get_wind_at_pos(pos1);
        assert!(wind1.is_some());
        let wind1 = wind1.unwrap();
        assert_eq!(wind1.direction, vec2(1.0, 0.0));
        assert_eq!(wind1.strength, 10.0);

        // Test 2: Position that maps to cell (1,1)
        let pos2 = vec2(-50.0, 0.0); // Should map to cell (1,1)
        let wind2 = field.get_wind_at_pos(pos2);
        assert!(wind2.is_some());
        let wind2 = wind2.unwrap();
        assert_eq!(wind2.direction, vec2(0.0, 1.0));
        assert_eq!(wind2.strength, 20.0);

        // Test 3: Position that maps to cell (3,2)
        let pos3 = vec2(150.0, -100.0); // Should map to cell (3,2)
        let wind3 = field.get_wind_at_pos(pos3);
        assert!(wind3.is_some());
        let wind3 = wind3.unwrap();
        assert_eq!(wind3.direction, vec2(-1.0, 0.0));
        assert_eq!(wind3.strength, 30.0);

        // Test 4: Position that maps to cell (2,0)
        let pos4 = vec2(50.0, 100.0); // Should map to cell (2,0)
        let wind4 = field.get_wind_at_pos(pos4);
        assert!(wind4.is_some());
        let wind4 = wind4.unwrap();
        assert_eq!(wind4.direction, vec2(0.0, -1.0));
        assert_eq!(wind4.strength, 40.0);

        // Test 5: Position in empty cell should return None
        let pos5 = vec2(50.0, 0.0); // Should map to cell (2,1) which has no wind
        let wind5 = field.get_wind_at_pos(pos5);
        assert!(wind5.is_none());

        // Test 6: Out of bounds position should return None
        let pos6 = vec2(300.0, 0.0); // Out of bounds
        let wind6 = field.get_wind_at_pos(pos6);
        assert!(wind6.is_none());
    }

    #[test]
    fn test_get_wind_at_pos_with_multiple_wind_sources() {
        let origin = pt2(0.0, 0.0);
        let bounds_size = vec2(200.0, 200.0); // 2x2 grid with 100x100 cells
        let grid_cols = 2;
        let grid_rows = 2;

        let mut field = WindField::new(origin, bounds_size, grid_cols, grid_rows);

        // Add multiple wind sources to the same cell to test wind combination
        if let Some(cell) = field.get_mut_cell(0, 0) {
            cell.add_wind(1, Wind::new_with(vec2(1.0, 0.0), 10.0)); // Right, strength 10
            cell.add_wind(2, Wind::new_with(vec2(0.0, 1.0), 10.0)); // Up, strength 10
        }

        // Position that maps to cell (0,0)
        let pos = vec2(-50.0, 50.0);
        let wind = field.get_wind_at_pos(pos);
        assert!(wind.is_some());
        let wind = wind.unwrap();

        // Combined wind should be the vector sum: (10,0) + (0,10) = (10,10)
        // Strength should be sqrt(10^2 + 10^2) = sqrt(200) ≈ 14.14
        // Direction should be (10,10).normalize() = (0.707, 0.707)
        let expected_strength = (10.0_f32.powi(2) + 10.0_f32.powi(2)).sqrt();
        let expected_direction = vec2(10.0, 10.0).normalize();

        assert!((wind.strength - expected_strength).abs() < 0.001);
        assert!((wind.direction.x - expected_direction.x).abs() < 0.001);
        assert!((wind.direction.y - expected_direction.y).abs() < 0.001);
    }

    #[test]
    fn test_get_wind_at_pos_cell_boundary_precision() {
        // Test positions very close to cell boundaries to ensure correct cell selection
        let origin = pt2(0.0, 0.0);
        let bounds_size = vec2(400.0, 400.0); // 4x4 grid with 100x100 cells
        let grid_cols = 4;
        let grid_rows = 4;

        let mut field = WindField::new(origin, bounds_size, grid_cols, grid_rows);

        // Add wind to adjacent cells across the X=0 boundary
        if let Some(cell) = field.get_mut_cell(1, 1) {
            cell.add_wind(1, Wind::new_with(vec2(1.0, 0.0), 100.0));
        }
        if let Some(cell) = field.get_mut_cell(2, 1) {
            cell.add_wind(2, Wind::new_with(vec2(-1.0, 0.0), 200.0));
        }

        // Debug: Let's trace what these positions should map to
        let pos1 = vec2(-0.1, 0.1); // Just left of X=0, should be cell (1,1)
        let grid1 = field.world_to_grid_coords(pos1);
        let idx1 = field.position_to_idx(pos1);
        println!("pos1 {:?} -> grid {:?} -> idx {:?}", pos1, grid1, idx1);

        let pos2 = vec2(0.1, 0.1); // Just right of X=0, should be cell (2,1)
        let grid2 = field.world_to_grid_coords(pos2);
        let idx2 = field.position_to_idx(pos2);
        println!("pos2 {:?} -> grid {:?} -> idx {:?}", pos2, grid2, idx2);

        // Test position just left of X=0 boundary - should be cell (1,1)
        let wind1 = field.get_wind_at_pos(pos1);
        assert!(wind1.is_some());
        assert_eq!(wind1.unwrap().strength, 100.0);

        // Test position just right of X=0 boundary - should be cell (2,1)
        let wind2 = field.get_wind_at_pos(pos2);
        assert!(wind2.is_some());
        assert_eq!(wind2.unwrap().strength, 200.0);

        // Additional test: Position exactly on boundary (edge case)
        let pos3 = vec2(0.0, 0.1); // Exactly on X=0, should map to cell (2,1) due to floor()
        let wind3 = field.get_wind_at_pos(pos3);
        assert!(wind3.is_some());
        assert_eq!(wind3.unwrap().strength, 200.0);
    }

    #[test]
    fn test_boundary_analysis_debug() {
        // Debug test to understand exact boundary behavior
        let origin = pt2(0.0, 0.0);
        let bounds_size = vec2(400.0, 400.0);
        let grid_cols = 4;
        let grid_rows = 4;

        let field = WindField::new(origin, bounds_size, grid_cols, grid_rows);

        // Test a series of positions around the X=0 boundary
        let test_positions = [
            vec2(-1.0, 0.0),
            vec2(-0.5, 0.0),
            vec2(-0.1, 0.0),
            vec2(0.0, 0.0),
            vec2(0.1, 0.0),
            vec2(0.5, 0.0),
            vec2(1.0, 0.0),
        ];

        for pos in test_positions {
            let grid_coords = field.world_to_grid_coords(pos);
            let cell_idx = field.position_to_idx(pos);
            println!(
                "Position {:?} -> Grid {:?} -> Cell {:?}",
                pos, grid_coords, cell_idx
            );
        }

        // Expected cell boundaries for a 4x4 grid:
        // Cell columns: 0=[-200,-100), 1=[-100,0), 2=[0,100), 3=[100,200)
        // Cell rows: 0=[100,200), 1=[0,100), 2=[-100,0), 3=[-200,-100)

        // Test specific boundary positions
        assert_eq!(field.position_to_idx(vec2(-100.1, 0.1)), Some((0, 1))); // Just left of cell 1
        assert_eq!(field.position_to_idx(vec2(-99.9, 0.1)), Some((1, 1))); // Just right into cell 1
        assert_eq!(field.position_to_idx(vec2(-0.1, 0.1)), Some((1, 1))); // Just left of X=0
        assert_eq!(field.position_to_idx(vec2(0.0, 0.1)), Some((2, 1))); // Exactly on X=0
        assert_eq!(field.position_to_idx(vec2(0.1, 0.1)), Some((2, 1))); // Just right of X=0
    }
}
