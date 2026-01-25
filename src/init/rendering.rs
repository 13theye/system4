use nannou::{text::Font, App};

use system4::{rendering::RenderState, settings::Settings};

use crate::init::WindowIds;

pub fn init_render_state(
    app: &App,
    window_ids: WindowIds,
    settings: &Settings,
    particle_limit: u32,
    terminal_font: Font,
) -> RenderState {
    RenderState::from_app(
        app,
        window_ids.audience,
        window_ids.performer,
        window_ids.control,
        settings.rendering.texture_width,
        settings.rendering.texture_height,
        settings.rendering.texture_samples,
        particle_limit,
        terminal_font,
    )
}
