/// GPU Particle Bridge
///
/// This module bridges System4's domain-specific particle system with Nnpipe's
/// generic GPU particle system. It handles:
/// - Converting ParticleCore to GpuParticle format
/// - Managing force field updates
/// - Coordinating GPU compute passes
/// - Bridging between CPU emitters and GPU physics
///
/// ```
use nannou::prelude::*;
use nnpipe::compute::{
    GpuParticle, GpuParticleConfig, GpuParticleSystem, GpuWindCircle, ParticleSpawnRequest,
};
use std::collections::HashMap;

use crate::groups::{Voice, VoiceId};

/// GPU Particle Bridge
///
/// Bridges System4's particle system with Nnpipe's GPU particle system.
/// Manages wind circle updates, particle spawning, and GPU physics execution.
///
/// # Architectural Change (2025-12-26)
///
/// This bridge now uses per-particle force computation instead of grid-based
/// force fields. Wind circles are uploaded directly to GPU and particles
/// compute forces in the shader, achieving pixel-perfect precision.
pub struct GpuParticleBridge {
    /// GPU particle system for physics simulation
    gpu_particle_system: GpuParticleSystem,

    /// Counter for alive count readback (every 10 frames)
    alive_count_readback_counter: u32,

    /// Cached GPU-computed alive count
    gpu_alive_count: u32,
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
        // Create GPU particle system with per-particle force computation
        let gpu_particle_system = GpuParticleSystem::new(device, config.clone())?;

        Ok(Self {
            gpu_particle_system,
            alive_count_readback_counter: 0,
            gpu_alive_count: 0,
        })
    }

    /// Upload wind circles directly to GPU for per-particle force computation
    ///
    /// This replaces the old grid-based force field system.
    /// Wind circles are uploaded as source parameters and particles
    /// compute forces directly in the shader.
    ///
    /// # Arguments
    ///
    /// * `queue` - WebGPU queue for uploads
    /// * `voices` - HashMap of voices containing wind circles
    pub fn update_forces(
        &mut self,
        queue: &wgpu::Queue,
        _encoder: &mut wgpu::CommandEncoder, // Encoder no longer needed (no compute pass)
        voices: &HashMap<VoiceId, Voice>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Convert all wind circles to GPU format
        let mut gpu_wind_circles = Vec::new();

        for voice in voices.values() {
            for circle in voice.wind_circles.values() {
                let params = circle.params();

                // Convert to GpuWindCircle format
                let gpu_circle = GpuWindCircle::new(
                    [params.center.x, params.center.y],
                    params.inner_radius,
                    params.outer_radius,
                    params.force,
                    params.gravity,
                    params.noise,
                );

                gpu_wind_circles.push(gpu_circle);
            }
        }

        println!(
            "    GPU Forces: {} wind circles uploaded for per-particle computation",
            gpu_wind_circles.len()
        );

        // Upload directly to particle system
        self.gpu_particle_system
            .upload_wind_circles(queue, &gpu_wind_circles);

        Ok(())
    }

    /// Upload spawn requests to GPU (new GPU-side spawning)
    ///
    /// Converts spawn requests from CPU emitters and uploads to GPU.
    /// The GPU spawn shader will find free slots and spawn particles.
    ///
    /// # Arguments
    ///
    /// * `queue` - WebGPU queue for uploads
    /// * `requests` - Slice of spawn requests
    ///
    /// # Returns
    ///
    /// Number of requests uploaded
    pub fn upload_spawn_requests(
        &mut self,
        queue: &wgpu::Queue,
        requests: &[ParticleSpawnRequest],
    ) -> usize {
        self.gpu_particle_system
            .upload_spawn_requests(queue, requests)
    }

    /// Encode GPU spawn pass (new GPU-side spawning)
    ///
    /// Encodes the spawn compute shader to find free slots and spawn particles.
    /// Call this before physics update.
    ///
    /// # Arguments
    ///
    /// * `encoder` - Command encoder to record compute passes
    pub fn encode_spawn(&self, encoder: &mut wgpu::CommandEncoder) {
        self.gpu_particle_system.encode_spawn(encoder);
    }

    /// Update voice limits for per-voice particle culling
    ///
    /// Uploads target particle counts for each voice based on volume and particle_limit.
    /// Call this before encode_cull() to update culling behavior.
    ///
    /// # Arguments
    ///
    /// * `queue` - WebGPU queue for uploads
    /// * `voices` - HashMap of voices containing volume and particle_limit params
    pub fn update_voice_limits(&mut self, queue: &wgpu::Queue, voices: &HashMap<VoiceId, Voice>) {
        use nnpipe::compute::VoiceLimit;

        // Build limits array (indexed by voice_id 0-3)
        let mut limits = [VoiceLimit::default(); 4];
        for voice in voices.values() {
            let voice_id = voice.id.to_i32();
            if voice_id >= 0 && voice_id < 4 {
                let target_count =
                    (voice.params.volume * voice.params.particle_limit as f32).round() as u32;
                limits[voice_id as usize] = VoiceLimit::new(target_count);
            }
        }

        self.gpu_particle_system.update_voice_limits(queue, &limits);
    }

    /// Clear culling counters before running cull pass
    ///
    /// Resets the per-voice particle count atomics. Call this before encode_cull().
    ///
    /// # Arguments
    ///
    /// * `queue` - WebGPU queue for uploads
    pub fn clear_cull_counts(&mut self, queue: &wgpu::Queue) {
        self.gpu_particle_system.clear_cull_counts(queue);
    }

    /// Encode particle culling pass
    ///
    /// Runs the two-pass culling algorithm to mark excess particles for fade-out.
    /// Call this after spawn but before physics.
    ///
    /// # Arguments
    ///
    /// * `encoder` - Command encoder to record compute passes
    pub fn encode_cull(&self, encoder: &mut wgpu::CommandEncoder) {
        self.gpu_particle_system.encode_cull(encoder);
    }

    /// Run full GPU physics update
    ///
    /// Encodes particle physics integration using per-particle force computation.
    /// No force field copy needed - wind circles are already uploaded.
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
        let mut updated_params = *self.gpu_particle_system.physics_params();
        updated_params.dt = framerate_factor;

        self.gpu_particle_system
            .update_physics_params(queue, &updated_params);

        // No force field copy needed - wind circles already uploaded in update_forces()

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

    /// Read GPU alive count conditionally (every 10 frames)
    ///
    /// This method performs CPU-GPU synchronization to read the alive count,
    /// but only does so every 10 frames to reduce performance impact.
    /// Between readbacks, it returns the cached value.
    ///
    /// # Arguments
    ///
    /// * `device` - WebGPU device
    /// * `queue` - WebGPU queue
    ///
    /// # Returns
    ///
    /// GPU-computed alive particle count (may be up to 10 frames stale)
    pub fn read_gpu_alive_count_conditional(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> u32 {
        self.alive_count_readback_counter += 1;

        // Only read every 10 frames to reduce sync overhead
        if self.alive_count_readback_counter.is_multiple_of(10) {
            // Create staging buffer for readback
            let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Alive Count Readback"),
                size: 4,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

            // Copy from GPU alive count buffer
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Alive Count Readback Encoder"),
            });
            encoder.copy_buffer_to_buffer(
                self.gpu_particle_system.alive_count_buffer(),
                0,
                &staging_buffer,
                0,
                4,
            );
            queue.submit(Some(encoder.finish()));

            // Map and read
            let buffer_slice = staging_buffer.slice(..);
            let (tx, rx) = std::sync::mpsc::channel();
            buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
                tx.send(result).unwrap();
            });
            device.poll(wgpu::Maintain::Wait);
            rx.recv().unwrap().unwrap();

            let data = buffer_slice.get_mapped_range();
            self.gpu_alive_count = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
            drop(data);
            staging_buffer.unmap();
        }

        self.gpu_alive_count
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

        // Return: (x, y, life, age) and print color for debugging
        let positions: Vec<(f32, f32, f32, f32)> = particles
            .iter()
            .take(count)
            .enumerate()
            .map(|(i, p)| {
                if i == 0 {
                    println!(
                        "  First particle color: ({:.3}, {:.3}, {:.3}, {:.3})",
                        p.color[0], p.color[1], p.color[2], p.color[3]
                    );
                }
                (
                    p.position[0], // x
                    p.position[1], // y
                    p.position[3], // life (w component of position)
                    p.velocity[3], // age (w component of velocity)
                )
            })
            .collect();

        drop(data);
        staging_buffer.unmap();

        positions
    }
}
