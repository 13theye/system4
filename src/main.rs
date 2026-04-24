//! System 4
//!
//! (c) 2026 13th Eye LLC for Tacit Group
//!
//!
//! src/main.rs

mod init;

use nannou::prelude::*;
use rand::rngs::ThreadRng;

use std::time::Instant;

use system4::{
    groups::VoiceId, intro::IntroImage, managers::VoiceManager, model::Model,
    ui::control_panel::update_control_ui, utils::IdGenerator, view::center_line::CenterLine,
    view::rhythm::RhythmView,
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
    let rhythm_view = RhythmView::new_from_settings(&settings);

    // Create CenterLine
    let center_line = CenterLine::default();

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

    let mut intro_image = IntroImage::new(&settings.path.intro_image);
    intro_image.load(app);

    Model {
        particle_system,
        voice_manager: VoiceManager::init(&settings),
        rhythm_view,
        center_line,
        clock,
        sequencer_service,
        original_tempo: settings.tempo.bpm,
        osc,
        osc_send,
        osc_loop,
        render_state,
        ui_state,
        id_generator: IdGenerator::new(),
        rng,
        command_queue: Vec::new(),
        auto_ai_pending_for_voice1: false,
        intro_image,
        engine_debug: false,

        #[cfg(target_os = "macos")]
        app_nap_token: acquire_app_nap_token(),
    }
}

#[cfg(target_os = "macos")]
fn acquire_app_nap_token() -> objc2::rc::Retained<
    objc2::runtime::ProtocolObject<dyn objc2_foundation::NSObjectProtocol>
> {
    use objc2_foundation::{ns_string, NSActivityOptions, NSProcessInfo};
    let options = NSActivityOptions::Background | NSActivityOptions::LatencyCritical;
    NSProcessInfo::processInfo()
        .beginActivityWithOptions_reason(options, ns_string!("Real-time OSC sequencer"))
}

fn main() {
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
        model.center_line.split_progress(),
    );

    // Update masks animations
    model.voice_manager.update_drone_masks(now);

    // Update centerline animation
    model.center_line.update(now);
}

fn read_feedback_ui_sliders(model: &Model) -> (f32, f32) {
    let voice0_feedback = model.get_feedback(VoiceId::Voice0);
    let voice3_feedback = model.get_feedback(VoiceId::Voice3);
    (voice0_feedback, voice3_feedback)
}
