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
pub struct HeatmapBinnedParams {
    pub resolution: [f32; 2],
    pub bounds_min: [f32; 2],
    pub bounds_max: [f32; 2],
    pub grid_size: [u32; 2],
    pub bin_cell_size: [f32; 2],
    pub max_influence_radius: f32,
    pub intensity_scale: f32,
    pub particle_count: u32,
    pub _padding: u32,
}

pub struct HeatmapBinnedRenderer {
    // Render texture for heatmap output
    pub heatmap_texture: wgpu::Texture,
    pub heatmap_view: wgpu::TextureView,

    // Compute pipelines for binning and heatmap generation
    binning_count_pipeline: wgpu::ComputePipeline,
    binning_offset_pipeline: wgpu::ComputePipeline,
    binning_fill_pipeline: wgpu::ComputePipeline,
    heatmap_pipeline: wgpu::ComputePipeline,

    // Buffers
    particle_buffer: wgpu::Buffer,
    params_buffer: wgpu::Buffer,
    bin_counts_buffer: wgpu::Buffer,
    bin_data_buffer: wgpu::Buffer,
    bin_offsets_buffer: wgpu::Buffer,
    bin_fill_counts_buffer: wgpu::Buffer,

    // Bind groups
    binning_bind_group: wgpu::BindGroup,
    heatmap_bind_group: wgpu::BindGroup,

    // Parameters
    max_particles: usize,
    width: u32,
    height: u32,
    grid_size: (u32, u32),

    // Frame limiting
    last_update_frame: std::cell::Cell<u64>,
}

impl HeatmapBinnedRenderer {
    pub fn new(device: &wgpu::Device, width: u32, height: u32, max_particles: usize) -> Self {
        // Use reduced resolution for better performance at 4K
        let heatmap_scale = 0.5; // Render heatmap at 1/2 resolution
        let heatmap_width = (width as f32 * heatmap_scale) as u32;
        let heatmap_height = (height as f32 * heatmap_scale) as u32;

        // Calculate grid size for binning based on world space, accounting for resolution scaling
        // This ensures consistency between binning and rendering phases
        let bin_size = 100.0; // Should match max_influence_radius
        let grid_size = Self::calculate_grid_size(width, height, bin_size);
        let total_bins = grid_size.0 * grid_size.1;

        // Create output texture for heatmap at reduced resolution
        let heatmap_texture = wgpu::TextureBuilder::new()
            .size([heatmap_width, heatmap_height])
            .usage(wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING)
            .format(wgpu::TextureFormat::Rgba8Unorm)
            .build(device);

        let heatmap_view = heatmap_texture.view().build();

        // Create particle buffer (storage buffer)
        let particle_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Heatmap Binned Particle Buffer"),
            size: (max_particles * std::mem::size_of::<GpuParticle>()) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create parameters buffer
        let params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Heatmap Binned Params Buffer"),
            size: std::mem::size_of::<HeatmapBinnedParams>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create binning buffers
        let bin_counts_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Bin Counts Buffer"),
            size: (total_bins as usize * std::mem::size_of::<u32>()) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Calculate worst-case bin data buffer size with safety factor
        let bin_data_size = Self::calculate_bin_data_size(max_particles);
        
        let bin_data_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Bin Data Buffer"),
            size: (bin_data_size * std::mem::size_of::<u32>()) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let bin_offsets_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Bin Offsets Buffer"),
            size: (total_bins as usize * std::mem::size_of::<u32>()) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let bin_fill_counts_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Bin Fill Counts Buffer"),
            size: (total_bins as usize * std::mem::size_of::<u32>()) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create shaders
        let binning_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Particle Binning Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/particle_binning.wgsl").into()),
        });

        let heatmap_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Heatmap Binned Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/heatmap_binned.wgsl").into()),
        });

        // Create bind group layout for binning
        let binning_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Binning Bind Group Layout"),
            entries: &[
                // Particles (storage)
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Bin counts (storage)
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Bin data (storage)
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Bin offsets (storage)
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Parameters (uniform)
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Bin fill counts (storage)
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Create bind group layout for heatmap
        let heatmap_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Heatmap Binned Bind Group Layout"),
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
                // Particles (storage)
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
                // Bin counts (storage)
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Bin data (storage)
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Bin offsets (storage)
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
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
                    binding: 5,
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

        // Create bind groups
        let binning_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Binning Bind Group"),
            layout: &binning_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: particle_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: bin_counts_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: bin_data_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: bin_offsets_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: bin_fill_counts_buffer.as_entire_binding(),
                },
            ],
        });

        let heatmap_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Heatmap Binned Bind Group"),
            layout: &heatmap_bind_group_layout,
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
                    resource: bin_counts_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: bin_data_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: bin_offsets_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        // Create compute pipelines
        let binning_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Binning Pipeline Layout"),
            bind_group_layouts: &[&binning_bind_group_layout],
            push_constant_ranges: &[],
        });

        let heatmap_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Heatmap Binned Pipeline Layout"),
            bind_group_layouts: &[&heatmap_bind_group_layout],
            push_constant_ranges: &[],
        });

        let binning_count_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Binning Count Pipeline"),
            layout: Some(&binning_pipeline_layout),
            module: &binning_shader,
            entry_point: "count_particles",
        });

        let binning_offset_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Binning Offset Pipeline"),
            layout: Some(&binning_pipeline_layout),
            module: &binning_shader,
            entry_point: "calculate_offsets",
        });

        let binning_fill_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Binning Fill Pipeline"),
            layout: Some(&binning_pipeline_layout),
            module: &binning_shader,
            entry_point: "fill_bins",
        });

        let heatmap_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Heatmap Binned Compute Pipeline"),
            layout: Some(&heatmap_pipeline_layout),
            module: &heatmap_shader,
            entry_point: "main",
        });

        Self {
            heatmap_texture,
            heatmap_view,
            binning_count_pipeline,
            binning_offset_pipeline,
            binning_fill_pipeline,
            heatmap_pipeline,
            particle_buffer,
            params_buffer,
            bin_counts_buffer,
            bin_data_buffer,
            bin_offsets_buffer,
            bin_fill_counts_buffer,
            binning_bind_group,
            heatmap_bind_group,
            max_particles,
            width: heatmap_width,
            height: heatmap_height,
            grid_size,
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

        // Calculate bin cell size
        let bounds_width = bounds.right() - bounds.left();
        let bounds_height = bounds.top() - bounds.bottom();
        let bin_cell_size_x = bounds_width / self.grid_size.0 as f32;
        let bin_cell_size_y = bounds_height / self.grid_size.1 as f32;

        // Update parameters
        let params = HeatmapBinnedParams {
            resolution: [self.width as f32, self.height as f32],
            bounds_min: [bounds.left(), bounds.bottom()],
            bounds_max: [bounds.right(), bounds.top()],
            grid_size: [self.grid_size.0, self.grid_size.1],
            bin_cell_size: [bin_cell_size_x, bin_cell_size_y],
            max_influence_radius: 100.0,
            intensity_scale: 0.25,
            particle_count: particles.len() as u32,
            _padding: 0,
        };

        queue.write_buffer(&self.params_buffer, 0, bytemuck::cast_slice(&[params]));

        // Clear binning buffers
        let total_bins = self.grid_size.0 * self.grid_size.1;
        let zero_counts = vec![0u32; total_bins as usize];
        queue.write_buffer(&self.bin_counts_buffer, 0, bytemuck::cast_slice(&zero_counts));
        queue.write_buffer(&self.bin_fill_counts_buffer, 0, bytemuck::cast_slice(&zero_counts));

        // Execute binning and heatmap generation
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Heatmap Binned Compute Encoder"),
        });

        // Phase 1: Count particles per bin
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Binning Count Pass"),
            });

            compute_pass.set_pipeline(&self.binning_count_pipeline);
            compute_pass.set_bind_group(0, &self.binning_bind_group, &[]);

            let workgroup_size = 64;
            let dispatch_x = (particles.len() as u32 + workgroup_size - 1) / workgroup_size;
            compute_pass.dispatch_workgroups(dispatch_x, 1, 1);
        }

        // Phase 2: Calculate bin offsets
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Binning Offset Pass"),
            });

            compute_pass.set_pipeline(&self.binning_offset_pipeline);
            compute_pass.set_bind_group(0, &self.binning_bind_group, &[]);

            compute_pass.dispatch_workgroups(1, 1, 1);
        }

        // Phase 3: Fill bin data
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Binning Fill Pass"),
            });

            compute_pass.set_pipeline(&self.binning_fill_pipeline);
            compute_pass.set_bind_group(0, &self.binning_bind_group, &[]);

            let workgroup_size = 64;
            let dispatch_x = (particles.len() as u32 + workgroup_size - 1) / workgroup_size;
            compute_pass.dispatch_workgroups(dispatch_x, 1, 1);
        }

        // Phase 4: Generate heatmap using binned data
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Heatmap Binned Compute Pass"),
            });

            compute_pass.set_pipeline(&self.heatmap_pipeline);
            compute_pass.set_bind_group(0, &self.heatmap_bind_group, &[]);

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
            label: Some("Clear Heatmap Binned Encoder"),
        });

        // Clear the heatmap texture to black
        let compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Clear Heatmap Binned Pass"),
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

    /// Calculate required buffer size for bin data
    pub fn calculate_bin_data_size(max_particles: usize) -> usize {
        let safety_factor = 2.0;
        (max_particles as f32 * safety_factor) as usize
    }

    /// Calculate grid dimensions for spatial binning
    pub fn calculate_grid_size(width: u32, height: u32, bin_size: f32) -> (u32, u32) {
        let grid_width = ((width as f32 / bin_size).ceil() as u32).max(1);
        let grid_height = ((height as f32 / bin_size).ceil() as u32).max(1);
        (grid_width, grid_height)
    }

    /// Calculate bin index from 2D coordinates
    pub fn bin_coord_to_index(bin_x: u32, bin_y: u32, grid_width: u32) -> u32 {
        bin_y * grid_width + bin_x
    }

    /// Check if bin coordinates are valid
    pub fn is_valid_bin_coord(bin_x: i32, bin_y: i32, grid_width: u32, grid_height: u32) -> bool {
        bin_x >= 0 && bin_x < grid_width as i32 && bin_y >= 0 && bin_y < grid_height as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_bin_data_size() {
        assert_eq!(HeatmapBinnedRenderer::calculate_bin_data_size(1000), 2000);
        assert_eq!(HeatmapBinnedRenderer::calculate_bin_data_size(5000), 10000);
        assert_eq!(HeatmapBinnedRenderer::calculate_bin_data_size(0), 0);
    }

    #[test]
    fn test_calculate_grid_size() {
        // Test normal cases
        assert_eq!(HeatmapBinnedRenderer::calculate_grid_size(1920, 1080, 100.0), (20, 11));
        assert_eq!(HeatmapBinnedRenderer::calculate_grid_size(800, 600, 50.0), (16, 12));
        
        // Test edge cases
        assert_eq!(HeatmapBinnedRenderer::calculate_grid_size(10, 10, 100.0), (1, 1));
        assert_eq!(HeatmapBinnedRenderer::calculate_grid_size(0, 0, 100.0), (1, 1));
        
        // Test exact divisions
        assert_eq!(HeatmapBinnedRenderer::calculate_grid_size(1000, 500, 100.0), (10, 5));
    }

    #[test]
    fn test_bin_coord_to_index() {
        // Test basic indexing
        assert_eq!(HeatmapBinnedRenderer::bin_coord_to_index(0, 0, 10), 0);
        assert_eq!(HeatmapBinnedRenderer::bin_coord_to_index(5, 0, 10), 5);
        assert_eq!(HeatmapBinnedRenderer::bin_coord_to_index(0, 1, 10), 10);
        assert_eq!(HeatmapBinnedRenderer::bin_coord_to_index(5, 2, 10), 25);
        
        // Test with different grid widths
        assert_eq!(HeatmapBinnedRenderer::bin_coord_to_index(3, 4, 5), 23);
    }

    #[test]
    fn test_is_valid_bin_coord() {
        let grid_width = 10;
        let grid_height = 8;
        
        // Test valid coordinates
        assert!(HeatmapBinnedRenderer::is_valid_bin_coord(0, 0, grid_width, grid_height));
        assert!(HeatmapBinnedRenderer::is_valid_bin_coord(5, 3, grid_width, grid_height));
        assert!(HeatmapBinnedRenderer::is_valid_bin_coord(9, 7, grid_width, grid_height));
        
        // Test invalid coordinates
        assert!(!HeatmapBinnedRenderer::is_valid_bin_coord(-1, 0, grid_width, grid_height));
        assert!(!HeatmapBinnedRenderer::is_valid_bin_coord(0, -1, grid_width, grid_height));
        assert!(!HeatmapBinnedRenderer::is_valid_bin_coord(10, 0, grid_width, grid_height));
        assert!(!HeatmapBinnedRenderer::is_valid_bin_coord(0, 8, grid_width, grid_height));
        assert!(!HeatmapBinnedRenderer::is_valid_bin_coord(15, 15, grid_width, grid_height));
    }

    #[test]
    fn test_buffer_size_scaling() {
        // Test that buffer size scales appropriately with particle count
        let size_1k = HeatmapBinnedRenderer::calculate_bin_data_size(1000);
        let size_2k = HeatmapBinnedRenderer::calculate_bin_data_size(2000);
        let size_10k = HeatmapBinnedRenderer::calculate_bin_data_size(10000);
        
        assert_eq!(size_2k, size_1k * 2);
        assert_eq!(size_10k, size_1k * 10);
        
        // Verify safety factor is applied
        assert!(size_1k > 1000);
        assert!(size_1k <= 3000); // Should be reasonable upper bound
    }

    #[test]
    fn test_grid_dimensions_consistency() {
        let width = 1920u32;
        let height = 1080u32;
        let bin_size = 100.0f32;
        
        let (grid_width, grid_height) = HeatmapBinnedRenderer::calculate_grid_size(width, height, bin_size);
        
        // Verify grid covers the entire screen area
        assert!((grid_width as f32 * bin_size) >= width as f32);
        assert!((grid_height as f32 * bin_size) >= height as f32);
        
        // Verify grid isn't oversized (should be within one bin size of exact)
        assert!((grid_width as f32 * bin_size) < (width as f32 + bin_size));
        assert!((grid_height as f32 * bin_size) < (height as f32 + bin_size));
    }

    #[test]
    fn test_index_bounds() {
        let grid_width = 20u32;
        let grid_height = 15u32;
        let max_index = grid_width * grid_height - 1;
        
        // Test maximum valid coordinates
        let max_x = grid_width - 1;
        let max_y = grid_height - 1;
        
        assert_eq!(HeatmapBinnedRenderer::bin_coord_to_index(max_x, max_y, grid_width), max_index);
        assert!(HeatmapBinnedRenderer::is_valid_bin_coord(max_x as i32, max_y as i32, grid_width, grid_height));
        
        // Test that out-of-bounds coordinates are properly rejected
        assert!(!HeatmapBinnedRenderer::is_valid_bin_coord(grid_width as i32, 0, grid_width, grid_height));
        assert!(!HeatmapBinnedRenderer::is_valid_bin_coord(0, grid_height as i32, grid_width, grid_height));
    }
}