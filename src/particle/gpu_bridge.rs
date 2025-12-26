/// GPU Particle Bridge
///
/// This module bridges System4's domain-specific particle system with Nnpipe's
/// generic GPU particle system. It handles:
/// - Converting ParticleCore to GpuParticle format
/// - Managing force field updates
/// - Coordinating GPU compute passes
/// - Bridging between CPU emitters and GPU physics
///
/// # Phase 3 Implementation
///
/// This is a Phase 3 implementation that eliminates CPU physics and the blocking
/// read_back operation from Phase 2. All physics now runs on GPU.
///
/// # Architecture
///
/// ```text
/// System4 Domain          Bridge                  Nnpipe GPU
/// ┌─────────────┐        ┌──────────────┐        ┌─────────────┐
/// │ Voice       │───────>│ GPU Bridge   │───────>│ GPU Particle│
/// │ WindCircle  │        │              │        │ System      │
/// │ Emitter     │        │ - Convert    │        │             │
/// │ ParticleCore│        │ - Upload     │        │ - Physics   │
/// └─────────────┘        │ - Coordinate │        │ - Forces    │
///                        └──────────────┘        └─────────────┘
/// ```
use nannou::prelude::*;
use nnpipe::compute::{
    ForceCell, GpuForceField, GpuParticle, GpuParticleConfig, GpuParticleSystem,
};
use std::collections::HashMap;

use crate::forces::{collect_noise_values, collect_wind_circle_adapters};
use crate::groups::{Voice, VoiceId};
use crate::particle::ParticleCore;

/// GPU Particle Bridge
///
/// Bridges System4's particle system with Nnpipe's GPU particle system.
/// Manages force field updates, particle spawning, and GPU physics execution.
pub struct GpuParticleBridge {
    /// GPU particle system for physics simulation
    gpu_particle_system: GpuParticleSystem,

    /// GPU force field for force combination
    gpu_force_field: GpuForceField,

    /// Intermediate storage for force field conversion
    force_field_cache: Vec<ForceCell>,
}

impl GpuParticleBridge {
    /// Create a new GPU particle bridge
    ///
    /// # Arguments
    ///
    /// * `device` - WebGPU device
    /// * `config` - GPU particle configuration
    ///
    /// # Returns
    ///
    /// Result containing the bridge or an error message
    pub fn new(
        device: &wgpu::Device,
        config: GpuParticleConfig,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // Create GPU particle system
        let gpu_particle_system = GpuParticleSystem::new(device, config.clone())?;

        // Create GPU force field with matching configuration
        let force_field_config = nnpipe::compute::ForceFieldConfig {
            grid_width: config.grid_width,
            grid_height: config.grid_height,
            bounds: config.bounds,
            dt: 1.0, // default value for 1st update only
            damping: config.damping,
            max_force: config.max_force,
            noise_scale: config.noise_scale,
        };

        let gpu_force_field = GpuForceField::new(device, force_field_config)?;

        // Initialize force field cache
        let total_cells = (config.grid_width * config.grid_height) as usize;
        let force_field_cache = vec![ForceCell::default(); total_cells];

        Ok(Self {
            gpu_particle_system,
            gpu_force_field,
            force_field_cache,
        })
    }

    /// Update force field from voices
    ///
    /// Uploads wind circle sources to GPU and encodes force combination.
    /// The encoder must be submitted to the queue for the GPU work to execute.
    ///
    /// # Arguments
    ///
    /// * `queue` - WebGPU queue for uploads
    /// * `encoder` - Command encoder to record compute pass
    /// * `voices` - HashMap of voices containing wind circles
    pub fn update_forces(
        &mut self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        voices: &HashMap<VoiceId, Voice>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Collect wind circle adapters
        let adapters = collect_wind_circle_adapters(voices);
        println!(
            "    GPU Forces: {} wind circle adapters collected",
            adapters.len()
        );

        // Collect noise values
        let noise_values = collect_noise_values(voices);

        // Upload sources to GPU force field
        self.gpu_force_field
            .upload_sources(queue, &adapters, Some(&noise_values))?;

        // Debug: Print contribution count
        let contribution_count = self.gpu_force_field.contribution_count();
        println!(
            "    GPU Forces: {} contributions uploaded to GPU",
            contribution_count
        );

        // Encode force combination compute pass
        self.gpu_force_field.encode_combine(encoder);

        Ok(())
    }

    /// Copy force field from GPU force field to particle system
    ///
    /// This is needed because GpuForceField and GpuParticleSystem have separate
    /// force field buffers. We use a copy command to transfer the data.
    ///
    /// # Arguments
    ///
    /// * `encoder` - Command encoder to record buffer copy
    pub fn copy_force_field_to_particle_system(&self, encoder: &mut wgpu::CommandEncoder) {
        let src_buffer = self.gpu_force_field.get_force_buffer();
        let dst_buffer = self.gpu_particle_system.force_field_buffer();

        // Calculate buffer size
        let total_cells = self.force_field_cache.len();
        let buffer_size = (total_cells * std::mem::size_of::<ForceCell>()) as u64;

        // DEBUG: Log buffer copy details
        println!(
            "    Copying force field: {} cells, {} bytes",
            total_cells, buffer_size
        );

        // Copy force field buffer
        encoder.copy_buffer_to_buffer(src_buffer, 0, dst_buffer, 0, buffer_size);
    }

    /// Spawn particles from CPU emitters
    ///
    /// Converts ParticleCore instances to GpuParticle format and uploads to GPU.
    ///
    /// # Arguments
    ///
    /// * `queue` - WebGPU queue for uploads
    /// * `particles` - Slice of ParticleCore to spawn on GPU
    ///
    /// # Returns
    ///
    /// Number of particles actually spawned
    pub fn spawn_particles(&mut self, queue: &wgpu::Queue, particles: &[ParticleCore]) -> usize {
        if particles.is_empty() {
            return 0;
        }

        // Convert ParticleCore to GpuParticle
        let gpu_particles: Vec<GpuParticle> =
            particles.iter().map(|p| p.to_gpu_particle()).collect();

        // Upload to GPU
        self.gpu_particle_system
            .spawn_particles(queue, &gpu_particles)
    }

    /// Run full GPU physics update
    ///
    /// Encodes all compute passes needed for a single frame:
    /// 1. Force field combination (already encoded in update_forces)
    /// 2. Copy force field to particle system
    /// 3. Particle physics integration
    ///
    /// # Arguments
    ///
    /// * `queue` - WebGPU queue for uploading updated params
    /// * `encoder` - Command encoder to record compute passes
    /// * `framerate_factor` - Frame-rate independent time step (dt / 0.0167)
    pub fn encode_physics_update(
        &mut self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        framerate_factor: f32,
    ) {
        // Update physics parameters with actual framerate_factor for frame-rate independence
        let mut updated_params = *self.gpu_particle_system.params();
        updated_params.physics[0] = framerate_factor; // physics.x = dt

        self.gpu_particle_system
            .update_physics_params(queue, &updated_params);

        // Copy force field to particle system
        self.copy_force_field_to_particle_system(encoder);

        // Encode particle physics simulation
        self.gpu_particle_system.encode_simulate(encoder);
    }

    /// Encode render population (Phase 4)
    ///
    /// This compute pass populates the render vertex buffer from particle state,
    /// filtering out dead particles and creating a dense array for rendering.
    ///
    /// # Arguments
    ///
    /// * `encoder` - Command encoder to record compute passes
    ///
    /// # Call Order
    ///
    /// This should be called after `encode_physics_update()` but before rendering.
    pub fn encode_render_populate(&self, encoder: &mut wgpu::CommandEncoder) {
        use std::time::Instant;

        // Clear alive count before render populate
        let clear_start = Instant::now();
        self.gpu_particle_system.encode_clear_alive_count(encoder);
        println!("      clear_alive_count: {:?}", clear_start.elapsed());

        // Encode render population compute pass
        let populate_start = Instant::now();
        self.gpu_particle_system.encode_render_populate(encoder);
        println!("      render_populate_pass: {:?}", populate_start.elapsed());
    }

    /// Get reference to render vertex buffer (Phase 4)
    ///
    /// This buffer contains the GPU-populated render vertices ready for rendering.
    /// Use this with ParticleRenderer::encode_from_buffer().
    ///
    /// # Returns
    ///
    /// Reference to the render vertex buffer
    pub fn render_vertex_buffer(&self) -> &wgpu::Buffer {
        self.gpu_particle_system.render_vertex_buffer()
    }

    /// Get reference to alive count buffer (Phase 4)
    ///
    /// This buffer contains the GPU-computed alive particle count.
    ///
    /// # Returns
    ///
    /// Reference to the alive count buffer
    pub fn alive_count_buffer(&self) -> &wgpu::Buffer {
        self.gpu_particle_system.alive_count_buffer()
    }

    /// Read back GPU-computed alive count (Phase 4)
    ///
    /// This is a blocking operation. Use sparingly for debugging or UI.
    ///
    /// # Arguments
    ///
    /// * `device` - WebGPU device
    /// * `queue` - WebGPU queue
    ///
    /// # Returns
    ///
    /// Number of alive particles
    pub fn read_back_gpu_alive_count(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> u32 {
        self.gpu_particle_system
            .read_back_alive_count(device, queue)
    }

    /// Swap particle buffers after frame completion
    ///
    /// Call this after submitting the command encoder to swap the double-buffered
    /// particle state for the next frame.
    pub fn end_frame(&mut self) {
        self.gpu_particle_system.swap_buffers();
    }

    /// Get alive particle count
    pub fn alive_count(&self) -> usize {
        self.gpu_particle_system.alive_count()
    }

    /// Get maximum particle capacity
    pub fn max_particles(&self) -> usize {
        self.gpu_particle_system.max_particles()
    }

    /// Get reference to current particle buffer
    ///
    /// This buffer contains the current frame's particle state and can be
    /// used for rendering or readback.
    pub fn current_particle_buffer(&self) -> &wgpu::Buffer {
        self.gpu_particle_system.current_particle_buffer()
    }

    /// Clear all particles
    pub fn clear(&mut self, queue: &wgpu::Queue) {
        self.gpu_particle_system.clear(queue);
    }

    /// Debug readback: Read first N particle positions from GPU
    ///
    /// This is a BLOCKING operation that reads back GPU memory to CPU.
    /// Use sparingly for debugging only!
    ///
    /// # Arguments
    ///
    /// * `device` - WebGPU device
    /// * `queue` - WebGPU queue
    /// * `count` - Number of particles to read (default: 10)
    ///
    /// # Returns
    ///
    /// Vector of (pos_x, pos_y, vel_x, vel_y) tuples
    pub fn debug_read_particle_positions(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        count: usize,
    ) -> Vec<(f32, f32, f32, f32)> {
        use nannou::wgpu;

        let count = count.min(self.max_particles());
        let buffer_size = (std::mem::size_of::<GpuParticle>() * count) as u64;

        // Create staging buffer for readback
        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Debug Particle Readback"),
            size: buffer_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Copy from GPU particle buffer to staging
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Debug Readback Encoder"),
        });

        encoder.copy_buffer_to_buffer(
            self.current_particle_buffer(),
            0,
            &staging_buffer,
            0,
            buffer_size,
        );

        queue.submit(Some(encoder.finish()));

        // Map and read (BLOCKING!)
        let buffer_slice = staging_buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
        device.poll(wgpu::Maintain::Wait);
        rx.recv().unwrap().unwrap();

        let data = buffer_slice.get_mapped_range();
        let particles: &[GpuParticle] = bytemuck::cast_slice(&data);

        let positions: Vec<(f32, f32, f32, f32)> = particles
            .iter()
            .take(count)
            .map(|p| (p.position[0], p.position[1], p.velocity[0], p.velocity[1]))
            .collect();

        drop(data);
        staging_buffer.unmap();

        positions
    }
}
