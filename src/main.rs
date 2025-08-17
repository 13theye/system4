// System 4
//
// (c) 2025 13th Eye LLC & Tacit Group
//
//
// src/main.rs

use nannou::{prelude::*, rand::rngs::ThreadRng, text::Font, wgpu::TextureReshaper};
use nannou_egui::Egui;
use nnpipe::*;
use thread_priority::*;

use std::{collections::HashMap, fs};

use system4::{
    config::*,
    forces::WindCircle,
    fps::FpsManager,
    osc::{OscCommand, OscController, OscSender},
    particle::ParticleSystem,
    rendering::HeatmapRenderer,
    terminals::{TerminalParams, TerminalSystem},
    utils::IdGenerator,
    view::Voice,
};

const DEFAULT_PARTICLE_SIZE: f32 = 8.0;
const DEFAULT_PARTICLE_RGB: (f32, f32, f32) = (0.63, 0.63, 0.64);
//const DEFAULT_PARTICLE_RGB: (f32, f32, f32) = (0.27, 0.27, 0.26);
// full brightness color for terminal
const TERMINAL_START_RGBA: (f32, f32, f32, f32) = (0.0, 0.85, 0.0, 1.0);
// dimmed brightness color for terminal
const TERMINAL_END_RGBA: (f32, f32, f32, f32) = (0.0, 0.3, 0.0, 1.0);
const TERMINAL_NUM_LINES: usize = 8;
const TERMINAL_LINE_MARGIN: f32 = 8.0;
const TERMINAL_CHARS_PER_SECOND: f32 = 0.6; // final is 0.6

struct Model {
    particle_system: ParticleSystem,
    particle_limit: u32,

    terminal_system: TerminalSystem,

    // OSC
    osc: OscController,
    osc_send: OscSender,
    osc_loop: OscSender,

    // Windows' texture reshapers
    render_size: Vec2,
    render_rect: Rect,
    audience_window_id: WindowId,
    performer_window_id: WindowId,
    control_window_id: WindowId,
    audience_reshaper: TextureReshaper,
    performer_reshaper: TextureReshaper,

    // Nannou API
    draw: nannou::Draw,           // for drawing to the main texture
    audience_draw: nannou::Draw,  // for drawing UI elements to audience_window only
    performer_draw: nannou::Draw, // for drawing UI elements to performer_window only
    control_draw: nannou::Draw,   // for drawing UI elements to ui_window only

    // Rendering engine
    rendering: Nnpipe,
    heatmap_renderer: HeatmapRenderer,
    dpi_scale: f32,
    font: Font,

    // Simple ID counter
    id_generator: IdGenerator,

    // Egui
    egui: Egui,

    // Random
    rng: ThreadRng,

    // FPS display
    fps: FpsManager,

    // Frame counter for optimization
    frame_count: u64,

    // UI state
    selected_circle_id: HashMap<Voice, Option<usize>>,

    // Debug stuff
    show_bounds: bool,
    show_forces: bool,
}

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

    let particle_system = ParticleSystem::new(
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
    #[cfg(target_os = "macos")]
    set_macos_window_behavior(&audience_window);
    #[cfg(target_os = "macos")]
    set_macos_window_behavior(&performer_window);
    #[cfg(target_os = "macos")]
    set_macos_window_behavior(&control_window);

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
    let draw = nannou::Draw::new();

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

    let effects = PipelineBuilder::new()
        .name("Effects Pipeline")
        .feedback(hi_config, 1.0, 1.0)
        .update_scene()
        .brightness_extract(med_config, 1.2)
        .downsample(lo_config)
        .gaussian_blur_passes(lo_config, 2, 2.0, 5.0)
        .bloom_composite_with_curve(hi_config, 2.0, 3.0)
        .inversion(hi_config)
        .build(device);

    if let Ok(effect) = effects {
        rendering.add_effect(effect);
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
        particle_limit,
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
        draw,
        audience_draw,
        performer_draw,
        control_draw,
        rendering,
        heatmap_renderer,
        egui,
        rng,
        fps,
        frame_count: 0,
        selected_circle_id: HashMap::from([
            (Voice::Voice1, None),
            (Voice::Voice2, None),
            (Voice::Voice3, None),
            (Voice::Voice4, None),
        ]),
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
    nannou::app(model).update(update).run();
}

fn update(app: &App, model: &mut Model, _update: Update) {
    // Increment frame counter
    model.frame_count += 1;

    // First, render GPU heatmap with current particle positions
    let window = app.main_window();
    let device = window.device();
    let queue = window.queue();

    // Set background
    model.draw.background().color(BLACK);

    // Update FPS counter
    model.fps.update();

    // Update control UI
    update_control_ui(app, model);

    // Process OSC commands
    model.osc.process_messages();
    let commands = model.osc.take_commands();
    process_osc(model, commands);

    // Update particle system and get live positions
    let particle_positions = model
        .particle_system
        .update(&mut model.rng, model.show_forces);

    // Render heatmap
    /*
    model.heatmap_renderer.render_heatmap(
        device,
        queue,
        &particle_positions,
        model.render_rect,
        model.frame_count,
    );


    // Draw heatmap as texture
    let heatmap_view = model.heatmap_renderer.get_heatmap_view();
    model.draw.texture(heatmap_view).wh(model.render_size);
     */

    // Draw particles
    model.particle_system.draw(&model.draw);

    // Update terminals
    let finish_signals = model.terminal_system.update(&model.draw);

    // When a terminal start sequence is finished, send the OSC command to turn on the drone
    for (voice, finish_signal) in finish_signals {
        if finish_signal {
            let id = voice.to_i32();
            model.osc_send.send_drone_on_off(id, 1);
            model.particle_system.set_is_spawning(&voice, true);
        }
    }

    render_and_post(app, model);
}

fn audience_view(app: &App, model: &Model, frame: Frame) {
    // Get the post-processed texture view
    let _scene_view = model.rendering.get_scene_view();

    // Update reshaper if needed (could be cached in Model)
    model
        .rendering
        .draw_to_frame(&model.audience_reshaper, &frame);

    // Show screen bounds if enabled
    if model.show_bounds {
        draw_bounds(app, model);

        // Then draw over the texture
        let _ = model.audience_draw.to_frame(app, &frame);
    }
}

fn performer_view(app: &App, model: &Model, frame: Frame) {
    // Get the raw scene texture view
    let _scene_view = model.rendering.get_scene_view();

    // Draw game content to the frame
    model
        .rendering
        .draw_to_frame(&model.performer_reshaper, &frame);

    // Show force vectors if enabled
    if model.show_forces {
        let performer_rect = app.window(model.performer_window_id).unwrap().rect();

        // Create a scaled draw context that matches texture coordinates
        let texture_size = model.rendering.scene_texture.size();

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

// ******************************* Rendering and Capture *****************************
fn render_and_post(app: &App, model: &mut Model) {
    // Get the window device and queue
    let window = app.main_window();
    let device = window.device();
    let queue = window.queue();

    // Render the game to texture
    model.rendering.render_scene(device, queue, &model.draw);

    model.rendering.post_process(device, queue);
    //model.rendering.direct_to_view(device, queue);
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

    let ctx = model.egui.begin_frame();

    // Set text style settings
    let style = (*ctx.style()).clone();
    ctx.set_style(adjust_style_from(style));

    let mut show_forces_changed = false;

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

                // Wind Circle Settings - use horizontal layout for two vertical sections

                // Voice 1 (col 2)
                ui.vertical(|ui| {
                    ui.set_min_width(350.0);
                    ui.heading("Voice 1: Drone");
                    ui.add_space(2.0);

                    let settings = model
                        .particle_system
                        .forces
                        .get_circle_params_by_voice(Voice::Voice1);
                    if settings.is_empty() {
                        ui.label("No wind circles found");
                        ui.label("1: Create wind circle");
                    } else {
                        // Handle circle selection - set default if none selected
                        let current_selection = model
                            .selected_circle_id
                            .get(&Voice::Voice1)
                            .copied()
                            .flatten();
                        if current_selection.is_none()
                            || !settings.contains_key(&current_selection.unwrap())
                        {
                            let new_selection = settings.keys().next().copied();
                            model
                                .selected_circle_id
                                .insert(Voice::Voice1, new_selection);
                        }

                        if let Some(selected_id) = model
                            .selected_circle_id
                            .get(&Voice::Voice1)
                            .copied()
                            .flatten()
                        {
                            // Dropdown to select circle
                            let current_voice_selection = model
                                .selected_circle_id
                                .get(&Voice::Voice1)
                                .copied()
                                .flatten();
                            /*
                            egui::ComboBox::from_id_source("voice1_circle_selector")
                                .selected_text(format!("Circle {}", selected_id))
                                .show_ui(ui, |ui| {
                                    for (id, _) in settings.iter() {
                                        ui.selectable_value(
                                            &mut current_voice_selection,
                                            Some(*id),
                                            format!("Circle {}", id),
                                        );
                                    }
                                });
                                 */

                            // Update the selection if it changed
                            if current_voice_selection
                                != model
                                    .selected_circle_id
                                    .get(&Voice::Voice1)
                                    .copied()
                                    .flatten()
                            {
                                model
                                    .selected_circle_id
                                    .insert(Voice::Voice1, current_voice_selection);
                            }
                        }
                    }

                    ui.add_space(10.0);

                    if let Some(selected_id) = model
                        .selected_circle_id
                        .get(&Voice::Voice1)
                        .copied()
                        .flatten()
                    {
                        // Get current parameter values by cloning them
                        let current_params = model
                            .particle_system
                            .forces
                            .get_circle_params_all()
                            .get(&selected_id)
                            .cloned();

                        if let Some(params) = current_params {
                            // Radius slider
                            let mut radius = params.outer_radius;
                            if ui
                                .add(
                                    egui::Slider::new(&mut radius, 0.0..=1100.0)
                                        .text("OR")
                                        .custom_formatter(|n, _| format!("{:.3}", n)),
                                )
                                .changed()
                            {
                                if let Some(circle) = model
                                    .particle_system
                                    .forces
                                    .wind_circles
                                    .get_mut(&selected_id)
                                {
                                    circle.params_mut().outer_radius(radius);
                                }
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
                                if let Some(circle) = model
                                    .particle_system
                                    .forces
                                    .wind_circles
                                    .get_mut(&selected_id)
                                {
                                    circle.params_mut().inner_radius(inner_radius);
                                }
                            }

                            // Alpha slider
                            let mut alpha = model
                                .particle_system
                                .alpha_limits
                                .get(&Voice::Voice1)
                                .copied()
                                .unwrap_or(1.0);
                            if ui
                                .add(
                                    egui::Slider::new(&mut alpha, 0.0..=1.0)
                                        .text("Brightness")
                                        .custom_formatter(|n, _| format!("{:.3}", n)),
                                )
                                .changed()
                            {
                                model.particle_system.set_alpha_limit(&Voice::Voice1, alpha);
                            }

                            // Volume slider
                            let mut volume = model
                                .particle_system
                                .particle_num_factors
                                .get(&Voice::Voice1)
                                .copied()
                                .unwrap_or(0.2);
                            if ui
                                .add(
                                    egui::Slider::new(&mut volume, 0.0..=1.0)
                                        .text("Volume")
                                        .custom_formatter(|n, _| format!("{:.3}", n)),
                                )
                                .changed()
                            {
                                model
                                    .particle_system
                                    .set_num_particles(&Voice::Voice1, volume);
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
                                if let Some(circle) = model
                                    .particle_system
                                    .forces
                                    .wind_circles
                                    .get_mut(&selected_id)
                                {
                                    circle.params_mut().strength(strength);
                                }
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
                                if let Some(circle) = model
                                    .particle_system
                                    .forces
                                    .wind_circles
                                    .get_mut(&selected_id)
                                {
                                    circle.params_mut().center_bias(center_bias);
                                }
                            }

                            // Feedback slider
                            let mut feedback = model
                                .particle_system
                                .feedback
                                .get(&Voice::Voice1)
                                .copied()
                                .unwrap_or(0.0);
                            if ui
                                .add(
                                    egui::Slider::new(&mut feedback, 0.0..=1.0)
                                        .text("Feedback")
                                        .custom_formatter(|n, _| format!("{:.3}", n)),
                                )
                                .changed()
                            {
                                model.particle_system.set_feedback(&Voice::Voice1, feedback);
                            }

                            // Center X slider
                            let mut center_x = params.center.x;
                            if ui
                                .add(
                                    egui::Slider::new(&mut center_x, -2000.0..=2000.0)
                                        .text("Ctr X"),
                                )
                                .changed()
                            {
                                if let Some(circle) = model
                                    .particle_system
                                    .forces
                                    .wind_circles
                                    .get_mut(&selected_id)
                                {
                                    {
                                        let current_y = circle.params().center.y;
                                        circle.params_mut().center(vec2(center_x, current_y));
                                    }
                                }
                            }

                            // Center Y slider
                            let mut center_y = params.center.y;
                            if ui
                                .add(
                                    egui::Slider::new(&mut center_y, -1100.0..=1100.0)
                                        .text("Ctr Y"),
                                )
                                .changed()
                            {
                                if let Some(circle) = model
                                    .particle_system
                                    .forces
                                    .wind_circles
                                    .get_mut(&selected_id)
                                {
                                    {
                                        let current_x = circle.params().center.x;
                                        circle.params_mut().center(vec2(current_x, center_y));
                                    }
                                }
                            }
                        } else {
                            ui.label("No parameters available");
                        }
                    } else {
                        ui.label("No circle selected");
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

                    let settings = model
                        .particle_system
                        .forces
                        .get_circle_params_by_voice(Voice::Voice4);
                    if settings.is_empty() {
                        ui.label("No wind circles found");
                        ui.label("4: Create wind circle");
                    } else {
                        // Handle circle selection - set default if none selected
                        let current_selection = model
                            .selected_circle_id
                            .get(&Voice::Voice4)
                            .copied()
                            .flatten();
                        if current_selection.is_none()
                            || !settings.contains_key(&current_selection.unwrap())
                        {
                            let new_selection = settings.keys().next().copied();
                            model
                                .selected_circle_id
                                .insert(Voice::Voice4, new_selection);
                        }

                        if let Some(selected_id) = model
                            .selected_circle_id
                            .get(&Voice::Voice4)
                            .copied()
                            .flatten()
                        {
                            // Dropdown to select circle
                            let current_voice_selection = model
                                .selected_circle_id
                                .get(&Voice::Voice4)
                                .copied()
                                .flatten();
                            /* Leaving this out because only one circle per voice
                            egui::ComboBox::from_id_source("voice4_circle_selector")
                                .selected_text(format!("Circle {}", selected_id))
                                .show_ui(ui, |ui| {
                                    for (id, _) in settings.iter() {
                                        ui.selectable_value(
                                            &mut current_voice_selection,
                                            Some(*id),
                                            format!("Circle {}", id),
                                        );
                                    }
                                });
                                 */

                            // Update the selection if it changed
                            if current_voice_selection
                                != model
                                    .selected_circle_id
                                    .get(&Voice::Voice4)
                                    .copied()
                                    .flatten()
                            {
                                model
                                    .selected_circle_id
                                    .insert(Voice::Voice4, current_voice_selection);
                            }
                        }
                    }

                    ui.add_space(10.0);

                    if let Some(selected_id) = model
                        .selected_circle_id
                        .get(&Voice::Voice4)
                        .copied()
                        .flatten()
                    {
                        // Get current parameter values by cloning them
                        let current_params = model
                            .particle_system
                            .forces
                            .get_circle_params_all()
                            .get(&selected_id)
                            .cloned();

                        if let Some(params) = current_params {
                            // Radius slider
                            let mut radius = params.outer_radius;
                            if ui
                                .add(
                                    egui::Slider::new(&mut radius, 0.0..=1100.0)
                                        .text("OR")
                                        .custom_formatter(|n, _| format!("{:.3}", n)),
                                )
                                .changed()
                            {
                                if let Some(circle) = model
                                    .particle_system
                                    .forces
                                    .wind_circles
                                    .get_mut(&selected_id)
                                {
                                    circle.params_mut().outer_radius(radius);
                                }
                            }

                            // Inner radius slider
                            let mut inner_radius = params.inner_radius;
                            if ui
                                .add(
                                    egui::Slider::new(&mut inner_radius, 0.0..=1100.0)
                                        .text("IR")
                                        .custom_formatter(|n, _| format!("{:.3}", n)),
                                )
                                .changed()
                            {
                                if let Some(circle) = model
                                    .particle_system
                                    .forces
                                    .wind_circles
                                    .get_mut(&selected_id)
                                {
                                    circle.params_mut().inner_radius(inner_radius);
                                }
                            }

                            // Alpha slider
                            let mut alpha = model
                                .particle_system
                                .alpha_limits
                                .get(&Voice::Voice4)
                                .copied()
                                .unwrap_or(1.0);
                            if ui
                                .add(
                                    egui::Slider::new(&mut alpha, 0.0..=1.0)
                                        .text("Brightness")
                                        .custom_formatter(|n, _| format!("{:.3}", n)),
                                )
                                .changed()
                            {
                                model.particle_system.set_alpha_limit(&Voice::Voice4, alpha);
                            }

                            // Volume slider
                            let mut volume = model
                                .particle_system
                                .particle_num_factors
                                .get(&Voice::Voice4)
                                .copied()
                                .unwrap_or(0.2);
                            if ui
                                .add(
                                    egui::Slider::new(&mut volume, 0.0..=1.0)
                                        .text("Volume")
                                        .custom_formatter(|n, _| format!("{:.3}", n)),
                                )
                                .changed()
                            {
                                model
                                    .particle_system
                                    .set_num_particles(&Voice::Voice4, volume);
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
                                if let Some(circle) = model
                                    .particle_system
                                    .forces
                                    .wind_circles
                                    .get_mut(&selected_id)
                                {
                                    circle.params_mut().strength(strength);
                                }
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
                                if let Some(circle) = model
                                    .particle_system
                                    .forces
                                    .wind_circles
                                    .get_mut(&selected_id)
                                {
                                    circle.params_mut().center_bias(center_bias);
                                }
                            }

                            // Feedback slider
                            let mut feedback = model
                                .particle_system
                                .feedback
                                .get(&Voice::Voice4)
                                .copied()
                                .unwrap_or(0.0);
                            if ui
                                .add(
                                    egui::Slider::new(&mut feedback, 0.0..=1.0)
                                        .text("Feedback")
                                        .custom_formatter(|n, _| format!("{:.3}", n)),
                                )
                                .changed()
                            {
                                model.particle_system.set_feedback(&Voice::Voice4, feedback);
                            }

                            // Center X slider
                            let mut center_x = params.center.x;
                            if ui
                                .add(
                                    egui::Slider::new(&mut center_x, -2000.0..=2000.0)
                                        .text("Ctr X"),
                                )
                                .changed()
                            {
                                if let Some(circle) = model
                                    .particle_system
                                    .forces
                                    .wind_circles
                                    .get_mut(&selected_id)
                                {
                                    {
                                        let current_y = circle.params().center.y;
                                        circle.params_mut().center(vec2(center_x, current_y));
                                    }
                                }
                            }

                            // Center Y slider
                            let mut center_y = params.center.y;
                            if ui
                                .add(
                                    egui::Slider::new(&mut center_y, -1100.0..=1100.0)
                                        .text("Ctr Y"),
                                )
                                .changed()
                            {
                                if let Some(circle) = model
                                    .particle_system
                                    .forces
                                    .wind_circles
                                    .get_mut(&selected_id)
                                {
                                    {
                                        let current_x = circle.params().center.x;
                                        circle.params_mut().center(vec2(current_x, center_y));
                                    }
                                }
                            }
                        } else {
                            ui.label("No parameters available");
                        }
                    } else {
                        ui.label("No circle selected");
                    }
                }); // end Voice 4
            });
        });

    if show_forces_changed {
        model.particle_system.forces.force_update_all();
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
        .stroke(rgba(0.5, 1.0, 0.5, 0.5)) // Green outline
        .stroke_weight(2.0)
        .no_fill();
}

// ************************ OSC   *************************************

fn process_osc(model: &mut Model, commands: Vec<OscCommand>) {
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
            OscCommand::ParticlesGravity { id, val } => {
                set_gravity(model, id, val);
            }
            OscCommand::ParticlesTrail { id, val } => {
                set_feedback(model, id, val);
            }
            OscCommand::ParticlesNumParticles { id, val } => {
                set_num_particles(model, id, val);
            }
            OscCommand::EraseDrone { id } => {
                erase_drone(model, id);
            }
            OscCommand::ParticlesAlpha { id, val } => {
                set_alpha(model, id, val);
            }
            OscCommand::ParticlesForce { id, val } => {
                set_force(model, id, val);
            }
            OscCommand::ParticlesInnerRadius { id, val } => {
                set_radius_inner(model, id, val);
            }
            OscCommand::ParticlesOuterRadius { id, val } => {
                set_radius_outer(model, id, val);
            }
            _ => {}
        }
    }
}

fn erase_drone(model: &mut Model, id: i32) {
    let voice = Voice::from_i32(id);
    model.particle_system.kill_voice(&voice);
    model.particle_system.forces.update(model.show_forces);
    model.osc_send.send_drone_on_off(id, 0);
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

fn set_alpha(model: &mut Model, id: i32, alpha: f32) {
    let voice = Voice::from_i32(id);
    model.particle_system.set_alpha_limit(&voice, alpha);
}

fn set_gravity(model: &mut Model, id: i32, gravity: f32) {
    let voice = Voice::from_i32(id);
    model.particle_system.set_gravity(&voice, gravity);
}

fn set_force(model: &mut Model, id: i32, force: f32) {
    let voice = Voice::from_i32(id);
    let strength = force * 30.0; // 30 is the max strength of the wind circle
    model.particle_system.set_strength(&voice, strength);
}

fn set_radius_inner(model: &mut Model, id: i32, val: f32) {
    let voice = Voice::from_i32(id);
    model.particle_system.set_radius_inner(&voice, val);
}

fn set_radius_outer(model: &mut Model, id: i32, val: f32) {
    let voice = Voice::from_i32(id);
    model.particle_system.set_radius_outer(&voice, val);
}

fn set_num_particles(model: &mut Model, id: i32, num_particles: f32) {
    let voice = Voice::from_i32(id);

    model
        .particle_system
        .set_num_particles(&voice, num_particles);
}

fn set_feedback(model: &mut Model, id: i32, feedback: f32) {
    let voice = Voice::from_i32(id);
    model.particle_system.set_feedback(&voice, feedback);
}

impl Drop for Model {
    fn drop(&mut self) {
        println!("\n\nApp terminating, sending kill drone signals...");
        erase_drone(self, 1);
        erase_drone(self, 4);
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

/// Set macOS window behaviors so that Spaces and Mission Control doesn't interrupt rendering.
#[cfg(target_os = "macos")]
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
