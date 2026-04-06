use nannou::prelude::*;

use crate::model::Model;

/// Draw the performer window's contents
pub fn performer_view(app: &App, model: &Model, frame: Frame) {
    let rendering = model.render_state.render_engine.borrow_mut();

    // Get the raw scene texture view
    let _scene_view = rendering.get_scene_view();

    // Draw game content to the frame
    rendering.draw_to_frame(&model.render_state.performer_reshaper, &frame);

    // Mirror intro image overlay onto performer window
    if model.intro_image.is_visible() {
        let performer_rect = app
            .window(model.render_state.performer_window_id)
            .unwrap()
            .rect();
        model
            .intro_image
            .draw(&model.render_state.performer_draw, performer_rect);
    }
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
