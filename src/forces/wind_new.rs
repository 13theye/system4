// src/forces/wind.rs
//
// Rewrite of wind system

use crate::{groups::VoiceId, particle::ParticleCore};
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

/// Original Wind struct has been simplified as a simple Vec2 encoding both strengh and direction.
#[derive(Debug, Default, Clone)]
pub struct WindNew {
    pub velocity: Vec2,
}

impl WindNew {
    /// Create a default wind, equivalent to Vec2::ZERO
    pub fn zero() -> Self {
        Self::default()
    }

    /// Create a new wind with the given vector
    pub fn new(velocity: Vec2) -> Self {
        Self { velocity }
    }

    /// Create a new Wind with a direction and speed
    pub fn new_with(direction: Vec2, speed: f32) -> Self {
        Self {
            velocity: direction * speed,
        }
    }

    /// Return the strength of the wind as a scalar
    pub fn strength(&self) -> f32 {
        self.velocity.length()
    }

    /// Return the direction component of the wind as a unit vector
    pub fn direction(&self) -> Vec2 {
        self.velocity.try_normalize().unwrap_or(Vec2::ZERO)
    }

    /// Apply the Wind to a ParticleCore with mass variation factor
    #[inline]
    pub fn apply(&self, particle: &mut ParticleCore, mass_variation_factor: f32) {
        // Activate the particle
        particle.activate();

        // Calculate the difference between wind's target velocity and particle's current velocity
        let diff = self.velocity - particle.velocity;

        // Calculate effective mass with variation factor
        let effective_mass = particle.mass * (1.0 + mass_variation_factor);

        // Calculate inertial resistance based on current momentum using effective mass
        let current_speed = particle.velocity.length();
        let momentum_magnitude = effective_mass * current_speed;

        // Inertial resistance: particles with higher momentum resist changes more
        let inertia_coefficient = 0.1; // Adjust this to control resistance strength
        let inertia_factor = 1.0 / (1.0 + momentum_magnitude * inertia_coefficient);

        // Apply the force with inertial resistance using effective mass
        let force = diff * inertia_factor;
        particle.acceleration += force / effective_mass;
    }
}

/// A flattened vector representing all Wind Cells using Struct-of-Arrays layout.
/// Each cell contains separate vectors for source IDs and winds for better cache performance.
/// INVARIANT: source_ids[i].len() == winds[i].len() for all i
#[derive(Default)]
pub struct WindTable {
    pub source_ids: Vec<Vec<u64>>,
    pub winds: Vec<Vec<WindNew>>,
}

impl WindTable {
    /// Get the winds at a given cell index
    pub fn get_winds(&self, index: usize) -> Option<&Vec<WindNew>> {
        self.winds.get(index)
    }

    /// Get mutable reference to winds at a given cell index
    pub fn get_winds_mut(&mut self, index: usize) -> Option<&mut Vec<WindNew>> {
        self.winds.get_mut(index)
    }

    /// Get the source IDs at a given cell index
    pub fn get_source_ids(&self, index: usize) -> Option<&Vec<u64>> {
        self.source_ids.get(index)
    }

    /// Check that the SoA invariant holds (source_ids and winds have same length)
    #[cfg(debug_assertions)]
    fn verify_invariant(&self, cell_idx: usize) {
        debug_assert_eq!(
            self.source_ids[cell_idx].len(),
            self.winds[cell_idx].len(),
            "WindTable SoA invariant violated at cell {}: source_ids.len() = {}, winds.len() = {}",
            cell_idx,
            self.source_ids[cell_idx].len(),
            self.winds[cell_idx].len()
        );
    }
}

/// A flattened vector representing all Wind Cells.
/// Each cell contains the combined wind acting on it.
/// None = no wind (particles unaffected), Some(WindNew) = wind force applied
#[derive(Default)]
pub struct WindsCombined {
    pub cells: Vec<Option<WindNew>>,
}

impl WindsCombined {
    pub fn get(&self, index: usize) -> Option<&Option<WindNew>> {
        self.cells.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut Option<WindNew>> {
        self.cells.get_mut(index)
    }
}

/// Parameters for the WindField
#[derive(Default)]
pub struct WindFieldParams {
    origin: Vec2,
    bounds_size: Vec2,
    grid_cols: usize,
    grid_rows: usize,
    cell_size: Vec2,
}

/// The WindField is the orchestrator of the WindCells.
pub struct WindField {
    wind_table: WindTable,
    winds_combined: WindsCombined,
    params: WindFieldParams,
}

impl WindField {
    /// Create a new WindField, with a center origin, x&y size, number of columns and number of rows.
    /// Top-left is (0,0).
    pub fn new(origin: Vec2, bounds_size: Vec2, grid_cols: usize, grid_rows: usize) -> Self {
        let total_cells = grid_cols * grid_rows;
        let params = WindFieldParams {
            origin,
            bounds_size,
            grid_cols,
            grid_rows,
            cell_size: Vec2::new(
                bounds_size.x / grid_cols as f32,
                bounds_size.y / grid_rows as f32,
            ),
        };
        Self {
            wind_table: WindTable {
                source_ids: vec![Vec::new(); total_cells],
                winds: vec![Vec::new(); total_cells],
            },
            winds_combined: WindsCombined {
                cells: vec![None; total_cells],
            },
            params,
        }
    }

    /// Convert 2D grid coords to 1D index
    pub fn get_cell_index(&self, x: usize, y: usize) -> Option<usize> {
        if x >= self.params.grid_cols || y >= self.params.grid_rows {
            return None;
        }
        Some(y * self.params.grid_cols + x)
    }

    /// Given a particle, determines which WindCell contains the particle, then applies the WindCell's Wind on that particle.
    /// If there is no wind at the particle's position, no force is applied.
    #[inline]
    pub fn apply(&self, particle: &mut ParticleCore, mass_variation_factor: f32) {
        let Some(wind) = self.get_wind_at_pos(particle.position) else {
            return; // No wind = no force applied
        };
        wind.apply(particle, mass_variation_factor);
    }

    /// Add or update a wind source at the specified grid position
    pub fn add_wind(&mut self, x: usize, y: usize, source_id: u64, wind: WindNew) {
        let Some(idx) = self.get_cell_index(x, y) else {
            return;
        };

        // Find if this source already exists
        if let Some(pos) = self.wind_table.source_ids[idx]
            .iter()
            .position(|&id| id == source_id)
        {
            // Update existing wind
            self.wind_table.winds[idx][pos] = wind;
        } else {
            // Add new source
            self.wind_table.source_ids[idx].push(source_id);
            self.wind_table.winds[idx].push(wind);
        }

        #[cfg(debug_assertions)]
        self.wind_table.verify_invariant(idx);
    }

    /// Remove a wind source from the specified grid position
    pub fn remove_wind(&mut self, x: usize, y: usize, source_id: u64) {
        let Some(idx) = self.get_cell_index(x, y) else {
            return;
        };

        // Find and remove the source
        if let Some(pos) = self.wind_table.source_ids[idx]
            .iter()
            .position(|&id| id == source_id)
        {
            self.wind_table.source_ids[idx].remove(pos);
            self.wind_table.winds[idx].remove(pos);
        }

        #[cfg(debug_assertions)]
        self.wind_table.verify_invariant(idx);
    }

    /// Add a wind from a WindCircle at the specified grid position
    /// Uses hash_voice_circle to generate a unique source ID
    pub fn add_circle_wind(
        &mut self,
        x: usize,
        y: usize,
        circle_id: usize,
        voice_id: VoiceId,
        wind: WindNew,
    ) {
        let source_id = hash_voice_circle(voice_id, circle_id);
        self.add_wind(x, y, source_id, wind);
    }

    /// Remove a wind from a WindCircle at the specified grid position
    /// Uses hash_voice_circle to generate the source ID
    pub fn remove_circle_wind(&mut self, x: usize, y: usize, circle_id: usize, voice_id: VoiceId) {
        let source_id = hash_voice_circle(voice_id, circle_id);
        self.remove_wind(x, y, source_id);
    }

    pub fn force_update_all(&mut self) {
        // Hot path: iterate only over winds, no source_ids access for 100% cache utilization
        self.wind_table
            .winds
            .iter()
            .enumerate()
            .for_each(|(idx, winds)| {
                if winds.is_empty() {
                    self.winds_combined.cells[idx] = None;
                } else {
                    self.winds_combined.cells[idx] = Some(WindNew::new(
                        winds
                            .iter()
                            .fold(Vec2::ZERO, |accumulator, w| accumulator + w.velocity),
                    ));
                }
            });
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
                let variations: Vec<f32> = (0..self.wind_table.winds.len())
                    .map(|_| rng.random_range(-1.0..=1.0) * variation_factor)
                    .collect();
                cell_variations.insert(hash_key, variations);
            }
        }

        // Parallel update with pre-computed per-circle variations
        // SoA optimization: iterate over source_ids and winds separately
        self.wind_table
            .source_ids
            .par_iter()
            .zip(self.wind_table.winds.par_iter())
            .enumerate()
            .zip(self.winds_combined.cells.par_iter_mut())
            .for_each(|((index, (source_ids, winds)), combined_cell)| {
                // Calculate combined angle variation for this cell based on all circles affecting it
                let mut combined_variation = 0.0f32;
                let mut variation_count = 0;

                for &source_id in source_ids.iter() {
                    // Use source_id to look up noise variations
                    if let Some(variations) = cell_variations.get(&source_id) {
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

                // Sum all wind vectors (hot path: only access winds array for 100% cache utilization)
                let total_force: Vec2 = winds
                    .iter()
                    .fold(Vec2::ZERO, |accumulator, w| accumulator + w.velocity);

                // Apply angle variation if present
                let final_vector = if final_variation != 0.0 && total_force.length() > 0.0 {
                    let angle_offset = final_variation * MAX_WIND_ANGLE_DEVIATION;
                    let cos_a = angle_offset.cos();
                    let sin_a = angle_offset.sin();

                    vec2(
                        total_force.x * cos_a - total_force.y * sin_a,
                        total_force.x * sin_a + total_force.y * cos_a,
                    )
                } else {
                    total_force
                };

                *combined_cell = if final_vector.length_squared() > 0.0 {
                    Some(WindNew::new(final_vector))
                } else {
                    None
                };
            });
    }

    /******************* Grid accessors ****************************************/

    /// Calculate the center position (origin) of a cell at grid coordinates (x, y)
    pub fn get_cell_origin(&self, x: usize, y: usize) -> Option<Vec2> {
        if x >= self.params.grid_cols || y >= self.params.grid_rows {
            return None;
        }

        // Calculate cell center in grid space
        let grid_x = (x as f32 + 0.5) * self.params.cell_size.x;
        let grid_y = (y as f32 + 0.5) * self.params.cell_size.y;

        // Transform back to world coordinates
        let world_x = grid_x - self.params.bounds_size.x / 2.0;
        let world_y = -(grid_y - self.params.bounds_size.y / 2.0);

        Some(vec2(world_x, world_y))
    }

    /// Get combined wind at a position in ParticleSystem coordinates
    /// Returns None if position is out of bounds or cell has no wind
    pub fn get_wind_at_pos(&self, position: Vec2) -> Option<WindNew> {
        let Some((x, y)) = self.position_to_idx(position) else {
            return None;
        };

        if let Some(idx) = self.get_cell_index(x, y) {
            return self
                .winds_combined
                .get(idx)
                .and_then(|opt| opt.as_ref())
                .cloned();
        }

        None
    }

    /// Get the source IDs and winds at a grid position (0,0 is top left)
    /// Returns (source_ids, winds) - both slices have the same length
    pub fn get_wind_cell_data(&self, x: usize, y: usize) -> Option<(&[u64], &[WindNew])> {
        let index = self.get_cell_index(x, y)?;
        let source_ids = self.wind_table.source_ids.get(index)?;
        let winds = self.wind_table.winds.get(index)?;
        Some((source_ids.as_slice(), winds.as_slice()))
    }

    /// Clear the Wind from a cell at a grid position (0,0 is top left)
    pub fn clear_cell(&mut self, x: usize, y: usize) {
        let Some(idx) = self.get_cell_index(x, y) else {
            return;
        };
        self.wind_table.source_ids[idx].clear();
        self.wind_table.winds[idx].clear();
        self.winds_combined.cells[idx] = None;

        #[cfg(debug_assertions)]
        self.wind_table.verify_invariant(idx);
    }

    /// Take a center-origin position and convert it to a grid position index (0,0 is top left)
    fn position_to_idx(&self, pos: Vec2) -> Option<(usize, usize)> {
        let transformed = self.world_to_grid_coords(pos);
        let i = transformed.x.floor().max(0.0) as usize;
        let j = transformed.y.floor().max(0.0) as usize;

        if i < self.params.grid_cols && j < self.params.grid_rows {
            Some((i, j))
        } else {
            None // Out of bounds
        }
    }

    /// Helper method to transform world coordinates to grid coordinates (floating point)
    fn world_to_grid_coords(&self, pos: Vec2) -> Vec2 {
        let x1 = pos.x + self.params.bounds_size.x / 2.0;
        let y1 = -pos.y + self.params.bounds_size.y / 2.0;

        vec2(x1 / self.params.cell_size.x, y1 / self.params.cell_size.y)
    }

    /******************* Drawing methods ****************************************/

    /// Draw the WindField with grid, vectors, and origin
    pub fn draw(&self, draw: &Draw, scale_x: f32, scale_y: f32, div_factor: usize) {
        self.draw_origin(draw, scale_x, scale_y);
        self.draw_grid(draw, scale_x, scale_y);
        self.draw_vectors(draw, scale_x, scale_y, div_factor);
    }

    /// Draw all the Wind vectors
    fn draw_vectors(&self, draw: &Draw, scale_x: f32, scale_y: f32, div_factor: usize) {
        // Draw wind vectors from selected cells based on div_factor
        for (index, combined_wind_opt) in self.winds_combined.cells.iter().enumerate() {
            // Skip cells with no wind
            let Some(combined_wind) = combined_wind_opt else {
                continue;
            };

            if combined_wind.velocity.length() == 0.0 {
                continue;
            }

            // Calculate row and column from the flat index
            let col = index % self.params.grid_cols;
            let row = index / self.params.grid_cols;

            // Sample based on div_factor using deterministic pattern
            if !(col + row).is_multiple_of(div_factor) {
                continue;
            }

            // Get cell origin
            let Some(cell_origin) = self.get_cell_origin(col, row) else {
                continue;
            };

            // Draw wind vector from cell origin
            let vector_scale = 3.0; // Increased scale for better visibility
            let direction = combined_wind.direction();
            let strength = combined_wind.strength();
            let vector_end = cell_origin + direction * strength * vector_scale;

            draw.line()
                .start(cell_origin * vec2(scale_x, scale_y))
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
        let cols = self.params.grid_cols;
        let rows = self.params.grid_rows;
        let cell_w = self.params.cell_size.x * scale_x;
        let cell_h = self.params.cell_size.y * scale_y;

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
        for (index, combined_wind_opt) in self.winds_combined.cells.iter().enumerate() {
            // Calculate row and column from the flat index
            let col = index % self.params.grid_cols;
            let row = index / self.params.grid_cols;

            let Some(cell_origin) = self.get_cell_origin(col, row) else {
                continue;
            };

            let color: Rgba = if let Some(combined_wind) = combined_wind_opt {
                if combined_wind.strength() > 0.0 {
                    let direction = combined_wind.direction();
                    let h = direction.y.atan2(direction.x) / (2.0 * PI);
                    let s = combined_wind.strength() / 30.0;
                    Rgba::from(hsv(h, s, 1.0))
                } else {
                    rgba(0.0, 0.0, 0.0, 0.0)
                }
            } else {
                rgba(0.0, 0.0, 0.0, 0.0)
            };

            draw.rect()
                .xy(cell_origin * vec2(scale_x, scale_y))
                .w_h(
                    self.params.cell_size.x * scale_x,
                    self.params.cell_size.y * scale_y,
                )
                .stroke_color(rgba(0.3, 0.3, 0.3, 0.3))
                .stroke_weight(1.0)
                .color(color);
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
