use nannou::{prelude::*, text::Font, wgpu::TextureReshaper, App};
use nnpipe::{
    renderers::{HeatmapRenderer, ParticleGpu, ParticleRenderer, SegmentRenderer},
    Nnpipe, PipelineBuilder, TextureConfig,
};
use std::cell::RefCell;

pub type GpuParticleBuffer = Vec<ParticleGpu>;

/// RenderState encapsulates all rendering-related state.
/// This includes:
/// - GPU buffers for particle data
/// - Rendering pipeline and renderers
/// - Texture reshapers for multi-window rendering
/// - Window IDs and drawing contexts
/// - Rendering metadata (counts, size, DPI)
pub struct RenderState {
    // GPU buffers
    //pub gpu_particle_buffer: GpuParticleBuffer,

    // Rendering engine
    pub render_engine: RefCell<Nnpipe>,
    pub heatmap_renderer: HeatmapRenderer,

    // Voice-specific renderers (isolated buffers per voice)
    pub particle_renderer_voice0: ParticleRenderer,
    pub particle_renderer_voice3: ParticleRenderer,
    pub segment_renderer_voice0: SegmentRenderer,
    pub segment_renderer_voice3: SegmentRenderer,

    // Texture reshapers and window data
    pub render_size: Vec2,
    pub render_rect: Rect,
    pub audience_window_id: WindowId,
    pub performer_window_id: WindowId,
    pub control_window_id: WindowId,
    pub audience_reshaper: TextureReshaper,
    pub performer_reshaper: TextureReshaper,

    // Draw contexts for each window
    pub audience_draw: nannou::Draw,
    pub performer_draw: nannou::Draw,
    pub control_draw: nannou::Draw,

    // Rendering metadata
    pub dpi_scale: f32,
    pub font: Font,
}

impl RenderState {
    /// Get the rendering pipeline
    pub fn engine(&self) -> &RefCell<Nnpipe> {
        &self.render_engine
    }

    /*
    /// Get the GPU particle buffer
    pub fn gpu_particle_buffer(&self) -> &GpuParticleBuffer {
        &self.gpu_particle_buffer
    }

    /// Get mutable access to the GPU particle buffer
    pub fn gpu_particle_buffer_mut(&mut self) -> &mut GpuParticleBuffer {
        &mut self.gpu_particle_buffer
    }
     */

    /// Set the ParticleRender and SegmentRenderer engine debug flags
    pub fn set_render_engines_debug(&mut self, debug: bool) {
        self.particle_renderer_voice0.set_engine_debug(debug);
        self.particle_renderer_voice3.set_engine_debug(debug);
        self.segment_renderer_voice0.set_engine_debug(debug);
        self.segment_renderer_voice3.set_engine_debug(debug);
    }

    /// Create a RenderState from the Nannou app with window IDs and settings
    /// This encapsulates all the complex rendering initialization logic
    #[allow(clippy::too_many_arguments)]
    pub fn from_app(
        app: &App,
        audience_window_id: WindowId,
        performer_window_id: WindowId,
        control_window_id: WindowId,
        texture_width: u32,
        texture_height: u32,
        texture_samples: u32,
        particle_limit: u32,
        dpi_scale: f32,
        font: Font,
    ) -> Self {
        let render_size = vec2(texture_width as f32, texture_height as f32);
        let render_rect = Rect::from_x_y_w_h(0.0, 0.0, render_size.x, render_size.y);

        // Get windows
        let Some(audience_window) = app.window(audience_window_id) else {
            eprintln!("Audience window not found. Exiting app.");
            std::process::exit(1);
        };
        let Some(performer_window) = app.window(performer_window_id) else {
            eprintln!("Performer window not found. Exiting app.");
            std::process::exit(1);
        };

        // Get device from audience window
        let device = audience_window.device();

        // Initialize empty GPU buffer
        //let gpu_particle_buffer = Vec::new();

        // Create main rendering pipeline
        let mut rendering = Nnpipe::new(device, texture_width, texture_height, texture_samples);

        // Create renderers
        let heatmap_renderer = HeatmapRenderer::new(
            device,
            texture_width,
            texture_height,
            particle_limit as usize,
        );

        let hi_config = TextureConfig {
            width: texture_width,
            height: texture_height,
            format: wgpu::TextureFormat::Rgba16Float,
        };

        // Create separate renderers for each voice to ensure buffer isolation
        let particle_renderer_voice0 =
            ParticleRenderer::new(device, hi_config, particle_limit as usize);
        let particle_renderer_voice3 =
            ParticleRenderer::new(device, hi_config, particle_limit as usize);
        let segment_renderer_voice0 =
            SegmentRenderer::new(device, hi_config, particle_limit as usize);
        let segment_renderer_voice3 =
            SegmentRenderer::new(device, hi_config, particle_limit as usize);

        // Create texture reshapers for multi-window rendering
        let audience_reshaper =
            rendering.create_reshaper_for_post_processed(device, &audience_window);
        let performer_reshaper =
            rendering.create_reshaper_for_post_processed(device, &performer_window);

        // Create draw contexts
        let audience_draw = nannou::Draw::new();
        let performer_draw = nannou::Draw::new();
        let control_draw = nannou::Draw::new();

        // Set up texture configurations
        let lo_config = TextureConfig {
            width: texture_width / 2,
            height: texture_height / 2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
        };

        let med_config = TextureConfig {
            width: texture_width,
            height: texture_height,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
        };

        // Create named textures for the pipeline
        rendering.create_named_texture(device, "terminal", hi_config);
        rendering.create_named_texture(device, "particles_voice_0", hi_config);
        rendering.create_named_texture(device, "particles_voice_3", hi_config);
        rendering.create_named_texture(device, "particles_combined", hi_config);
        rendering.create_named_texture(device, "heatmap", hi_config);
        rendering.create_named_texture(device, "heatmap_processed", hi_config);
        rendering.create_named_texture(device, "processed_composited", hi_config);
        rendering.create_named_texture(device, "post-processed", hi_config);

        // Build and add rendering pipelines
        Self::setup_rendering_pipelines(&mut rendering, device, hi_config, med_config, lo_config);

        Self {
            //gpu_particle_buffer,
            render_engine: RefCell::new(rendering),
            heatmap_renderer,
            particle_renderer_voice0,
            particle_renderer_voice3,
            segment_renderer_voice0,
            segment_renderer_voice3,
            render_size,
            render_rect,
            audience_window_id,
            performer_window_id,
            control_window_id,
            audience_reshaper,
            performer_reshaper,
            audience_draw,
            performer_draw,
            control_draw,
            dpi_scale,
            font,
        }
    }

    /// Set up all the rendering effect pipelines
    fn setup_rendering_pipelines(
        rendering: &mut Nnpipe,
        device: &wgpu::Device,
        hi_config: TextureConfig,
        med_config: TextureConfig,
        lo_config: TextureConfig,
    ) {
        // Combine voice textures into a single particles texture
        match PipelineBuilder::new()
            .name("Combine Voice Particles")
            .input_textures(&["particles_voice_0", "particles_voice_3"])
            .output_texture("particles_combined")
            .simple_over_composite(hi_config, 1.0)
            .build(device)
        {
            Ok(effect) => {
                rendering.add_multi_pipeline("combine_voices", effect);
                println!("Successfully created combine_voices pipeline");
            }
            Err(e) => {
                eprintln!("ERROR: Failed to create combine_voices pipeline: {:?}", e);
            }
        }

        // Particle effects pipeline (currently disabled in original code)
        if let Ok(effect) = PipelineBuilder::new()
            .name("Particle Effects Pipeline")
            .input_texture("particles_combined")
            .feedback(hi_config, 1.0, 60.0)
            .output_texture("particle_processed")
            .build(device)
        {
            rendering.add_multi_pipeline("particle_effects", effect);
        }

        // Heatmap effects pipeline
        if let Ok(effect) = PipelineBuilder::new()
            .name("Heatmap Effects Pipeline")
            .input_texture("heatmap")
            .feedback(hi_config, 1.0, 10.0)
            .output_texture("heatmap_processed")
            .build(device)
        {
            rendering.add_multi_pipeline("heatmap_effects", effect);
        }

        // Composite step pipeline
        if let Ok(effect) = PipelineBuilder::new()
            .name("Composite Step Pipeline")
            .input_textures(&["particles_combined", "heatmap_processed"])
            .output_texture("processed_composited")
            .simple_additive_composite(hi_config, 0.5)
            .build(device)
        {
            rendering.add_multi_pipeline("composite_step", effect);
        }

        // Bloom effects pipeline
        if let Ok(effect) = PipelineBuilder::new()
            .name("Particle Effects Pipeline")
            .input_texture("particles_combined")
            .output_texture("post-processed")
            .brightness_extract(med_config, 0.7)
            .downsample(lo_config)
            .gaussian_blur_passes(lo_config, 2, 2.0, 5.0)
            .bloom_composite_with_curve(hi_config, 2.0, 3.0)
            .build(device)
        {
            rendering.add_multi_pipeline("effects", effect);
        }

        // Final composite overlay
        if let Ok(effect) = PipelineBuilder::new()
            .name("Final overlay composite")
            .input_textures(&["post-processed", "terminal"])
            .simple_over_composite(hi_config, 1.0)
            .build(device)
        {
            rendering.add_multi_pipeline("final composite", effect);
        }
    }
}
