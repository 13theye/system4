//! System 4
//!
//! (c) 2025 13th Eye LLC & Tacit Group
//!
//!
//! src/main.rs

mod init;

use nannou::prelude::*;
use rand::rngs::ThreadRng;
use thread_priority::*;

use std::time::Instant;

use system4::{
    groups::VoiceId,
    managers::VoiceManager,
    model::Model,
    text::{TextPaneId, TextSlot},
    ui::control_panel::update_control_ui,
    utils::IdGenerator,
    view::rhythm::RhythmView,
};

fn model(app: &App) -> Model {
    let settings = init::config::load_settings();

    let per_voice_particle_limit = settings.particles.per_voice_limit;

    // Particle limit is the per-voice limit * the number of particle voices
    let particle_limit = per_voice_particle_limit * 2;
    let render_size = init::config::render_size(&settings);

    // DPI scale is used to scale the size of draw objects to account for DPI scaling.
    let dpi_scale = settings.rendering.dpi_scale;

    let (clock, sequencer_service) = init::timing::init_clock_and_sequencer(&settings);
    let (osc, osc_send, osc_loop) = init::osc::init_osc(&settings);

    let particle_system = init::particles::init_particle_system(render_size, particle_limit);

    // Create RhythmView
    let rhythm_view = RhythmView::new();

    let window_ids = init::windows::create_windows(app, &settings);

    let font = init::text::load_font(app);
    let text_overlay = init::text::init_text_overlay(render_size, &font);
    let ui_state = init::ui::init_ui(app, window_ids, text_overlay);

    // Set up rng
    let rng = ThreadRng::default();

    let render_state = init::rendering::init_render_state(
        app,
        window_ids,
        &settings,
        particle_limit,
        dpi_scale,
        font,
    );

    Model {
        particle_system,
        voice_manager: VoiceManager::init(&settings),
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
        engine_debug: false,
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

fn update_feedback(model: &mut Model) {
    // Read feedback value for segment length before updating particle system
    let voice0_feedback = model.get_feedback(VoiceId::Voice0);
    let voice3_feedback = model.get_feedback(VoiceId::Voice3);

    // Update segment length based on Voice0 feedback slider
    if let Some(drone1) = model.voice_manager.get_drone_mut(VoiceId::Voice0) {
        drone1.set_segment_length(voice0_feedback);
    }

    // Update segment length based on Voice3 feedback slider
    if let Some(drone3) = model.voice_manager.get_drone_mut(VoiceId::Voice3) {
        drone3.set_segment_length(voice3_feedback);
    }
}

/// Main update loop
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
    model.voice_manager.update_ai(
        now,
        &mut model.sequencer_service,
        &mut model.rhythm_view,
        &mut model.rng,
    );

    // Update feedback-derived voice params (segment length)
    update_feedback(model);

    // Update Rhythm logical groups & views
    model
        .voice_manager
        .update_rhythms(model.clock.tempo(), &mut model.rhythm_view, now);

    // Update formations in transition states (including cleared/clearing ones)
    model.rhythm_view.update_all_transitions(now);

    // This enables particles to flash with rhythm
    //let event = events.iter().any(|e| *e);

    // Update particle system with ZERO-COPY optimization

    model.particle_system.update(
        &mut model.voice_manager.voices,
        &mut model.rng,
        now,
        app.duration.since_start.as_millis() as u32,
        model.engine_debug,
    );
}

/// Draw the audience view window's contents
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

        // ZERO-COPY: Write and encode particles and segments per voice
        let voice0_texture = rendering
            .get_named_texture("particles_voice_0")
            .expect("Fatal Error: Missing texture for Voice0");

        let voice3_texture = rendering
            .get_named_texture("particles_voice_3")
            .expect("Fatal Error: Missing texture for Voice3");

        for (voice_id, voice) in model.voice_manager.voices.iter() {
            if voice.as_drone().is_none() {
                continue;
            }

            let texture = match voice_id {
                VoiceId::Voice0 => voice0_texture,
                VoiceId::Voice3 => voice3_texture,
                _ => continue,
            };

            let (particle_count, segment_instance_count) = model.particle_system.gpu_write(
                voice_id,
                voice,
                queue,
                &model.render_state.particle_renderer,
                &model.render_state.segment_renderer,
                model.engine_debug,
            );

            if particle_count == 0 {
                continue;
            }

            model
                .render_state
                .particle_renderer
                .encode_only(&mut encoder, particle_count, texture);

            model.render_state.segment_renderer.encode_only(
                &mut encoder,
                segment_instance_count,
                texture,
            );
        }

        // Combine voice textures
        if let Err(e) = rendering.execute_named_pipeline("combine_voices", device, &mut encoder) {
            eprintln!("Error executing combine_voices pipeline: {}", e);
        }

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
                model.voice_manager.current_ai_status_text(),
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

/// Draw the performer window's contents
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
            &model.voice_manager.voices,
            &model.render_state.performer_draw,
            scale_x,
            scale_y,
        );
    }

    // Then draw over the texture
    let _ = model.render_state.performer_draw.to_frame(app, &frame);
}

/// Draw the control window (UI)
fn control_view(app: &App, model: &Model, frame: Frame) {
    // Draw background first
    model.render_state.control_draw.background().color(BLACK);
    let _ = model.render_state.control_draw.to_frame(app, &frame);
    // Then draw egui UI on top
    model.ui_state.egui.draw_to_frame(&frame).unwrap();
}

// ******************************* Input Capture *****************************

/// Handle Raw Window Events, for Control window
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
                        /*
                        // Toggle debug and FPS display
                        model.ui_state.show_bounds = !model.ui_state.show_bounds;
                         */
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
