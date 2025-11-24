// src/forces/wind_circle.rs
//
// WindCircle implementation for wind_new.rs

use super::wind_new::{Wind, WindField};
use crate::groups::VoiceId;
use nannou::prelude::*;

/// Type alias for cell indices
#[derive(Debug, Clone, Copy)]
pub struct CellIdx {
    pub x: usize,
    pub y: usize,
}

/// A circular wind force that affects particles within a donut-shaped region
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
        for cell_idx in self.cell_idxs.iter() {
            field.remove_circle_wind(cell_idx.x, cell_idx.y, self.id, self.parent_voice);
        }
        self.cell_idxs.clear();
    }

    /// Add the circle's wind to the field, return the cells that were affected.
    /// The total numerical force for each cell is calculated once per frame in a later step.
    pub fn apply_to_field(&self, field: &mut WindField) -> Vec<CellIdx> {
        // Calculate bounding box using direct parameter access
        let (min_col, max_col, min_row, max_row) = self.calculate_bounding_box(field, &self.params);

        let mut affected_cells = Vec::new();

        // Update cells serially
        for col in min_col..max_col {
            for row in min_row..max_row {
                let Some(wind) = self.calculate_wind_for_cell(field, col, row, &self.params) else {
                    continue;
                };
                field.add_circle_wind(col, row, self.id, self.parent_voice, wind);
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

        let cell_size = field.params.cell_size;
        let radius_in_cells_x = outer_radius / cell_size.x;
        let radius_in_cells_y = outer_radius / cell_size.y;

        let min_col = (center_pos_transformed.x - radius_in_cells_x)
            .floor()
            .max(0.0) as usize;
        let max_col = (center_pos_transformed.x + radius_in_cells_x)
            .ceil()
            .min(field.params.grid_cols as f32) as usize;
        let min_row = (center_pos_transformed.y - radius_in_cells_y)
            .floor()
            .max(0.0) as usize;
        let max_row = (center_pos_transformed.y + radius_in_cells_y)
            .ceil()
            .min(field.params.grid_rows as f32) as usize;

        (min_col, max_col, min_row, max_row)
    }

    /// Calculate the Wind force within a cell. Returns the wind force if this cell contains one.
    fn calculate_wind_for_cell(
        &self,
        field: &WindField,
        col: usize,
        row: usize,
        params: &WindCircleParams,
    ) -> Option<Wind> {
        // Get cell origin
        let cell_origin = field.get_cell_origin(col, row)?;

        let distance_to_center = (cell_origin - params.center).length();
        let inner_radius = params.inner_radius;
        let outer_radius = params.outer_radius;

        if distance_to_center >= inner_radius && distance_to_center <= outer_radius {
            // Wind generation logic specific to circular fields
            let radius_vector = cell_origin - params.center;
            let radius_dir = radius_vector.normalize();
            let tangent_dir = vec2(radius_dir.y, -radius_dir.x); // tangential, 90 deg CCW from radial

            // Rotate the tangent vector by bias * 90 degrees
            let angle = params.gravity * -std::f32::consts::FRAC_PI_2; // PI/2 = 90 deg

            let sin_a = angle.sin();
            let cos_a = angle.cos();

            // Rotate tangent_dir by 'angle'
            let blended_direction = vec2(
                tangent_dir.x * cos_a - tangent_dir.y * sin_a,
                tangent_dir.x * sin_a + tangent_dir.y * cos_a,
            );

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
/// - Angle variation: Amount of random angle variation (0.0-1.0, where 1.0 = �90� deviation)
/// - Dirty: Flag to indicate that one or more parameters have changed so that WindField will recalculate
#[derive(Clone, Debug)]
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
    /// 0.0-1.0 factor for random angle variation, where 1.0 = full �90� deviation
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
