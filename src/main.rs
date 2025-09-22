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
use thread_priority::*;

use std::cell::RefCell;
use std::{
    collections::HashMap,
    fs,
    sync::atomic::{AtomicU64, Ordering},
};

use system4::{
    config::*,
    fps::FpsManager,
    model::{Model, controller::{self, Command, CommandInner, CommandSource, SimpleCommand, CompositeCommand}},
    osc::{OscController, OscSender},
    particle::{ParticleSystem, EMPTY_GPU_BUFFER},
    terminals::{command_input::CommandInput, commands::TerminalCommand, terminal_view::{TerminalViewManager, TerminalViewParams}, TextJustification},
    utils::IdGenerator,
    groups::VoiceId,
};

const DEFAULT_PARTICLE_SIZE: f32 = 4.0;
const DEFAULT_PARTICLE_RGB: (f32, f32, f32) = (0.73, 0.73, 0.74);
//const DEFAULT_PARTICLE_RGB: (f32, f32, f32) = (0.27, 0.27, 0.26);

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
    rendering.create_named_texture(device, "terminal", hi_config);
    rendering.create_named_texture(device, "particles", hi_config);
    rendering.create_named_texture(device, "heatmap", hi_config);
    //rendering.create_named_texture(device, "particle_processed", hi_config);
    rendering.create_named_texture(device, "heatmap_processed", hi_config);
    rendering.create_named_texture(device, "processed_composited", hi_config);
    rendering.create_named_texture(device, "post-processed", hi_config);

    let particle_effects = PipelineBuilder::new()
        .name("Particle Effects Pipeline")
        .input_texture("particles")
        .feedback(hi_config, 1.0, 60.0)
        .output_texture("particle_processed")
        .build(device);
    if let Ok(effect) = particle_effects {
        rendering.add_multi_pipeline("particle_effects", effect);
    }

    let heatmap_effects = PipelineBuilder::new()
        .name("Heatmap Effects Pipeline")
        .input_texture("heatmap")
        .feedback(hi_config, 1.0, 10.0)
        .output_texture("heatmap_processed")
        .build(device);

    if let Ok(effect) = heatmap_effects {
        rendering.add_multi_pipeline("heatmap_effects", effect);
    }

    let composite_step = PipelineBuilder::new()
        .name("Composite Step Pipeline")
        .input_textures(&["particles", "heatmap_processed"])
        .output_texture("processed_composited")
        .simple_additive_composite(hi_config, 0.5)
        .build(device);

    if let Ok(effect) = composite_step {
        rendering.add_multi_pipeline("composite_step", effect);
    }

    let effects = PipelineBuilder::new()
        .name("Particle Effects Pipeline")
        .input_texture("particles")
        .output_texture("post-processed")
        .brightness_extract(med_config, 0.7)
        .downsample(lo_config)
        .gaussian_blur_passes(lo_config, 2, 2.0, 5.0)
        .bloom_composite_with_curve(hi_config, 2.0, 3.0)
        //.inversion(hi_config, 1.0)
        .build(device);

    if let Ok(effect) = effects {
        rendering.add_multi_pipeline("effects", effect);
    }

    let final_composite = PipelineBuilder::new()
        .name("Final overlay composite")
        .input_textures(&["terminal", "post-processed"])
        .simple_additive_composite(hi_config, 1.0)
        .build(device);

    if let Ok(effect) = final_composite {
        rendering.add_multi_pipeline("final composite", effect);

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

    // Create terminal view manager
    let mut terminal_manager = TerminalViewManager::new();
    
    // Set up terminal parameters for command display
    let terminal_params = TerminalViewParams {
        origin: vec2(-500.0, 1000.0),
        num_lines: 7,
        width: 1000.0,
        line_spacing: 5.0,
        bright_color: rgba(0.7, 0.7, 0.7, 1.0),
        regular_color: rgba(0.2, 0.2, 0.2, 0.8),
        color_fade_secs: 1.0,
        chars_per_second: 6.0,
        font: font.clone(),
        font_size: 32,
        justification: TextJustification::TopLeft,
    };
    
    terminal_manager.add_new_terminal_view("main", VoiceId::Voice1, terminal_params);

    // Set up drone parameter displays for each voice
    let drone_params_voice1 = TerminalViewParams {
        origin: vec2(-1900.0, 1050.0),
        num_lines: 50,              // Multiple lines for individual parameters
        width: 1000.0,
        line_spacing: 5.0,
        bright_color: rgba(0.7, 0.7, 0.7, 1.0),
        regular_color: rgba(0.3, 0.3, 0.3, 0.8),
        color_fade_secs: 1.5,
        chars_per_second: 6.0,     // Faster typing for parameters
        font: font.clone(),
        font_size: 22,
        justification: TextJustification::TopLeft,
    };

    let drone_params_voice4 = TerminalViewParams {
        origin: vec2(1900.0, 1050.0),
        num_lines: 50,
        width: 1000.0,
        line_spacing: 5.0,
        bright_color: rgba(0.7, 0.7, 0.7, 1.0),
        regular_color: rgba(0.3, 0.3, 0.3, 0.8),
        color_fade_secs: 1.5,
        chars_per_second: 6.0,
        font: font.clone(),
        font_size: 22,
        justification: TextJustification::TopRight,
    };

    terminal_manager.add_drone_parameters_display(VoiceId::Voice1, drone_params_voice1);
    terminal_manager.add_drone_parameters_display(VoiceId::Voice4, drone_params_voice4);

    Model {
        particle_system,
        voices: HashMap::new(),
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
        command_input: CommandInput::new(),
        terminal_manager: RefCell::new(terminal_manager),
        command_queue: Vec::new(),
        active_tab: 0,
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
    let mut commands = model.osc.process_messages();
    model.command_queue.append(&mut commands);

    // Process unified command queue with priority resolution
    model.process_command_queue();

    // Update feedback render params
    controller::update_feedback(model, device, queue);

    // Update particle system
    model
        .particle_system
        .update(&mut model.voices, &mut model.rng, &mut model.gpu_buffers);
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
            .get(&VoiceId::Voice1)
            .unwrap_or(empty_gpu_buffer);
        let gpu_buffer4 = model
            .gpu_buffers
            .get(&VoiceId::Voice4)
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

        // Update and draw terminal view as overlay on top of post-processed texture
        if let Some(terminal_view) = model.terminal_manager.borrow_mut().get_terminal_view("main") {
            terminal_view.update(&rendering.draw);
        }

        // Update and draw drone parameter displays
        model.terminal_manager.borrow_mut().update_drone_parameter_displays(&rendering.draw);

        // Encode Nannou Draw
        rendering.encode_draw_commands_into(device, &mut encoder, "terminal");

        if let Err(e) = rendering.execute_named_pipeline("final composite", device, &mut encoder) {
            eprintln!("Error executing final composite pipeline: {}", e);
        }

        rendering.submit_command_encoder(device, queue, encoder);

        // Update reshaper if needed (could be cached in Model)
        rendering.draw_to_frame(&model.audience_reshaper, &frame);
    }
    // End Rendering context


    // Show screen bounds if enabled
    if model.show_bounds {
        draw_bounds(app, model);

    }

    // Draw over the texture
    let _ = model.audience_draw.to_frame(app, &frame);

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
            .draw_forces(&model.voices, &model.performer_draw, scale_x, scale_y);
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
        Key::C => {
            model.osc_loop.send_inner_radius(4, 0.5);
        }
        Key::R => {
            model.osc_loop.send_outer_radius(4, 0.5);
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

    // Handle keyboard input for command terminal
    if let nannou::winit::event::WindowEvent::KeyboardInput { input, .. } = event {
        if input.state == nannou::winit::event::ElementState::Pressed {
            if let Some(key) = input.virtual_keycode {
                use nannou::winit::event::VirtualKeyCode;

                match key {
                    VirtualKeyCode::Escape => {
                        model.command_input.clear();
                    }
                    _ => {
                        // Handle character input
                        // Note: This is simplified - in a real app you'd want proper text input handling
                        // Note: Enter key handling now moved to egui TextEdit response system
                    }
                }
            }
        }
    }

    // Note: Text input is now handled directly by egui TextEdit widget
    // through the TextBuffer trait implementation
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

    // Voice 1 parameters
    let voice1_circle_ids = model.get_wind_circle_ids(VoiceId::Voice1);
    let voice1_all_circle_params: Vec<(usize, _)> = voice1_circle_ids.iter()
        .filter_map(|&id| model.get_wind_circle_params(VoiceId::Voice1, id).cloned().map(|params| (id, params)))
        .collect();
    let voice1_all_noise: Vec<(usize, f32)> = voice1_circle_ids.iter()
        .map(|&id| (id, model.get_noise(VoiceId::Voice1, id)))
        .collect();
    let voice1_alpha = model.get_alpha_limit(VoiceId::Voice1);
    let voice1_volume = model.get_volume(VoiceId::Voice1);
    let voice1_feedback = model.get_feedback(VoiceId::Voice1);
    let voice1_vibration_offset = model.get_vibration(VoiceId::Voice1);

    // Voice 4 parameters
    let voice4_circle_ids = model.get_wind_circle_ids(VoiceId::Voice4);
    let voice4_all_circle_params: Vec<(usize, _)> = voice4_circle_ids.iter()
        .filter_map(|&id| model.get_wind_circle_params(VoiceId::Voice4, id).cloned().map(|params| (id, params)))
        .collect();
    let voice4_all_noise: Vec<(usize, f32)> = voice4_circle_ids.iter()
        .map(|&id| (id, model.get_noise(VoiceId::Voice4, id)))
        .collect();
    let voice4_alpha = model.get_alpha_limit(VoiceId::Voice4);
    let voice4_volume = model.get_volume(VoiceId::Voice4);
    let voice4_feedback = model.get_feedback(VoiceId::Voice4);
    let voice4_vibration_offset = model.get_vibration(VoiceId::Voice4);

    let ctx = model.egui.begin_frame();

    // Set text style settings
    let style = (*ctx.style()).clone();
    ctx.set_style(adjust_style_from(style));

    let mut show_forces_changed = false;
    let mut command_queue = Vec::<Command>::new();

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
                // Vertical 1: Instructions and status info (always visible)
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

                    // Push tab selector to bottom with expanding space
                    ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                        // Tab bar at bottom
                        ui.add_space(20.0);
                        ui.horizontal(|ui| {
                            if ui
                                .selectable_label(model.active_tab == 0, "Terminal")
                                .clicked()
                            {
                                model.active_tab = 0;
                            }
                            if ui
                                .selectable_label(model.active_tab == 1, "Voices")
                                .clicked()
                            {
                                model.active_tab = 1;
                            }
                        });
                    });
                });

                ui.separator();

                // Tab content (top-aligned)
                ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    match model.active_tab {
                        1 => {
                            // Voices tab content with scrollable columns
                            ui.horizontal(|ui| {
                                // Voice 1 (col 2) - column with scrollable content
                                ui.vertical(|ui| {
                                    ui.set_width(320.0);
                                    ui.set_min_height(height);
                                    ui.heading("Voice 1: Drone");
                                    ui.add_space(2.0);
                                    egui::ScrollArea::vertical()
                                        .id_source("voice1_scroll")
                                        .auto_shrink([false, false])
                                        .show(ui, |ui| {

                                        // Voice-level parameters (always shown)
                                        ui.add_space(5.0);

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
                                            command_queue.push(Command::new(CommandInner::Simple(SimpleCommand::Alpha {
                                                voice_id: VoiceId::Voice1,
                                                value: alpha,
                                            }), CommandSource::Ui));
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
                                            command_queue.push(Command::new(CommandInner::Simple(SimpleCommand::Volume {
                                                voice_id: VoiceId::Voice1,
                                                value: volume,
                                            }), CommandSource::Ui));
                                        }

                                        // Vibration offset slider
                                        let mut vibration_offset = voice1_vibration_offset;
                                        if ui
                                            .add(
                                                egui::Slider::new(&mut vibration_offset, 0.0..=1.0)
                                                    .text("Vibration")
                                                    .custom_formatter(|n, _| format!("{:.3}", n)),
                                            )
                                            .changed()
                                        {
                                            command_queue.push(
                                                Command::new(CommandInner::Simple(SimpleCommand::Vibration {
                                                    voice_id: VoiceId::Voice1,
                                                    value: vibration_offset,
                                                }), CommandSource::Ui),
                                            );
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
                                            command_queue.push(
                                                Command::new(CommandInner::Simple(SimpleCommand::Feedback {
                                                    voice_id: VoiceId::Voice1,
                                                    value: feedback,
                                                }), CommandSource::Ui),
                                            );
                                        }

                                        ui.add_space(10.0);
                                        ui.separator();
                                        ui.add_space(5.0);
                                        ui.label("Wind Circles:");

                                        if !voice1_all_circle_params.is_empty() {
                                            // Horizontal scroll area for multiple circles
                                            egui::ScrollArea::horizontal()
                                                .id_source("voice1_circles_scroll")
                                                .auto_shrink([false, false])
                                                .show(ui, |ui| {
                                                    ui.horizontal(|ui| {
                                                        for (circle_id, params) in &voice1_all_circle_params {
                                                            let circle_noise = voice1_all_noise.iter()
                                                                .find(|(id, _)| id == circle_id)
                                                                .map(|(_, noise)| *noise)
                                                                .unwrap_or(0.0);

                                                            // Each circle gets its own vertical column
                                                            ui.vertical(|ui| {
                                                                ui.set_width(140.0);
                                                                ui.label(format!("Circle {}", circle_id));
                                                                ui.add_space(5.0);

                                                                // Outer Radius slider
                                                                let mut radius = params.outer_radius;
                                                                if ui
                                                                    .add(
                                                                        egui::Slider::new(&mut radius, 0.0..=1100.0)
                                                                            .text("OR")
                                                                            .custom_formatter(|n, _| format!("{:.1}", n)),
                                                                    )
                                                                    .changed()
                                                                {
                                                                    command_queue.push(
                                                                        Command::new(CommandInner::Simple(SimpleCommand::OuterRadius {
                                                                            voice_id: VoiceId::Voice1,
                                                                            circle_id: *circle_id,
                                                                            value: radius,
                                                                        }), CommandSource::Ui),
                                                                    );
                                                                }

                                                                // Inner Radius slider
                                                                let mut inner_radius = params.inner_radius;
                                                                if ui
                                                                    .add(
                                                                        egui::Slider::new(&mut inner_radius, 0.0..=1100.0)
                                                                            .text("IR")
                                                                            .custom_formatter(|n, _| format!("{:.1}", n)),
                                                                    )
                                                                    .changed()
                                                                {
                                                                    command_queue.push(
                                                                        Command::new(CommandInner::Simple(SimpleCommand::InnerRadius {
                                                                            voice_id: VoiceId::Voice1,
                                                                            circle_id: *circle_id,
                                                                            value: inner_radius,
                                                                        }), CommandSource::Ui),
                                                                    );
                                                                }

                                                                // Force slider
                                                                let mut strength = params.force;
                                                                if ui
                                                                    .add(
                                                                        egui::Slider::new(&mut strength, 0.0..=30.0)
                                                                            .text("Force")
                                                                            .custom_formatter(|n, _| format!("{:.1}", n)),
                                                                    )
                                                                    .changed()
                                                                {
                                                                    command_queue.push(
                                                                        Command::new(CommandInner::Simple(SimpleCommand::Force {
                                                                            voice_id: VoiceId::Voice1,
                                                                            circle_id: *circle_id,
                                                                            value: strength,
                                                                        }), CommandSource::Ui),
                                                                    );
                                                                }

                                                                // Gravity slider
                                                                let mut center_bias = params.gravity;
                                                                if ui
                                                                    .add(
                                                                        egui::Slider::new(&mut center_bias, 0.0..=2.0)
                                                                            .text("Gravity")
                                                                            .custom_formatter(|n, _| format!("{:.2}", n)),
                                                                    )
                                                                    .changed()
                                                                {
                                                                    command_queue.push(
                                                                        Command::new(CommandInner::Simple(SimpleCommand::Gravity {
                                                                            voice_id: VoiceId::Voice1,
                                                                            circle_id: *circle_id,
                                                                            value: center_bias,
                                                                        }), CommandSource::Ui),
                                                                    );
                                                                }

                                                                // Noise slider
                                                                let mut angle_variation = circle_noise;
                                                                if ui
                                                                    .add(
                                                                        egui::Slider::new(&mut angle_variation, 0.0..=1.0)
                                                                            .text("Noise")
                                                                            .custom_formatter(|n, _| format!("{:.2}", n)),
                                                                    )
                                                                    .changed()
                                                                {
                                                                    command_queue.push(
                                                                        Command::new(CommandInner::Simple(SimpleCommand::Noise {
                                                                            voice_id: VoiceId::Voice1,
                                                                            circle_id: *circle_id,
                                                                            value: angle_variation,
                                                                        }), CommandSource::Ui),
                                                                    );
                                                                }

                                                                // Center X slider
                                                                let mut center_x = params.center.x;
                                                                if ui
                                                                    .add(
                                                                        egui::Slider::new(&mut center_x, -2000.0..=2000.0)
                                                                            .text("Ctr X")
                                                                            .custom_formatter(|n, _| format!("{:.0}", n)),
                                                                    )
                                                                    .changed()
                                                                {
                                                                    command_queue.push(
                                                                        Command::new(CommandInner::Simple(SimpleCommand::CenterX {
                                                                            voice_id: VoiceId::Voice1,
                                                                            circle_id: *circle_id,
                                                                            value: center_x,
                                                                        }), CommandSource::Ui),
                                                                    );
                                                                }

                                                                // Center Y slider
                                                                let mut center_y = params.center.y;
                                                                if ui
                                                                    .add(
                                                                        egui::Slider::new(&mut center_y, -1100.0..=1100.0)
                                                                            .text("Ctr Y")
                                                                            .custom_formatter(|n, _| format!("{:.0}", n)),
                                                                    )
                                                                    .changed()
                                                                {
                                                                    command_queue.push(
                                                                        Command::new(CommandInner::Simple(SimpleCommand::CenterY {
                                                                            voice_id: VoiceId::Voice1,
                                                                            circle_id: *circle_id,
                                                                            value: center_y,
                                                                        }), CommandSource::Ui),
                                                                    );
                                                                }
                                                            });

                                                            ui.add_space(10.0);
                                                        }
                                                    });
                                                });
                                        } else {
                                            ui.label("No wind circles found");
                                            ui.label("Use: makeDrone(1).begin();");
                                        }
                                    }); // end Voice 1 scroll area
                                }); // end Voice 1 column

                                // Voice 2 (col 3) - column with scrollable content
                                ui.vertical(|ui| {
                                    ui.set_width(320.0);
                                    ui.set_min_height(height);
                                    ui.heading("Voice 2: Rhythm");
                                    ui.add_space(2.0);
                                    egui::ScrollArea::vertical()
                                        .id_source("voice2_scroll")
                                        .auto_shrink([false, false])
                                        .show(ui, |_ui| {
                                    }); // end Voice 2 scroll area
                                }); // end Voice 2 column

                                // Voice 3 - column with scrollable content
                                ui.vertical(|ui| {
                                    ui.set_width(320.0);
                                    ui.set_min_height(height);
                                    ui.heading("Voice 3: Rhythm");
                                    ui.add_space(2.0);
                                    egui::ScrollArea::vertical()
                                        .id_source("voice3_scroll")
                                        .auto_shrink([false, false])
                                        .show(ui, |ui| {
                                        
                                        // Voice 3 rhythm controls placeholder
                                        ui.label("Rhythm controls");
                                        ui.label("coming soon...");
                                    }); // end Voice 3 scroll area
                                }); // end Voice 3 column

                                // Voice 4: Column 5 - column with scrollable content
                                ui.vertical(|ui| {
                                    ui.set_width(320.0);
                                    ui.set_min_height(height);
                                    ui.heading("Voice 4: Drone");
                                    ui.add_space(2.0);
                                    egui::ScrollArea::vertical()
                                        .id_source("voice4_scroll")
                                        .auto_shrink([false, false])
                                        .show(ui, |ui| {

                                        // Voice-level parameters (always shown)
                                        ui.add_space(5.0);

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
                                            command_queue.push(Command::new(CommandInner::Simple(SimpleCommand::Alpha {
                                                voice_id: VoiceId::Voice4,
                                                value: alpha,
                                            }), CommandSource::Ui));
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
                                            command_queue.push(Command::new(CommandInner::Simple(SimpleCommand::Volume {
                                                voice_id: VoiceId::Voice4,
                                                value: volume,
                                            }), CommandSource::Ui));
                                        }

                                        // Vibration offset slider
                                        let mut vibration_offset = voice4_vibration_offset;
                                        if ui
                                            .add(
                                                egui::Slider::new(&mut vibration_offset, 0.0..=1.0)
                                                    .text("Vibration")
                                                    .custom_formatter(|n, _| format!("{:.3}", n)),
                                            )
                                            .changed()
                                        {
                                            command_queue.push(
                                                Command::new(CommandInner::Simple(SimpleCommand::Vibration {
                                                    voice_id: VoiceId::Voice4,
                                                    value: vibration_offset,
                                                }), CommandSource::Ui),
                                            );
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
                                            command_queue.push(
                                                Command::new(CommandInner::Simple(SimpleCommand::Feedback {
                                                    voice_id: VoiceId::Voice4,
                                                    value: feedback,
                                                }), CommandSource::Ui),
                                            );
                                        }

                                        ui.add_space(10.0);
                                        ui.separator();
                                        ui.add_space(5.0);
                                        ui.label("Wind Circles:");

                                        if !voice4_all_circle_params.is_empty() {
                                            // Horizontal scroll area for multiple circles
                                            egui::ScrollArea::horizontal()
                                                .id_source("voice4_circles_scroll")
                                                .auto_shrink([false, false])
                                                .show(ui, |ui| {
                                                    ui.horizontal(|ui| {
                                                        for (circle_id, params) in &voice4_all_circle_params {
                                                            let circle_noise = voice4_all_noise.iter()
                                                                .find(|(id, _)| id == circle_id)
                                                                .map(|(_, noise)| *noise)
                                                                .unwrap_or(0.0);

                                                            // Each circle gets its own vertical column
                                                            ui.vertical(|ui| {
                                                                ui.set_width(140.0);
                                                                ui.label(format!("Circle {}", circle_id));
                                                                ui.add_space(5.0);

                                                                // Outer Radius slider
                                                                let mut radius = params.outer_radius;
                                                                if ui
                                                                    .add(
                                                                        egui::Slider::new(&mut radius, 0.0..=1100.0)
                                                                            .text("OR")
                                                                            .custom_formatter(|n, _| format!("{:.1}", n)),
                                                                    )
                                                                    .changed()
                                                                {
                                                                    command_queue.push(
                                                                        Command::new(CommandInner::Simple(SimpleCommand::OuterRadius {
                                                                            voice_id: VoiceId::Voice4,
                                                                            circle_id: *circle_id,
                                                                            value: radius,
                                                                        }), CommandSource::Ui),
                                                                    );
                                                                }

                                                                // Inner Radius slider
                                                                let mut inner_radius = params.inner_radius;
                                                                if ui
                                                                    .add(
                                                                        egui::Slider::new(&mut inner_radius, 0.0..=1100.0)
                                                                            .text("IR")
                                                                            .custom_formatter(|n, _| format!("{:.1}", n)),
                                                                    )
                                                                    .changed()
                                                                {
                                                                    command_queue.push(
                                                                        Command::new(CommandInner::Simple(SimpleCommand::InnerRadius {
                                                                            voice_id: VoiceId::Voice4,
                                                                            circle_id: *circle_id,
                                                                            value: inner_radius,
                                                                        }), CommandSource::Ui),
                                                                    );
                                                                }

                                                                // Force slider
                                                                let mut strength = params.force;
                                                                if ui
                                                                    .add(
                                                                        egui::Slider::new(&mut strength, 0.0..=30.0)
                                                                            .text("Force")
                                                                            .custom_formatter(|n, _| format!("{:.1}", n)),
                                                                    )
                                                                    .changed()
                                                                {
                                                                    command_queue.push(
                                                                        Command::new(CommandInner::Simple(SimpleCommand::Force {
                                                                            voice_id: VoiceId::Voice4,
                                                                            circle_id: *circle_id,
                                                                            value: strength,
                                                                        }), CommandSource::Ui),
                                                                    );
                                                                }

                                                                // Gravity slider
                                                                let mut center_bias = params.gravity;
                                                                if ui
                                                                    .add(
                                                                        egui::Slider::new(&mut center_bias, 0.0..=2.0)
                                                                            .text("Gravity")
                                                                            .custom_formatter(|n, _| format!("{:.2}", n)),
                                                                    )
                                                                    .changed()
                                                                {
                                                                    command_queue.push(
                                                                        Command::new(CommandInner::Simple(SimpleCommand::Gravity {
                                                                            voice_id: VoiceId::Voice4,
                                                                            circle_id: *circle_id,
                                                                            value: center_bias,
                                                                        }), CommandSource::Ui),
                                                                    );
                                                                }

                                                                // Noise slider
                                                                let mut angle_variation = circle_noise;
                                                                if ui
                                                                    .add(
                                                                        egui::Slider::new(&mut angle_variation, 0.0..=1.0)
                                                                            .text("Noise")
                                                                            .custom_formatter(|n, _| format!("{:.2}", n)),
                                                                    )
                                                                    .changed()
                                                                {
                                                                    command_queue.push(
                                                                        Command::new(CommandInner::Simple(SimpleCommand::Noise {
                                                                            voice_id: VoiceId::Voice4,
                                                                            circle_id: *circle_id,
                                                                            value: angle_variation,
                                                                        }), CommandSource::Ui),
                                                                    );
                                                                }

                                                                // Center X slider
                                                                let mut center_x = params.center.x;
                                                                if ui
                                                                    .add(
                                                                        egui::Slider::new(&mut center_x, -2000.0..=2000.0)
                                                                            .text("Ctr X")
                                                                            .custom_formatter(|n, _| format!("{:.0}", n)),
                                                                    )
                                                                    .changed()
                                                                {
                                                                    command_queue.push(
                                                                        Command::new(CommandInner::Simple(SimpleCommand::CenterX {
                                                                            voice_id: VoiceId::Voice4,
                                                                            circle_id: *circle_id,
                                                                            value: center_x,
                                                                        }), CommandSource::Ui),
                                                                    );
                                                                }

                                                                // Center Y slider
                                                                let mut center_y = params.center.y;
                                                                if ui
                                                                    .add(
                                                                        egui::Slider::new(&mut center_y, -1100.0..=1100.0)
                                                                            .text("Ctr Y")
                                                                            .custom_formatter(|n, _| format!("{:.0}", n)),
                                                                    )
                                                                    .changed()
                                                                {
                                                                    command_queue.push(
                                                                        Command::new(CommandInner::Simple(SimpleCommand::CenterY {
                                                                            voice_id: VoiceId::Voice4,
                                                                            circle_id: *circle_id,
                                                                            value: center_y,
                                                                        }), CommandSource::Ui),
                                                                    );
                                                                }
                                                            });

                                                            ui.add_space(10.0);
                                                        }
                                                    });
                                                });
                                        } else {
                                            ui.label("No wind circles found");
                                            ui.label("Use: makeDrone(4).begin();");
                                        }
                                    }); // end Voice 4 scroll area
                                }); // end Voice 4 column
                            }); // end voices horizontal layout
                        }
                        0 => {
                            // NTerminal tab content - two column layout with scrollbars
                            ui.horizontal(|ui| {
                                // Left column: Command input and status - column with scrollable content
                                ui.vertical(|ui| {
                                    ui.set_width(480.0);
                                    ui.set_min_height(height);
                                    ui.heading("Terminal Interface");
                                    ui.add_space(2.0);
                                    egui::ScrollArea::vertical()
                                        .id_source("terminal_input_scroll")
                                        .auto_shrink([false, false])
                                        .show(ui, |ui| {

                                    // Multi-line text input using TextBuffer implementation
                                    ui.label("Input:");
                                    ui.add_space(5.0);
                                    
                                    // Editable text area using CommandInput as TextBuffer
                                    egui::Frame::none()
                                        .fill(egui::Color32::BLACK)
                                        .stroke(egui::Stroke::new(1.0, egui::Color32::WHITE))
                                        .inner_margin(egui::style::Margin::symmetric(8.0, 8.0))
                                        .show(ui, |ui| {
                                            ui.set_min_size(egui::vec2(280.0, 150.0));
                                            ui.set_max_height(150.0);
                    
                                            egui::ScrollArea::vertical()
                                                .max_width(380.0)
                                                .max_height(150.0)
                                                .show(ui, |ui| {
                                                    // Use TextEdit with proper Enter key handling
                                                    let response = ui.add(
                                                        egui::TextEdit::multiline(&mut model.command_input)
                                                            .font(egui::TextStyle::Body)
                                                            .frame(false)
                                                            .min_size(egui::vec2(280.0, 150.0))
                                                            .interactive(true)
                                                            .desired_width(f32::INFINITY)
                                                            .lock_focus(true)
                                                            .hint_text("Type command here...")
                                                    );

                                                    // Update terminal display with live command text
                                                    if response.changed() {
                                                        model.terminal_manager.borrow_mut().update_from_command_input("main", &model.command_input);
                                                    }
                                                    
                                                    // Handle Enter key press through egui input system
                                                    // Check for Enter key pressed while the text field has focus
                                                    if response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                                        && model.command_input.is_ready_for_execution() {
                                                            if let Some(command) = model.command_input.try_execute() {
                                                                println!("Executing command: {:?}", command);
                                                                
                                                                // Execute the command using unified VoiceCommand system
                                                                match command {
                                                                    TerminalCommand::CreateDrone(config) => {
                                                                        let voice_command = config.to_create_command(CommandSource::Terminal);
                                                                        println!("Queuing CreateDrone command with config: {:?}", config);
                                                                        command_queue.push(voice_command);
                                                                    }
                                                                    TerminalCommand::ModifyVoice { voice, config } => {
                                                                        let voice_enum = VoiceId::from_i32(voice);
                                                                        let voice_command = config.to_modify_command(voice_enum, CommandSource::Terminal);
                                                                        println!("Queuing ModifyVoice command for voice: {:?} with config: {:?}", voice, config);
                                                                        command_queue.push(voice_command);
                                                                    }
                                                                    TerminalCommand::ModifyVoiceCircle { voice, circle, config} => {
                                                                        let voice_enum = VoiceId::from_i32(voice);

                                                                        // Generate circle-specific parameter commands
                                                                        let parameter_commands = config.generate_circle_parameter_commands(
                                                                            voice_enum,
                                                                            circle as usize,
                                                                            CommandSource::Terminal
                                                                        );

                                                                        println!("Queueing {} circle parameter commands for voice: {:?} circle: {:?}",
                                                                                parameter_commands.len(), voice, circle);

                                                                        for cmd in parameter_commands {
                                                                            command_queue.push(cmd);
                                                                        }
                                                                    }
                                                                    TerminalCommand::ListCircles { voice } => {
                                                                        let voice_enum = VoiceId::from_i32(voice);
                                                                        let voice_command = Command::new(
                                                                            CommandInner::Simple(SimpleCommand::ListCircles { voice_id: voice_enum }),
                                                                            CommandSource::Terminal
                                                                        );
                                                                        println!("Queueing ListCircles command for voice: {:?}", voice);
                                                                        command_queue.push(voice_command);
                                                                    }
                                                                    TerminalCommand::NewCircle { voice, config } => {
                                                                        let voice_enum = VoiceId::from_i32(voice);
                                                                        let voice_command = Command::new(
                                                                            CommandInner::Composite(CompositeCommand::NewCircle { voice_id: voice_enum, config }),
                                                                            CommandSource::Terminal
                                                                        );
                                                                        println!("Queueing NewCircle command for voice: {:?}", voice);
                                                                        command_queue.push(voice_command);
                                                                    }
                                                                }
                                                                
                                                                // Clear the input after successful execution
                                                                model.command_input.clear();
                                                                
                                                                // Clear the terminal display
                                                                model.terminal_manager.borrow_mut().clear_terminal("main");
                                                            }
                                                        }
                                                    
                                                });
                                        });

                                    ui.add_space(10.0);
                                    
                                    // Command status display
                                    ui.horizontal(|ui| {
                                        ui.label("Status:");
                                        
                                        // Priority: Show execution results first
                                        if let Some(success) = model.command_input.last_success() {
                                            ui.colored_label(egui::Color32::GREEN, format!("✅ {}", success));
                                        } else if let Some(error) = model.command_input.last_error() {
                                            ui.colored_label(egui::Color32::RED, format!("❌ Error: {}", error));
                                        } else if model.command_input.is_ready_for_execution() {
                                            ui.colored_label(egui::Color32::LIGHT_GREEN, "Ready to execute (press Enter)");
                                        } else if model.command_input.is_empty() {
                                            ui.colored_label(egui::Color32::GRAY, "Ready for input");
                                        } else {
                                            ui.colored_label(egui::Color32::YELLOW, "Add semicolon (;) to execute");
                                        }
                                    });
                                    
                                    // Show formatted display preview
                                    ui.add_space(5.0);
                                    ui.label("Preview:");
                                    ui.add_space(2.0);
                                    
                                    egui::Frame::none()
                                        .fill(egui::Color32::DARK_GRAY)
                                        .stroke(egui::Stroke::new(1.0, egui::Color32::GRAY))
                                        .inner_margin(egui::style::Margin::symmetric(6.0, 6.0))
                                        .show(ui, |ui| {
                                            let display_text = model.command_input.display();
                                            if display_text.is_empty() {
                                                ui.colored_label(egui::Color32::GRAY, "Command preview will appear here...");
                                            } else {
                                                ui.label(display_text);
                                            }
                                        });

                                    }); // end left column scroll area
                                }); // end left column
                                
                                ui.separator();
                                
                                // Right column: Examples and help - column with scrollable content
                                ui.vertical(|ui| {
                                    ui.set_width(480.0);
                                    ui.set_min_height(height);
                                    ui.heading("Examples");
                                    ui.add_space(2.0);
                                    egui::ScrollArea::vertical()
                                        .id_source("terminal_help_scroll")
                                        .auto_shrink([false, false])
                                        .show(ui, |ui| {

                                    for example in
                                        system4::terminals::command_input::CommandInput::get_examples()
                                    {
                                        ui.label(format!("• {}", example));
                                        ui.add_space(2.0);
                                    }
                                    
                                    ui.add_space(20.0);
                                    ui.heading("Controls");
                                    ui.add_space(5.0);
                                    ui.label("• Type commands and press Enter to add lines");
                                    ui.label("• Commands ending with ';' will execute");
                                    ui.label("• Backspace to edit, Escape to clear");
                                    
                                    ui.add_space(15.0);
                                    ui.heading("Syntax Guide");
                                    ui.add_space(5.0);
                                    ui.label("• Create: makeDrone(voice).params().begin();");
                                    ui.label("• Modify: drone(voice).params().set();");
                                    ui.label("• Parameters:brightness(), volume(), gravity(), etc");
                                    ui.label("• Values: strings in \"quotes\", numbers");
                                    }); // end right column scroll area
                                }); // end right column
                            }); // end terminal horizontal layout
                        }
                        _ => {}
                    }
                }); // end top-aligned layout
            }); // end main horizontal layout
        });

    drop(ctx);

    // Queue all UI voice commands for priority processing
    for command in command_queue {
        model.queue_command(command);
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


/*
fn _erase_drone(model: &mut Model, id: i32) {
    let voice = VoiceId::from_i32(id);
    model.particle_system.kill_voice(&voice);
    model.particle_system.forces.recalculate_once();
    model.osc_send.send_drone_on_off(id, 0);
}
*/

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
