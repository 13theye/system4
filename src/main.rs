// System 4
//
// (c) 2025 13th Eye LLC & Tacit Group
//
//
// src/main.rs

use nannou::{prelude::*, rand::rngs::ThreadRng, text::Font};
use nannou_egui::Egui;
use nnpipe::renderers::{HeatmapRenderer, ParticleRenderer, SegmentParams, SegmentRenderer};
use nnpipe::*;
use system4::voice::controller;
use thread_priority::*;

use std::cell::RefCell;
use std::{
    collections::HashMap,
    fs,
    sync::atomic::{AtomicU64, Ordering},
};

use system4::{
    config::*,
    forces::WindCircle,
    fps::FpsManager,
    model::Model,
    osc::{OscCommand, OscController, OscSender},
    particle::{ParticleSystem, EMPTY_GPU_BUFFER},
    terminals::{TerminalParams, TerminalSystem},
    utils::IdGenerator,
    voice::{controller::VoiceParameterChange, Voice},
};

const DEFAULT_PARTICLE_SIZE: f32 = 4.0;
const DEFAULT_PARTICLE_RGB: (f32, f32, f32) = (0.73, 0.73, 0.74);
//const DEFAULT_PARTICLE_RGB: (f32, f32, f32) = (0.27, 0.27, 0.26);
// full brightness color for terminal
const TERMINAL_START_RGBA: (f32, f32, f32, f32) = (0.0, 0.85, 0.0, 1.0);
// dimmed brightness color for terminal
const TERMINAL_END_RGBA: (f32, f32, f32, f32) = (0.0, 0.3, 0.0, 1.0);
const TERMINAL_NUM_LINES: usize = 8;
const TERMINAL_LINE_MARGIN: f32 = 8.0;
const TERMINAL_CHARS_PER_SECOND: f32 = 0.6; // final is 0.6

fn model(app: &App) -> Model {
    // Load config
    let config = Config::load().expect("\nSystem 4: FAILED TO LOAD CONFIG.TOML\n");

    // Main game data elements
    let particle_limit = config.particles.limit;

    let render_size = vec2(
        config.rendering.texture_width as f32,
        config.rendering.texture_height as f32,
    );

    let render_rect = Rect::from_x_y_w_h(0.0, 0.0, render_size.x, render_size.y);

    let osc = OscController::new(config.osc_receive.receive_port).unwrap();
    let osc_send = OscSender::new(&config.osc_send).unwrap();

    let osc_loop_config = OscSendConfig {
        target_addr: config.osc_loop.target_addr,
        target_port: config.osc_loop.target_port,
    };
    let osc_loop = OscSender::new(&osc_loop_config).unwrap();

    // DPI scale is used to scale the size of draw objects to account for DPI scaling.
    let dpi_scale = config.rendering.dpi_scale;

    let mut particle_system = ParticleSystem::new(
        pt2(0.0, 0.0),
        render_size.x,
        render_size.y,
        DEFAULT_PARTICLE_SIZE,
        rgb(
            DEFAULT_PARTICLE_RGB.0,
            DEFAULT_PARTICLE_RGB.1,
            DEFAULT_PARTICLE_RGB.2,
        ),
        particle_limit,
        dpi_scale,
    );

    particle_system.set_mass_variation_enabled(true);
    particle_system.set_mass_variation_amount(0.5);

    // Create window
    let audience_window_id = app
        .new_window()
        .title("Tacit Group: System_4 0.1.0")
        .size(config.audience_window.width, config.audience_window.height)
        .msaa_samples(1)
        .view(audience_view)
        .build()
        .unwrap();

    let performer_window_id = app
        .new_window()
        .title("System_4 Performance Monitor v0.1.0")
        .size(
            config.performer_window.width,
            config.performer_window.height,
        )
        .msaa_samples(1)
        .view(performer_view)
        .build()
        .unwrap();

    let control_window_id = app
        .new_window()
        .title("System_4 Performer Control v0.1.0")
        .size(config.control_window.width, config.control_window.height)
        .msaa_samples(1)
        .key_pressed(key_pressed)
        .raw_event(raw_window_event)
        .view(control_view)
        .build()
        .unwrap();

    let Some(audience_window) = app.window(audience_window_id) else {
        eprintln!("Audience window not found. Exiting app.");
        std::process::exit(1);
    };
    let Some(performer_window) = app.window(performer_window_id) else {
        eprintln!("Performer window not found. Exiting app.");
        std::process::exit(1);
    };

    let Some(control_window) = app.window(control_window_id) else {
        eprintln!("Control window not found. Exiting app.");
        std::process::exit(1);
    };

    // Set macOS window flags
    /*
    #[cfg(target_os = "macos")]
    set_macos_window_behavior(&audience_window);
    #[cfg(target_os = "macos")]
    set_macos_window_behavior(&performer_window);
    #[cfg(target_os = "macos")]
    set_macos_window_behavior(&control_window);
     */

    println!(
        "Audience window scale: {:?}",
        audience_window.scale_factor()
    );
    println!(
        "Performer window scale: {:?}",
        performer_window.scale_factor()
    );
    println!("Control window scale: {:?}", control_window.scale_factor());

    // Set up render texture
    // the device isn't tied to window, but it's nannou's way of getting the handle.
    let device = audience_window.device();

    // Create Nnpipe
    let gpu_buffers = HashMap::new();

    let mut rendering = Nnpipe::new(
        device,
        config.rendering.texture_width,
        config.rendering.texture_height,
        config.rendering.texture_samples,
    );

    // Create heatmap renderer
    let heatmap_renderer = HeatmapRenderer::new(
        device,
        config.rendering.texture_width,
        config.rendering.texture_height,
        particle_limit as usize,
    );

    // Create reshapers for both windows
    let audience_reshaper = rendering.create_reshaper_for_post_processed(device, &audience_window);
    let performer_reshaper =
        rendering.create_reshaper_for_post_processed(device, &performer_window);
    let audience_draw = nannou::Draw::new();
    let performer_draw = nannou::Draw::new();
    let control_draw = nannou::Draw::new();

    // Set up effects pipeline

    let lo_config = TextureConfig {
        width: config.rendering.texture_width / 2,
        height: config.rendering.texture_height / 2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
    };

    let med_config = TextureConfig {
        width: config.rendering.texture_width,
        height: config.rendering.texture_height,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
    };

    let hi_config = TextureConfig {
        width: config.rendering.texture_width,
        height: config.rendering.texture_height,
        format: wgpu::TextureFormat::Rgba16Float,
    };

    // Create particle renderer
    let particle_renderer1: ParticleRenderer = ParticleRenderer::new(device, hi_config, 25000);
    let particle_renderer4: ParticleRenderer = ParticleRenderer::new(device, hi_config, 25000);

    // Create segment renderer
    let segment_params = SegmentParams::new(1.0, 2.0);
    let segment_renderer1 = SegmentRenderer::new(device, hi_config, 25000, segment_params);
    let segment_renderer4 = SegmentRenderer::new(device, hi_config, 25000, segment_params);

    // Create pipeline textures
    rendering.create_named_texture(device, "particles", hi_config);
    rendering.create_named_texture(device, "heatmap", hi_config);
    //rendering.create_named_texture(device, "particle_processed", hi_config);
    rendering.create_named_texture(device, "heatmap_processed", hi_config);
    rendering.create_named_texture(device, "processed_composited", hi_config);

    /*
    let particle_effects = PipelineBuilder::new()
        .name("Particle Effects Pipeline")
        .input_texture("particles")
        .inversion(hi_config, 1.0)
        .output_texture("particle_processed")
        .build(device);
    if let Ok(effect) = particle_effects {
        rendering.add_multi_pipeline("particle_effects", effect);
    }
     */

    let heatmap_effects = PipelineBuilder::new()
        .name("Heatmap Effects Pipeline")
        .input_texture("heatmap")
        .feedback(hi_config, 1.0, 1.0)
        .output_texture("heatmap_processed")
        .build(device);

    if let Ok(effect) = heatmap_effects {
        rendering.add_multi_pipeline("heatmap_effects", effect);
    }

    let composite_step = PipelineBuilder::new()
        .name("Composite Step Pipeline")
        .input_textures(&["particles", "heatmap_processed"])
        .output_texture("processed_composited")
        .simple_additive_composite(hi_config, 1.0)
        .build(device);

    if let Ok(effect) = composite_step {
        rendering.add_multi_pipeline("composite_step", effect);
    }

    let effects = PipelineBuilder::new()
        .name("Particle Effects Pipeline")
        .input_texture("processed_composited")
        .brightness_extract(med_config, 0.7)
        .downsample(lo_config)
        .gaussian_blur_passes(lo_config, 2, 2.0, 5.0)
        .bloom_composite_with_curve(hi_config, 2.0, 3.0)
        .inversion(hi_config, 1.0)
        .build(device);

    if let Ok(effect) = effects {
        rendering.add_multi_pipeline("effects", effect);
    }

    // Set up egui
    let egui = Egui::from_window(&control_window);

    // Set up rng
    let rng = ThreadRng::default();

    // --- Load Font for Nannou Draw (Hangul) ---
    // Assumes "assets/gulim.ttf" exists relative to the executable
    // or relative to the project root if running with `cargo run`
    let assets = app.assets_path().expect("Could not find assets directory");
    let font_path = assets.join("terminal_font.ttf");
    let font_bytes = fs::read(&font_path)
        .unwrap_or_else(|_| panic!("Failed to read font file at {:?}", font_path));
    let font = Font::from_bytes(font_bytes)
        .unwrap_or_else(|_| panic!("Failed to load font at {:?}", font_path));

    // Create FPS manager
    let mut fps = FpsManager::new_with(true, false);
    let performer_rect = app.window(performer_window_id).unwrap().rect();
    fps.set_draw_position(pt2(
        performer_rect.left() + 40.0,
        performer_rect.top() - 10.0,
    ));

    // Create terminal system
    let terminal_system = TerminalSystem::new();

    Model {
        particle_system,
        terminal_system,
        osc,
        osc_send,
        osc_loop,
        render_size,
        render_rect,
        dpi_scale,
        font,
        id_generator: IdGenerator::new(),
        audience_window_id,
        performer_window_id,
        control_window_id,
        audience_reshaper,
        performer_reshaper,
        audience_draw,
        performer_draw,
        control_draw,
        gpu_buffers,
        rendering: RefCell::new(rendering),
        heatmap_renderer,
        particle_renderer1,
        particle_renderer4,

        segment_renderer1,
        segment_renderer4,

        egui,
        rng,
        fps,
        frame_count: 0,
        update_ticks: 0,
        show_bounds: false,
        show_forces: false,
    }
}

fn main() {
    // Set main thread to high priority to prevent animation interruptions
    let thread_priority = ThreadPriority::Max;
    let result = set_current_thread_priority(thread_priority);

    if let Err(e) = result {
        println!("Warning: Failed to set main thread priority: {:?}", e);
    } else {
        println!(
            "Main thread priority set to {:?}: {:?}",
            thread_priority, result
        );
    }
    nannou::app(model)
        .loop_mode(nannou::LoopMode::rate_fps(60.0)) // Run at 120fps regardless of display refresh rate
        .update(update)
        .run();
}

fn update(app: &App, model: &mut Model, _update: Update) {
    // Increment frame counter
    model.frame_count += 1;
    model.update_ticks += 1;

    // Update FPS counter
    model.fps.update();

    // Get GPU resources
    let window = app.main_window();
    let device = window.device();
    let queue = window.queue();

    // Update control UI
    update_control_ui(app, model);

    // Process OSC commands
    model.osc.process_messages();
    let commands = model.osc.take_commands();
    process_osc(model, commands);

    // Update feedback render params
    controller::update_feedback(model, device, queue);

    // Update particle system
    model
        .particle_system
        .update(&mut model.rng, model.show_forces, &mut model.gpu_buffers);

    // Update terminals
    let finish_signals = model
        .terminal_system
        .update(&model.rendering.borrow_mut().draw);

    // When a terminal start sequence is finished, send the OSC command to turn on the drone
    for (voice, finish_signal) in finish_signals {
        if finish_signal {
            let id = voice.to_i32();
            model.osc_send.send_drone_on_off(id, 1);
            model.particle_system.set_is_spawning(&voice, true);
        }
    }
}

fn audience_view(app: &App, model: &Model, frame: Frame) {
    if !should_render(model) {
        return;
    }

    // Begin Rendering context
    {
        let mut rendering = model.rendering.borrow_mut();

        // Get GPU resources
        let window = app.main_window();
        let device = window.device();
        let mut encoder = rendering.create_command_encoder(device);
        let queue = window.queue();

        // Clear all textures
        rendering.draw.background().color(BLACK);
        rendering.encode_clear_all_textures(&mut encoder, wgpu::Color::BLACK);

        // Encode Nannou Draw
        rendering.encode_draw_commands(device, &mut encoder);

        // Retrieve buffer or use empty buffer
        let empty_gpu_buffer = &EMPTY_GPU_BUFFER;
        let gpu_buffer1 = model
            .gpu_buffers
            .get(&Voice::Voice1)
            .unwrap_or(empty_gpu_buffer);
        let gpu_buffer4 = model
            .gpu_buffers
            .get(&Voice::Voice4)
            .unwrap_or(empty_gpu_buffer);

        // Encode particles
        model.particle_renderer1.encode_into(
            &mut encoder,
            queue,
            &gpu_buffer1.0,
            rendering.get_named_texture("particles").unwrap(),
        );

        model.particle_renderer4.encode_into(
            &mut encoder,
            queue,
            &gpu_buffer4.0,
            rendering.get_named_texture("particles").unwrap(),
        );

        model.segment_renderer1.encode_into(
            &mut encoder,
            queue,
            &gpu_buffer1.1,
            rendering.get_named_texture("particles").unwrap(),
        );

        model.segment_renderer4.encode_into(
            &mut encoder,
            queue,
            &gpu_buffer4.1,
            rendering.get_named_texture("particles").unwrap(),
        );

        // Encode heatmap
        model.heatmap_renderer.encode_into(
            device,
            &mut encoder,
            queue,
            &gpu_buffer1.0,
            model.render_rect,
            model.frame_count,
            rendering.get_named_texture("heatmap").unwrap(),
        );

        model.heatmap_renderer.encode_into(
            device,
            &mut encoder,
            queue,
            &gpu_buffer4.0,
            model.render_rect,
            model.frame_count,
            rendering.get_named_texture("heatmap").unwrap(),
        );

        //Encode post processing
        if let Err(e) = rendering.execute_named_pipeline("heatmap_effects", device, &mut encoder) {
            eprintln!("Error executing heatmap_effects pipeline: {}", e);
        }

        if let Err(e) = rendering.execute_named_pipeline("composite_step", device, &mut encoder) {
            eprintln!("Error executing composite_step pipeline: {}", e);
        }

        if let Err(e) = rendering.execute_named_pipeline("effects", device, &mut encoder) {
            eprintln!("Error executing effects pipeline: {}", e);
        }

        rendering.submit_command_encoder(device, queue, encoder);

        // Update reshaper if needed (could be cached in Model)
        rendering.draw_to_frame(&model.audience_reshaper, &frame);
    }
    // End Rendering context

    // Show screen bounds if enabled
    if model.show_bounds {
        draw_bounds(app, model);
        // Then draw over the texture
        let _ = model.audience_draw.to_frame(app, &frame);
    }
}

fn performer_view(app: &App, model: &Model, frame: Frame) {
    let rendering = model.rendering.borrow_mut();

    // Get the raw scene texture view
    let _scene_view = rendering.get_scene_view();

    // Draw game content to the frame
    rendering.draw_to_frame(&model.performer_reshaper, &frame);

    // Show force vectors if enabled
    if model.show_forces {
        let performer_rect = app.window(model.performer_window_id).unwrap().rect();

        // Create a scaled draw context that matches texture coordinates
        let texture_size = rendering.scene_texture.size();

        // Calculate scale factor from texture to window
        let scale_x = performer_rect.w() / texture_size[0] as f32;
        let scale_y = performer_rect.h() / texture_size[1] as f32;

        // Apply transform to match texture coordinates
        model
            .particle_system
            .draw_forces(&model.performer_draw, scale_x, scale_y);
    }

    // Then draw over the texture
    let _ = model.performer_draw.to_frame(app, &frame);
}

fn control_view(app: &App, model: &Model, frame: Frame) {
    // Draw background first
    model.control_draw.background().color(BLACK);
    let _ = model.control_draw.to_frame(app, &frame);
    // Then draw egui UI on top
    model.egui.draw_to_frame(&frame).unwrap();
}

// ******************************* Input Capture *****************************

fn key_pressed(_app: &App, model: &mut Model, key: Key) {
    // For now, all human boards are controlled by the same keyboard
    match key {
        Key::P => {
            // Toggle debug and FPS display
            model.show_bounds = !model.show_bounds;
        }
        Key::Key1 => {
            model.osc_loop.send_make_drone(1, 100, 20, 0, 0, 0);
        }
        Key::Key4 => {
            model.osc_loop.send_make_drone(4, 100, 20, 0, 0, 0);
        }
        Key::C => {
            model.osc_loop.send_inner_radius(4, 0.5);
        }
        Key::R => {
            model.osc_loop.send_outer_radius(4, 0.5);
        }
        Key::X => {
            model.osc_loop.send_erase_drone(1);
            model.osc_loop.send_erase_drone(4);
        }
        Key::M => {
            model
                .osc_loop
                .send_mask_change_bounds(1, 0, 0, 1920, 1080, 10.0);
        }
        Key::V => {
            model.osc_loop.send_vibration(1, 0.5);
        }

        _ => {}
    }
}

fn raw_window_event(_app: &App, model: &mut Model, event: &nannou::winit::event::WindowEvent) {
    model.egui.handle_raw_event(event);
}

// ************************ Control UI display  *************************************

fn update_control_ui(app: &App, model: &mut Model) {
    let Some(control_window) = app.window(model.control_window_id) else {
        eprintln!("Control window not found. Exiting app.");
        std::process::exit(1);
    };
    let rect = control_window.rect();
    let height = rect.h() - 5.0;
    let width = rect.w() - 5.0;

    // Extract all parameters before creating egui context to avoid borrowing conflicts
    let voice1_circle_params = model.get_wind_circle_params(Voice::Voice1).cloned();
    let voice1_alpha = model.get_alpha_limit(Voice::Voice1);
    let voice1_volume = model.get_volume(Voice::Voice1);
    let voice1_feedback = model.get_feedback(Voice::Voice1);
    let voice1_angle_variation = model.get_angle_variation(Voice::Voice1);
    let voice1_position_offset = model.get_position_offset_factor(Voice::Voice1);

    let voice4_circle_params = model.get_wind_circle_params(Voice::Voice4).cloned();
    let voice4_alpha = model.get_alpha_limit(Voice::Voice4);
    let voice4_volume = model.get_volume(Voice::Voice4);
    let voice4_feedback = model.get_feedback(Voice::Voice4);
    let voice4_angle_variation = model.get_angle_variation(Voice::Voice4);
    let voice4_position_offset = model.get_position_offset_factor(Voice::Voice4);

    let ctx = model.egui.begin_frame();

    // Set text style settings
    let style = (*ctx.style()).clone();
    ctx.set_style(adjust_style_from(style));

    let mut show_forces_changed = false;
    let mut parameter_changes = Vec::<VoiceParameterChange>::new();

    egui::Window::new("Control Panel")
        .fixed_pos(egui::pos2(0.0, 0.0))
        .default_size(egui::vec2(width, height))
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .frame(egui::Frame {
            fill: egui::Color32::from_rgba_unmultiplied(0, 0, 0, 0), // Dark background
            stroke: egui::Stroke::new(0.0, egui::Color32::from_rgb(10, 10, 10)), // Subtle border
            inner_margin: egui::style::Margin::symmetric(10.0, 10.0), // Padding
            outer_margin: egui::style::Margin::same(0.0),            // No margin
            rounding: egui::Rounding::same(1.0),                     // Slightly rounded corners
            shadow: egui::epaint::Shadow::NONE,
        })
        .show(&ctx, |ui| {
            ui.horizontal(|ui| {
                // Vertical 1: Instructions and status info
                ui.vertical(|ui| {
                    ui.set_min_size(egui::vec2(150.0, height));
                    // Status info
                    ui.label(format!(
                        "Particles: {}",
                        model.particle_system.get_particle_count()
                    ));
                    // FPS
                    ui.label(format!("FPS: {:.1}", model.fps.fps()));
                    ui.add_space(15.0);

                    show_forces_changed =
                        ui.checkbox(&mut model.show_forces, "Show Forces").changed();
                    ui.add_space(30.0);

                    // Instructions section
                    ui.label("...");
                    ui.label("P: Debug view");
                });

                // Voice 1 (col 2)
                ui.vertical(|ui| {
                    // Wind Circle Settings - use horizontal layout for two vertical sections
                    let current_params = voice1_circle_params;

                    ui.set_min_width(350.0);
                    ui.heading("Voice 1: Drone");
                    ui.add_space(2.0);

                    if let Some(params) = current_params {
                        ui.add_space(10.0);

                        // Outer Radius slider
                        let mut radius = params.outer_radius;
                        if ui
                            .add(
                                egui::Slider::new(&mut radius, 0.0..=1100.0)
                                    .text("OR")
                                    .custom_formatter(|n, _| format!("{:.3}", n)),
                            )
                            .changed()
                        {
                            parameter_changes.push(VoiceParameterChange::OuterRadius {
                                voice: Voice::Voice1,
                                value: radius,
                            });
                        }

                        // Inner Radius slider
                        let mut inner_radius = params.inner_radius;
                        if ui
                            .add(
                                egui::Slider::new(&mut inner_radius, 0.0..=1100.0)
                                    .text("IR")
                                    .custom_formatter(|n, _| format!("{:.3}", n)),
                            )
                            .changed()
                        {
                            parameter_changes.push(VoiceParameterChange::InnerRadius {
                                voice: Voice::Voice1,
                                value: inner_radius,
                            });
                        }

                        // Alpha slider
                        let mut alpha = voice1_alpha;
                        if ui
                            .add(
                                egui::Slider::new(&mut alpha, 0.0..=1.0)
                                    .text("Brightness")
                                    .custom_formatter(|n, _| format!("{:.3}", n)),
                            )
                            .changed()
                        {
                            parameter_changes.push(VoiceParameterChange::Alpha {
                                voice: Voice::Voice1,
                                value: alpha,
                            });
                        }

                        // Volume slider
                        let mut volume = voice1_volume;
                        if ui
                            .add(
                                egui::Slider::new(&mut volume, 0.0..=1.0)
                                    .text("Volume")
                                    .custom_formatter(|n, _| format!("{:.3}", n)),
                            )
                            .changed()
                        {
                            parameter_changes.push(VoiceParameterChange::Volume {
                                voice: Voice::Voice1,
                                value: volume,
                            });
                        }

                        // Strength slider
                        let mut strength = params.strength;
                        if ui
                            .add(
                                egui::Slider::new(&mut strength, 0.0..=30.0)
                                    .text("Force")
                                    .custom_formatter(|n, _| format!("{:.3}", n)),
                            )
                            .changed()
                        {
                            parameter_changes.push(VoiceParameterChange::Strength {
                                voice: Voice::Voice1,
                                value: strength,
                            });
                        }

                        // Center bias slider
                        let mut center_bias = params.center_bias;
                        if ui
                            .add(
                                egui::Slider::new(&mut center_bias, 0.0..=2.0)
                                    .text("Gravity")
                                    .custom_formatter(|n, _| format!("{:.3}", n)),
                            )
                            .changed()
                        {
                            parameter_changes.push(VoiceParameterChange::CenterBias {
                                voice: Voice::Voice1,
                                value: center_bias,
                            });
                        }

                        // Angle variation slider
                        let mut angle_variation = voice1_angle_variation;
                        if ui
                            .add(
                                egui::Slider::new(&mut angle_variation, 0.0..=1.0)
                                    .text("Noise")
                                    .custom_formatter(|n, _| format!("{:.3}", n)),
                            )
                            .changed()
                        {
                            parameter_changes.push(VoiceParameterChange::AngleVariation {
                                voice: Voice::Voice1,
                                value: angle_variation,
                            });
                        }

                        // Position offset slider
                        let mut position_offset = voice1_position_offset;
                        if ui
                            .add(
                                egui::Slider::new(&mut position_offset, 0.0..=1.0)
                                    .text("Vibration")
                                    .custom_formatter(|n, _| format!("{:.3}", n)),
                            )
                            .changed()
                        {
                            parameter_changes.push(VoiceParameterChange::PositionOffset {
                                voice: Voice::Voice1,
                                value: position_offset,
                            });
                        }

                        // Feedback slider
                        let mut feedback = voice1_feedback;
                        if ui
                            .add(
                                egui::Slider::new(&mut feedback, 0.0..=1.0)
                                    .text("Feedback")
                                    .custom_formatter(|n, _| format!("{:.3}", n)),
                            )
                            .changed()
                        {
                            parameter_changes.push(VoiceParameterChange::Feedback {
                                voice: Voice::Voice1,
                                value: feedback,
                            });
                        }

                        // Center X slider
                        let mut center_x = params.center.x;
                        if ui
                            .add(egui::Slider::new(&mut center_x, -2000.0..=2000.0).text("Ctr X"))
                            .changed()
                        {
                            parameter_changes.push(VoiceParameterChange::ForceCenterX {
                                voice: Voice::Voice1,
                                value: center_x,
                            });
                        }

                        // Center Y slider
                        let mut center_y = params.center.y;
                        if ui
                            .add(egui::Slider::new(&mut center_y, -1100.0..=1100.0).text("Ctr Y"))
                            .changed()
                        {
                            parameter_changes.push(VoiceParameterChange::ForceCenterY {
                                voice: Voice::Voice1,
                                value: center_y,
                            });
                        }
                    } else {
                        ui.label("No wind circles found");
                        ui.label("1: Create wind circle");
                    }
                }); // end Voice 1

                // Voice 2 (col 3)
                ui.vertical(|ui| {
                    ui.set_min_width(300.0);
                    ui.heading("Voice 2: Rhythm");
                    ui.add_space(5.0);
                }); // end Voice 2

                // Voice 3
                ui.vertical(|ui| {
                    ui.set_min_width(300.0);
                    ui.heading("Voice 3: Rhythm");
                    ui.add_space(5.0);
                }); // end Voice 3

                // Voice 4: Column 5
                ui.vertical(|ui| {
                    ui.set_min_width(300.0);
                    ui.heading("Voice 4: Drone");
                    ui.add_space(2.0);

                    let current_params = voice4_circle_params;

                    if let Some(params) = current_params {
                        ui.add_space(10.0);

                        // Outer Radius slider
                        let mut radius = params.outer_radius;
                        if ui
                            .add(
                                egui::Slider::new(&mut radius, 0.0..=1100.0)
                                    .text("OR")
                                    .custom_formatter(|n, _| format!("{:.3}", n)),
                            )
                            .changed()
                        {
                            parameter_changes.push(VoiceParameterChange::OuterRadius {
                                voice: Voice::Voice4,
                                value: radius,
                            });
                        }

                        // Inner Radius slider
                        let mut inner_radius = params.inner_radius;
                        if ui
                            .add(
                                egui::Slider::new(&mut inner_radius, 0.0..=1100.0)
                                    .text("IR")
                                    .custom_formatter(|n, _| format!("{:.3}", n)),
                            )
                            .changed()
                        {
                            parameter_changes.push(VoiceParameterChange::InnerRadius {
                                voice: Voice::Voice4,
                                value: inner_radius,
                            });
                        }

                        // Alpha slider
                        let mut alpha = voice4_alpha;
                        if ui
                            .add(
                                egui::Slider::new(&mut alpha, 0.0..=1.0)
                                    .text("Brightness")
                                    .custom_formatter(|n, _| format!("{:.3}", n)),
                            )
                            .changed()
                        {
                            parameter_changes.push(VoiceParameterChange::Alpha {
                                voice: Voice::Voice4,
                                value: alpha,
                            });
                        }

                        // Volume slider
                        let mut volume = voice4_volume;
                        if ui
                            .add(
                                egui::Slider::new(&mut volume, 0.0..=1.0)
                                    .text("Volume")
                                    .custom_formatter(|n, _| format!("{:.3}", n)),
                            )
                            .changed()
                        {
                            parameter_changes.push(VoiceParameterChange::Volume {
                                voice: Voice::Voice4,
                                value: volume,
                            });
                        }

                        // Strength slider
                        let mut strength = params.strength;
                        if ui
                            .add(
                                egui::Slider::new(&mut strength, 0.0..=30.0)
                                    .text("Force")
                                    .custom_formatter(|n, _| format!("{:.3}", n)),
                            )
                            .changed()
                        {
                            parameter_changes.push(VoiceParameterChange::Strength {
                                voice: Voice::Voice4,
                                value: strength,
                            });
                        }

                        // Center bias slider
                        let mut center_bias = params.center_bias;
                        if ui
                            .add(
                                egui::Slider::new(&mut center_bias, 0.0..=2.0)
                                    .text("Gravity")
                                    .custom_formatter(|n, _| format!("{:.3}", n)),
                            )
                            .changed()
                        {
                            parameter_changes.push(VoiceParameterChange::CenterBias {
                                voice: Voice::Voice4,
                                value: center_bias,
                            });
                        }

                        // Angle variation slider
                        let mut angle_variation = voice4_angle_variation;
                        if ui
                            .add(
                                egui::Slider::new(&mut angle_variation, 0.0..=1.0)
                                    .text("Noise")
                                    .custom_formatter(|n, _| format!("{:.3}", n)),
                            )
                            .changed()
                        {
                            parameter_changes.push(VoiceParameterChange::AngleVariation {
                                voice: Voice::Voice4,
                                value: angle_variation,
                            });
                        }

                        // Position offset slider
                        let mut position_offset = voice4_position_offset;
                        if ui
                            .add(
                                egui::Slider::new(&mut position_offset, 0.0..=1.0)
                                    .text("Vibration")
                                    .custom_formatter(|n, _| format!("{:.3}", n)),
                            )
                            .changed()
                        {
                            parameter_changes.push(VoiceParameterChange::PositionOffset {
                                voice: Voice::Voice4,
                                value: position_offset,
                            });
                        }

                        // Feedback slider
                        let mut feedback = voice4_feedback;
                        if ui
                            .add(
                                egui::Slider::new(&mut feedback, 0.0..=1.0)
                                    .text("Feedback")
                                    .custom_formatter(|n, _| format!("{:.3}", n)),
                            )
                            .changed()
                        {
                            parameter_changes.push(VoiceParameterChange::Feedback {
                                voice: Voice::Voice4,
                                value: feedback,
                            });
                        }

                        // Center X slider
                        let mut center_x = params.center.x;
                        if ui
                            .add(egui::Slider::new(&mut center_x, -2000.0..=2000.0).text("Ctr X"))
                            .changed()
                        {
                            parameter_changes.push(VoiceParameterChange::ForceCenterX {
                                voice: Voice::Voice4,
                                value: center_x,
                            });
                        }

                        // Center Y slider
                        let mut center_y = params.center.y;
                        if ui
                            .add(egui::Slider::new(&mut center_y, -1100.0..=1100.0).text("Ctr Y"))
                            .changed()
                        {
                            parameter_changes.push(VoiceParameterChange::ForceCenterY {
                                voice: Voice::Voice4,
                                value: center_y,
                            });
                        }
                    } else {
                        ui.label("No wind circles found");
                        ui.label("4: Create wind circle");
                    }
                }); // end Voice 4
            });
        });

    drop(ctx);

    // Apply all parameter changes outside the UI closure - no borrowing conflicts!
    for change in parameter_changes {
        model.apply_voice_parameter_change(change);
    }
}

fn adjust_style_from(style: egui::Style) -> egui::Style {
    let mut style = style;
    // Set font sizes for different text styles
    style.text_styles = [
        (
            egui::TextStyle::Heading,
            egui::FontId::new(15.0, egui::FontFamily::Monospace),
        ),
        (
            egui::TextStyle::Body,
            egui::FontId::new(13.0, egui::FontFamily::Monospace),
        ),
        (
            egui::TextStyle::Button,
            egui::FontId::new(13.0, egui::FontFamily::Monospace),
        ),
        (
            egui::TextStyle::Small,
            egui::FontId::new(11.0, egui::FontFamily::Monospace),
        ),
    ]
    .into_iter()
    .collect();

    // Increase spacing for better usability
    style.spacing.item_spacing = egui::vec2(10.0, 5.0);
    style.spacing.button_padding = egui::vec2(8.0, 2.0);
    style.spacing.slider_width = 150.0; // Make sliders wider

    // Set colors
    let mut visuals = style.visuals.clone();
    visuals.widgets.active.bg_fill = egui::Color32::from_rgb(120, 120, 120); // Active button color
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(70, 70, 70); // Hover color
    visuals.window_fill = egui::Color32::from_rgba_unmultiplied(0, 0, 0, 0); // Window background
    visuals.window_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(10, 10, 10));

    style.visuals = visuals;

    style
}

// ************************ Debug display  *************************************

fn draw_bounds(app: &App, model: &Model) {
    let draw = &model.audience_draw;
    let rect = app.window(model.audience_window_id).unwrap().rect();

    // Draw (+,+) axes
    draw.line()
        .points(pt2(0.0, 0.0), pt2(25.0, 0.0))
        .color(RED)
        .stroke_weight(1.0);
    draw.line()
        .points(pt2(0.0, 0.0), pt2(0.0, 25.0))
        .color(BLUE)
        .stroke_weight(1.0);

    // Draw rect bounds
    draw.rect()
        .xy(pt2(0.0, 0.0))
        .wh(pt2(rect.w(), rect.h()))
        .stroke(rgba(0.4, 1.0, 0.4, 0.75)) // Green outline
        .stroke_weight(5.0)
        .no_fill();
}

// ************************ OSC   *************************************

fn process_osc(model: &mut Model, commands: Vec<OscCommand>) {
    let mut parameter_changes = Vec::<VoiceParameterChange>::new();

    for command in commands {
        match command {
            OscCommand::MakeDrone {
                id,
                alpha,
                num_particles,
                force,
                gravity,
                trail,
            } => {
                make_drone(model, id, alpha, num_particles, force, gravity, trail);
            }
            OscCommand::EraseDrone { id } => {
                erase_drone(model, id);
            }
            OscCommand::ParticlesGravity { id, val } => {
                let voice = Voice::from_i32(id);
                parameter_changes.push(VoiceParameterChange::CenterBias { voice, value: val });
            }
            OscCommand::ParticlesTrail { id, val } => {
                let voice = Voice::from_i32(id);
                parameter_changes.push(VoiceParameterChange::Feedback { voice, value: val });
            }
            OscCommand::ParticlesNumParticles { id, val } => {
                let voice = Voice::from_i32(id);
                parameter_changes.push(VoiceParameterChange::Volume { voice, value: val });
            }
            OscCommand::ParticlesAlpha { id, val } => {
                let voice = Voice::from_i32(id);
                parameter_changes.push(VoiceParameterChange::Alpha { voice, value: val });
            }
            OscCommand::ParticlesForce { id, val } => {
                let voice = Voice::from_i32(id);
                parameter_changes.push(VoiceParameterChange::Strength { voice, value: val });
            }
            OscCommand::ParticlesInnerRadius { id, val } => {
                let voice = Voice::from_i32(id);
                parameter_changes.push(VoiceParameterChange::InnerRadius { voice, value: val });
            }
            OscCommand::ParticlesOuterRadius { id, val } => {
                let voice = Voice::from_i32(id);
                parameter_changes.push(VoiceParameterChange::OuterRadius { voice, value: val });
            }
            OscCommand::ParticlesNoise { id, val } => {
                let voice = Voice::from_i32(id);
                parameter_changes.push(VoiceParameterChange::AngleVariation { voice, value: val });
            }
            OscCommand::ParticlesVibration { id, val } => {
                let voice = Voice::from_i32(id);
                parameter_changes.push(VoiceParameterChange::PositionOffset { voice, value: val });
            }
            OscCommand::MaskChangeBounds {
                id,
                x,
                y,
                w,
                h,
                duration,
            } => {
                let voice = Voice::from_i32(id);
                parameter_changes.push(VoiceParameterChange::MaskChangeBounds {
                    voice,
                    rect: Rect::from_x_y_w_h(x as f32, y as f32, w as f32, h as f32),
                    duration,
                })
            }
            _ => {}
        }
    }

    // Apply all OSC parameter changes using the same unified system as UI
    for change in parameter_changes {
        model.apply_voice_parameter_change(change);
    }
}

/// Start the Voice and begin "make drone" automated display
#[allow(clippy::too_many_arguments)]
fn make_drone(
    model: &mut Model,
    id: i32,
    alpha: i32,
    num_particles: i32,
    force: i32,
    gravity: i32,
    trail: i32,
) {
    let voice = Voice::from_i32(id);
    let center = match voice {
        Voice::Voice1 => pt2(-1280.0, 200.0),
        Voice::Voice4 => pt2(1280.0, 200.0),
        _ => pt2(0.0, 0.0),
    };
    let (outer_radius, inner_radius) = (620.0, 200.0);
    let center_bias = (gravity as f32) / 100.0;
    let strength = ((force as f32) / 100.0) + 10.0;

    let circle = WindCircle::new(
        model.id_generator.generate(),
        voice,
        center,
        outer_radius,
        inner_radius,
        strength,
        center_bias,
    );

    // Create the drone / circle
    let mask_rect = model.particle_system.make_drone_with(
        &mut model.id_generator,
        voice,
        circle,
        alpha,
        num_particles,
        trail,
    );

    let num_lines = TERMINAL_NUM_LINES;
    let line_margin = TERMINAL_LINE_MARGIN;
    let font_size = 40;
    let terminal_height = (font_size as f32 + line_margin * 2.0) * num_lines as f32;

    // Position terminal: mask bottom - half terminal height - 20pt padding
    let terminal_origin = vec2(
        mask_rect.x() as f32, // Center horizontally with mask
        mask_rect.bottom() - terminal_height / 2.0 - 30.0, // Position below mask with padding
    );
    println!("terminal_origin: {:?}", terminal_origin);

    // Create the terminal
    let terminal_params = TerminalParams {
        origin: terminal_origin,
        num_lines,
        line_width: mask_rect.w(),
        line_margin,
        start_color: rgba(
            TERMINAL_START_RGBA.0,
            TERMINAL_START_RGBA.1,
            TERMINAL_START_RGBA.2,
            TERMINAL_START_RGBA.3,
        ),
        end_color: rgba(
            TERMINAL_END_RGBA.0,
            TERMINAL_END_RGBA.1,
            TERMINAL_END_RGBA.2,
            TERMINAL_END_RGBA.3,
        ),
        color_fade_secs: 2.0,
        chars_per_second: TERMINAL_CHARS_PER_SECOND,
        font: model.font.clone(),
        font_size,
    };
    let Some(terminal) =
        model
            .terminal_system
            .add_new_terminal(voice, terminal_params, model.dpi_scale)
    else {
        println!("Failed to add terminal for player {}", voice);
        return;
    };

    // Normalizing and naming mess
    let brightness = (alpha as f32) / 100.0;
    let volume = (num_particles as f32) / 100.0;
    let trail = (trail as f32) / 100.0;

    terminal.begin_start_sequence(brightness, volume, strength, center_bias, trail);
}

fn erase_drone(model: &mut Model, id: i32) {
    let voice = Voice::from_i32(id);
    model.particle_system.kill_voice(&voice);
    model.particle_system.forces.recalculate_once();
    model.osc_send.send_drone_on_off(id, 0);
}

/// Set macOS window behaviors so that Spaces and Mission Control doesn't interrupt rendering.
#[cfg(target_os = "macos")]
#[allow(dead_code)]
fn set_macos_window_behavior(window: &Window) {
    use nannou::winit::platform::macos::WindowExtMacOS;
    // removed the NSUInteger type below because it's just an alias for usize.
    // this allowed for the removal of objc2_foundation as a dependency.
    // use objc2_foundation::NSUInteger;

    let ns_window = window.winit_window().ns_window();

    // Combine behaviors
    const NS_WINDOW_COLLECTION_BEHAVIOR_CAN_JOIN_ALL_SPACES: usize = 1 << 0;
    const NS_WINDOW_COLLECTION_BEHAVIOR_STATIONARY: usize = 1 << 4;
    const NS_WINDOW_COLLECTION_BEHAVIOR_FULL_SCREEN_PRIMARY: usize = 1 << 7;

    let behavior = NS_WINDOW_COLLECTION_BEHAVIOR_CAN_JOIN_ALL_SPACES
        | NS_WINDOW_COLLECTION_BEHAVIOR_STATIONARY
        | NS_WINDOW_COLLECTION_BEHAVIOR_FULL_SCREEN_PRIMARY;

    unsafe {
        use objc2_app_kit::{NSWindow, NSWindowCollectionBehavior};
        let ns_window = ns_window as *mut NSWindow;
        (*ns_window)
            .setCollectionBehavior(NSWindowCollectionBehavior::from_bits_truncate(behavior));
    }
}

/// Static variable that ticks whenever a render happens
static RENDER_TICKS: AtomicU64 = AtomicU64::new(0);

/// A helper to decouple simulation with rendering.
fn should_render(model: &Model) -> bool {
    let render_ticks = RENDER_TICKS.load(Ordering::SeqCst);
    let should_render = model.update_ticks != render_ticks;
    if should_render {
        RENDER_TICKS.fetch_add(model.update_ticks - render_ticks, Ordering::SeqCst);
    }
    should_render
}
