//! src/force/field.rs
//!
//! Force field system supporting both CPU and GPU force computation
//!
//! The `ForceFields` struct manages a hybrid force field that can compute forces
//! either on the CPU (using `WindField`) or GPU (using `GpuForceField`). In Phase 2,
//! GPU computes forces but CPU applies them during physics integration.
//!
//! # Architecture
//!
//! - CPU mode: `WindField` computes and stores forces in `winds_combined`
//! - GPU mode: `GpuForceField` computes forces, `read_back_gpu()` transfers to `gpu_force_cache`
//! - Physics: `apply_forces_to_particle()` routes to CPU or GPU path based on `use_gpu` flag
//!
//! # Coordinate Systems
//!
//! - World space: Center origin (0,0), +X right, +Y up
//! - Grid space: Top-left origin (0,0), +X right, +Y down
//! - Transformations handled by `position_to_grid_idx()` and GPU adapter
//!
//! # Example
//!
//! ```ignore
//! let mut forces = ForceFields::new(pt2(0.0, 0.0), vec2(800.0, 600.0), 64, 48);
//! forces.init_gpu(device, true)?; // Enable GPU mode
//! forces.update_gpu(voices, queue, encoder)?;
//! forces.read_back_gpu(device, queue); // Blocking transfer to CPU
//! forces.apply_forces_to_particle(&mut particle, 0.0); // Uses GPU forces
//! ```
use nannou::prelude::*;
use nnpipe::compute::{ForceFieldConfig, ForceFieldParams, GpuForceField};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use crate::{
    forces::WindField,
    groups::{Voice, VoiceId},
    particle::ParticleCore,
};

/// Inertial resistance coefficient for wind force application
/// Higher values = particles with momentum resist changes more strongly
/// Matches Wind::apply() coefficient
const INERTIA_COEFFICIENT: f32 = 0.1;

/// Create a unique hash from voice_id and circle_id for noise parameter indexing
fn hash_voice_circle(voice_id: VoiceId, circle_id: usize) -> u64 {
    let mut hasher = DefaultHasher::new();
    (voice_id, circle_id).hash(&mut hasher);
    hasher.finish()
}

/// The ForceField tracks the forces that are acting on the particles.
/// It provides a coordinate space to align forces to screen locations.
pub struct ForceFields {
    // CPU force field
    pub wind_field: WindField,

    // GPU force field (optional)
    pub gpu_force_field: Option<GpuForceField>,

    // Flag to enable GPU force field computation
    pub use_gpu: bool,

    // Cache for GPU-computed forces (populated by read_back)
    gpu_force_cache: Vec<[f32; 2]>,

    // Origin in the World Coordinate Space
    // Kept for future use
    #[allow(dead_code)]
    origin: Vec2,
    bounds_size: Vec2,
    grid_cols: usize,
    grid_rows: usize,
    cell_size: Vec2,
}

impl ForceFields {
    pub fn new(origin: Vec2, bounds_size: Vec2, grid_cols: usize, grid_rows: usize) -> Self {
        let cell_size = Vec2::new(
            bounds_size.x / grid_cols as f32,
            bounds_size.y / grid_rows as f32,
        );

        Self {
            wind_field: WindField::new(origin, bounds_size, grid_cols, grid_rows),
            gpu_force_field: None,
            use_gpu: false,
            gpu_force_cache: Vec::new(),
            origin,
            bounds_size,
            grid_cols,
            grid_rows,
            cell_size,
        }
    }

    /// Initialize GPU force field
    ///
    /// Creates a GPU force field with the same dimensions as the CPU wind field.
    /// Call this after creating the ForceFields and having access to the wgpu device.
    ///
    /// # Arguments
    ///
    /// * `device` - WebGPU device for GPU resource creation
    /// * `enable` - Whether to enable GPU computation (default false for backward compatibility)
    ///
    /// # Errors
    ///
    /// Returns `Err` if:
    /// - GPU device does not support required compute shader features
    /// - Out of GPU memory when allocating force field buffers
    /// - Shader compilation fails (should not happen with validated shaders)
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut forces = ForceFields::new(origin, bounds, 64, 48);
    /// forces.init_gpu(device, true)?; // Enable GPU immediately
    /// ```
    pub fn init_gpu(
        &mut self,
        device: &wgpu::Device,
        enable: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let config = ForceFieldConfig {
            grid_width: self.grid_cols as u32,
            grid_height: self.grid_rows as u32,
            bounds: [
                -self.bounds_size.x / 2.0,
                -self.bounds_size.y / 2.0,
                self.bounds_size.x / 2.0,
                self.bounds_size.y / 2.0,
            ],
            dt: 0.016, // 60 FPS
            damping: 0.98,
            max_force: 100.0,
            noise_scale: 0.01,
            inertia_coefficient: 0.1, // Match CPU momentum-based inertial resistance
        };

        self.gpu_force_field = Some(GpuForceField::new(device, config)?);
        self.use_gpu = enable;

        // Initialize cache with zeros
        let total_cells = self.grid_cols * self.grid_rows;
        self.gpu_force_cache = vec![[0.0, 0.0]; total_cells];

        Ok(())
    }

    /// Enable or disable GPU force field computation
    pub fn set_use_gpu(&mut self, use_gpu: bool) {
        if self.gpu_force_field.is_some() {
            self.use_gpu = use_gpu;
        }
    }

    /// Get reference to GPU force field
    pub fn gpu_force_field(&self) -> Option<&GpuForceField> {
        self.gpu_force_field.as_ref()
    }

    /// Get mutable reference to GPU force field
    pub fn gpu_force_field_mut(&mut self) -> Option<&mut GpuForceField> {
        self.gpu_force_field.as_mut()
    }

    /// Get GPU force field parameters
    pub fn gpu_params(&self) -> Option<&ForceFieldParams> {
        self.gpu_force_field.as_ref().map(|ff| ff.params())
    }

    /// Update ForceField with all Voices' WindCircles with per-circle angle variations
    ///
    /// This method updates BOTH CPU and GPU force fields. The GPU update is encoded
    /// but not executed - you must submit the command buffer separately.
    pub fn update(
        &mut self,
        voices: &mut HashMap<VoiceId, Voice>,
        rng: &mut rand::rngs::ThreadRng,
    ) {
        let mut circle_noise_values: HashMap<u64, f32> = HashMap::new();

        // Update each circle and collect per-circle noise values using hash keys
        for voice in voices.values_mut() {
            // Update the wind circle meta-force
            for circle in voice.wind_circles.values_mut() {
                circle.update(&mut self.wind_field);
                let hash_key = hash_voice_circle(voice.id, circle.id);
                circle_noise_values.insert(hash_key, circle.params().noise);
            }
        }

        // Update CPU wind field
        self.wind_field
            .par_update_all_combined_cells(rng, &circle_noise_values);
    }

    /// Update GPU force field
    ///
    /// Uploads wind circle sources to GPU and encodes the combination compute pass.
    /// The encoder must be submitted to the queue for the GPU work to execute.
    ///
    /// # Arguments
    ///
    /// * `voices` - HashMap of voices containing WindCircles
    /// * `queue` - WebGPU queue for uploading data
    /// * `encoder` - Command encoder to record compute pass
    ///
    /// # Returns
    ///
    /// Number of force contributions uploaded to GPU
    pub fn update_gpu(
        &mut self,
        voices: &HashMap<VoiceId, Voice>,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        if !self.use_gpu {
            return Ok(0);
        }

        let Some(gpu_ff) = self.gpu_force_field.as_mut() else {
            return Ok(0);
        };

        // Collect wind circle adapters
        let adapters = crate::forces::collect_wind_circle_adapters(voices);

        // Collect noise values
        let noise_values = crate::forces::collect_noise_values(voices);

        // Upload sources to GPU
        gpu_ff.upload_sources(queue, &adapters, Some(&noise_values))?;

        // Encode compute pass
        gpu_ff.encode_combine(encoder);

        Ok(adapters.len())
    }

    /// Get cached GPU forces
    ///
    /// Returns the most recently read-back GPU force field.
    /// Will be empty if read_back_gpu() has not been called.
    pub fn gpu_force_cache(&self) -> &[[f32; 2]] {
        &self.gpu_force_cache
    }

    /// Update all Winds in this ForceField
    pub fn force_update_all(&mut self) {
        // Use a dummy RNG and empty variations for compatibility
        let mut dummy_rng = rand::rng();
        let empty_variations = HashMap::new();
        self.wind_field
            .par_update_all_combined_cells(&mut dummy_rng, &empty_variations);
    }

    /// Apply all applicable forces to a particle with mass variation factor
    /// OPTIMIZED: Now works with ParticleCore for better cache locality
    ///
    /// If GPU force field is enabled, reads from cached GPU forces.
    /// Otherwise, uses CPU wind field.
    pub fn apply_forces_to_particle(
        &self,
        particle: &mut ParticleCore,
        mass_variation_factor: f32,
    ) {
        if self.use_gpu {
            // Use GPU-computed forces
            self.apply_gpu_force_to_particle(particle, mass_variation_factor);
        } else {
            // Use CPU wind field
            self.wind_field.apply(particle, mass_variation_factor);
        }
    }

    /// Apply GPU-computed force to a particle
    ///
    /// Reads force from the GPU force cache based on particle position and applies
    /// it using the same physics model as `Wind::apply()`.
    ///
    /// # Physics Model
    ///
    /// The GPU force field stores combined wind velocities (target velocities for particles).
    /// The force applied is proportional to the difference between the target velocity and
    /// the particle's current velocity, scaled by inertial resistance.
    ///
    /// Force calculation:
    /// 1. `diff = target_velocity - particle.velocity`
    /// 2. `inertia_factor = 1.0 / (1.0 + momentum * 0.1)` (higher momentum = more resistance)
    /// 3. `force = diff * inertia_factor`
    /// 4. `acceleration += force / effective_mass`
    ///
    /// # Arguments
    ///
    /// * `particle` - Mutable reference to particle to apply force to
    /// * `mass_variation_factor` - Random variation in `[-amount, +amount]` to vary effective mass
    ///
    /// # Panics
    ///
    /// Does not panic. Returns early if particle position is out of bounds.
    //#[inline] // removed Inline until benchmarking proves benefits
    fn apply_gpu_force_to_particle(&self, particle: &mut ParticleCore, mass_variation_factor: f32) {
        // Get force at particle position
        let Some(force_vec) = self.get_gpu_force_at_pos(particle.position) else {
            return; // Out of bounds or no force
        };

        // Activate the particle
        particle.activate();

        // Apply force using the same physics as Wind::apply()
        // The GPU force field stores combined wind velocities, so we use the same logic

        // Calculate the difference between force's target velocity and particle's current velocity
        let diff = force_vec - particle.velocity;

        // Calculate effective mass with variation factor
        let effective_mass = particle.mass * (1.0 + mass_variation_factor);

        // Calculate inertial resistance based on current momentum using effective mass
        let current_speed = particle.velocity.length();
        let momentum_magnitude = effective_mass * current_speed;

        // Inertial resistance: particles with higher momentum resist changes more
        let inertia_factor = 1.0 / (1.0 + momentum_magnitude * INERTIA_COEFFICIENT);

        // Apply the force with inertial resistance using effective mass
        let force = diff * inertia_factor;
        particle.acceleration += force / effective_mass;
    }

    /// Get GPU-computed force at a world position
    ///
    /// Returns None if position is out of bounds or if there's no valid force.
    fn get_gpu_force_at_pos(&self, position: Vec2) -> Option<Vec2> {
        let (x, y) = self.position_to_grid_idx(position)?;
        let index = y * self.grid_cols + x;

        self.gpu_force_cache
            .get(index)
            .copied()
            .map(|[fx, fy]| vec2(fx, fy))
    }

    /// Convert world position to grid indices
    ///
    /// This matches the coordinate transformation in WindField::position_to_idx()
    ///
    /// Transforms from world coordinates (center origin, +Y up) to grid indices
    /// (top-left origin, +Y down).
    ///
    /// # Coordinate Transformation
    ///
    /// 1. Translate from center origin to top-left: `x' = x + bounds_width/2`
    /// 2. Flip Y axis: `y' = -y + bounds_height/2`
    /// 3. Convert to grid indices: `i = floor(x' / cell_width)`, `j = floor(y' / cell_height)`
    ///
    /// # Returns
    ///
    /// - `Some((i, j))` if position is within bounds `[0, grid_cols) x [0, grid_rows)`
    /// - `None` if position is out of bounds
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Particle at center of screen (world origin)
    /// let idx = forces.position_to_grid_idx(vec2(0.0, 0.0));
    /// assert_eq!(idx, Some((grid_cols/2, grid_rows/2)));
    /// ```
    fn position_to_grid_idx(&self, pos: Vec2) -> Option<(usize, usize)> {
        // Transform from world coordinates (center origin) to grid coordinates (top-left origin)
        let x1 = pos.x + self.bounds_size.x / 2.0;
        let y1 = -pos.y + self.bounds_size.y / 2.0;

        // Early return for negative coordinates (OOB for left/top origin)
        if x1 < 0.0 || y1 < 0.0 {
            return None;
        }

        // Convert to grid indices
        let i = (x1 / self.cell_size.x).floor() as usize;
        let j = (y1 / self.cell_size.y).floor() as usize;

        if i < self.grid_cols && j < self.grid_rows {
            Some((i, j))
        } else {
            None // Out of bounds
        }
    }

    /// Recalculate all applicable forces in this ForceField
    pub fn recalculate_once(&mut self) {
        // Use the regular force_update_all for recalculation without variation
        self.force_update_all();
    }
}

#[derive(Debug, PartialEq)]
pub struct CellIdx {
    pub x: usize,
    pub y: usize,
}
