// System 3
//
// (c) 2025 13th Eye & Tacit Group
//
//
// src/main.rs

use nannou::rand::{rngs::ThreadRng, Rng};
use nannou::{prelude::*, wgpu::TextureReshaper};
use nannou_egui::Egui;
use nnpipe::*;

use system3::{config::*, fps::FpsManager, particle::ParticleSystem};

struct Model {
    particle_system: ParticleSystem,

    // Windows' texture reshapers
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
    let default_size = 6.0;
    let default_color = rgba(0.7, 0.7, 0.7, 1.0);
    let window_size = vec2(
        config.rendering.texture_width as f32,
        config.rendering.texture_height as f32,
    );
    let particle_system = ParticleSystem::new(
        pt2(0.0, 0.0),
        window_size.x,
        window_size.y,
        default_size,
        default_color,
    );

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
        .title("System_3 Performance Monitor v0.1.0")
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

    let control_window_id = app
        .new_window()
        .title("System_3 Performer Control v0.1.0")
        .size(config.control_window.width, config.control_window.height)
        .msaa_samples(1)
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
    let audience_reshaper = rendering.create_reshaper_for_raw_scene(device, &audience_window);
    let performer_reshaper = rendering.create_reshaper_for_raw_scene(device, &performer_window);
    let audience_draw = nannou::Draw::new();
    let performer_draw = nannou::Draw::new();
    let control_draw = nannou::Draw::new();

    // Set up egui
    let egui = Egui::from_window(&control_window);

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
        control_window_id,
        audience_reshaper,
        performer_reshaper,
        draw,
        audience_draw,
        performer_draw,
        control_draw,
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

    let rect = model.particle_system.bounds_rect;

    for _ in 0..20 {
        let y = model.rng.gen_range(-1200.0..1200.0);
        if y > -50.0 && y < 50.0 {
            continue;
        }
        let position: Vec2 = if y > 0.0 {
            vec2(rect.left() - 10.0, y)
        } else {
            vec2(rect.right() + 10.0, y)
        };
        let velocity = if y > 0.0 {
            vec2(10.0, 0.0)
        } else {
            vec2(-10.0, 0.0)
        };

        model
            .particle_system
            .add_particle_with_velocity(position, velocity);
    }
    /*
    for _ in 0..18 {
        let x = model.rng.gen_range(-1900.0..1900.0);
        if x > -350.0 && x < 350.0 {
            continue;
        }
        let position: Vec2 = if x > 0.0 {
            vec2(x, rect.top() - 10.0)
        } else {
            vec2(x, rect.bottom() - 10.0)
        };
        let velocity = if x > 0.0 {
            vec2(0.0, -5.0)
        } else {
            vec2(0.0, 5.0)
        };


        model
            .particle_system
            .add_particle_with_velocity(position, velocity);

    }
    */

    // Update particles
    model.particle_system.update(&model.draw);

    // Draw particles
    model.particle_system.draw(&model.draw);

    render_and_post(app, model);
}

fn audience_view(app: &App, model: &Model, frame: Frame) {
    // Get the post-processed texture view
    let _scene_view = model.rendering.get_scene_view();

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

fn control_view(app: &App, model: &Model, frame: Frame) {}

// ******************************* Rendering and Capture *****************************
fn render_and_post(app: &App, model: &mut Model) {
    // Get the window device and queue
    let window = app.main_window();
    let device = window.device();
    let queue = window.queue();

    // Render the game to texture and post-process
    model.rendering.render_scene(device, queue, &model.draw);
    //model.rendering.post_process(device, queue);
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
        Key::C => {
            /*
            model.particle_system.forces.wind.make_circular_field(
                pt2(0.0, 0.0),
                500.0,
                900.0,
                20.0,
            );
             */
            /*
            model
                .particle_system
                .forces
                .add_gravity_source(vec2(0.0, 0.0), 10000.0);
             */
        }
        Key::Space => {
            let rect = model.particle_system.bounds_rect;
            for _ in 0..10 {
                let y = model.rng.gen_range(-1000.0..1000.0);
                let position: Vec2 = if y > 0.0 {
                    vec2(rect.left() - 10.0, y)
                } else {
                    vec2(rect.right() + 10.0, y)
                };
                let velocity = if y > 0.0 {
                    vec2(20.0, 0.0)
                } else {
                    vec2(-20.0, 0.0)
                };

                model
                    .particle_system
                    .add_particle_with_velocity(position, velocity);
            }
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
