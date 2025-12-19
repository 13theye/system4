use nannou::prelude::*;

use system4::settings::Settings;

pub fn load_settings() -> Settings {
    Settings::load().expect("\nSystem 4: FAILED TO LOAD CONFIG.TOML\n")
}

pub fn render_size(settings: &Settings) -> Vec2 {
    vec2(
        settings.rendering.texture_width as f32,
        settings.rendering.texture_height as f32,
    )
}
