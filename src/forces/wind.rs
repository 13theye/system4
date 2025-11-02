// src/forces/wind.rs
//
// Grid-based wind force for particle system

use crate::{
    forces::CellIdx,
    groups::VoiceId,
    particle::{Particle, ParticleCore},
};
use nannou::prelude::*;
use rayon::prelude::*;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

/// Maximum wind angle deviation in radians (90 degrees)
const MAX_WIND_ANGLE_DEVIATION: f32 = std::f32::consts::PI;

/// Create a unique hash from voice_id and circle_id for noise parameter indexing
fn hash_voice_circle(voice_id: VoiceId, circle_id: usize) -> u64 {
    let mut hasher = DefaultHasher::new();
    (voice_id, circle_id).hash(&mut hasher);
    hasher.finish()
}

/// A wind is a simple vector force that is applied to a particle.
/// It has a direction and a strength.
/// The direction is a unit vector that points in the direction of the wind.
/// The strength is a scalar that multiplies the direction to get the actual force.
/// The force is calculated as the difference between the wind's target velocity
/// and the particle's current velocity. Particle intertia is also considered.
/// The force is then added to the particle's acceleration.
#[derive(Clone, Copy, Debug, Default)]
pub struct Wind {
    // direction is a unit vector
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

    /// Apply the Wind to a ParticleCore with mass variation factor
    /// OPTIMIZED: Works with ParticleCore for better cache locality
    #[inline]
    pub fn apply(&self, particle: &mut ParticleCore, mass_variation_factor: f32) {
        // Activate the particle
        particle.activate();

        // Get the x and y components of particle's current velocity
        let particle_vx = particle.velocity.x;
        let particle_vy = particle.velocity.y;

        // Get the x and y components of wind's target velocity
        let wind_vx = self.direction.x * self.strength;
        let wind_vy = self.direction.y * self.strength;

        // Calculate the difference in each component
        let diff_x = wind_vx - particle_vx;
        let diff_y = wind_vy - particle_vy;

        // Calculate effective mass with variation factor
        let effective_mass = particle.mass * (1.0 + mass_variation_factor);

        // Calculate inertial resistance based on current momentum using effective mass
        let current_speed = particle.velocity.length();
        let momentum_magnitude = effective_mass * current_speed;

        // Inertial resistance: particles with higher momentum resist changes more
        let inertia_coefficient = 0.1; // Adjust this to control resistance strength
        let inertia_factor = 1.0 / (1.0 + momentum_magnitude * inertia_coefficient);

        // Apply the force with inertial resistance using effective mass
        let force = vec2(diff_x, diff_y) * inertia_factor;
        particle.acceleration += force / effective_mass;
    }

    /// Legacy method for Particle (backward compatibility)
    #[allow(dead_code)]
    pub fn _apply_legacy(&self, particle: &mut Particle, mass_variation_factor: f32) {
        particle.activate();
        let particle_vx = particle.velocity.x;
        let particle_vy = particle.velocity.y;
        let wind_vx = self.direction.x * self.strength;
        let wind_vy = self.direction.y * self.strength;
        let diff_x = wind_vx - particle_vx;
        let diff_y = wind_vy - particle_vy;
        let effective_mass = particle.mass * (1.0 + mass_variation_factor);
        let current_speed = particle.velocity.length();
        let momentum_magnitude = effective_mass * current_speed;
        let inertia_coefficient = 0.1;
        let inertia_factor = 1.0 / (1.0 + momentum_magnitude * inertia_coefficient);
        let force = vec2(diff_x, diff_y) * inertia_factor;
        particle.acceleration += force / effective_mass;
    }
}

/// A WindCell is a basic unit of a WindField.
/// It contains a list of winds that are acting on it,
/// and a combined wind cache that is the sum of all the winds.
/// The combined wind is recalculated when the cell or wind is updated.
pub struct WindCell {
    winds: HashMap<u64, Wind>,
    combined_wind: Option<Wind>,
    origin: Vec2,

    #[allow(dead_code)]
    // rect is used for debugging
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
    pub fn add_wind(&mut self, source_id: u64, wind: Wind) {
        self.winds.insert(source_id, wind);
        self.needs_update = true;
    }

    /// Remove a Wind from this WindCell.
    pub fn remove_wind(&mut self, id: u64) {
        self.winds.remove(&id);
        self.needs_update = true;
    }

    /// Get the sum of all Winds in this WindCell, recalculating if necessary.
    pub fn get_updated_combined_wind(&mut self, angle_variation: f32) -> Option<Wind> {
        if self.needs_update || angle_variation != 0.0 {
            self.calculate_combined_wind(angle_variation);
        }
        self.combined_wind
    }

    /// Get the sum of all Winds in this WindCell, without recalculating.
    pub fn get_combined_wind(&self) -> Option<Wind> {
        self.combined_wind
    }

    /// Calculate the combined effects of all Winds in this WindCell with angle variation.
    fn calculate_combined_wind(&mut self, angle_variation: f32) {
        if self.winds.is_empty() {
            self.combined_wind = None;
            return;
        }

        let mut total_force = vec2(0.0, 0.0);
        for wind in self.winds.values() {
            total_force += wind.direction * wind.strength;
        }

        let combined_strength = total_force.length();
        let mut combined_direction = if combined_strength > 0.0 {
            total_force.normalize()
        } else {
            vec2(0.0, 0.0) // if no force, direction doesn't matter.
        };

        // Apply angle variation if variation is non-zero and we have a valid direction
        if angle_variation != 0.0 && combined_strength > 0.0 {
            // Calculate actual angle deviation using pre-computed variation
            let angle_offset = angle_variation * MAX_WIND_ANGLE_DEVIATION;

            // Apply rotation to the combined direction
            let cos_a = angle_offset.cos();
            let sin_a = angle_offset.sin();

            combined_direction = vec2(
                combined_direction.x * cos_a - combined_direction.y * sin_a,
                combined_direction.x * sin_a + combined_direction.y * cos_a,
            );
        }

        self.combined_wind = Some(Wind::new_with(combined_direction, combined_strength));
        self.needs_update = false;
    }
}

/// The WindField is a grid of WindCells.
/// It is used to apply wind forces to particles.
/// The grid is used to quickly find the wind force at a given position.
pub struct WindField {
    // Flattened grid of winds. Use get_cell_index() to convert (x,y) to 1D index.
    cells: Vec<WindCell>,

    // Origin should align with ParticleSystem origin
    origin: Vec2,
    bounds_size: Vec2,
    grid_cols: usize,
    grid_rows: usize,
    cell_size: Vec2,
}

impl WindField {
    /// Convert 2D grid coordinates to 1D index
    fn get_cell_index(&self, x: usize, y: usize) -> usize {
        y * self.grid_cols + x
    }

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

        let mut cells = Vec::with_capacity(grid_cols * grid_rows);

        for row in 0..grid_rows {
            for col in 0..grid_cols {
                let cell_origin = top_left
                    + Vec2::new(
                        col as f32 * cell_size.x + cell_size.x / 2.0,
                        -(row as f32 * cell_size.y + cell_size.y / 2.0), // Negative Y to go downward
                    );
                let cell = WindCell::new_from_origin(cell_origin, cell_size);
                cells.push(cell);
            }
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
    /// OPTIMIZED: Works with ParticleCore for better cache locality
    #[inline]
    pub fn apply(&self, particle: &mut ParticleCore, mass_variation_factor: f32) {
        let Some(wind) = self.get_wind_at_pos(particle.position) else {
            return;
        };
        wind.apply(particle, mass_variation_factor);
    }

    /// Legacy method for Particle (backward compatibility)
    #[allow(dead_code)]
    pub fn _apply_legacy(&self, particle: &mut Particle, mass_variation_factor: f32) {
        let Some(wind) = self.get_wind_at_pos(particle.position()) else {
            return;
        };
        wind._apply_legacy(particle, mass_variation_factor);
    }

    /// Force a recalculation of all cells in the WindField.
    pub fn force_update_all(&mut self) {
        for cell in &mut self.cells {
            let _ = cell.get_updated_combined_wind(0.0); // No angle variation
        }
    }

    /// Force a recalculation of all cells in the WindField in parallel with per-circle angle variations
    pub fn par_force_update_all(
        &mut self,
        rng: &mut rand::rngs::ThreadRng,
        circle_noise_values: &HashMap<u64, f32>,
    ) {
        use rand::Rng;

        // Pre-compute random variations for all cells for each circle that has angle variation
        let mut cell_variations: HashMap<u64, Vec<f32>> = HashMap::new();
        for (&hash_key, &variation_factor) in circle_noise_values.iter() {
            if variation_factor > 0.0 {
                let variations: Vec<f32> = self
                    .cells
                    .iter()
                    .map(|_| rng.random_range(-1.0..=1.0) * variation_factor)
                    .collect();
                cell_variations.insert(hash_key, variations);
            }
        }

        // Parallel update with pre-computed per-circle variations
        self.cells
            .par_iter_mut()
            .enumerate()
            .for_each(|(index, cell)| {
                // Calculate combined angle variation for this cell based on all circles affecting it
                let mut combined_variation = 0.0f32;
                let mut variation_count = 0;

                for &hash_key in cell.winds.keys() {
                    // Use hash key directly to look up noise variations
                    if let Some(variations) = cell_variations.get(&hash_key) {
                        if let Some(&variation) = variations.get(index) {
                            combined_variation += variation;
                            variation_count += 1;
                        }
                    }
                }

                // Average the variations if multiple circles affect this cell
                let final_variation = if variation_count > 0 {
                    combined_variation / variation_count as f32
                } else {
                    0.0
                };

                let _ = cell.get_updated_combined_wind(final_variation);
            });
    }

    /******************* Grid accessors *******************/

    /// Get combined wind at a position in ParticleSystem coordinates
    pub fn get_wind_at_pos(&self, position: Vec2) -> Option<Wind> {
        let (x, y) = self.position_to_idx(position)?;

        self.get_wind(x, y)
    }

    /// Get the cell at a grid position (0,0 is top left)
    pub fn get_cell(&self, x: usize, y: usize) -> Option<&WindCell> {
        if x >= self.grid_cols || y >= self.grid_rows {
            return None;
        }
        let index = self.get_cell_index(x, y);
        self.cells.get(index)
    }

    /// Get a mutable reference to the cell at a grid position (0,0 is top left)
    pub fn get_mut_cell(&mut self, x: usize, y: usize) -> Option<&mut WindCell> {
        if x >= self.grid_cols || y >= self.grid_rows {
            return None;
        }
        let index = self.get_cell_index(x, y);
        self.cells.get_mut(index)
    }

    /// Get the combined wind at a grid position (0,0 is top left)
    pub fn get_wind(&self, x: usize, y: usize) -> Option<Wind> {
        if x >= self.grid_cols || y >= self.grid_rows {
            return None;
        }
        let index = self.get_cell_index(x, y);
        let cell = self.cells.get(index)?;
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
    fn position_to_idx(&self, pos: Vec2) -> Option<(usize, usize)> {
        let transformed = self.world_to_grid_coords(pos);
        let i = transformed.x.floor().max(0.0) as usize;
        let j = transformed.y.floor().max(0.0) as usize;

        if i < self.grid_cols && j < self.grid_rows {
            Some((i, j))
        } else {
            None // Out of bounds
        }
    }

    /// Helper method to transform world coordinates to grid coordinates (floating point)
    fn world_to_grid_coords(&self, pos: Vec2) -> Vec2 {
        let x1 = pos.x + self.bounds_size.x / 2.0;
        let y1 = -pos.y + self.bounds_size.y / 2.0;

        vec2(x1 / self.cell_size.x, y1 / self.cell_size.y)
    }

    /******************* Draw for Performer *******************/

    /// Draw the WindField
    pub fn draw(&self, draw: &Draw, scale_x: f32, scale_y: f32, div_factor: usize) {
        self.draw_origin(draw, scale_x, scale_y);
        self.draw_grid(draw, scale_x, scale_y);
        self.draw_vectors(draw, scale_x, scale_y, div_factor);
    }

    /// Draw all the Wind vectors
    fn draw_vectors(&self, draw: &Draw, scale_x: f32, scale_y: f32, div_factor: usize) {
        // Draw wind vectors from selected cells based on div_factor
        for (index, cell) in self.cells.iter().enumerate() {
            let Some(wind) = &cell.combined_wind else {
                continue;
            };

            // Calculate row and column from the flat index
            let col = index % self.grid_cols;
            let row = index / self.grid_cols;

            // Sample based on div_factor using deterministic pattern
            if !(col + row).is_multiple_of(div_factor) {
                continue;
            }

            // Draw wind vector from cell origin
            let vector_scale = 3.0; // Increased scale for better visibility
            let vector_end = cell.origin + wind.direction * wind.strength * vector_scale;

            draw.line()
                .start(cell.origin * vec2(scale_x, scale_y))
                .end(vector_end * vec2(scale_x, scale_y))
                .color(rgba(0.0, 0.8, 1.0, 0.2))
                .stroke_weight(5.0);

            draw.rect()
                .xy(vector_end * vec2(scale_x, scale_y))
                .wh(vec2(20.0 * scale_x, 20.0 * scale_y))
                .color(rgba(1.0, 0.0, 0.0, 0.2))
                .stroke_weight(0.0);
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
        for cell in &self.cells {
            let color: Rgba = if let Some(wind) = cell.combined_wind {
                let h = wind.direction.y.atan2(wind.direction.x) / (2.0 * PI);
                let s = wind.strength / 30.0;
                Rgba::from(hsv(h, s, 1.0))
            } else {
                rgba(0.0, 0.0, 0.0, 0.0)
            };

            draw.rect()
                .xy(cell.origin * vec2(scale_x, scale_y))
                .w_h(self.cell_size.x * scale_x, self.cell_size.y * scale_y)
                .stroke_color(rgba(0.3, 0.3, 0.3, 0.3))
                .stroke_weight(1.0)
                .color(color);
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
    pub parent_voice: VoiceId,
    cell_idxs: Vec<CellIdx>, // Indices of cells that are affected by the circle
    params: WindCircleParams, // Params of the circle
}

#[allow(clippy::too_many_arguments)]
impl WindCircle {
    pub fn new(
        id: usize,
        parent_voice: VoiceId,
        center: Vec2,
        radius: f32,
        width: f32,
        strength: f32,
        center_bias: f32,
        noise: f32,
    ) -> Self {
        let config = WindCircleParams {
            center,
            outer_radius: radius,
            inner_radius: width,
            force: strength,
            gravity: center_bias,
            noise,
            dirty: true,
        };
        Self {
            id,
            parent_voice,
            cell_idxs: Vec::new(),
            params: config,
        }
    }

    /// Recalculate wind within each grid inside this circle, if the parameters have changed
    pub fn update(&mut self, field: &mut WindField) {
        if self.has_changes() {
            self.remove_from_field(field);
            self.cell_idxs = self.apply_to_field(field);
            self.clear_changes();
        }
    }

    /// Remove the circle's wind from the field
    pub fn remove_from_field(&mut self, field: &mut WindField) {
        let hash_key = hash_voice_circle(self.parent_voice, self.id);
        for cell_idx in self.cell_idxs.iter() {
            let Some(cell) = field.get_mut_cell(cell_idx.x, cell_idx.y) else {
                continue;
            };
            cell.remove_wind(hash_key);

            // In debug mode, pre-calculate combined wind so we can draw the field
            // no longer needed because we update all cells on every frame now
            /*
            if show_forces {
                let _ = cell.get_updated_combined_wind();
            }
            */
        }
        self.cell_idxs.clear();
    }

    /// Add the circle's wind to the field, return the cells that were affected.
    /// The total numerical force for each cell is calculated once per frame in a later step.
    pub fn apply_to_field(&self, field: &mut WindField) -> Vec<CellIdx> {
        // Calculate bounding box using direct parameter access
        let (min_col, max_col, min_row, max_row) = self.calculate_bounding_box(field, &self.params);

        let mut affected_cells = Vec::new();
        let hash_key = hash_voice_circle(self.parent_voice, self.id);

        // Update cells serially
        for col in min_col..max_col {
            for row in min_row..max_row {
                let Some(cell) = field.get_mut_cell(col, row) else {
                    continue;
                };
                let Some(wind) = self.calculate_wind_for_cell(cell, &self.params) else {
                    continue;
                };
                cell.add_wind(hash_key, wind);

                // In debug mode, pre-calculate combined wind so we can draw the field
                // no longer needed because we update all cells on every frame now
                /*
                if show_forces {
                    let _ = cell.get_updated_combined_wind();
                }
                */

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
            let angle = params.gravity * -std::f32::consts::FRAC_PI_2; // π/2 = 90°

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

            Some(Wind::new_with(blended_direction, params.force))
        } else {
            None
        }
    }

    /// Returns a bounding Rect in screen coordinates that encompasses the entire WindCircle
    pub fn rect(&self) -> Rect {
        let center = self.params.center;
        let outer_radius = self.params.outer_radius;

        Rect::from_x_y_w_h(center.x, center.y, outer_radius * 2.0, outer_radius * 2.0)
    }

    /******************* Methods to change circle properties *******************/

    /// Returns true if the WindCircle has parameter changes that have not been applied.
    pub fn has_changes(&self) -> bool {
        self.params.dirty
    }

    /// Clear the needs_recalculation flag, indicating that the parameters have been applied.
    pub fn clear_changes(&mut self) {
        self.params.dirty = false;
    }

    /// Return a reference to the WindCircleParams
    pub fn params(&self) -> &WindCircleParams {
        &self.params
    }

    /// Return a mutable reference to the WindCircleParams
    pub fn params_mut(&mut self) -> &mut WindCircleParams {
        &mut self.params
    }

    /// Draw the center of the WindCircle
    pub fn draw_center(&self, draw: &Draw, scale_x: f32, scale_y: f32) {
        let center = self.params.center;
        draw.ellipse()
            .xy(center * vec2(scale_x, scale_y))
            .w_h(40.0 * scale_x, 40.0 * scale_y)
            .color(rgba(1.0, 0.2, 0.0, 0.2));
    }

    /// Draw the WindCircle with outer and inner radius circles
    pub fn draw(&self, draw: &Draw, scale_x: f32, scale_y: f32) {
        let center = self.params.center * vec2(scale_x, scale_y);
        let outer_radius = self.params.outer_radius;
        let inner_radius = self.params.inner_radius;

        // Draw outer radius circle
        draw.ellipse()
            .xy(center)
            .radius(outer_radius * scale_x.min(scale_y))
            .stroke_color(rgba(0.8, 0.4, 0.0, 0.6))
            .stroke_weight(2.0)
            .no_fill();

        // Draw inner radius circle
        draw.ellipse()
            .xy(center)
            .radius(inner_radius * scale_x.min(scale_y))
            .stroke_color(rgba(0.8, 0.4, 0.0, 0.4))
            .stroke_weight(1.0)
            .no_fill();
    }
}

/// Parameters for a WindCircle.
/// - Center: The centerpoint of the circle in ParticleSystem space
/// - Outer radius: The outer radius of the circle
/// - Inner radius: The radius of the hole in the center of the circle
/// - Strength: The strength of the wind applied within the circle
/// - Center bias: 0.0 is tangential, 1.0 is radial inward, 2.0 is tangential in the opposite direction
/// - Angle variation: Amount of random angle variation (0.0-1.0, where 1.0 = ±90° deviation)
/// - Dirty: Flag to indicate that one or more parameters have changed so that WindField will recalculate
#[derive(Clone)]
pub struct WindCircleParams {
    /// center of the circle in the ParticleSystem space
    pub center: Vec2,
    /// outer circle radius
    pub outer_radius: f32,
    /// inner hole radius
    pub inner_radius: f32,
    /// strength of the wind
    pub force: f32,
    /// 0.0 = purely tangential, 1.0 = purely radial inward, 2.0 = tangential in the opposite direction
    pub gravity: f32,
    /// 0.0-1.0 factor for random angle variation, where 1.0 = full ±90° deviation
    pub noise: f32,
    /// True if settings changed and cells need recalculation
    pub dirty: bool,
}

impl Default for WindCircleParams {
    fn default() -> Self {
        Self {
            center: Vec2::ZERO,
            outer_radius: 0.0,
            inner_radius: 0.0,
            force: 0.0,
            gravity: 0.0,
            noise: 0.0,
            dirty: true,
        }
    }
}

impl WindCircleParams {
    /// Set the center of the WindCircle
    pub fn set_center(&mut self, center: Vec2) {
        if self.center != center {
            self.center = center;
            self.dirty = true;
        }
    }

    pub fn set_center_x(&mut self, x: f32) {
        if self.center.x != x {
            self.center.x = x;
            self.dirty = true;
        }
    }

    pub fn set_center_y(&mut self, y: f32) {
        if self.center.y != y {
            self.center.y = y;
            self.dirty = true;
        }
    }

    /// Set the OR of the WindCircle
    pub fn set_outer_radius(&mut self, radius: f32) {
        if self.outer_radius != radius {
            self.outer_radius = radius;
            self.dirty = true;
        }
    }

    /// Set the IR of the WindCircle
    pub fn set_inner_radius(&mut self, radius: f32) {
        if self.inner_radius != radius {
            self.inner_radius = radius;
            self.dirty = true;
        }
    }

    /// Set the strength of the WindCircle
    pub fn set_force(&mut self, force: f32) {
        if self.force != force {
            self.force = force;
            self.dirty = true;
        }
    }

    /// Set the center bias of the WindCircle
    pub fn set_gravity(&mut self, gravity: f32) {
        if self.gravity != gravity {
            self.gravity = gravity;
            self.dirty = true;
        }
    }

    /// Set the angle variation of the WindCircle
    pub fn set_noise(&mut self, noise: f32) {
        let clamped_noise = noise.clamp(0.0, 1.0);
        if self.noise != clamped_noise {
            self.noise = clamped_noise;
            self.dirty = true;
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
