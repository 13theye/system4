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

use system3::{config::*, forces::WindCircle, fps::FpsManager, particle::ParticleSystem};

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

    // UI state
    selected_circle_id: Option<usize>,

    // Debug stuff
    show_bounds: bool,
    show_forces: bool,
}

fn model(app: &App) -> Model {
    // Load config
    let config = Config::load().expect("\nSystem 3: FAILED TO LOAD CONFIG.TOML\n");

    // Main game data elements
    let default_size = 6.0;
    let default_color = rgba(0.73, 0.73, 0.73, 1.0);
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
        .build()
        .unwrap();

    let control_window_id = app
        .new_window()
        .title("System_3 Performer Control v0.1.0")
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
    let control_draw = nannou::Draw::new();

    // Set up egui
    let egui = Egui::from_window(&control_window);

    // Set up rng
    let rng = ThreadRng::default();

    // Create FPS manager
    let mut fps = FpsManager::new_with(true, false);
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
        selected_circle_id: None,
        show_bounds: false,
        show_forces: false,
    }
}

fn main() {
    nannou::app(model).update(update).run();
}

fn update(app: &App, model: &mut Model, _update: Update) {
    model.draw.background().color(BLACK);

    // Update FPS counter
    model.fps.update();

    // Update control UI
    update_control_ui(app, model);

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
    model.particle_system.update(model.show_forces);

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
        let texture_size = vec2(model.rendering.width as f32, model.rendering.height as f32);

        // Calculate scale factor from texture to window
        let scale_x = performer_rect.w() / texture_size.x;
        let scale_y = performer_rect.h() / texture_size.y;

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
            model.show_bounds = !model.show_bounds;
        }
        Key::C => {
            let circle = WindCircle::new(1, pt2(0.0, 0.0), 500.0, 900.0, 20.0, 0.8);

            model.particle_system.forces.add_wind_circle(circle);

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
                    ui.set_min_size(egui::vec2(200.0, height));
                    // Status info
                    ui.label(format!(
                        "Particles: {}",
                        model.particle_system.particles.len()
                    ));
                    // FPS
                    ui.label(format!("FPS: {:.1}", model.fps.fps()));
                    ui.add_space(15.0);

                    show_forces_changed =
                        ui.checkbox(&mut model.show_forces, "Show Forces").changed();
                    ui.add_space(30.0);

                    // Instructions section
                    ui.label("Space: add particles");
                    ui.label("P: Toggle debug mode");
                });

                // Wind Circle Settings - use horizontal layout for two vertical sections

                // Vertical 2: Heading and dropdown
                ui.vertical(|ui| {
                    ui.set_min_width(150.0);
                    ui.heading("Wind Circle Settings");
                    ui.add_space(5.0);

                    let settings = model.particle_system.forces.get_circle_params();
                    if settings.is_empty() {
                        ui.label("No wind circles found");
                        ui.label("C: Create wind circle");
                    } else {
                        // Handle circle selection - set default if none selected
                        if model.selected_circle_id.is_none()
                            || !settings.contains_key(&model.selected_circle_id.unwrap())
                        {
                            model.selected_circle_id = settings.keys().next().copied();
                        }

                        if let Some(selected_id) = model.selected_circle_id {
                            // Dropdown to select circle
                            egui::ComboBox::from_label("Select Circle")
                                .selected_text(format!("Circle {}", selected_id))
                                .show_ui(ui, |ui| {
                                    for (id, _) in settings.iter() {
                                        ui.selectable_value(
                                            &mut model.selected_circle_id,
                                            Some(*id),
                                            format!("Circle {}", id),
                                        );
                                    }
                                });
                        }
                    }

                    ui.set_min_width(300.0);
                    ui.add_space(10.0);

                    if let Some(selected_id) = model.selected_circle_id {
                        // Get current parameter values by cloning them
                        let current_params = model
                            .particle_system
                            .forces
                            .get_circle_params()
                            .get(&selected_id)
                            .cloned();

                        if let Some(params) = current_params {
                            // Radius slider
                            let mut radius = params.radius;
                            if ui
                                .add(egui::Slider::new(&mut radius, 0.0..=1100.0).text("Radius"))
                                .changed()
                            {
                                if let Some(circle) = model
                                    .particle_system
                                    .forces
                                    .wind_circles
                                    .get_mut(&selected_id)
                                {
                                    circle.with_params_write(|p| p.radius(radius));
                                }
                            }

                            // Width slider
                            let mut width = params.width;
                            if ui
                                .add(egui::Slider::new(&mut width, 0.0..=2000.0).text("Width"))
                                .changed()
                            {
                                if let Some(circle) = model
                                    .particle_system
                                    .forces
                                    .wind_circles
                                    .get_mut(&selected_id)
                                {
                                    circle.with_params_write(|p| p.width(width));
                                }
                            }

                            // Strength slider
                            let mut strength = params.strength;
                            if ui
                                .add(egui::Slider::new(&mut strength, 0.0..=50.0).text("Strength"))
                                .changed()
                            {
                                if let Some(circle) = model
                                    .particle_system
                                    .forces
                                    .wind_circles
                                    .get_mut(&selected_id)
                                {
                                    circle.with_params_write(|p| p.strength(strength));
                                }
                            }

                            // Center bias slider
                            let mut center_bias = params.center_bias;
                            if ui
                                .add(
                                    egui::Slider::new(&mut center_bias, 0.0..=1.0)
                                        .text("Center Bias"),
                                )
                                .changed()
                            {
                                if let Some(circle) = model
                                    .particle_system
                                    .forces
                                    .wind_circles
                                    .get_mut(&selected_id)
                                {
                                    circle.with_params_write(|p| p.center_bias(center_bias));
                                }
                            }

                            ui.add_space(5.0);
                            ui.label("Center Position:");

                            // Center X slider
                            let mut center_x = params.center.x;
                            if ui
                                .add(
                                    egui::Slider::new(&mut center_x, -2000.0..=2000.0)
                                        .text("Center X"),
                                )
                                .changed()
                            {
                                if let Some(circle) = model
                                    .particle_system
                                    .forces
                                    .wind_circles
                                    .get_mut(&selected_id)
                                {
                                    circle.with_params_write(|p| {
                                        p.center(vec2(center_x, p.center.y))
                                    });
                                }
                            }

                            // Center Y slider
                            let mut center_y = params.center.y;
                            if ui
                                .add(
                                    egui::Slider::new(&mut center_y, -1100.0..=1100.0)
                                        .text("Center Y"),
                                )
                                .changed()
                            {
                                if let Some(circle) = model
                                    .particle_system
                                    .forces
                                    .wind_circles
                                    .get_mut(&selected_id)
                                {
                                    circle.with_params_write(|p| {
                                        p.center(vec2(p.center.x, center_y))
                                    });
                                }
                            }
                        } else {
                            ui.label("No parameters available");
                        }
                    } else {
                        ui.label("No circle selected");
                    }
                });
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
