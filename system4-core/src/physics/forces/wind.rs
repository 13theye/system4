// system4-core/src/physics/forces/wind.rs
// Grid-based wind force for particle system

use crate::constants::{INERTIA_COEFFICIENT, MAX_WIND_ANGLE_DEVIATION};
use crate::physics::particles::ParticleCore;
use crate::physics::VoiceId;
use glam::Vec2;
use rayon::prelude::*;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

/// A wind is a simple vector force that is applied to a particle.
/// It has a direction and a strength.
/// The direction is a unit vector that points in the direction of the wind.
/// The strength is a scalar that multiplies the direction to get the actual force.
/// The force is calculated as the difference between the wind's target velocity
/// and the particle's current velocity. Particle inertia is also considered.
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
            direction: Vec2::ZERO,
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
        let inertia_factor = 1.0 / (1.0 + momentum_magnitude * INERTIA_COEFFICIENT);

        // Apply the force with inertial resistance using effective mass
        let force = Vec2::new(diff_x, diff_y) * inertia_factor;
        particle.acceleration += force / effective_mass;
    }

    // Accessors
    pub fn direction(&self) -> Vec2 {
        self.direction
    }

    pub fn strength(&self) -> f32 {
        self.strength
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
    size: Vec2,
    needs_update: bool,
}

impl WindCell {
    pub fn new_from_origin(origin: Vec2, size: Vec2) -> Self {
        Self {
            winds: HashMap::new(),
            combined_wind: None,
            origin,
            size,
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

    // Accessors for drawing
    pub fn origin(&self) -> Vec2 {
        self.origin
    }

    pub fn combined_wind(&self) -> Option<Wind> {
        self.combined_wind
    }

    /// Calculate the combined effects of all Winds in this WindCell with angle variation.
    fn calculate_combined_wind(&mut self, angle_variation: f32) {
        if self.winds.is_empty() {
            self.combined_wind = None;
            return;
        }

        let mut total_force = Vec2::ZERO;
        for wind in self.winds.values() {
            total_force += wind.direction * wind.strength;
        }

        let combined_strength = total_force.length();
        let mut combined_direction = if combined_strength > 0.0 {
            total_force.normalize()
        } else {
            Vec2::ZERO // if no force, direction doesn't matter.
        };

        // Apply angle variation if variation is non-zero and we have a valid direction
        if angle_variation != 0.0 && combined_strength > 0.0 {
            // Calculate actual angle deviation using pre-computed variation
            let angle_offset = angle_variation * MAX_WIND_ANGLE_DEVIATION;

            // Apply rotation to the combined direction
            let cos_a = angle_offset.cos();
            let sin_a = angle_offset.sin();

            combined_direction = Vec2::new(
                combined_direction.x * cos_a - combined_direction.y * sin_a,
                combined_direction.x * sin_a + combined_direction.y * cos_a,
            );
        }

        self.combined_wind = Some(Wind::new_with(combined_direction, combined_strength));
        self.needs_update = false;
    }

    /// Apply the combined wind of this cell to a particle
    #[inline]
    pub fn apply(&self, particle: &mut ParticleCore, mass_variation_factor: f32) {
        if let Some(wind) = self.combined_wind {
            wind.apply(particle, mass_variation_factor);
        }
    }
}

/// Index for accessing cells in a grid
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CellIdx {
    pub col: usize,
    pub row: usize,
}

impl CellIdx {
    pub fn new(col: usize, row: usize) -> Self {
        Self { col, row }
    }
}

/// A WindField is a grid of WindCells that cover a rectangular area.
/// Each cell can have multiple wind forces acting on it.
pub struct WindField {
    cells: Vec<WindCell>,
    origin: Vec2,
    bounds_size: Vec2,
    grid_cols: usize,
    grid_rows: usize,
    cell_size: Vec2,
}

impl WindField {
    pub fn new(origin: Vec2, bounds_size: Vec2, grid_cols: usize, grid_rows: usize) -> Self {
        let cell_size = Vec2::new(
            bounds_size.x / grid_cols as f32,
            bounds_size.y / grid_rows as f32,
        );

        let mut cells = Vec::with_capacity(grid_cols * grid_rows);
        for row in 0..grid_rows {
            for col in 0..grid_cols {
                let cell_origin = Vec2::new(
                    origin.x + col as f32 * cell_size.x,
                    origin.y + row as f32 * cell_size.y,
                );
                cells.push(WindCell::new_from_origin(cell_origin, cell_size));
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

    /// Get the cell index for a given position
    pub fn get_cell_idx(&self, position: Vec2) -> Option<CellIdx> {
        // Calculate relative position from origin
        let rel_x = position.x - self.origin.x;
        let rel_y = position.y - self.origin.y;

        // Check if within bounds
        if rel_x < 0.0 || rel_x >= self.bounds_size.x || rel_y < 0.0 || rel_y >= self.bounds_size.y
        {
            return None;
        }

        let col = (rel_x / self.cell_size.x) as usize;
        let row = (rel_y / self.cell_size.y) as usize;

        // Clamp to valid indices
        let col = col.min(self.grid_cols - 1);
        let row = row.min(self.grid_rows - 1);

        Some(CellIdx::new(col, row))
    }

    /// Get a mutable reference to a cell
    pub fn get_cell_mut(&mut self, idx: CellIdx) -> Option<&mut WindCell> {
        let index = idx.row * self.grid_cols + idx.col;
        self.cells.get_mut(index)
    }

    /// Get a reference to a cell
    pub fn get_cell(&self, idx: CellIdx) -> Option<&WindCell> {
        let index = idx.row * self.grid_cols + idx.col;
        self.cells.get(index)
    }

    /// Apply wind force to a particle based on its position
    #[inline]
    pub fn apply(&self, particle: &mut ParticleCore, mass_variation_factor: f32) {
        if let Some(idx) = self.get_cell_idx(particle.position) {
            if let Some(cell) = self.get_cell(idx) {
                cell.apply(particle, mass_variation_factor);
            }
        }
    }

    /// Update all cells with angle variations (parallel)
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

    /// Clear all winds from all cells
    pub fn clear_all_winds(&mut self) {
        for cell in &mut self.cells {
            cell.winds.clear();
            cell.combined_wind = None;
            cell.needs_update = true;
        }
    }

    /// Add a wind to a specific cell
    pub fn add_wind_to_cell(&mut self, idx: CellIdx, source_id: u64, wind: Wind) {
        if let Some(cell) = self.get_cell_mut(idx) {
            cell.add_wind(source_id, wind);
        }
    }

    /// Get a mutable cell by column and row
    pub fn get_mut_cell(&mut self, col: usize, row: usize) -> Option<&mut WindCell> {
        if col >= self.grid_cols || row >= self.grid_rows {
            return None;
        }
        let idx = CellIdx::new(col, row);
        self.get_cell_mut(idx)
    }

    /// Convert world coordinates to grid coordinates
    pub fn world_to_grid_coords(&self, world_pos: Vec2) -> Vec2 {
        // Calculate offset from top-left corner
        let offset_from_origin = world_pos - self.origin;

        // Add half the bounds to shift to top-left origin
        let from_top_left = offset_from_origin + self.bounds_size / 2.0;

        // Convert to grid coordinates
        Vec2::new(
            from_top_left.x / self.cell_size.x,
            from_top_left.y / self.cell_size.y,
        )
    }

    // Accessors for drawing and debugging
    pub fn origin(&self) -> Vec2 {
        self.origin
    }

    pub fn bounds_size(&self) -> Vec2 {
        self.bounds_size
    }

    pub fn grid_cols(&self) -> usize {
        self.grid_cols
    }

    pub fn grid_rows(&self) -> usize {
        self.grid_rows
    }

    pub fn cell_size(&self) -> Vec2 {
        self.cell_size
    }
}

// ========== WindCircle - Meta-force that creates circular wind patterns ==========

/// Create a unique hash from voice_id and circle_id for indexing
fn hash_voice_circle(voice_id: VoiceId, circle_id: usize) -> u64 {
    let mut hasher = DefaultHasher::new();
    (voice_id, circle_id).hash(&mut hasher);
    hasher.finish()
}

/// A WindCircle defines a circular area where wind forces are applied.
/// The circle creates a radial/tangential wind pattern based on its parameters.
#[derive(Clone)]
pub struct WindCircle {
    pub id: usize,
    pub parent_voice: VoiceId,
    cell_idxs: Vec<CellIdx>, // Indices of cells affected by the circle
    params: WindCircleParams,
}

impl WindCircle {
    #[allow(clippy::too_many_arguments)]
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
        let params = WindCircleParams {
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
            params,
        }
    }

    /// Update the circle's wind in the field if parameters have changed
    pub fn update(&mut self, field: &mut WindField) {
        if self.has_changes() {
            self.remove_from_field(field);
            let affected = self.apply_to_field(field);
            self.cell_idxs = affected;
            self.clear_changes();
        }
    }

    /// Remove the circle's wind from the field
    pub fn remove_from_field(&mut self, field: &mut WindField) {
        let hash_key = hash_voice_circle(self.parent_voice, self.id);
        for cell_idx in self.cell_idxs.iter() {
            if let Some(cell) = field.get_mut_cell(cell_idx.col, cell_idx.row) {
                cell.remove_wind(hash_key);
            }
        }
        self.cell_idxs.clear();
    }

    /// Apply the circle's wind to the field, return affected cells
    pub fn apply_to_field(&self, field: &mut WindField) -> Vec<CellIdx> {
        let (min_col, max_col, min_row, max_row) = self.calculate_bounding_box(field);
        let mut affected_cells = Vec::new();
        let hash_key = hash_voice_circle(self.parent_voice, self.id);

        for col in min_col..max_col {
            for row in min_row..max_row {
                if let Some(cell) = field.get_mut_cell(col, row) {
                    if let Some(wind) = self.calculate_wind_for_cell(cell) {
                        cell.add_wind(hash_key, wind);
                        affected_cells.push(CellIdx::new(col, row));
                    }
                }
            }
        }

        affected_cells
    }

    /// Calculate bounding box of cells affected by this circle
    fn calculate_bounding_box(&self, field: &WindField) -> (usize, usize, usize, usize) {
        let outer_radius = self.params.outer_radius + self.params.inner_radius / 2.0;
        let center_pos_transformed = field.world_to_grid_coords(self.params.center);

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

    /// Calculate wind force for a cell (if within the circle's area)
    fn calculate_wind_for_cell(&self, cell: &WindCell) -> Option<Wind> {
        let distance_to_center = (cell.origin - self.params.center).length();

        if distance_to_center >= self.params.inner_radius
            && distance_to_center <= self.params.outer_radius
        {
            let radius_vector = cell.origin - self.params.center;
            let radius_dir = radius_vector.normalize();
            let tangent_dir = Vec2::new(radius_dir.y, -radius_dir.x); // 90° CCW from radial

            // Rotate tangent vector by gravity parameter
            let angle = self.params.gravity * -std::f32::consts::FRAC_PI_2;
            let sin_a = angle.sin();
            let cos_a = angle.cos();

            let blended_direction = Vec2::new(
                tangent_dir.x * cos_a - tangent_dir.y * sin_a,
                tangent_dir.x * sin_a + tangent_dir.y * cos_a,
            );

            Some(Wind::new_with(blended_direction, self.params.force))
        } else {
            None
        }
    }

    /// Check if circle has parameter changes
    pub fn has_changes(&self) -> bool {
        self.params.dirty
    }

    /// Clear the dirty flag
    pub fn clear_changes(&mut self) {
        self.params.dirty = false;
    }

    /// Get reference to parameters
    pub fn params(&self) -> &WindCircleParams {
        &self.params
    }

    /// Get mutable reference to parameters
    pub fn params_mut(&mut self) -> &mut WindCircleParams {
        &mut self.params
    }
}

/// Parameters for a WindCircle
#[derive(Clone)]
pub struct WindCircleParams {
    pub center: Vec2,
    pub outer_radius: f32,
    pub inner_radius: f32,
    pub force: f32,
    /// 0.0 = tangential, 1.0 = radial inward, 2.0 = opposite tangential
    pub gravity: f32,
    /// 0.0-1.0 factor for angle variation (1.0 = ±90°)
    pub noise: f32,
    /// True if parameters changed and need recalculation
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

    pub fn set_outer_radius(&mut self, radius: f32) {
        if self.outer_radius != radius {
            self.outer_radius = radius;
            self.dirty = true;
        }
    }

    pub fn set_inner_radius(&mut self, radius: f32) {
        if self.inner_radius != radius {
            self.inner_radius = radius;
            self.dirty = true;
        }
    }

    pub fn set_force(&mut self, force: f32) {
        if self.force != force {
            self.force = force;
            self.dirty = true;
        }
    }

    pub fn set_gravity(&mut self, gravity: f32) {
        if self.gravity != gravity {
            self.gravity = gravity;
            self.dirty = true;
        }
    }

    pub fn set_noise(&mut self, noise: f32) {
        let clamped_noise = noise.clamp(0.0, 1.0);
        if self.noise != clamped_noise {
            self.noise = clamped_noise;
            self.dirty = true;
        }
    }
}
