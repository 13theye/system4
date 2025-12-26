/// GPU adapter for WindCircle force sources
///
/// This module provides a bridge between System4's WindCircle and
/// nnpipe's GPU force field system. It converts WindCircle parameters
/// into force vectors for affected grid cells.
use nannou::prelude::*;
use nnpipe::compute::{ForceFieldParams, ForceSource};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use super::{WindCircle, WindCircleParams};
use crate::groups::VoiceId;

/// Create a unique hash from voice_id and circle_id for noise parameter indexing
fn hash_voice_circle(voice_id: VoiceId, circle_id: usize) -> u64 {
    let mut hasher = DefaultHasher::new();
    (voice_id, circle_id).hash(&mut hasher);
    hasher.finish()
}

/// GPU adapter for WindCircle
///
/// This struct wraps a WindCircle and implements the ForceSource trait,
/// allowing it to be used with the GPU force field system. It computes
/// which grid cells are affected by the wind circle's donut-shaped region
/// and calculates the force vectors for each cell.
///
/// # Force Calculation
///
/// The WindCircle creates a tangential force that can be biased toward
/// radial direction. The force calculation matches the CPU implementation:
/// 1. Only cells within [inner_radius, outer_radius] are affected
/// 2. Force direction is tangent to the circle (perpendicular to radius)
/// 3. `gravity` parameter blends tangential with radial direction
/// 4. Force magnitude is determined by the `force` parameter
pub struct GpuWindCircleAdapter<'a> {
    circle: &'a WindCircle,
    voice_id: VoiceId,
}

impl<'a> GpuWindCircleAdapter<'a> {
    /// Create a new GPU adapter for a WindCircle
    ///
    /// # Arguments
    ///
    /// * `circle` - Reference to the WindCircle to adapt
    /// * `voice_id` - Voice ID for unique source identification
    pub fn new(circle: &'a WindCircle, voice_id: VoiceId) -> Self {
        Self { circle, voice_id }
    }

    /// Convert world position to grid coordinates
    ///
    /// IMPORTANT: This must match the coordinate transform in particle_physics.wgsl!
    /// The shader uses: grid_pos = (world_pos - bounds.min) * (1/cell_size)
    fn world_to_grid_coords(params: &ForceFieldParams, pos: Vec2) -> Vec2 {
        // Match the GPU shader's coordinate transform exactly:
        // grid_pos.x = (pos.x - bounds.x) * cell_size.z  // cell_size.z = 1/cell_width
        // grid_pos.y = (pos.y - bounds.y) * cell_size.w  // cell_size.w = 1/cell_height
        //
        // Where bounds.x = min_x (-width/2) and bounds.y = min_y (-height/2)
        vec2(
            (pos.x - params.bounds[0]) * params.cell_size[2],
            (pos.y - params.bounds[1]) * params.cell_size[3]
        )
    }

    /// Get cell center position in world coordinates
    ///
    /// Inverse of world_to_grid_coords. Must be consistent with the shader.
    fn get_cell_center(params: &ForceFieldParams, col: usize, row: usize) -> Vec2 {
        // Inverse of the grid coordinate transform:
        // world_pos.x = grid_x * cell_width + bounds.x
        // world_pos.y = grid_y * cell_height + bounds.y
        let grid_x = col as f32 + 0.5;  // Center of cell
        let grid_y = row as f32 + 0.5;

        let world_x = grid_x * params.cell_size[0] + params.bounds[0];
        let world_y = grid_y * params.cell_size[1] + params.bounds[1];

        vec2(world_x, world_y)
    }

    /// Calculate wind force for a cell using WindCircle logic
    ///
    /// This replicates the logic from WindCircle::calculate_wind_for_cell
    fn calculate_wind_for_cell(
        params: &ForceFieldParams,
        col: usize,
        row: usize,
        circle_params: &WindCircleParams,
    ) -> Option<Vec2> {
        // Get cell center in world coordinates
        let cell_origin = Self::get_cell_center(params, col, row);

        let distance_to_center = (cell_origin - circle_params.center).length();
        let inner_radius = circle_params.inner_radius;
        let outer_radius = circle_params.outer_radius;

        // Check if cell is within the donut-shaped region
        if distance_to_center >= inner_radius && distance_to_center <= outer_radius {
            // Wind generation logic specific to circular fields
            let radius_vector = cell_origin - circle_params.center;
            let radius_dir = radius_vector.normalize();
            let tangent_dir = vec2(radius_dir.y, -radius_dir.x); // tangential, 90 deg CCW from radial

            // Rotate the tangent vector by bias * 90 degrees
            let angle = circle_params.gravity * -std::f32::consts::FRAC_PI_2; // PI/2 = 90 deg

            let sin_a = angle.sin();
            let cos_a = angle.cos();

            // Rotate tangent_dir by 'angle' to blend with radial direction
            let blended_direction = vec2(
                tangent_dir.x * cos_a - tangent_dir.y * sin_a,
                tangent_dir.x * sin_a + tangent_dir.y * cos_a,
            );

            // Apply force magnitude
            Some(blended_direction * circle_params.force)
        } else {
            None
        }
    }
}

impl<'a> ForceSource for GpuWindCircleAdapter<'a> {
    fn compute_affected_cells(&self, params: &ForceFieldParams) -> Vec<(u32, u32, [f32; 2])> {
        let mut cells = Vec::new();

        let circle_params = self.circle.params();

        // Calculate bounding box in grid coordinates
        let outer_radius = circle_params.outer_radius + circle_params.inner_radius / 2.0;
        let center_grid = Self::world_to_grid_coords(params, circle_params.center);

        // Calculate radius in grid cells (use max of x/y cell size for conservative bound)
        let radius_cells_x = outer_radius * params.cell_size[2];
        let radius_cells_y = outer_radius * params.cell_size[3];

        let min_col = (center_grid.x - radius_cells_x).floor().max(0.0) as usize;
        let max_col = (center_grid.x + radius_cells_x)
            .ceil()
            .min(params.grid_width() as f32) as usize;
        let min_row = (center_grid.y - radius_cells_y).floor().max(0.0) as usize;
        let max_row = (center_grid.y + radius_cells_y)
            .ceil()
            .min(params.grid_height() as f32) as usize;

        // Iterate over cells in bounding box
        for col in min_col..max_col {
            for row in min_row..max_row {
                if let Some(wind_velocity) =
                    Self::calculate_wind_for_cell(params, col, row, circle_params)
                {
                    cells.push((col as u32, row as u32, [wind_velocity.x, wind_velocity.y]));
                }
            }
        }

        cells
    }

    fn source_id(&self) -> u64 {
        hash_voice_circle(self.voice_id, self.circle.id)
    }

    fn noise_factor(&self) -> f32 {
        self.circle.params().noise
    }
}

/// Collect all WindCircles from voices and create GPU adapters
///
/// This is a convenience function for preparing WindCircles for GPU upload.
///
/// # Arguments
///
/// * `voices` - HashMap of voices containing WindCircles
///
/// # Returns
///
/// Vector of GpuWindCircleAdapter instances
pub fn collect_wind_circle_adapters(
    voices: &std::collections::HashMap<VoiceId, crate::groups::Voice>,
) -> Vec<GpuWindCircleAdapter<'_>> {
    let total_circles: usize = voices.values().map(|v| v.wind_circles.len()).sum();

    let mut adapters = Vec::with_capacity(total_circles);

    for voice in voices.values() {
        for circle in voice.wind_circles.values() {
            adapters.push(GpuWindCircleAdapter::new(circle, voice.id));
        }
    }

    adapters
}

/// Extract noise values from all WindCircles
///
/// Creates a HashMap mapping source_id (voice+circle hash) to noise factor.
/// This is used by the GPU force field to apply per-source noise variations.
///
/// # Arguments
///
/// * `voices` - HashMap of voices containing WindCircles
///
/// # Returns
///
/// HashMap mapping source_id to noise factor
pub fn collect_noise_values(
    voices: &std::collections::HashMap<VoiceId, crate::groups::Voice>,
) -> std::collections::HashMap<u64, f32> {
    let mut noise_values = std::collections::HashMap::new();

    for voice in voices.values() {
        for circle in voice.wind_circles.values() {
            let source_id = hash_voice_circle(voice.id, circle.id);
            noise_values.insert(source_id, circle.params().noise);
        }
    }

    noise_values
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_adapter_force_calculation() {
        // Create a test WindCircle
        let circle = WindCircle::new(
            0,
            VoiceId::Voice0,
            vec2(0.0, 0.0), // center
            100.0,          // radius
            50.0,           // width (inner radius)
            10.0,           // strength
            0.0,            // center_bias (purely tangential)
            0.1,            // noise
        );

        // Create adapter
        let adapter = GpuWindCircleAdapter::new(&circle, VoiceId::Voice0);

        // Create test force field params
        let params = ForceFieldParams::new(
            64,                             // grid_width
            64,                             // grid_height
            [-320.0, -240.0, 320.0, 240.0], // bounds (640x480 centered)
            0.016,                          // dt
            0.98,                           // damping
            100.0,                          // max_force
            0.01,                           // noise_scale
        );

        // Compute affected cells
        let cells = adapter.compute_affected_cells(&params);

        // Should have some affected cells
        assert!(
            !cells.is_empty(),
            "WindCircle should affect at least one cell"
        );

        // All cells should have non-zero force
        for (_x, _y, force) in &cells {
            let mag = (force[0] * force[0] + force[1] * force[1]).sqrt();
            assert!(mag > 0.0, "Force magnitude should be non-zero");
        }

        // Verify source_id is deterministic
        let id1 = adapter.source_id();
        let id2 = adapter.source_id();
        assert_eq!(id1, id2, "Source ID should be stable");

        // Verify noise factor
        assert_eq!(adapter.noise_factor(), 0.1);
    }

    #[test]
    fn test_collect_adapters() {
        use std::collections::HashMap;

        let mut voices = HashMap::new();

        // Create a test voice with wind circles
        let mut voice = crate::groups::Voice::new_with_id(VoiceId::Voice0);
        voice.wind_circles.insert(
            0,
            WindCircle::new(
                0,
                VoiceId::Voice0,
                vec2(0.0, 0.0),
                100.0,
                50.0,
                10.0,
                0.0,
                0.1,
            ),
        );
        voice.wind_circles.insert(
            1,
            WindCircle::new(
                1,
                VoiceId::Voice0,
                vec2(100.0, 100.0),
                80.0,
                40.0,
                8.0,
                0.5,
                0.2,
            ),
        );

        voices.insert(VoiceId::Voice0, voice);

        // Collect adapters
        let adapters = collect_wind_circle_adapters(&voices);
        assert_eq!(adapters.len(), 2, "Should collect 2 wind circles");

        // Collect noise values
        let noise_values = collect_noise_values(&voices);
        assert_eq!(noise_values.len(), 2, "Should collect 2 noise values");
    }
}
