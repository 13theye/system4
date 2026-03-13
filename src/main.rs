//! System 4
//!
//! (c) 2026 13th Eye LLC for Tacit Group
//!
//!
//! src/main.rs

mod init;

use nannou::prelude::*;
use rand::rngs::ThreadRng;
use thread_priority::*;

use std::time::Instant;

use system4::{
    groups::VoiceId, managers::VoiceManager, model::Model, ui::control_panel::update_control_ui,
    utils::IdGenerator, view::center_line::CenterLine, view::rhythm::RhythmView,
};

fn model(app: &App) -> Model {
    let settings = init::config::load_settings();

    let per_voice_particle_limit = settings.particles.per_voice_limit;

    // Particle limit is the per-voice limit * the number of particle voices
    let particle_limit = per_voice_particle_limit * 2;
    let render_size = init::config::render_size(&settings);

    let (clock, sequencer_service) = init::timing::init_clock_and_sequencer(&settings);
    let (osc, osc_send, osc_loop) = init::osc::init_osc(&settings);

    let particle_system = init::particles::init_particle_system(render_size, particle_limit);

    // Create RhythmView
    let rhythm_view = RhythmView::new();
    let center_line = CenterLine::new();

    let window_ids = init::windows::create_windows(app, &settings);

    let terminal_font = init::text::load_font(app);
    let text_overlay = init::text::init_text_overlay(render_size, &terminal_font);
    let ui_state = init::ui::init_ui(app, window_ids, text_overlay);

    // Set up rng
    let rng = ThreadRng::default();

    let render_state = init::rendering::init_render_state(
        app,
        window_ids,
        &settings,
        particle_limit,
        terminal_font,
    );

    Model {
        particle_system,
        voice_manager: VoiceManager::init(&settings),
        rhythm_view,
        center_line,
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
    let ai_stream_events = model.voice_manager.update_ai(
        now,
        &mut model.sequencer_service,
        &mut model.rhythm_view,
        &mut model.rng,
    );

    // Process AI stream events and fire OSC signals if matching
    if let Some(ai_stream_events) = ai_stream_events {
        use system4::managers::AiStreamEvent;

        ai_stream_events.iter().for_each(|e| {
            if let AiStreamEvent::StreamStarted(id) = e {
                model.osc_send.send_ai_typing(id.to_i32());
            }

            if let AiStreamEvent::StreamFinished(id) = e {
                model.osc_send.send_ai_finished(id.to_i32());
            }
        });
    }

    // Read feedback slider values and update drones before updating particle system
    let (v0_feedback, v3_feedback) = read_feedback_ui_sliders(model);
    if let Some(drone0) = model.voice_manager.get_mut_drone(VoiceId::Voice0) {
        drone0.set_segment_length(v0_feedback);
    }
    if let Some(drone3) = model.voice_manager.get_mut_drone(VoiceId::Voice3) {
        drone3.set_segment_length(v3_feedback);
    }

    // Update active Rhythm logical groups & views
    model
        .voice_manager
        .update_rhythms(model.clock.tempo(), &mut model.rhythm_view, now);

    // Update formations in transition states (including cleared/clearing ones)
    model.rhythm_view.update_transitions(now);

    // This enables particles to flash with rhythm
    //let event = events.iter().any(|e| *e);

    // Update particle system
    model.particle_system.update(
        &mut model.voice_manager.voices,
        &mut model.rng,
        now,
        app.duration.since_start.as_millis() as u32,
        model.engine_debug,
    );

    // Update masks animations
    model.voice_manager.update_drone_masks(now);

    // Update centerline animation
    model.center_line.update(now);
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

fn read_feedback_ui_sliders(model: &Model) -> (f32, f32) {
    let voice0_feedback = model.get_feedback(VoiceId::Voice0);
    let voice3_feedback = model.get_feedback(VoiceId::Voice3);
    (voice0_feedback, voice3_feedback)
}
