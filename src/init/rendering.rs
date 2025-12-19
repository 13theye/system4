use nannou::{text::Font, App};

use system4::{rendering::RenderState, settings::Settings};

use crate::init::WindowIds;

pub fn init_render_state(
    app: &App,
    window_ids: WindowIds,
    settings: &Settings,
    particle_limit: u32,
    dpi_scale: f32,
    font: Font,
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
        dpi_scale,
        font,
    )
}
