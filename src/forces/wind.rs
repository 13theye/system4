// src/forces/wind.rs
//
// Grid-based wind force for particle system

use crate::{forces::CellIdx, particle::Particles, view::Voice};
use nannou::prelude::*;
use rayon::prelude::*;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

/// A wind is a simple vector force that is applied to a particle.
/// It has a direction and a strength.
/// The direction is a unit vector that points in the direction of the wind.
/// The strength is a scalar that multiplies the direction to get the actual force.
/// The force is calculated as the difference between the wind's target velocity
/// and the particle's current velocity. Particle intertia is also considered.
/// The force is then added to the particle's acceleration.
#[derive(Clone, Copy, Debug, Default)]
pub struct Wind {
    direction: Vec2,
    strength: f32,
}
impl Wind {
    pub fn new() -> Self {
        Self {
            direction: vec2(0.0, 0.0),
            strength: 0.0,
        }
    }

    /// Create a new Wind with a direction and strength
    pub fn new_with(direction: Vec2, strength: f32) -> Self {
        Self {
            direction,
            strength,
        }
    }

    /// Apply the Wind to a Particle
    pub fn apply(&self, acc_x: &mut f32, acc_y: &mut f32, vel_x: &f32, vel_y: &f32, mass: &f32) {
        // Calculate the x and y components of wind's target velocity
        let wind_vx = self.direction.x * self.strength;
        let wind_vy = self.direction.y * self.strength;

        // Calculate the difference in each component
        let diff_x = wind_vx - vel_x;
        let diff_y = wind_vy - vel_y;

        // Calculate inertial resistance based on current momentum
        let current_speed = vel_x.hypot(*vel_y);
        let momentum_magnitude = mass * current_speed;

        // Inertial resistance: particles with higher momentum resist changes more
        let inertia_coefficient = 0.1; // Adjust this to control resistance strength
        let inertia_factor = 1.0 / (1.0 + momentum_magnitude * inertia_coefficient);

        // Apply the force with inertial resistance
        let force_x = diff_x * inertia_factor;
        let force_y = diff_y * inertia_factor;
        *acc_x += force_x / mass;
        *acc_y += force_y / mass;
    }
}

/// A WindCell is a basic unit of a WindField.
/// It contains a list of winds that are acting on it,
/// and a combined wind cache that is the sum of all the winds.
/// The combined wind is recalculated when the cell or wind is updated.
pub struct WindCell {
    winds: HashMap<usize, Wind>,
    combined_wind: Option<Wind>,
    origin: Vec2,
    rect: Rect,
    needs_update: bool,
}

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

    /// Add a Wind to this WindCell.
    pub fn add_wind(&mut self, source_id: usize, wind: Wind) {
        self.winds.insert(source_id, wind);
        self.needs_update = true;
    }

    /// Remove a Wind from this WindCell.
    pub fn remove_wind(&mut self, id: usize) {
        self.winds.remove(&id);
        self.needs_update = true;
    }

    /// Get the sum of all Winds in this WindCell, recalculating if necessary.
    pub fn check_and_get_combined_wind(&mut self) -> Option<Wind> {
        if self.needs_update {
            self.calculate_combined_wind();
        }
        self.combined_wind
    }

    /// Return the combined wind without checking if an update is needed
    pub fn get_combined_wind(&self) -> Option<Wind> {
        self.combined_wind
    }

    /// Calculate the combined effects of all Winds in this WindCell.
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

/// The WindField is a grid of WindCells.
/// It is used to apply wind forces to particles.
/// The grid is used to quickly find the wind force at a given position.
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

impl WindField {
    /// Create a new WindField with a center origin, x&y size, number of columns and number of rows.
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

    /// Given a particle, determines the WindCell that contains the particle, and then applies the WindCell's Wind on that particle.
    pub fn apply_to_particles(&mut self, particles: &mut Particles) {
        // 1. Update cells that need updating
        self.cells
            .iter_mut()
            .flat_map(|row| row.iter_mut())
            .filter(|cell| cell.needs_update)
            .for_each(|cell| cell.calculate_combined_wind());

        particles
            .acc_x
            .par_iter_mut()
            .zip(particles.acc_y.par_iter_mut())
            .zip(particles.vel_x.par_iter())
            .zip(particles.vel_y.par_iter())
            .zip(particles.pos_x.par_iter())
            .zip(particles.pos_y.par_iter())
            .zip(particles.mass.par_iter())
            .for_each(
                |((((((acc_x, acc_y), vel_x), vel_y), &pos_x), &pos_y), mass)| {
                    if let Some(wind) = self.get_wind_at_pos(pos_x, pos_y) {
                        wind.apply(acc_x, acc_y, vel_x, vel_y, mass);
                    }
                },
            );
    }

    /// Force a recalculation of all cells in the WindField.
    pub fn recalculate_all_cells(&mut self) {
        for col in 0..self.grid_cols {
            for row in 0..self.grid_rows {
                if let Some(cell) = self.get_mut_cell(col, row) {
                    let _ = cell.check_and_get_combined_wind();
                }
            }
        }
    }

    /******************* Grid accessors *******************/

    /// Get combined wind at a position in ParticleSystem coordinates
    pub fn get_wind_at_pos(&self, pos_x: f32, pos_y: f32) -> Option<Wind> {
        let (x, y) = self.position_to_idx(pos_x, pos_y)?;

        self.get_wind(x, y)
    }

    /// Get the cell at a grid position (0,0 is top left)
    pub fn get_cell(&self, x: usize, y: usize) -> Option<&WindCell> {
        let col = self.cells.get(x)?;
        col.get(y)
    }

    /// Get a mutable reference to the cell at a grid position (0,0 is top left)
    pub fn get_mut_cell(&mut self, x: usize, y: usize) -> Option<&mut WindCell> {
        let col = self.cells.get_mut(x)?;
        col.get_mut(y)
    }

    /// Get the combined wind at a grid position (0,0 is top left)
    pub fn get_wind(&self, x: usize, y: usize) -> Option<Wind> {
        let col = self.cells.get(x)?;
        let cell = col.get(y)?;
        cell.get_combined_wind()
    }

    /// Clear the Wind from a cell at a grid position (0,0 is top left)
    pub fn clear_cell(&mut self, x: usize, y: usize) {
        let Some(cell) = self.get_mut_cell(x, y) else {
            return;
        };
        cell.combined_wind = None;
    }

    /// Take a center-origin position and convert it to a grid position index (0,0 is top left)
    fn position_to_idx(&self, pos_x: f32, pos_y: f32) -> Option<(usize, usize)> {
        let transformed = self.world_to_grid_coords(pos_x, pos_y);
        let i = transformed.x.floor() as isize;
        let j = transformed.y.floor() as isize;

        if i >= 0 && j >= 0 && (i as usize) < self.grid_cols && (j as usize) < self.grid_rows {
            Some((i as usize, j as usize))
        } else {
            None // Out of bounds
        }
    }

    /// Helper method to transform world coordinates to grid coordinates (floating point)
    fn world_to_grid_coords(&self, pos_x: f32, pos_y: f32) -> Vec2 {
        let x1 = pos_x + self.bounds_size.x / 2.0;
        let y1 = -pos_y + self.bounds_size.y / 2.0;

        vec2(x1 / self.cell_size.x, y1 / self.cell_size.y)
    }

    /******************* Draw for Performer *******************/

    /// Draw the WindField
    pub fn draw(&self, draw: &Draw, scale_x: f32, scale_y: f32) {
        self.draw_origin(draw, scale_x, scale_y);
        self.draw_grid(draw, scale_x, scale_y);
        self.draw_vectors(draw, scale_x, scale_y);
    }

    /// Draw all the Wind vectors
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

    /// Draw a grid of lines that represent the cells -- fast but less accurate
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

    /// Draw a grid of rectangles that represent the cells -- slow but more accurate
    pub fn draw_grid_rect(&self, draw: &Draw, scale_x: f32, scale_y: f32) {
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

    /// Draw the origin of this WindField
    fn draw_origin(&self, draw: &Draw, scale_x: f32, scale_y: f32) {
        draw.ellipse()
            .xy(self.origin * vec2(scale_x, scale_y))
            .w_h(20.0 * scale_x, 20.0 * scale_y)
            .color(rgba(0.0, 1.0, 0.0, 0.2));
    }
}

/******************* WindCircle ******************************* */

/// A WindCircle defines a circular area. Under this area, a Wind is applied
/// to the WindCell that the circle covers.
/// - The circle is defined by a center, radius, width, strength, and center bias.
/// - The center bias is a value between 0.0 and 1.0 that controls the balance between tangential and radial inward wind.
/// - A center bias between 1.0 and 2.0 moves back toward a tangential wind, in the opposite direction.
/// - The wind is applied to the WindField by calculating the bounding box of the circle and applying the wind to the cells within the box.
/// - The wind is then removed from the WindField by removing the wind from the cells that are within the circle.
#[derive(Clone)]
pub struct WindCircle {
    pub id: usize,
    pub parent_voice: Voice,
    cell_idxs: Vec<CellIdx>, // Indices of cells that are affected by the circle
    params: Arc<RwLock<WindCircleParams>>, // Params of the circle
}

impl WindCircle {
    pub fn new(
        id: usize,
        parent_voice: Voice,
        center: Vec2,
        radius: f32,
        width: f32,
        strength: f32,
        center_bias: f32,
    ) -> Self {
        let config = WindCircleParams {
            center,
            outer_radius: radius,
            inner_radius: width,
            strength,
            center_bias,
            needs_recalculation: true,
        };
        Self {
            id,
            parent_voice,
            cell_idxs: Vec::new(),
            params: Arc::new(RwLock::new(config)),
        }
    }

    /// Recalculate wind within each grid inside this circle, if the parameters have changed
    pub fn update(&mut self, field: &mut WindField, show_forces: bool) {
        if self.has_changes() {
            self.remove_from_field(field, show_forces);
            self.cell_idxs = self.apply_to_field(field, show_forces);
            self.clear_changes();
        }
    }

    /// Remove the circle's wind from the field
    pub fn remove_from_field(&mut self, field: &mut WindField, show_forces: bool) {
        for cell_idx in self.cell_idxs.iter() {
            let Some(cell) = field.get_mut_cell(cell_idx.x, cell_idx.y) else {
                continue;
            };
            cell.remove_wind(self.id);

            if show_forces {
                let _ = cell.check_and_get_combined_wind();
            }
        }
        self.cell_idxs.clear();
    }

    /// Apply the circle's wind to the field, return the cells that were affected
    pub fn apply_to_field(&self, field: &mut WindField, show_forces: bool) -> Vec<CellIdx> {
        // Get a copy of the config
        let params = self.params.read().unwrap_or_else(|poisoned| {
            eprintln!("Warning: RwLock was poisoned. Recovering...");
            poisoned.into_inner() // You still get access to the data
        });

        // Calculate bounding box
        let (min_col, max_col, min_row, max_row) = self.calculate_bounding_box(field, &params);

        let mut affected_cells = Vec::new();

        // Update cells serially
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
                    let _ = cell.check_and_get_combined_wind();
                }

                affected_cells.push(CellIdx { x: col, y: row });
            }
        }

        affected_cells
    }

    /// Get the bounding box containing all cells that are affected by the circle
    fn calculate_bounding_box(
        &self,
        field: &WindField,
        params: &WindCircleParams,
    ) -> (usize, usize, usize, usize) {
        // Calculate the outer radius for bounding box
        let outer_radius = params.outer_radius + params.inner_radius / 2.0;

        // Use the same coordinate transformation as position_to_idx for consistency
        let center_pos_transformed = field.world_to_grid_coords(params.center.x, params.center.y);

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

    /// Calculate the Wind force within a cell. Returns the wind force if this cell contains one.
    fn calculate_wind_for_cell(&self, cell: &WindCell, params: &WindCircleParams) -> Option<Wind> {
        let distance_to_center = (cell.origin - params.center).length();
        let inner_radius = params.inner_radius;
        let outer_radius = params.outer_radius;

        if distance_to_center >= inner_radius && distance_to_center <= outer_radius {
            // Wind generation logic specific to circular fields
            let radius_vector = cell.origin - params.center;
            let radius_dir = radius_vector.normalize();
            let tangent_dir = vec2(radius_dir.y, -radius_dir.x); // tangential, 90° CCW from radial

            // Rotate the tangent vector by bias * 90 degrees
            let angle = params.center_bias * -std::f32::consts::FRAC_PI_2; // π/2 = 90°

            let sin_a = angle.sin();
            let cos_a = angle.cos();

            // Rotate tangent_dir by 'angle'
            let blended_direction = vec2(
                tangent_dir.x * cos_a - tangent_dir.y * sin_a,
                tangent_dir.x * sin_a + tangent_dir.y * cos_a,
            );
            /*
            let tangential_direction = vec2(radius_vector.y, -radius_vector.x).normalize();
            let radial_inward_direction = -radius_vector.normalize();

            let blended_direction = (tangential_direction * (1.0 - params.center_bias)
                + radial_inward_direction * params.center_bias)
                .normalize();
             */

            Some(Wind::new_with(blended_direction, params.strength))
        } else {
            None
        }
    }

    /******************* Methods to change circle properties *******************/

    /// Returns true if the WindCircle has parameter changes that have not been applied.
    pub fn has_changes(&self) -> bool {
        self.with_params_read(|params| params.needs_recalculation)
    }

    /// Clear the needs_recalculation flag, indicating that the parameters have been applied.
    pub fn clear_changes(&mut self) {
        self.with_params_write(|params| params.needs_recalculation = false);
    }

    /// Return the WindCircleParams wrapped in an Arc<RwLock>
    pub fn params_arc(&self) -> Arc<RwLock<WindCircleParams>> {
        self.params.clone()
    }

    /// Abstracted function to read the parameters of the WindCircle.
    pub fn with_params_read<R>(&self, f: impl FnOnce(&WindCircleParams) -> R) -> R {
        let params = self.params.read().unwrap_or_else(|poisoned| {
            eprintln!("Warning: RwLock was poisoned. Recovering...");
            poisoned.into_inner() // You still get access to the data
        });
        f(&params)
    }

    /// Abstracted function to change parameters of the WindCircle.
    pub fn with_params_write<R>(&self, f: impl FnOnce(&mut WindCircleParams) -> R) -> R {
        let mut params = self.params.write().unwrap_or_else(|poisoned| {
            eprintln!("Warning: RwLock was poisoned. Recovering...");
            poisoned.into_inner() // You still get access to the data
        });
        f(&mut params)
    }

    /// Draw the center of the WindCircle
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

/// Parameters for a WindCircle.
/// - Center: The centerpoint of the circle in ParticleSystem space
/// - Outer radius: The outer radius of the circle
/// - Inner radius: The radius of the hole in the center of the circle
/// - Strength: The strength of the wind applied within the circle
/// - Center bias: 0.0 is tangential, 1.0 is radial inward, 2.0 is tangential in the opposite direction
/// - Needs recalculation: Flag to indicate that one or more parameters have changed so that WindField will recalculate
#[derive(Clone)]
pub struct WindCircleParams {
    pub center: Vec2, // center of the circle in the ParticleSystem space
    pub outer_radius: f32,
    pub inner_radius: f32,         // inner hole radius
    pub strength: f32,             // strength of the wind
    pub center_bias: f32,          // 0.0 = purely tangential, 1.0 = purely radial inward
    pub needs_recalculation: bool, // if settings changed, we need to recalculate the cells
}

impl WindCircleParams {
    /// Set the center of the WindCircle
    pub fn center(&mut self, center: Vec2) {
        if self.center != center {
            self.center = center;
            self.needs_recalculation = true;
        }
    }

    /// Set the OR of the WindCircle
    pub fn outer_radius(&mut self, radius: f32) {
        if self.outer_radius != radius {
            self.outer_radius = radius;
            self.needs_recalculation = true;
        }
    }

    /// Set the IR of the WindCircle
    pub fn inner_radius(&mut self, radius: f32) {
        if self.inner_radius != radius {
            self.inner_radius = radius;
            self.needs_recalculation = true;
        }
    }

    /// Set the strength of the WindCircle
    pub fn strength(&mut self, strength: f32) {
        if self.strength != strength {
            self.strength = strength;
            self.needs_recalculation = true;
        }
    }

    /// Set the center bias of the WindCircle
    pub fn center_bias(&mut self, center_bias: f32) {
        if self.center_bias != center_bias {
            self.center_bias = center_bias;
            self.needs_recalculation = true;
        }
    }
}
