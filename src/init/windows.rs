use nannou::prelude::*;

use system4::settings::Settings;

#[derive(Clone, Copy, Debug)]
pub struct WindowIds {
    pub audience: WindowId,
    pub performer: WindowId,
    pub control: WindowId,
}

pub fn create_windows(app: &App, settings: &Settings) -> WindowIds {
    // Create windows
    let audience = app
        .new_window()
        .title("Tacit Group: System_4 v0.2.0")
        .size(
            settings.audience_window.width,
            settings.audience_window.height,
        )
        .msaa_samples(1)
        .view(system4::view::windows::audience::audience_view)
        .build()
        .unwrap();

    let performer = app
        .new_window()
        .title("System_4 Performance Monitor")
        .size(
            settings.performer_window.width,
            settings.performer_window.height,
        )
        .msaa_samples(1)
        .key_pressed(system4::view::windows::key_capture::performer_key_pressed)
        .view(system4::view::windows::performer::performer_view)
        .build()
        .unwrap();

    let control = app
        .new_window()
        .title("System_4 Performer Control")
        .size(
            settings.control_window.width,
            settings.control_window.height,
        )
        .msaa_samples(1)
        .raw_event(system4::view::windows::key_capture::raw_window_event)
        .view(system4::view::windows::control::control_view)
        .build()
        .unwrap();

    let Some(audience_window) = app.window(audience) else {
        eprintln!("Audience window not found. Exiting app.");
        std::process::exit(1);
    };
    let Some(performer_window) = app.window(performer) else {
        eprintln!("Performer window not found. Exiting app.");
        std::process::exit(1);
    };

    let Some(control_window) = app.window(control) else {
        eprintln!("Control window not found. Exiting app.");
        std::process::exit(1);
    };

    println!(
        "Audience window scale: {:?}",
        audience_window.scale_factor()
    );
    println!(
        "Performer window scale: {:?}",
        performer_window.scale_factor()
    );
    println!("Control window scale: {:?}", control_window.scale_factor());

    WindowIds {
        audience,
        performer,
        control,
    }
}
