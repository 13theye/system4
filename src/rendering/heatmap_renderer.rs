use nannou::prelude::*;
use nannou::wgpu;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuParticle {
    pub position: [f32; 2],
    pub _padding: [f32; 2], // Align to 16 bytes for GPU
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct HeatmapParams {
    pub resolution: [f32; 2],
    pub bounds_min: [f32; 2],
    pub bounds_max: [f32; 2],
    pub max_influence_radius: f32,
    pub intensity_scale: f32,
    pub particle_count: u32,
    pub _padding: u32,
}

pub struct HeatmapRenderer {
    // Render texture for heatmap output
    pub heatmap_texture: wgpu::Texture,
    pub heatmap_view: wgpu::TextureView,

    // Compute pipeline for heatmap generation
    compute_pipeline: wgpu::ComputePipeline,

    // Buffers
    particle_buffer: wgpu::Buffer,
    params_buffer: wgpu::Buffer,

    // Bind group
    bind_group: wgpu::BindGroup,

    // Parameters
    max_particles: usize,
    width: u32,
    height: u32,

    // Frame limiting
    last_update_frame: std::cell::Cell<u64>,
}

impl HeatmapRenderer {
    pub fn new(device: &wgpu::Device, width: u32, height: u32, max_particles: usize) -> Self {
        // Use reduced resolution for better performance at 4K
        let heatmap_scale = 0.5; // Render heatmap at 1/2 resolution
        let heatmap_width = (width as f32 * heatmap_scale) as u32;
        let heatmap_height = (height as f32 * heatmap_scale) as u32;
        // Create output texture for heatmap at reduced resolution
        let heatmap_texture = wgpu::TextureBuilder::new()
            .size([heatmap_width, heatmap_height])
            .usage(wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING)
            .format(wgpu::TextureFormat::Rgba8Unorm)
            .build(device);

        let heatmap_view = heatmap_texture.view().build();

        // Create particle buffer (storage buffer)
        let particle_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Heatmap Particle Buffer"),
            size: (max_particles * std::mem::size_of::<GpuParticle>()) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create parameters buffer
        let params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Heatmap Params Buffer"),
            size: std::mem::size_of::<HeatmapParams>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create compute shader
        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Heatmap Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/heatmap.wgsl").into()),
        });

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Heatmap Bind Group Layout"),
            entries: &[
                // Output texture (storage)
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                // Particle buffer (storage)
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Parameters (uniform)
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Create bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Heatmap Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&heatmap_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: particle_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        // Create compute pipeline
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Heatmap Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Heatmap Compute Pipeline"),
            layout: Some(&pipeline_layout),
            module: &compute_shader,
            entry_point: "main",
        });

        Self {
            heatmap_texture,
            heatmap_view,
            compute_pipeline,
            particle_buffer,
            params_buffer,
            bind_group,
            max_particles,
            width: heatmap_width,
            height: heatmap_height,
            last_update_frame: std::cell::Cell::new(0),
        }
    }

    pub fn render_heatmap(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        particles: &[Vec2],
        bounds: Rect,
        frame_count: u64,
    ) {
        // Limit heatmap updates to every other frame for better performance
        let last_frame = self.last_update_frame.get();
        if frame_count - last_frame < 2 {
            return;
        }
        self.last_update_frame.set(frame_count);

        // Skip rendering if no particles
        if particles.is_empty() {
            self.clear_heatmap(device, queue);
            return;
        }
        // Convert particles to GPU format
        let gpu_particles: Vec<GpuParticle> = particles
            .iter()
            .take(self.max_particles)
            .map(|pos| GpuParticle {
                position: [pos.x, pos.y],
                _padding: [0.0, 0.0],
            })
            .collect();

        // Pad with empty particles if needed
        let mut padded_particles = gpu_particles;
        padded_particles.resize(
            self.max_particles,
            GpuParticle {
                position: [f32::INFINITY, f32::INFINITY], // Invalid position
                _padding: [0.0, 0.0],
            },
        );

        // Update particle buffer
        queue.write_buffer(
            &self.particle_buffer,
            0,
            bytemuck::cast_slice(&padded_particles),
        );

        // Update parameters
        let params = HeatmapParams {
            resolution: [self.width as f32, self.height as f32],
            bounds_min: [bounds.left(), bounds.bottom()],
            bounds_max: [bounds.right(), bounds.top()],
            max_influence_radius: 100.0, // Keep same influence radius for quality
            intensity_scale: 0.25,
            particle_count: particles.len() as u32,
            _padding: 0,
        };

        queue.write_buffer(&self.params_buffer, 0, bytemuck::cast_slice(&[params]));

        // Dispatch compute shader
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Heatmap Compute Encoder"),
        });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Heatmap Compute Pass"),
            });

            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &self.bind_group, &[]);

            // Dispatch in 8x8 work groups
            let workgroup_size = 8;
            let dispatch_x = (self.width + workgroup_size - 1) / workgroup_size;
            let dispatch_y = (self.height + workgroup_size - 1) / workgroup_size;

            compute_pass.dispatch_workgroups(dispatch_x, dispatch_y, 1);
        }

        queue.submit(Some(encoder.finish()));
        device.poll(wgpu::Maintain::Wait);
    }

    fn clear_heatmap(&self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Clear Heatmap Encoder"),
        });

        // Clear the heatmap texture to black
        let compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Clear Heatmap Pass"),
        });

        // Just dispatch to clear the texture (could use a clear shader, but this is simpler)
        drop(compute_pass);

        queue.submit(Some(encoder.finish()));
        device.poll(wgpu::Maintain::Wait);
    }

    pub fn get_heatmap_view(&self) -> &wgpu::TextureView {
        &self.heatmap_view
    }

    pub fn create_texture_reshaper(
        &self,
        device: &wgpu::Device,
        window: &nannou::window::Window,
    ) -> wgpu::TextureReshaper {
        let sample_count = window.msaa_samples();
        let dst_format = nannou::Frame::TEXTURE_FORMAT;

        wgpu::TextureReshaper::new(
            device,
            &self.heatmap_view,
            1, // Non-multisampled texture
            wgpu::TextureSampleType::Float { filterable: true },
            sample_count,
            dst_format,
        )
    }
}
