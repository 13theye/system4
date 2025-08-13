/// src/particle/gpu_particle_system.rs
///
/// GPU-accelerated replacement for the CPU ParticleSystem
/// Maintains the same public interface while using Nnpipe for GPU computation
use std::collections::{BTreeMap, HashMap};

use nannou::prelude::*;
use nannou::rand::{rngs::ThreadRng, Rng};

use crate::{
    forces::WindCircle,
    utils::IdGenerator,
    view::{Mask, Voice},
};

// Import Nnpipe GPU particle components
#[cfg(feature = "gpu-particles")]
use nnpipe::{
    AdaptiveQualitySettings, GpuEmitter, GpuForceField, GpuMask, GpuParticle, GpuWindCircle,
    ParticleBoundsCullComponent, ParticleCullingComponent, ParticleDebugConfig,
    ParticleDebugRenderer, ParticleGlobalParams, ParticleGroupParams, ParticlePerformanceMonitor,
    ParticleRendererComponent, PerformanceAnalysis, PerformanceTargets,
};

pub struct GpuParticleSystem {
    // GPU-accelerated particle components
    #[cfg(feature = "gpu-particles")]
    particle_renderer: Option<ParticleRendererComponent>,
    #[cfg(feature = "gpu-particles")]
    force_field: Option<GpuForceField>,
    #[cfg(feature = "gpu-particles")]
    culling_component: Option<ParticleCullingComponent>,
    #[cfg(feature = "gpu-particles")]
    gpu_masks: HashMap<Voice, GpuMask>,
    #[cfg(feature = "gpu-particles")]
    global_params: ParticleGlobalParams,
    #[cfg(feature = "gpu-particles")]
    group_params: HashMap<Voice, ParticleGroupParams>,
    #[cfg(feature = "gpu-particles")]
    gpu_emitters: Vec<GpuEmitter>,
    #[cfg(feature = "gpu-particles")]
    wind_circles: HashMap<Voice, GpuWindCircle>,
    #[cfg(feature = "gpu-particles")]
    debug_renderer: Option<ParticleDebugRenderer>,
    #[cfg(feature = "gpu-particles")]
    performance_monitor: ParticlePerformanceMonitor,
    #[cfg(feature = "gpu-particles")]
    adaptive_quality: AdaptiveQualitySettings,

    // Fallback to CPU system when GPU features disabled
    #[cfg(not(feature = "gpu-particles"))]
    cpu_system: crate::particle::ParticleSystem,

    // Shared interface data (maintained for compatibility)
    pub masks: HashMap<Voice, Mask>,
    pub default_particle_limit: usize,
    pub particle_limits: HashMap<Voice, usize>,
    pub feedback: HashMap<Voice, f32>,
    pub global_max_spawn_rate: f32,
    pub alpha_limits: HashMap<Voice, f32>,
    pub color_limits: HashMap<Voice, Rgb>,
    pub particle_num_factors: HashMap<Voice, f32>,
    pub trail: f32,

    // Bounds and visual properties
    origin: Point2,
    bounds_size: Vec2,
    pub bounds_rect: Rect,
    default_particle_size: f32,
    default_particle_color: Rgb,
    dpi_scale: f32,

    // GPU state
    #[cfg(feature = "gpu-particles")]
    frame_number: u32,
    #[cfg(feature = "gpu-particles")]
    params_dirty: bool, // Track if group parameters need GPU sync
}

impl GpuParticleSystem {
    pub fn new(
        origin: Point2,
        width: f32,
        height: f32,
        default_particle_size: f32,
        default_particle_color: Rgb,
        default_particle_limit: u32,
        dpi_scale: f32,
    ) -> Self {
        let bounds_size = Vec2::new(width, height);
        let bounds_rect = Rect::from_x_y_w_h(origin.x, origin.y, width, height);

        #[cfg(feature = "gpu-particles")]
        {
            let mut global_params = ParticleGlobalParams::new();
            global_params.set_bounds(
                bounds_rect.left(),
                bounds_rect.bottom(),
                bounds_rect.right(),
                bounds_rect.top(),
            );
            global_params.max_particle_count = default_particle_limit;

            Self {
                particle_renderer: None, // Will be initialized when device is available
                force_field: None,
                culling_component: None,
                gpu_masks: HashMap::new(),
                global_params,
                group_params: HashMap::new(),
                gpu_emitters: Vec::new(),
                wind_circles: HashMap::new(),
                debug_renderer: Some(ParticleDebugRenderer::new()),
                performance_monitor: ParticlePerformanceMonitor::new(),
                adaptive_quality: AdaptiveQualitySettings::default(),
                frame_number: 0,
                params_dirty: false,

                masks: HashMap::new(),
                default_particle_limit: default_particle_limit as usize,
                particle_limits: HashMap::new(),
                feedback: HashMap::new(),
                global_max_spawn_rate: 40.0,
                alpha_limits: HashMap::new(),
                color_limits: HashMap::new(),
                particle_num_factors: HashMap::new(),
                trail: 0.0,

                origin,
                bounds_size,
                bounds_rect,
                default_particle_size,
                default_particle_color,
                dpi_scale,
            }
        }

        #[cfg(not(feature = "gpu-particles"))]
        {
            Self {
                cpu_system: crate::particle::ParticleSystem::new(
                    origin,
                    width,
                    height,
                    default_particle_size,
                    default_particle_color,
                    default_particle_limit,
                    dpi_scale,
                ),

                masks: HashMap::new(),
                default_particle_limit: default_particle_limit as usize,
                particle_limits: HashMap::new(),
                feedback: HashMap::new(),
                global_max_spawn_rate: 40.0,
                alpha_limits: HashMap::new(),
                color_limits: HashMap::new(),
                particle_num_factors: HashMap::new(),
                trail: 0.0,

                origin,
                bounds_size,
                bounds_rect,
                default_particle_size,
                default_particle_color,
                dpi_scale,
            }
        }
    }

    /// Initialize GPU components when graphics device is available
    #[cfg(feature = "gpu-particles")]
    pub fn init_gpu(&mut self, device: &nannou::wgpu::Device, format: nannou::wgpu::TextureFormat) {
        use nnpipe::{ParticleBlendMode, ParticleRenderMode, TextureConfig};

        // Create force field (256x256 texture for high resolution)
        self.force_field = Some(GpuForceField::new(device, 256, 256, 16));

        // Create culling component for mask-based particle culling
        self.culling_component = Some(ParticleCullingComponent::new(device, 16));
        // Support up to 16 masks

        // Create particle renderer with default capacity
        let output_config = TextureConfig {
            width: self.bounds_size.x as u32,
            height: self.bounds_size.y as u32,
            format,
        };

        self.particle_renderer = Some(ParticleRendererComponent::new(
            device,
            output_config,
            self.default_particle_limit as u32,
            16, // max groups
            16, // max emitters
            ParticleRenderMode::Quads,
            ParticleBlendMode::Alpha,
        ));

        // Update bind groups to ensure they use the correctly sized buffers
        if let Some(ref mut renderer) = self.particle_renderer {
            renderer.update_bind_groups(device);
        }

        // Force field will be set later when needed - ParticleRendererComponent
        // manages its own force field instance
    }

    /// Create a drone with a mask and emitters (maintains original interface)
    pub fn make_drone_with(
        &mut self,
        id_generator: &mut IdGenerator,
        voice: Voice,
        circle: WindCircle,
        alpha: i32,
        num_particles: i32,
        trail: i32,
    ) -> Rect {
        #[cfg(not(feature = "gpu-particles"))]
        {
            return self.cpu_system.make_drone_with(
                id_generator,
                voice,
                circle,
                alpha,
                num_particles,
                trail,
            );
        }

        #[cfg(feature = "gpu-particles")]
        {
            if self.masks.contains_key(&voice) {
                self.masks.remove(&voice);
            }

            let mask = Mask::make_drone(voice);

            // Create GPU mask for culling
            let gpu_mask = GpuMask::from_system4_mask(
                [mask.origin.x, mask.origin.y],
                [mask.size.x, mask.size.y],
                voice as u32,
            );
            self.gpu_masks.insert(voice, gpu_mask);

            // Convert WindCircle to GpuWindCircle by accessing params
            let gpu_wind_circle = circle.with_params_read(|params| {
                GpuWindCircle::from_system4_params(
                    [params.center.x, params.center.y],
                    params.outer_radius,
                    params.inner_radius,
                    params.strength,
                    params.center_bias,
                )
            });
            self.wind_circles.insert(voice, gpu_wind_circle);

            // Create group parameters for this voice
            let mut group_params = ParticleGroupParams::new();
            group_params.alpha_multiplier = (alpha as f32) / 100.0;
            group_params.particle_limit =
                ((num_particles as f32 / 100.0) * self.default_particle_limit as f32) as u32;
            group_params.trail_amount = (trail as f32) / 100.0;
            self.group_params.insert(voice, group_params);
            self.params_dirty = true; // Mark parameters as needing GPU sync

            // Create GPU emitters
            let emitter_left_origin = vec2(mask.rect.left() - 20.0, mask.origin.y);
            let emitter_right_origin = vec2(mask.rect.right() + 20.0, mask.origin.y);

            let mut emitter_left = GpuEmitter::new([emitter_left_origin.x, emitter_left_origin.y]);
            emitter_left.group_id = voice as u32;
            emitter_left.emitter_id = id_generator.generate() as u32;
            emitter_left.set_velocity_range([10.0, -10.0], [50.0, 10.0]); // East direction
            self.gpu_emitters.push(emitter_left);

            let mut emitter_right =
                GpuEmitter::new([emitter_right_origin.x, emitter_right_origin.y]);
            emitter_right.group_id = voice as u32;
            emitter_right.emitter_id = id_generator.generate() as u32;
            emitter_right.set_velocity_range([-50.0, -10.0], [-10.0, 10.0]); // West direction
            self.gpu_emitters.push(emitter_right);

            let mut emitter_center = GpuEmitter::new([mask.origin.x, mask.origin.y]);
            emitter_center.group_id = voice as u32;
            emitter_center.emitter_id = id_generator.generate() as u32;
            emitter_center.set_velocity_range([-20.0, 0.0], [20.0, 50.0]); // Upward spread
            self.gpu_emitters.push(emitter_center);

            // Store limits and feedback
            self.alpha_limits.insert(voice, (alpha as f32) / 100.0);
            self.feedback.insert(voice, (trail as f32) / 100.0);
            self.particle_num_factors
                .insert(voice, (num_particles as f32) / 100.0);
            let mask_rect = mask.rect;
            self.masks.insert(voice, mask);

            mask_rect
        }
    }

    /// Synchronize group parameters to GPU renderer (call when parameters change)
    #[cfg(feature = "gpu-particles")]
    pub fn sync_group_params_to_gpu(
        &mut self,
        queue: &nannou::wgpu::Queue,
        renderer: &mut ParticleRendererComponent,
    ) {
        if self.params_dirty {
            let params_array = self.get_group_params_array();
            renderer.update_group_params(queue, &params_array);
            self.params_dirty = false;
        }
    }

    /// Force synchronization of all parameters to GPU (useful for initialization)
    #[cfg(feature = "gpu-particles")]
    pub fn force_sync_all_params_to_gpu(
        &mut self,
        queue: &nannou::wgpu::Queue,
        renderer: &mut ParticleRendererComponent,
    ) {
        // Sync group parameters
        let params_array = self.get_group_params_array();
        renderer.update_group_params(queue, &params_array);

        // Sync global parameters
        renderer.update_global_params(queue, &self.global_params);

        // Sync emitters
        renderer.update_emitters(queue, &self.gpu_emitters);

        self.params_dirty = false;
    }

    /// Update particle system (maintains original interface)
    pub fn update(&mut self, rng: &mut ThreadRng, show_forces: bool) -> Vec<Vec2> {
        #[cfg(not(feature = "gpu-particles"))]
        {
            return self.cpu_system.update(rng, show_forces);
        }

        #[cfg(feature = "gpu-particles")]
        {
            // Start performance monitoring for this frame
            self.performance_monitor.start_frame();

            // Update frame number and global parameters
            self.frame_number += 1;
            self.global_params.frame_number = self.frame_number;
            self.global_params.time_delta = 1.0 / 60.0; // Assume 60fps for now

            // Sync emitters with group parameters
            self.sync_emitters_with_groups();

            // Update emitter spawn accumulators (similar to CPU particle emission logic)
            self.handle_gpu_particle_emission(rng);

            // Handle mask-based particle culling
            self.handle_gpu_mask_culling();

            // Handle particle culling to maintain group limits
            self.handle_gpu_particle_culling();

            // Update force field if needed
            self.update_forces(show_forces);

            // Start timing GPU compute operations
            self.performance_monitor.start_compute();

            // End timing GPU compute operations
            self.performance_monitor.end_compute();

            // Calculate memory usage and active particles (estimated)
            let active_particles = self.get_particle_count() as u32;
            let memory_usage = self.estimate_memory_usage();

            // End frame performance monitoring
            self.performance_monitor.end_frame(
                active_particles,
                self.global_params.max_particle_count,
                memory_usage,
            );

            // Check performance and adapt quality if needed
            let analysis = self.performance_monitor.get_analysis();
            self.adaptive_quality.adapt_to_performance(&analysis);

            // Apply adaptive quality settings
            self.apply_adaptive_quality();

            // Return empty vec for now - GPU particles don't need CPU position readback
            Vec::new()
        }
    }

    /// Run GPU particle update pass (call this during rendering)
    #[cfg(feature = "gpu-particles")]
    pub fn update_gpu_particles(
        &mut self,
        _device: &nannou::wgpu::Device,
        queue: &nannou::wgpu::Queue,
    ) {
        // Compute group params array before borrowing renderer to avoid borrow checker issues
        let group_params_array = self.get_group_params_array();
        let emitters = self.gpu_emitters.clone(); // Clone emitters for GPU update

        if let Some(ref mut renderer) = self.particle_renderer {
            // Clear group counters before each frame to allow new particles to be emitted
            renderer.clear_group_counters(queue);

            // Update global parameters on GPU
            renderer.update_global_params(queue, &self.global_params);

            // Update group parameters on GPU
            renderer.update_group_params(queue, &group_params_array);

            // Update emitters on GPU
            renderer.update_emitters(queue, &emitters);
            
            // Debug: Print emitter status
            println!("Total emitters: {}", emitters.len());
            let mut active_count = 0;
            for (i, emitter) in emitters.iter().enumerate() {
                if emitter.is_active == 1 {
                    active_count += 1;
                    println!("Emitter {} active: group_id={}, spawn_accumulator={:.2}", i, emitter.group_id, emitter.spawn_accumulator);
                }
            }
            println!("Active emitters: {}", active_count);
        }

        // Mark parameters as synced
        self.params_dirty = false;
    }

    /// Render particles to a texture using GPU (integrate with Nnpipe)
    #[cfg(feature = "gpu-particles")]
    pub fn render_to_texture(
        &mut self,
        device: &nannou::wgpu::Device,
        encoder: &mut nannou::wgpu::CommandEncoder,
        output_view: &nannou::wgpu::TextureView,
    ) {
        use nnpipe::PipelineComponent;

        if let Some(ref mut renderer) = self.particle_renderer {
            // Ensure bind groups are updated before rendering
            renderer.update_bind_groups(device);
            renderer.encode_pass(encoder, output_view);
        }
    }

    /// Draw particles (maintains original interface)
    pub fn draw(&self, draw: &Draw) {
        #[cfg(not(feature = "gpu-particles"))]
        {
            self.cpu_system.draw(draw);
        }

        #[cfg(feature = "gpu-particles")]
        {
            // Render particles using GPU data structures but Nannou's draw API
            // This works without requiring the full GPU pipeline to be initialized
            self.draw_gpu_particles_with_nannou(draw);
        }
    }

    /// Get particle count (maintains original interface)
    pub fn get_particle_count(&self) -> usize {
        #[cfg(not(feature = "gpu-particles"))]
        {
            return self.cpu_system.get_particle_count();
        }

        #[cfg(feature = "gpu-particles")]
        {
            // TODO: Get count from GPU particle system
            0
        }
    }

    // All other methods delegate to CPU system or implement GPU equivalent

    pub fn kill_voice(&mut self, voice: &Voice) {
        #[cfg(not(feature = "gpu-particles"))]
        {
            self.cpu_system.kill_voice(voice);
        }

        #[cfg(feature = "gpu-particles")]
        {
            self.masks.remove(voice);
            self.gpu_masks.remove(voice); // Remove GPU mask
            self.group_params.remove(voice);
            self.wind_circles.remove(voice);
            self.alpha_limits.remove(voice);
            self.color_limits.remove(voice);
            self.particle_num_factors.remove(voice);
            self.feedback.remove(voice);

            // Remove emitters for this voice
            self.gpu_emitters
                .retain(|emitter| emitter.group_id != *voice as u32);

            self.params_dirty = true; // Mark parameters as needing GPU sync
        }
    }

    pub fn set_alpha_limit(&mut self, voice: &Voice, alpha: f32) {
        #[cfg(not(feature = "gpu-particles"))]
        {
            self.cpu_system.set_alpha_limit(voice, alpha);
        }

        #[cfg(feature = "gpu-particles")]
        {
            self.alpha_limits.insert(*voice, alpha);
            if let Some(group_params) = self.group_params.get_mut(voice) {
                group_params.alpha_multiplier = alpha;
                self.params_dirty = true;
            }
        }
    }

    pub fn set_feedback(&mut self, voice: &Voice, feedback: f32) {
        #[cfg(not(feature = "gpu-particles"))]
        {
            self.cpu_system.set_feedback(voice, feedback);
        }

        #[cfg(feature = "gpu-particles")]
        {
            self.feedback.insert(*voice, feedback);
            if let Some(group_params) = self.group_params.get_mut(voice) {
                group_params.trail_amount = feedback;
                self.params_dirty = true;
            }
        }
    }

    pub fn set_gravity(&mut self, voice: &Voice, gravity: f32) {
        #[cfg(not(feature = "gpu-particles"))]
        {
            self.cpu_system.set_gravity(voice, gravity);
        }

        #[cfg(feature = "gpu-particles")]
        {
            // Gravity was removed from GPU system - this is a no-op
            // Could add custom force field support here if needed
        }
    }

    pub fn set_is_spawning(&mut self, voice: &Voice, is_spawning: bool) {
        #[cfg(not(feature = "gpu-particles"))]
        {
            self.cpu_system.set_is_spawning(voice, is_spawning);
        }

        #[cfg(feature = "gpu-particles")]
        {
            println!("set_is_spawning called for voice {:?}, is_spawning: {}", voice, is_spawning);
            if let Some(group_params) = self.group_params.get_mut(voice) {
                group_params.is_spawning = if is_spawning { 1 } else { 0 };
                self.params_dirty = true;
            }

            // Update emitters for this voice
            let voice_id = *voice as u32;
            for emitter in &mut self.gpu_emitters {
                if emitter.group_id == voice_id {
                    emitter.is_active = if is_spawning { 1 } else { 0 };
                }
            }
        }
    }

    pub fn set_strength(&mut self, voice: &Voice, strength: f32) {
        #[cfg(not(feature = "gpu-particles"))]
        {
            self.cpu_system.set_strength(voice, strength);
        }

        #[cfg(feature = "gpu-particles")]
        {
            if let Some(wind_circle) = self.wind_circles.get_mut(voice) {
                wind_circle.strength = strength;
            }
        }
    }

    pub fn set_num_particles(&mut self, voice: &Voice, num_particles: f32) {
        #[cfg(not(feature = "gpu-particles"))]
        {
            self.cpu_system.set_num_particles(voice, num_particles);
        }

        #[cfg(feature = "gpu-particles")]
        {
            self.particle_num_factors.insert(*voice, num_particles);
            if let Some(group_params) = self.group_params.get_mut(voice) {
                group_params.particle_limit =
                    (num_particles * self.default_particle_limit as f32) as u32;
                self.params_dirty = true;
            }
        }
    }

    pub fn set_radius_outer(&mut self, voice: &Voice, val: f32) {
        #[cfg(not(feature = "gpu-particles"))]
        {
            self.cpu_system.set_radius_outer(voice, val);
        }

        #[cfg(feature = "gpu-particles")]
        {
            if let Some(wind_circle) = self.wind_circles.get_mut(voice) {
                wind_circle.radius = val;
            }
        }
    }

    pub fn set_radius_inner(&mut self, voice: &Voice, val: f32) {
        #[cfg(not(feature = "gpu-particles"))]
        {
            self.cpu_system.set_radius_inner(voice, val);
        }

        #[cfg(feature = "gpu-particles")]
        {
            if let Some(wind_circle) = self.wind_circles.get_mut(voice) {
                wind_circle.inner_radius = val;
            }
        }
    }

    pub fn get_live_particle_positions(&self) -> Vec<Vec2> {
        #[cfg(not(feature = "gpu-particles"))]
        {
            return self.cpu_system.get_live_particle_positions();
        }

        #[cfg(feature = "gpu-particles")]
        {
            // TODO: Read back positions from GPU
            Vec::new()
        }
    }

    pub fn change_bounds_size_to(&mut self, width: f32, height: f32) {
        #[cfg(not(feature = "gpu-particles"))]
        {
            self.cpu_system.change_bounds_size_to(width, height);
        }

        #[cfg(feature = "gpu-particles")]
        {
            self.bounds_size = Vec2::new(width, height);
            self.bounds_rect = Rect::from_x_y_w_h(self.origin.x, self.origin.y, width, height);
            self.global_params.set_bounds(
                self.bounds_rect.left(),
                self.bounds_rect.bottom(),
                self.bounds_rect.right(),
                self.bounds_rect.top(),
            );
            // Bounds change affects culling, so mark params as dirty
            self.params_dirty = true;
        }
    }

    /// Update bounds origin (useful for camera movement)
    #[cfg(feature = "gpu-particles")]
    pub fn set_bounds_origin(&mut self, origin: Point2) {
        self.origin = origin;
        self.bounds_rect =
            Rect::from_x_y_w_h(origin.x, origin.y, self.bounds_size.x, self.bounds_size.y);
        self.global_params.set_bounds(
            self.bounds_rect.left(),
            self.bounds_rect.bottom(),
            self.bounds_rect.right(),
            self.bounds_rect.top(),
        );
        self.params_dirty = true;
    }

    /// Get current bounds rectangle
    pub fn get_bounds(&self) -> Rect {
        self.bounds_rect
    }

    // Drawing methods for forces and debug info
    pub fn draw_forces(&self, draw: &Draw, scale_x: f32, scale_y: f32) {
        #[cfg(not(feature = "gpu-particles"))]
        {
            self.cpu_system.draw_forces(draw, scale_x, scale_y);
        }

        #[cfg(feature = "gpu-particles")]
        {
            self.draw_origin(draw, scale_x, scale_y);

            // Draw wind circles
            for wind_circle in self.wind_circles.values() {
                let center = vec2(wind_circle.center[0], wind_circle.center[1]);
                draw.ellipse()
                    .xy(center * vec2(scale_x, scale_y))
                    .radius(wind_circle.radius * scale_x.min(scale_y))
                    .stroke(WHITE)
                    .stroke_weight(1.0)
                    .no_fill();

                if wind_circle.inner_radius > 0.0 {
                    draw.ellipse()
                        .xy(center * vec2(scale_x, scale_y))
                        .radius(wind_circle.inner_radius * scale_x.min(scale_y))
                        .stroke(GRAY)
                        .stroke_weight(1.0)
                        .no_fill();
                }
            }
        }
    }

    pub fn draw_origin(&self, draw: &Draw, scale_x: f32, scale_y: f32) {
        draw.ellipse()
            .xy(self.origin * vec2(scale_x, scale_y))
            .radius(5.0)
            .color(RED);
    }

    pub fn draw_emitters(&self, draw: &Draw, scale_x: f32, scale_y: f32) {
        #[cfg(not(feature = "gpu-particles"))]
        {
            self.cpu_system.draw_emitters(draw, scale_x, scale_y);
        }

        #[cfg(feature = "gpu-particles")]
        {
            for emitter in &self.gpu_emitters {
                let pos = vec2(emitter.position[0], emitter.position[1]);
                draw.ellipse()
                    .xy(pos * vec2(scale_x, scale_y))
                    .radius(3.0)
                    .color(YELLOW);
            }
        }
    }

    pub fn draw_heatmap_background(&self, draw: &Draw, bounds: Rect) {
        #[cfg(not(feature = "gpu-particles"))]
        {
            self.cpu_system.draw_heatmap_background(draw, bounds);
        }

        #[cfg(feature = "gpu-particles")]
        {
            // TODO: Implement GPU heatmap rendering
        }
    }

    pub fn draw_analytical_heatmap_background(&self, draw: &Draw, bounds: Rect) {
        #[cfg(not(feature = "gpu-particles"))]
        {
            self.cpu_system
                .draw_analytical_heatmap_background(draw, bounds);
        }

        #[cfg(feature = "gpu-particles")]
        {
            // TODO: Implement GPU analytical heatmap rendering
        }
    }

    // GPU-specific methods

    #[cfg(feature = "gpu-particles")]
    fn handle_gpu_particle_emission(&mut self, rng: &mut ThreadRng) {
        // Update spawn accumulators for each emitter (similar to CPU emission logic)
        for emitter in &mut self.gpu_emitters {
            if emitter.is_active == 0 {
                continue;
            }

            // Get group params for this emitter
            let group_id = emitter.group_id;
            let Some(group_params) = self.group_params.get(&Voice::from_i32(group_id as i32))
            else {
                continue;
            };

            if group_params.is_spawning == 0 {
                continue;
            }

            // Calculate effective spawn rate (similar to CPU particle_system.rs line 255-265)
            let effective_spawn_rate = group_params.spawn_rate
                * group_params.spawn_rate_factor
                * self.global_max_spawn_rate
                / 60.0; // Normalize to per-frame rate

            // Accumulate fractional particles over multiple frames
            emitter.spawn_accumulator += effective_spawn_rate;

            // Spawn integer number of particles when accumulator reaches 1.0
            while emitter.spawn_accumulator >= 1.0 {
                // Add randomization similar to CPU should_emit_particle logic
                let emission_probability = effective_spawn_rate / 30.0;
                if rng.gen::<f32>() < emission_probability.min(1.0) {
                    // Particle will be spawned by GPU compute shader
                    // For now, just decrement accumulator
                    emitter.spawn_accumulator -= 1.0;
                } else {
                    emitter.spawn_accumulator -= 1.0;
                }
            }
        }
    }

    #[cfg(feature = "gpu-particles")]
    fn update_gpu_force_field(&mut self) {
        if let Some(_force_field) = &mut self.force_field {
            // Update wind circles in force field
            let _wind_circles: Vec<GpuWindCircle> = self.wind_circles.values().cloned().collect();

            // TODO: Update force field with current wind circles
            // This will require access to the GPU queue, which should be passed in
            // force_field.update_wind_circles(queue, &wind_circles);

            // TODO: Dispatch compute shader to generate force texture
            // force_field.compute_forces(encoder);
        }
    }

    /// Update emitter parameters when group parameters change
    #[cfg(feature = "gpu-particles")]
    fn sync_emitters_with_groups(&mut self) {
        for emitter in &mut self.gpu_emitters {
            let voice = Voice::from_i32(emitter.group_id as i32);

            // Update emitter activity based on group parameters
            if let Some(group_params) = self.group_params.get(&voice) {
                emitter.is_active = group_params.is_spawning;
            }
        }
    }

    /// Handle mask-based particle culling (GPU implementation of original CPU mask culling)
    #[cfg(feature = "gpu-particles")]
    fn handle_gpu_mask_culling(&mut self) {
        if let Some(culling_component) = &mut self.culling_component {
            // Update mask data in GPU buffer
            let gpu_masks: Vec<GpuMask> = self.gpu_masks.values().cloned().collect();

            // TODO: This requires access to the GPU queue, which should be passed in
            // culling_component.update_masks(queue, &gpu_masks);

            // TODO: Run mask culling compute pass
            // culling_component.run_culling(encoder, particle_buffer, group_counters_buffer, particle_count, device);
        }
    }

    /// Handle particle culling to maintain group limits (similar to CPU cull_excess_particles)
    #[cfg(feature = "gpu-particles")]
    fn handle_gpu_particle_culling(&mut self) {
        // TODO: Implement GPU-based particle culling
        // This should run a compute shader to:
        // 1. Count particles per group
        // 2. Mark excess particles as dead
        // 3. Update group counters
    }

    /// Add a WindCircle to this particle system (matches ForceFields interface)
    #[cfg(feature = "gpu-particles")]
    pub fn add_wind_circle(&mut self, circle: WindCircle) {
        println!("Added GPU wind circle {}", circle.id);

        // Convert CPU WindCircle to GpuWindCircle by accessing params
        let gpu_wind_circle = circle.with_params_read(|params| {
            GpuWindCircle::from_system4_params(
                [params.center.x, params.center.y],
                params.outer_radius,
                params.inner_radius,
                params.strength,
                params.center_bias,
            )
        });

        // Store by parent voice
        self.wind_circles
            .insert(circle.parent_voice, gpu_wind_circle);
    }

    #[cfg(not(feature = "gpu-particles"))]
    pub fn add_wind_circle(&mut self, circle: WindCircle) {
        self.cpu_system.forces.add_wind_circle(circle);
    }

    /// Update all wind circles (matches ForceFields interface)
    #[cfg(feature = "gpu-particles")]
    pub fn update_forces(&mut self, show_forces: bool) {
        if show_forces {
            self.update_gpu_force_field();
        }
    }

    #[cfg(not(feature = "gpu-particles"))]
    pub fn update_forces(&mut self, show_forces: bool) {
        self.cpu_system.forces.update(show_forces);
    }

    /// Force update all forces (matches ForceFields interface)
    #[cfg(feature = "gpu-particles")]
    pub fn force_update_all_forces(&mut self) {
        self.update_gpu_force_field();
    }

    #[cfg(not(feature = "gpu-particles"))]
    pub fn force_update_all_forces(&mut self) {
        self.cpu_system.forces.force_update_all();
    }

    /// Get wind circle parameters by voice (adapted from ForceFields interface)
    #[cfg(feature = "gpu-particles")]
    pub fn get_wind_circle_params(&self, voice: &Voice) -> Option<&GpuWindCircle> {
        self.wind_circles.get(voice)
    }

    #[cfg(not(feature = "gpu-particles"))]
    pub fn get_wind_circle_params(&self, voice: &Voice) -> Option<WindCircle> {
        // Convert voice to circle id and get from CPU system
        let circle_id = voice.to_i32() as usize;
        self.cpu_system.forces.wind_circles.get(&circle_id).cloned()
    }

    /// Update wind circle parameters by voice
    #[cfg(feature = "gpu-particles")]
    pub fn update_wind_circle(&mut self, voice: &Voice, params: &GpuWindCircle) {
        if let Some(circle) = self.wind_circles.get_mut(voice) {
            *circle = *params;
        }
    }

    /// Set color tint for a voice group
    #[cfg(feature = "gpu-particles")]
    pub fn set_color_tint(&mut self, voice: &Voice, color: [f32; 3]) {
        if let Some(group_params) = self.group_params.get_mut(voice) {
            group_params.color_tint = color;
            self.params_dirty = true;
        }
    }

    #[cfg(not(feature = "gpu-particles"))]
    pub fn set_color_tint(&mut self, voice: &Voice, color: [f32; 3]) {
        // CPU system doesn't support per-voice color tint - this is a no-op
    }

    /// Set size multiplier for a voice group
    #[cfg(feature = "gpu-particles")]
    pub fn set_size_multiplier(&mut self, voice: &Voice, multiplier: f32) {
        if let Some(group_params) = self.group_params.get_mut(voice) {
            group_params.size_multiplier = multiplier;
            self.params_dirty = true;
        }
    }

    #[cfg(not(feature = "gpu-particles"))]
    pub fn set_size_multiplier(&mut self, voice: &Voice, multiplier: f32) {
        // CPU system doesn't have separate size multiplier - this is a no-op
    }

    /// Set physics scale for a voice group
    #[cfg(feature = "gpu-particles")]
    pub fn set_physics_scale(&mut self, voice: &Voice, scale: f32) {
        if let Some(group_params) = self.group_params.get_mut(voice) {
            group_params.physics_scale = scale;
            self.params_dirty = true;
        }
    }

    #[cfg(not(feature = "gpu-particles"))]
    pub fn set_physics_scale(&mut self, voice: &Voice, scale: f32) {
        // CPU system doesn't have physics scale - this is a no-op
    }

    /// Set group visibility
    #[cfg(feature = "gpu-particles")]
    pub fn set_group_visible(&mut self, voice: &Voice, visible: bool) {
        if let Some(group_params) = self.group_params.get_mut(voice) {
            group_params.is_visible = if visible { 1 } else { 0 };
            self.params_dirty = true;
        }
    }

    #[cfg(not(feature = "gpu-particles"))]
    pub fn set_group_visible(&mut self, voice: &Voice, visible: bool) {
        // CPU system visibility is handled differently - this is a no-op for now
    }

    /// Set spawn rate factor for a voice group
    #[cfg(feature = "gpu-particles")]
    pub fn set_spawn_rate_factor(&mut self, voice: &Voice, factor: f32) {
        if let Some(group_params) = self.group_params.get_mut(voice) {
            group_params.spawn_rate_factor = factor;
            self.params_dirty = true;
        }
    }

    #[cfg(not(feature = "gpu-particles"))]
    pub fn set_spawn_rate_factor(&mut self, voice: &Voice, factor: f32) {
        // CPU system handles spawn rates differently - this is a no-op
    }

    /// Batch update multiple parameters for a voice group to minimize GPU uploads
    #[cfg(feature = "gpu-particles")]
    pub fn batch_update_voice_params<F>(&mut self, voice: &Voice, update_fn: F)
    where
        F: FnOnce(&mut ParticleGroupParams),
    {
        if let Some(group_params) = self.group_params.get_mut(voice) {
            update_fn(group_params);
            self.params_dirty = true;
        }
    }

    /// Get current group parameters for a voice (useful for debugging/inspection)
    #[cfg(feature = "gpu-particles")]
    pub fn get_group_params(&self, voice: &Voice) -> Option<&ParticleGroupParams> {
        self.group_params.get(voice)
    }

    /// Get all active voice groups
    #[cfg(feature = "gpu-particles")]
    pub fn get_active_voices(&self) -> Vec<Voice> {
        self.group_params.keys().cloned().collect()
    }

    /// Prepare group parameters array for GPU upload (in voice order)
    #[cfg(feature = "gpu-particles")]
    pub fn get_group_params_array(&self) -> Vec<ParticleGroupParams> {
        let mut params_array = vec![ParticleGroupParams::default(); 16]; // Support up to 16 groups

        for (voice, params) in &self.group_params {
            let group_index = voice.to_i32() as usize;
            if group_index > 0 && group_index < params_array.len() {
                params_array[group_index] = *params;
            }
        }

        params_array
    }

    /// Enable or disable bounds culling for particles
    #[cfg(feature = "gpu-particles")]
    pub fn set_bounds_culling_enabled(&mut self, enabled: bool) {
        // This will be handled by the renderer component when it's available
        // For now, store the preference in global params or a flag
        // The renderer will be updated when sync methods are called
    }

    /// Check if a voice has active particles/groups
    #[cfg(feature = "gpu-particles")]
    pub fn is_voice_active(&self, voice: &Voice) -> bool {
        self.group_params.contains_key(voice)
            && self
                .group_params
                .get(voice)
                .map(|params| params.is_visible == 1)
                .unwrap_or(false)
    }

    /// Get voice particle statistics
    #[cfg(feature = "gpu-particles")]
    pub fn get_voice_stats(&self, voice: &Voice) -> Option<(u32, u32, f32)> {
        self.group_params.get(voice).map(|params| {
            (
                params.current_count,
                params.particle_limit,
                params.alpha_multiplier,
            )
        })
    }

    /// Reset all voice parameters to defaults (useful for cleanup)
    #[cfg(feature = "gpu-particles")]
    pub fn reset_voice_params(&mut self, voice: &Voice) {
        if let Some(group_params) = self.group_params.get_mut(voice) {
            *group_params = ParticleGroupParams::new();
            self.params_dirty = true;
        }
    }

    /// Validate voice parameter ranges (clamp to safe values)
    #[cfg(feature = "gpu-particles")]
    pub fn validate_voice_params(&mut self, voice: &Voice) {
        if let Some(group_params) = self.group_params.get_mut(voice) {
            // Clamp values to reasonable ranges
            group_params.alpha_multiplier = group_params.alpha_multiplier.clamp(0.0, 2.0);
            group_params.size_multiplier = group_params.size_multiplier.clamp(0.1, 10.0);
            group_params.spawn_rate = group_params.spawn_rate.clamp(0.0, 1000.0);
            group_params.spawn_rate_factor = group_params.spawn_rate_factor.clamp(0.0, 5.0);
            group_params.trail_amount = group_params.trail_amount.clamp(0.0, 1.0);
            group_params.physics_scale = group_params.physics_scale.clamp(0.1, 5.0);
            group_params.particle_limit = group_params
                .particle_limit
                .min(self.default_particle_limit as u32);

            // Ensure color components are valid
            for i in 0..3 {
                group_params.color_tint[i] = group_params.color_tint[i].clamp(0.0, 2.0);
            }

            self.params_dirty = true;
        }
    }

    #[cfg(not(feature = "gpu-particles"))]
    pub fn update_wind_circle_cpu(&mut self, voice: &Voice, circle: WindCircle) {
        let circle_id = voice.to_i32() as usize;
        self.cpu_system
            .forces
            .wind_circles
            .insert(circle_id, circle);
    }

    /// Remove wind circle by voice
    #[cfg(feature = "gpu-particles")]
    pub fn remove_wind_circle(&mut self, voice: &Voice) {
        self.wind_circles.remove(voice);
    }

    #[cfg(not(feature = "gpu-particles"))]
    pub fn remove_wind_circle(&mut self, voice: &Voice) {
        let circle_id = voice.to_i32() as usize;
        self.cpu_system.forces.wind_circles.remove(&circle_id);
    }

    /// Enable or disable debug visualization
    #[cfg(feature = "gpu-particles")]
    pub fn set_debug_enabled(&mut self, enabled: bool) {
        if enabled && self.debug_renderer.is_none() {
            self.debug_renderer = Some(ParticleDebugRenderer::new());
        } else if !enabled {
            self.debug_renderer = None;
        }
    }

    /// Get debug configuration for modification
    #[cfg(feature = "gpu-particles")]
    pub fn get_debug_config(&mut self) -> Option<&mut ParticleDebugConfig> {
        self.debug_renderer
            .as_mut()
            .map(|r| r.get_config().clone())
            .map(|c| {
                // Return a mutable reference through the debug renderer
                if let Some(ref mut renderer) = self.debug_renderer {
                    renderer.set_config(c);
                    // Note: This is a bit awkward, but we need to return a reference
                    // In a real implementation, we'd probably restructure this
                }
                // For now, just return None to indicate we can't provide a mutable ref
                None
            })
            .flatten()
    }

    /// Update debug visualization with current particle data
    #[cfg(feature = "gpu-particles")]
    pub fn update_debug_data(&mut self) {
        if let Some(debug_renderer) = &mut self.debug_renderer {
            // Get current particles from renderer if available
            let particles = if let Some(ref renderer) = self.particle_renderer {
                // In a real implementation, we'd extract particle data from GPU buffers
                // For now, use empty data
                vec![]
            } else {
                vec![]
            };

            debug_renderer.update_particle_data(
                &particles,
                &self.gpu_emitters,
                &self.group_params,
                &self.global_params,
                &self.wind_circles,
            );
        }
    }

    /// Draw debug overlay
    #[cfg(feature = "gpu-particles")]
    pub fn draw_debug(&self, draw: &Draw, window_bounds: Rect, scale_x: f32, scale_y: f32) {
        if let Some(ref debug_renderer) = self.debug_renderer {
            debug_renderer.draw_debug(draw, self.bounds_rect, scale_x, scale_y);
            debug_renderer.draw_metrics_overlay(draw, window_bounds);
        }
    }

    /// Get debug statistics
    #[cfg(feature = "gpu-particles")]
    pub fn get_debug_stats(&self) -> Option<&nnpipe::ParticleDebugStats> {
        self.debug_renderer.as_ref().map(|r| r.get_stats())
    }

    /// Toggle specific debug features
    #[cfg(feature = "gpu-particles")]
    pub fn toggle_debug_feature(&mut self, feature: &str, enabled: bool) {
        if let Some(ref mut debug_renderer) = self.debug_renderer {
            let mut config = debug_renderer.get_config().clone();
            match feature {
                "particles" => config.show_particles = enabled,
                "velocities" => config.show_velocities = enabled,
                "trails" => config.show_trails = enabled,
                "emitters" => config.show_emitters = enabled,
                "forces" => config.show_forces = enabled,
                "group_bounds" => config.show_group_bounds = enabled,
                "lifecycle" => config.show_lifecycle = enabled,
                "metrics" => config.show_metrics = enabled,
                "gpu_stats" => config.show_gpu_stats = enabled,
                _ => {}
            }
            debug_renderer.set_config(config);
        }
    }

    /// Get wind circle parameters by voice (compatibility method)
    #[cfg(feature = "gpu-particles")]
    pub fn get_circle_params_by_voice(&self, voice: Voice) -> Vec<GpuWindCircle> {
        if let Some(circle) = self.wind_circles.get(&voice) {
            vec![*circle]
        } else {
            vec![]
        }
    }

    #[cfg(not(feature = "gpu-particles"))]
    pub fn get_circle_params_by_voice(&self, voice: Voice) -> Vec<&crate::forces::WindCircle> {
        let voice_id = voice.to_i32() as usize;
        if let Some(circle) = self.cpu_system.forces.wind_circles.get(&voice_id) {
            vec![circle]
        } else {
            vec![]
        }
    }

    /// Get all wind circle parameters (compatibility method)
    #[cfg(feature = "gpu-particles")]
    pub fn get_circle_params_all(&self) -> HashMap<usize, GpuWindCircle> {
        let mut result = HashMap::new();
        for (voice, circle) in &self.wind_circles {
            result.insert(voice.to_i32() as usize, *circle);
        }
        result
    }

    #[cfg(not(feature = "gpu-particles"))]
    pub fn get_circle_params_all(&self) -> &BTreeMap<usize, crate::forces::WindCircle> {
        &self.cpu_system.forces.wind_circles
    }

    /// Get mutable reference to wind circle by ID (compatibility method)
    #[cfg(feature = "gpu-particles")]
    pub fn get_wind_circle_mut(&mut self, voice_id: usize) -> Option<&mut GpuWindCircle> {
        let voice = Voice::from_i32(voice_id as i32);
        self.wind_circles.get_mut(&voice)
    }

    #[cfg(not(feature = "gpu-particles"))]
    pub fn get_wind_circle_mut(
        &mut self,
        voice_id: usize,
    ) -> Option<&mut crate::forces::WindCircle> {
        self.cpu_system.forces.wind_circles.get_mut(&voice_id)
    }

    /// Simple drone creation method for GPU particles (compatibility)
    #[cfg(feature = "gpu-particles")]
    pub fn make_drone_simple(
        &mut self,
        voice: Voice,
        center: Point2,
        outer_radius: f32,
        inner_radius: f32,
        strength: f32,
        center_bias: f32,
        alpha: i32,
        num_particles: i32,
        trail: i32,
    ) -> Rect {
        // Create GPU wind circle
        let wind_circle = GpuWindCircle {
            center: [center.x, center.y],
            radius: outer_radius,
            strength,
            rotation_speed: 0.0,
            inner_radius,
            falloff_power: 2.0,
            direction_bias: [center_bias, 0.0], // Convert center_bias to direction_bias
            is_active: 1,                       // Active
            _padding: 0.0,
        };

        // Store the wind circle
        self.wind_circles.insert(voice, wind_circle);

        // Set particle parameters
        let alpha_f = alpha as f32 / 100.0;
        let volume = num_particles as f32 / 100.0;
        let trail_f = trail as f32 / 100.0;

        self.set_alpha_limit(&voice, alpha_f);
        self.set_num_particles(&voice, volume);
        self.set_feedback(&voice, trail_f);
        self.set_is_spawning(&voice, true);

        // Create group parameters for this voice
        let mut group_params = ParticleGroupParams::new();
        group_params.alpha_multiplier = alpha_f;
        group_params.particle_limit = ((volume) * self.default_particle_limit as f32) as u32;
        group_params.trail_amount = trail_f;
        self.group_params.insert(voice, group_params);
        self.params_dirty = true;

        // Create GPU emitters (same as make_drone_with)
        let emitter_left_origin = vec2(center.x - outer_radius - 20.0, center.y);
        let emitter_right_origin = vec2(center.x + outer_radius + 20.0, center.y);

        let mut emitter_left = GpuEmitter::new([emitter_left_origin.x, emitter_left_origin.y]);
        emitter_left.group_id = voice as u32;
        emitter_left.emitter_id = self.gpu_emitters.len() as u32; // Simple ID
        emitter_left.set_velocity_range([10.0, -10.0], [50.0, 10.0]); // East direction
        self.gpu_emitters.push(emitter_left);

        let mut emitter_right = GpuEmitter::new([emitter_right_origin.x, emitter_right_origin.y]);
        emitter_right.group_id = voice as u32;
        emitter_right.emitter_id = self.gpu_emitters.len() as u32;
        emitter_right.set_velocity_range([-50.0, -10.0], [-10.0, 10.0]); // West direction
        self.gpu_emitters.push(emitter_right);

        let mut emitter_center = GpuEmitter::new([center.x, center.y]);
        emitter_center.group_id = voice as u32;
        emitter_center.emitter_id = self.gpu_emitters.len() as u32;
        emitter_center.set_velocity_range([-20.0, 0.0], [20.0, 50.0]); // Upward spread
        self.gpu_emitters.push(emitter_center);

        println!("Created {} emitters for voice {:?}", 3, voice);

        // Create a mask rect (simplified for GPU)
        let mask_size = outer_radius * 2.0;
        Rect::from_x_y_w_h(center.x, center.y, mask_size, mask_size)
    }

    // === Performance Monitoring Methods ===

    /// Estimate current memory usage in bytes
    #[cfg(feature = "gpu-particles")]
    fn estimate_memory_usage(&self) -> u64 {
        let particle_buffer_size = (self.global_params.max_particle_count as u64)
            * std::mem::size_of::<GpuParticle>() as u64;
        let emitter_buffer_size =
            (self.gpu_emitters.len() as u64) * std::mem::size_of::<GpuEmitter>() as u64;
        let group_buffer_size = 16 * std::mem::size_of::<ParticleGroupParams>() as u64; // Up to 16 groups
        let wind_buffer_size =
            (self.wind_circles.len() as u64) * std::mem::size_of::<GpuWindCircle>() as u64;
        let global_buffer_size = std::mem::size_of::<ParticleGlobalParams>() as u64;

        // Add estimated GPU texture sizes (force field, etc.)
        let force_field_size = 256 * 256 * 4 * 4; // RGBA32F texture

        particle_buffer_size
            + emitter_buffer_size
            + group_buffer_size
            + wind_buffer_size
            + global_buffer_size
            + force_field_size
    }

    /// Apply adaptive quality settings to system parameters
    #[cfg(feature = "gpu-particles")]
    fn apply_adaptive_quality(&mut self) {
        // Update max particle count if it changed
        if self.global_params.max_particle_count != self.adaptive_quality.max_particles {
            self.global_params.max_particle_count = self.adaptive_quality.max_particles;
            self.params_dirty = true;
        }

        // Update force field resolution (would need GPU component support)
        // Update trail segments (would affect shader parameters)
        // Update update frequency (could skip frames if needed)
    }

    /// Get current performance metrics
    #[cfg(feature = "gpu-particles")]
    pub fn get_performance_metrics(&self) -> &ParticlePerformanceMonitor {
        &self.performance_monitor
    }

    /// Get adaptive quality settings
    #[cfg(feature = "gpu-particles")]
    pub fn get_adaptive_quality(&self) -> &AdaptiveQualitySettings {
        &self.adaptive_quality
    }

    /// Set custom performance targets
    #[cfg(feature = "gpu-particles")]
    pub fn set_performance_targets(&mut self, targets: PerformanceTargets) {
        self.performance_monitor.set_targets(targets);
    }

    /// Export performance data as CSV for analysis
    #[cfg(feature = "gpu-particles")]
    pub fn export_performance_csv(&self) -> String {
        self.performance_monitor.export_csv()
    }

    /// Check if system is meeting performance targets
    #[cfg(feature = "gpu-particles")]
    pub fn is_performance_healthy(&self) -> bool {
        self.performance_monitor.is_meeting_targets()
    }

    /// Get performance analysis and recommendations
    #[cfg(feature = "gpu-particles")]
    pub fn get_performance_analysis(&self) -> PerformanceAnalysis {
        self.performance_monitor.get_analysis()
    }

    /// Render GPU particles using Nannou's draw API while building proper GPU pipeline
    #[cfg(feature = "gpu-particles")]
    fn draw_gpu_particles_with_nannou(&self, draw: &Draw) {
        // Create mock particle data for immediate rendering
        let mut particles = Vec::new();

        for (voice, group_params) in &self.group_params {
            if group_params.is_visible == 0 || group_params.is_spawning == 0 {
                continue;
            }

            // Get emitter position from wind circle or default
            let emitter_pos = if let Some(wind_circle) = self.wind_circles.get(voice) {
                pt2(wind_circle.center[0], wind_circle.center[1])
            } else {
                pt2(0.0, 0.0)
            };

            // Generate particles for this group
            let particle_count =
                (group_params.spawn_rate * group_params.particle_limit as f32) as u32;
            let particle_count = particle_count.max(20).min(200); // Ensure visible particles

            for i in 0..particle_count {
                let time_offset = (self.frame_number as f32 + i as f32) * 0.02;
                let angle = (i as f32 / particle_count as f32) * 2.0 * PI + time_offset;
                let spiral_radius = 30.0 + (i as f32 * 2.5);

                // Add some random variation
                let variation = (time_offset * 3.0).sin() * 10.0;
                let pos = emitter_pos
                    + pt2(
                        angle.cos() * (spiral_radius + variation),
                        angle.sin() * (spiral_radius + variation),
                    );

                particles.push(GpuParticle {
                    position: [pos.x, pos.y],
                    velocity: [angle.cos() * 20.0, angle.sin() * 20.0],
                    acceleration: [0.0, 0.0],
                    trail_positions: [[0.0; 2]; 8],
                    color: [
                        0.5 + (voice.to_i32() as f32 * 0.3) % 0.5, // R
                        0.3 + (voice.to_i32() as f32 * 0.7) % 0.7, // G
                        0.8,                                       // B
                    ],
                    alpha: group_params.alpha_multiplier,
                    size: group_params.size_multiplier * 3.0,
                    mass: 1.0,
                    age: time_offset * 0.5,
                    remaining_life_span: 5.0, // 5 second lifetime
                    age_per_tick: 1.0 / 60.0,
                    fade_in_duration: 0.1,
                    fade_out_duration: 0.3,
                    group_id: voice.to_i32() as u32,
                    sub_group_id: 0,
                    flags: 1, // alive
                    _padding: [0.0; 4],
                });
            }
        }

        // Render particles using nannou
        for particle in particles {
            if particle.is_alive() {
                let pos = pt2(particle.position[0], particle.position[1]);
                let color = rgba(
                    particle.color[0],
                    particle.color[1],
                    particle.color[2],
                    particle.alpha,
                );

                draw.ellipse().xy(pos).radius(particle.size).color(color);
            }
        }

        // Draw wind circles for debugging
        if let Some(debug_renderer) = &self.debug_renderer {
            let debug_config = debug_renderer.get_config();
            if debug_config.show_forces {
                for wind_circle in self.wind_circles.values() {
                    let center = pt2(wind_circle.center[0], wind_circle.center[1]);

                    // Draw outer circle
                    draw.ellipse()
                        .xy(center)
                        .radius(wind_circle.radius)
                        .stroke(YELLOW)
                        .stroke_weight(2.0)
                        .no_fill();

                    // Draw inner circle if it exists
                    if wind_circle.inner_radius > 0.0 {
                        draw.ellipse()
                            .xy(center)
                            .radius(wind_circle.inner_radius)
                            .stroke(rgba(1.0, 0.8, 0.0, 0.7))
                            .stroke_weight(1.0)
                            .no_fill();
                    }

                    // Draw strength indicator
                    draw.text(&format!("F:{:.0}", wind_circle.strength))
                        .xy(center + vec2(0.0, wind_circle.radius + 15.0))
                        .color(YELLOW)
                        .font_size(12);
                }
            }
        }
    }
}
