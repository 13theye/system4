use crate::groups::VoiceId;

use nannou::{prelude::*, text::Font, wgpu::TextureReshaper, App};
use nnpipe::{
    renderers::{ParticleGpu, ParticleRenderer, SegmentRenderer},
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
    // Rendering engine
    pub render_engine: RefCell<Nnpipe>,

    // Heatmap graphics, currently unused and nnpipe impl is not working with the current direct-write
    // particle buffer setup.
    // pub heatmap_renderer: HeatmapRenderer,

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
    pub terminal_font: Font,
}

impl RenderState {
    /// Get the rendering pipeline
    pub fn engine(&self) -> &RefCell<Nnpipe> {
        &self.render_engine
    }

    /// Set the ParticleRender and SegmentRenderer engine debug flags
    pub fn set_render_engines_debug(&mut self, debug: bool) {
        self.particle_renderer_voice0.set_engine_debug(debug);
        self.particle_renderer_voice3.set_engine_debug(debug);
        self.segment_renderer_voice0.set_engine_debug(debug);
        self.segment_renderer_voice3.set_engine_debug(debug);
    }

    /// Get the particle texture name for a voice (straight alpha)
    pub fn get_texture_name(&self, voice_id: VoiceId) -> Option<String> {
        match voice_id {
            VoiceId::Voice0 => Some("particles_voice_0".to_string()),
            VoiceId::Voice3 => Some("particles_voice_3".to_string()),
            _ => None,
        }
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

        // Create main rendering pipeline
        let mut rendering = Nnpipe::new(device, texture_width, texture_height, texture_samples);

        // Create renderers
        /*
        let heatmap_renderer = HeatmapRenderer::new(
            device,
            texture_width,
            texture_height,
            particle_limit as usize,
        );
         */

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
        // All rendering textures use straight alpha - the composite shader handles conversion

        // Terminal texture for UI overlay (text)
        rendering.create_named_texture(device, "terminal", hi_config);

        // Rhythm formation textures (straight alpha)
        rendering.create_named_texture(device, "rhythm_alpha", hi_config);
        rendering.create_named_texture(device, "rhythm_solid", hi_config);

        // Intermediate texture for rhythm composite result
        rendering.create_named_texture(device, "rhythm_composited", hi_config);

        // Particle textures - straight alpha from particle/segment shaders and Nannou Draw masks
        // Both particles and masks render to the same texture using standard alpha blending
        rendering.create_named_texture(device, "particles_voice_0", hi_config);
        rendering.create_named_texture(device, "particles_voice_3", hi_config);

        // Combined particles texture (premultiplied alpha - output from composite shader)
        rendering.create_named_texture(device, "particles_combined", hi_config);

        /*
        rendering.create_named_texture(device, "heatmap", hi_config);
        rendering.create_named_texture(device, "heatmap_processed", hi_config);
        rendering.create_named_texture(device, "processed_composited", hi_config);
        */

        rendering.create_named_texture(device, "post-processed", hi_config);

        // Build and add rendering pipelines
        Self::setup_rendering_pipelines(&mut rendering, device, hi_config, med_config, lo_config);

        Self {
            //gpu_particle_buffer,
            render_engine: RefCell::new(rendering),
            // heatmap_renderer,
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
            terminal_font: font,
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
        // === Voice Combination ===
        // Combine straight-alpha voice textures into a single particles texture
        // The composite shader accepts straight alpha and outputs premultiplied alpha
        // Uses Lighten blend: brighter pixel wins
        match PipelineBuilder::new()
            .name("Combine Voice Particles")
            .input_textures(&["particles_voice_0", "particles_voice_3"])
            .output_texture("particles_combined")
            .simple_lighten_composite(hi_config, 1.0)
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

        // Bloom effects pipeline
        // Input: particles_combined (premultiplied alpha from composite)
        // Output: post-processed (premultiplied alpha)
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

        // Composite rhythm formations with alpha
        // Layer: post-processed (particles) → rhythm_voice1 (α=0.5) → rhythm_voice2 (α=0.5)
        // Input: post-processed (premultiplied), rhythm_voice1/2 (straight alpha, full opacity)
        // Output: rhythm_composited (intermediate texture)
        // The composite shader applies intensity=0.5 to rhythm textures for correct alpha
        if let Ok(effect) = PipelineBuilder::new()
            .name("Rhythm Composite")
            .input_textures(&["post-processed", "rhythm_alpha"])
            .output_texture("rhythm_composited") // Store result in intermediate texture
            .simple_over_composite(hi_config, 0.5) // Apply 0.5 alpha to rhythm layers
            .build(device)
        {
            rendering.add_multi_pipeline("rhythm composite", effect);
        }

        // Final composite overlay - add rhythm_solid & terminal on top with full opacity
        // Layer: rhythm_composited (particles + rhythms with alpha) → rhythm_solid -> terminal (α=1.0)
        // Input: rhythm_composited (premultiplied), rhythm_solid (straight alpha),terminal (straight alpha)
        if let Ok(effect) = PipelineBuilder::new()
            .name("Final overlay composite")
            .input_textures(&["rhythm_composited", "terminal"])
            .simple_over_composite(hi_config, 1.0) // Terminal at full opacity
            .build(device)
        {
            rendering.add_multi_pipeline("final composite", effect);
        }
    }
}
