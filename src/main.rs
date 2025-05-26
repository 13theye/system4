// System 3
//
// (c) 2025 13th Eye & Tacit Group
//
//
// src/main.rs

use nannou::rand::rngs::ThreadRng;
use nannou::{prelude::*, wgpu::TextureReshaper};
use nannou_egui::Egui;
use nnpipe::*;

use system3::{config::*, fps::FpsManager, particle::ParticleSystem};

struct Model {
    particle_system: ParticleSystem,

    // Windows' texture reshapers
    audience_window_id: WindowId,
    performer_window_id: WindowId,
    audience_reshaper: TextureReshaper,
    performer_reshaper: TextureReshaper,

    // Nannou API
    draw: nannou::Draw,           // for drawing to the main texture
    audience_draw: nannou::Draw,  // for drawing UI elements to audience_window only
    performer_draw: nannou::Draw, // for drawing UI elements to performer_window only

    // Rendering engine
    rendering: Nnpipe,

    // Egui
    egui: Egui,

    // Random
    rng: ThreadRng,

    // FPS display
    fps: FpsManager,

    debug: bool,
}

fn model(app: &App) -> Model {
    // Load config
    let config = Config::load().expect("\nSystem 3: FAILED TO LOAD CONFIG.TOML\n");

    // Main game data elements
    let default_size = 10.0;
    let default_color = rgba(0.9, 0.06, 0.06, 1.0);
    let particle_system = ParticleSystem::new(pt2(0.0, 0.0), default_size, default_color);

    // Create window
    let audience_window_id = app
        .new_window()
        .title("Tacit Group: System_3 0.1.0")
        .size(config.audience_window.width, config.audience_window.height)
        .msaa_samples(1)
        .view(audience_view)
        .build()
        .unwrap();

    let performer_window_id = app
        .new_window()
        .title("System_3 Performer Control v0.1.0")
        .size(
            config.performer_window.width,
            config.performer_window.height,
        )
        .msaa_samples(1)
        .view(performer_view)
        .key_pressed(key_pressed)
        .raw_event(raw_window_event)
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

    // Set up render texture
    // the device isn't tied to window, but it's nannou's way of getting the handle.
    let device = audience_window.device();
    let draw = nannou::Draw::new();

    let rendering = Nnpipe::new(
        device,
        config.rendering.texture_width,
        config.rendering.texture_height,
        config.rendering.texture_samples,
    );

    // Create reshapers for both windows
    let audience_reshaper = rendering.create_reshaper_for_post_processed(device, &audience_window);
    let performer_reshaper =
        rendering.create_reshaper_for_post_processed(device, &performer_window);
    let audience_draw = nannou::Draw::new();
    let performer_draw = nannou::Draw::new();

    // Set up egui
    let egui = Egui::from_window(&performer_window);

    // Set up rng
    let rng = ThreadRng::default();

    // Create FPS manager
    let mut fps = FpsManager::default();
    let performer_rect = app.window(performer_window_id).unwrap().rect();
    fps.set_draw_position(pt2(
        performer_rect.left() + 40.0,
        performer_rect.top() - 10.0,
    ));

    Model {
        particle_system,
        audience_window_id,
        performer_window_id,
        audience_reshaper,
        performer_reshaper,
        draw,
        audience_draw,
        performer_draw,
        rendering,
        egui,
        rng,
        fps,
        debug: false,
    }
}

fn main() {
    nannou::app(model).update(update).run();
}

fn update(app: &App, model: &mut Model, _update: Update) {
    model.draw.background().color(BLACK);

    // Update FPS counter
    model.fps.update(&model.performer_draw);

    // Add particles
    for _ in 0..10 {
        model
            .particle_system
            .add_particle_with_motion(&mut model.rng);
    }
    model.particle_system.update();
    model.particle_system.draw(&model.draw);

    render_and_post(app, model);
}

fn audience_view(app: &App, model: &Model, frame: Frame) {
    // Get the post-processed texture view
    let _post_processed_view = model.rendering.get_post_processed_view();

    // Update reshaper if needed (could be cached in Model)
    model
        .rendering
        .draw_to_frame(&model.audience_reshaper, &frame);

    // Handle FPS and origin display
    if model.debug {
        draw_debug(app, model);

        // Then draw audience UI over it
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

    // Draw egui UI
    model.egui.draw_to_frame(&frame).unwrap();

    // Handle FPS and origin display
    if model.debug {
        draw_debug(app, model);
        model.fps.draw(&model.performer_draw);
    }
    // Then draw performer UI over it
    let _ = model.performer_draw.to_frame(app, &frame);
}

// ******************************* Rendering and Capture *****************************
fn render_and_post(app: &App, model: &mut Model) {
    // Get the window device and queue
    let window = app.main_window();
    let device = window.device();
    let queue = window.queue();

    // Render the game to texture and post-process
    model.rendering.render_scene(device, queue, &model.draw);
    model.rendering.post_process(device, queue);
}

// ******************************* Input Capture *****************************

fn key_pressed(_app: &App, model: &mut Model, key: Key) {
    // For now, all human boards are controlled by the same keyboard
    match key {
        Key::P => {
            // Toggle debug and FPS display
            model.debug = !model.debug;
            model.fps.toggle();
        }
        Key::S => {
            todo!();
        }
        _ => {}
    }
}

fn raw_window_event(_app: &App, model: &mut Model, event: &nannou::winit::event::WindowEvent) {
    model.egui.handle_raw_event(event);
}

// ************************ Debug display  *************************************

fn draw_debug(app: &App, model: &Model) {
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
