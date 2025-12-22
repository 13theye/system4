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
use nnpipe::compute::{ForceCell, GpuForceField, GpuParticle, GpuParticleConfig, GpuParticleSystem};
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
            dt: config.dt,
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

        // Collect noise values
        let noise_values = collect_noise_values(voices);

        // Upload sources to GPU force field
        self.gpu_force_field
            .upload_sources(queue, &adapters, Some(&noise_values))?;

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
        let gpu_particles: Vec<GpuParticle> = particles
            .iter()
            .map(|p| p.to_gpu_particle())
            .collect();

        // Upload to GPU
        self.gpu_particle_system.spawn_particles(queue, &gpu_particles)
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
    /// * `encoder` - Command encoder to record compute passes
    pub fn encode_physics_update(&mut self, encoder: &mut wgpu::CommandEncoder) {
        // Copy force field to particle system
        self.copy_force_field_to_particle_system(encoder);

        // Encode particle physics simulation
        self.gpu_particle_system.encode_simulate(encoder);
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

    /// Read back GPU particle state to CPU (blocking operation)
    ///
    /// This method copies the current particle buffer from GPU to CPU memory.
    /// It is a blocking operation that stalls the CPU thread until the GPU
    /// completes all pending work and the buffer is mapped.
    ///
    /// # Performance Note
    ///
    /// This is necessary in Phase 3 because we still use CPU for rendering
    /// (GPU render population comes in Phase 4). The blocking readback is
    /// the performance bottleneck eliminated in Phase 4.
    ///
    /// # Arguments
    ///
    /// * `device` - WebGPU device
    /// * `queue` - WebGPU queue
    ///
    /// # Returns
    ///
    /// Vector of GPU particles in their current state
    pub fn read_back_particles(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Vec<GpuParticle> {
        let max_particles = self.max_particles();
        let particle_buffer_size = (max_particles * std::mem::size_of::<GpuParticle>()) as u64;

        // Create staging buffer for readback
        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Particle Readback Staging"),
            size: particle_buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        // Copy GPU buffer to staging
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Particle Readback Encoder"),
        });
        encoder.copy_buffer_to_buffer(
            self.current_particle_buffer(),
            0,
            &staging_buffer,
            0,
            particle_buffer_size,
        );
        queue.submit(Some(encoder.finish()));

        // Map and read (blocking operation)
        let buffer_slice = staging_buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).unwrap();
        });
        device.poll(wgpu::Maintain::Wait);
        receiver.recv().unwrap().unwrap();

        // Read particle data
        let data = buffer_slice.get_mapped_range();
        let particles: Vec<GpuParticle> = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        staging_buffer.unmap();

        particles
    }
}
