/// src/force/field.rs
///
/// Force field for field-based forces
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
    #[allow(dead_code)]
    bounds_size: Vec2,
    #[allow(dead_code)]
    grid_cols: usize,
    #[allow(dead_code)]
    grid_rows: usize,
    #[allow(dead_code)]
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

    /// Read back GPU force field to CPU cache
    ///
    /// This is a blocking operation that waits for GPU completion.
    /// The results are cached in `gpu_force_cache` for CPU physics to use.
    ///
    /// # Arguments
    ///
    /// * `device` - WebGPU device
    /// * `queue` - WebGPU queue
    ///
    /// # Returns
    ///
    /// Reference to cached GPU forces
    pub fn read_back_gpu(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) -> &[[f32; 2]] {
        if let Some(gpu_ff) = self.gpu_force_field.as_mut() {
            self.gpu_force_cache = gpu_ff.read_back(device, queue);
        }
        &self.gpu_force_cache
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
    pub fn apply_forces_to_particle(
        &self,
        particle: &mut ParticleCore,
        mass_variation_factor: f32,
    ) {
        // Apply wind with mass variation
        self.wind_field.apply(particle, mass_variation_factor);
    }

    /// Recalculate all applicable forces in this ForceField
    pub fn recalculate_once(&mut self) {
        // Use the regular force_update_all for recalculation without variation
        self.force_update_all();
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct CellIdx {
    pub x: usize,
    pub y: usize,
}
