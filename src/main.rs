// System 4
//
// (c) 2025 13th Eye LLC & Tacit Group
//
//
// src/main.rs

use fps::FpsManager;
use nannou::{prelude::*, text::Font};
use nannou_egui::Egui;
use prat::clockservice::ClockService;
use rand::rngs::ThreadRng;
use thread_priority::*;

use std::fs;
use std::time::Instant;

use system4::{
    groups::VoiceId,
    managers::{RhythmManager, VoiceManager},
    model::Model,
    osc::{OscController, OscSender},
    particle::ParticleSystem,
    rendering::RenderState,
    sequencer::SequencerService,
    settings::*,
    text::{
        layout::{
            HorizontalJustify, TextBoxAnchor, TextBoxLayout, TextBoxLayoutParams, VerticalFlow,
        },
        overlay::TextOverlay,
        params_dashboard,
        view::{TextPaneView, TextPaneViewParams, TextTheme},
        TextPane, TextPaneId, TextSlot,
    },
    ui::{control_panel::update_control_ui, UiState},
    utils::IdGenerator,
    view::rhythm::{RhythmView, RhythmViewUpdateParams},
};

const DEFAULT_PARTICLE_SIZE: f32 = 4.0;
const DEFAULT_PARTICLE_RGB: (f32, f32, f32) = (0.73, 0.73, 0.74);
//const DEFAULT_PARTICLE_RGB: (f32, f32, f32) = (0.27, 0.27, 0.26);

fn model(app: &App) -> Model {
    // Load config
    let settings = Settings::load().expect("\nSystem 4: FAILED TO LOAD CONFIG.TOML\n");

    // Main game data elements
    let particle_limit = settings.particles.limit;

    let render_size = vec2(
        settings.rendering.texture_width as f32,
        settings.rendering.texture_height as f32,
    );

    // Init clock
    let mut clock = ClockService::with()
        .tempo(settings.speed.bpm as f64)
        .quantum(4.0)
        .ppqn(24)
        .enable_ticks()
        .thread_priority(47)
        .build();
    // Start the clock thread or quit game if it fails
    clock
        .start_thread()
        .expect("\nSystem4: fatal error: Failed to start clock thread");

    clock
        .start_clock()
        .expect("System4: fatal error: Failed to start clock");

    let sequencer_service = SequencerService::with_clock_and_osc_config(&clock, &settings.osc_send)
        .build()
        .expect("System4: fatal error: Failed to build sequencer service");

    let osc = OscController::new(settings.osc_receive.receive_port).unwrap();
    let osc_send = OscSender::new(&settings.osc_send).unwrap();

    let osc_loop_config = OscSendConfig {
        target_addr: settings.osc_loop.target_addr,
        target_port: settings.osc_loop.target_port,
    };
    let osc_loop = OscSender::new(&osc_loop_config).unwrap();

    // DPI scale is used to scale the size of draw objects to account for DPI scaling.
    let dpi_scale = settings.rendering.dpi_scale;

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
    );

    particle_system.set_mass_variation_enabled(true);
    particle_system.set_mass_variation_amount(0.5);

    // Create RhythmView
    let rhythm_view = RhythmView::new();

    // Create window
    let audience_window_id = app
        .new_window()
        .title("Tacit Group: System_4 0.1.0")
        .size(
            settings.audience_window.width,
            settings.audience_window.height,
        )
        .msaa_samples(1)
        .view(audience_view)
        .build()
        .unwrap();

    let performer_window_id = app
        .new_window()
        .title("System_4 Performance Monitor v0.1.0")
        .size(
            settings.performer_window.width,
            settings.performer_window.height,
        )
        .msaa_samples(1)
        .view(performer_view)
        .build()
        .unwrap();

    let control_window_id = app
        .new_window()
        .title("System_4 Performer Control v0.1.0")
        .size(
            settings.control_window.width,
            settings.control_window.height,
        )
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

    // Rendering initialization moved to RenderState::from_app

    // Set up egui
    let egui = Egui::from_window(&control_window);

    // Set up rng
    let rng = ThreadRng::default();

    // --- Load Font for Nannou Draw  ---
    // Assumes "assets/terminal_font.ttf" exists relative to the executable
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

    // Create new unified text overlay for per-voice panes.
    let mut text_overlay = TextOverlay::new();

    // Ensure panes can display all pinned parameter lines + one command input line.
    let params_line_count = params_dashboard::default_tracked_keys().len();
    let min_pane_lines = params_line_count + 1;

    // Helper to create a text pane + view.
    let mut add_voice_pane = |voice: VoiceId,
                              anchor: TextBoxAnchor,
                              anchor_pos: Vec2,
                              width: f32,
                              num_lines: usize,
                              font_size: u32,
                              justify: HorizontalJustify,
                              theme: TextTheme| {
        let num_lines = num_lines.max(min_pane_lines);

        let line_spacing = 5.0;
        let line_height = font_size as f32 + line_spacing * 2.0;

        let layout = TextBoxLayout::new(TextBoxLayoutParams {
            anchor,
            anchor_pos,
            width,
            num_lines,
            line_height,
            vertical_flow: VerticalFlow::TopDown,
            horizontal_justify: justify,
        });

        let view = TextPaneView::new(TextPaneViewParams {
            layout,
            font: font.clone(),
            font_size,
            line_spacing,
            fade_delay_secs: 1.0,
            color_fade_secs: 1.5,
            chars_per_second: 0.0, // default: show immediately for overlay panes
            theme,
        });

        let mut pane = TextPane::new(num_lines);
        pane.set_slot_line_budget(TextSlot::Params, params_line_count);
        pane.set_slot_line_budget(TextSlot::CommandInput, 1);
        pane.set_slot_line_budget(TextSlot::AiStream, 2);

        text_overlay.insert_pane(TextPaneId::Voice(voice), pane, view);
    };

    // Arrange voice panes as four columns across the full render width.
    let gutter_x = 40.0;
    let gutter_y = 40.0;
    let columns = 4.0;

    let col_w = render_size.x / columns;
    let pane_w = (col_w - gutter_x).max(200.0);

    let left_edge = -render_size.x / 2.0;
    let top_y = render_size.y / 2.0 - gutter_y;

    for (i, voice) in [
        VoiceId::Voice0,
        VoiceId::Voice1,
        VoiceId::Voice2,
        VoiceId::Voice3,
    ]
    .iter()
    .copied()
    .enumerate()
    {
        let x0 = left_edge + i as f32 * col_w + gutter_x / 2.0;
        add_voice_pane(
            voice,
            TextBoxAnchor::TopLeft,
            vec2(x0, top_y),
            pane_w,
            8,
            28,
            HorizontalJustify::Left,
            TextTheme::default(),
        );
    }

    // Initialize rendering state with all GPU resources and pipelines
    let render_state = RenderState::from_app(
        app,
        audience_window_id,
        performer_window_id,
        control_window_id,
        settings.rendering.texture_width,
        settings.rendering.texture_height,
        settings.rendering.texture_samples,
        particle_limit,
        dpi_scale,
        font,
    );

    // Create UI state
    let ui_state = UiState::new(egui, fps, text_overlay);

    Model {
        particle_system,
        voice_manager: VoiceManager::new(),
        rhythm_manager: RhythmManager::new(&settings.openai_service),
        rhythm_view,
        clock,
        sequencer_service,
        osc,
        osc_send,
        osc_loop,
        render_state,
        ui_state,
        id_generator: IdGenerator::new(),
        rng,
        command_queue: Vec::new(),
        auto_ai_pending_for_voice1: false,
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
        .loop_mode(nannou::LoopMode::rate_fps(60.0)) // Run at __fps regardless of display refresh rate
        .update(update)
        .run();
}

// TODO: refactor to use app.duration.since_prev_update or update.since_last
fn update_feedback(model: &mut Model) {
    // Read feedback value for segment length before updating particle system
    let voice1_feedback = model.get_feedback(VoiceId::Voice0);
    let voice4_feedback = model.get_feedback(VoiceId::Voice3);

    // Update segment length based on Voice1 feedback slider
    if let Some(voice1) = model.voice_manager.get_voice_mut(VoiceId::Voice0) {
        voice1.set_segment_length(voice1_feedback);
    }

    // Update segment length based on Voice4 feedback slider
    if let Some(voice4) = model.voice_manager.get_voice_mut(VoiceId::Voice3) {
        voice4.set_segment_length(voice4_feedback);
    }
}

fn update(app: &App, model: &mut Model, _update: Update) {
    let now = Instant::now();

    // Update FPS counter
    model.ui_state.fps.update();

    // Update control UI
    update_control_ui(app, model);

    // Process OSC commands
    let commands = model.osc.process_messages();
    for cmd in commands {
        model.queue_command(cmd);
    }

    // Process unified command queue with priority resolution
    model.process_command_queue(now);

    // Poll AI rhythm responses (if any) and apply them to rhythms
    model.rhythm_manager.update_ai(
        now,
        &mut model.sequencer_service,
        &mut model.rhythm_view,
        &mut model.rng,
    );

    // Update feedback-derived voice params (segment length)
    update_feedback(model);

    let mut events = Vec::new();

    // Update Rhythm logical groups & views
    for (voice_id, rhythm) in model.rhythm_manager.rhythms_mut().iter_mut() {
        let (current_slot, current_wing) = rhythm.update();

        let params = rhythm.get_params();
        if let Some(current_wing) = current_wing {
            if params.wings.contains(&current_wing) {
                events.push(true);
            }
        }

        let update_params = RhythmViewUpdateParams {
            current_slot,
            current_wing,
            tempo: model.clock.tempo(),
            subdivision: rhythm.get_subdivision().to_owned(),
        };

        model
            .rhythm_view
            .update_voice(voice_id, rhythm.get_params(), &update_params, now);
    }

    // Update formations in transition states (including cleared/clearing ones)
    model.rhythm_view.update_all_transitions(now);

    // This enables particles to flash with rhythm
    //let event = events.iter().any(|e| *e);

    // Update particle system with ZERO-COPY optimization
    // Get GPU queue for direct staging memory writes
    let window = app.main_window();
    let queue = window.queue();

    let (particles_written, segments_written) = model.particle_system.update_zero_copy(
        model.voice_manager.voices_mut(),
        &mut model.rng,
        queue,
        &model.render_state.particle_renderer,
        &model.render_state.segment_renderer,
        now,
    );

    // Store counts for rendering
    model.render_state.particle_count = particles_written;
    model.render_state.segment_instance_count = segments_written;
}

fn audience_view(app: &App, model: &Model, frame: Frame) {
    // Begin Rendering context
    {
        let mut rendering = model.render_state.render_engine.borrow_mut();

        // Get GPU resources
        let window = app.main_window();
        let device = window.device();
        let mut encoder = rendering.create_command_encoder(device);
        let queue = window.queue();

        // Clear all textures
        //rendering.draw.background().color(BLACK);
        rendering.encode_clear_all_textures(&mut encoder, wgpu::Color::TRANSPARENT);

        // Encode Nannou Draw
        rendering.encode_draw_commands(device, &mut encoder);

        // ZERO-COPY: Encode particles and segments without re-uploading
        // Data was already written directly to GPU staging in update_zero_copy
        model.render_state.particle_renderer.encode_only(
            &mut encoder,
            model.render_state.particle_count,
            rendering.get_named_texture("particles").unwrap(),
        );

        model.render_state.segment_renderer.encode_only(
            &mut encoder,
            model.render_state.segment_instance_count,
            rendering.get_named_texture("particles").unwrap(),
        );

        // Encode heatmap (still uses legacy buffer for now)
        // will not work in the current ZERO-COPY implementation because buffer will
        // remain empty.
        model.render_state.heatmap_renderer.encode_into(
            device,
            &mut encoder,
            queue,
            &model.render_state.gpu_particle_buffer,
            model.render_state.render_rect,
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

        // Draw all rhythm views
        model.rhythm_view.draw_all(&rendering.draw);

        // Unified text overlay (new system)
        let now = Instant::now();
        {
            let mut overlay = model.ui_state.text_overlay.borrow_mut();

            // Always route AI status text into Voice2's history (not live text).
            overlay.push_ai_status_history_if_changed(
                model.rhythm_manager.current_ai_status_text(),
                now,
            );

            overlay.update_and_draw_all(&rendering.draw, now);
        }

        // Encode Nannou Draw
        rendering.encode_draw_commands_into(device, &mut encoder, "terminal");

        if let Err(e) = rendering.execute_named_pipeline("final composite", device, &mut encoder) {
            eprintln!("Error executing final composite pipeline: {}", e);
        }

        rendering.submit_command_encoder(device, queue, encoder);

        // Update reshaper if needed (could be cached in Model)
        rendering.draw_to_frame(&model.render_state.audience_reshaper, &frame);
    }
    // End Rendering context

    // Show screen bounds if enabled
    if model.ui_state.show_bounds {
        draw_bounds(app, model);
    }

    // Draw over the texture
    let _ = model.render_state.audience_draw.to_frame(app, &frame);
}

fn performer_view(app: &App, model: &Model, frame: Frame) {
    let rendering = model.render_state.render_engine.borrow_mut();

    // Get the raw scene texture view
    let _scene_view = rendering.get_scene_view();

    // Draw game content to the frame
    rendering.draw_to_frame(&model.render_state.performer_reshaper, &frame);

    // Show force vectors if enabled
    if model.ui_state.show_forces {
        let performer_rect = app
            .window(model.render_state.performer_window_id)
            .unwrap()
            .rect();

        // Create a scaled draw context that matches texture coordinates
        let texture_size = rendering.scene_texture.size();

        // Calculate scale factor from texture to window
        let scale_x = performer_rect.w() / texture_size[0] as f32;
        let scale_y = performer_rect.h() / texture_size[1] as f32;

        // Apply transform to match texture coordinates
        model.particle_system.draw_forces(
            model.voice_manager.voices(),
            &model.render_state.performer_draw,
            scale_x,
            scale_y,
        );
    }

    // Then draw over the texture
    let _ = model.render_state.performer_draw.to_frame(app, &frame);
}

fn control_view(app: &App, model: &Model, frame: Frame) {
    // Draw background first
    model.render_state.control_draw.background().color(BLACK);
    let _ = model.render_state.control_draw.to_frame(app, &frame);
    // Then draw egui UI on top
    model.ui_state.egui.draw_to_frame(&frame).unwrap();
}

// ******************************* Input Capture *****************************

fn raw_window_event(_app: &App, model: &mut Model, event: &nannou::winit::event::WindowEvent) {
    model.ui_state.egui.handle_raw_event(event);

    // Handle keyboard input for command terminal
    if let nannou::winit::event::WindowEvent::KeyboardInput { input, .. } = event {
        if input.state == nannou::winit::event::ElementState::Pressed {
            if let Some(key) = input.virtual_keycode {
                use nannou::winit::event::VirtualKeyCode;

                match key {
                    VirtualKeyCode::Escape => {
                        for voice in [
                            VoiceId::Voice0,
                            VoiceId::Voice1,
                            VoiceId::Voice2,
                            VoiceId::Voice3,
                        ] {
                            if let Some(input) = model.ui_state.command_inputs.get_mut(&voice) {
                                input.clear();
                            }
                            model
                                .ui_state
                                .text_overlay
                                .borrow_mut()
                                .clear_live_slot(TextPaneId::Voice(voice), TextSlot::CommandInput);
                        }
                    }
                    VirtualKeyCode::P => {
                        // Toggle debug and FPS display
                        model.ui_state.show_bounds = !model.ui_state.show_bounds;
                    }
                    _ => {}
                }
            }
        }
    }

    // Note: Text input is now handled directly by egui TextEdit widget
    // through the TextBuffer trait implementation
}

// ************************ Debug display  *************************************

fn draw_bounds(app: &App, model: &Model) {
    let draw = &model.render_state.audience_draw;
    let rect = app
        .window(model.render_state.audience_window_id)
        .unwrap()
        .rect();

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
