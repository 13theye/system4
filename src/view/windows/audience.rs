//! src/view/windows/audience.rs
//!
//! The functions used to render the contents of the audience window.
//!
//! fn audience_view() - called every frame by main.rs for the
//!                      Nannou update loop.

use crate::{groups::VoiceId, model::Model};

use nannou::prelude::*;
use std::time::Instant;

/// Draw the audience view window's contents
pub fn audience_view(app: &App, model: &Model, frame: Frame) {
    // Begin Rendering context
    {
        let mut rendering = model.render_state.render_engine.borrow_mut();

        // Get GPU resources
        let Some(window) = app.window(model.render_state.audience_window_id) else {
            eprintln!("Audience window not found. Exiting app.");
            std::process::exit(1);
        };

        let device = window.device();
        let mut encoder = rendering.create_command_encoder(device);
        let queue = window.queue();

        // Clear all textures
        //rendering.draw.background().color(BLACK);
        rendering.encode_clear_all_textures(&mut encoder, wgpu::Color::TRANSPARENT);

        // Encode Nannou Draw
        rendering.encode_draw_commands(device, &mut encoder);

        /************ Particle and segment drawing ************* */

        // ZERO-COPY: Write and encode particles and segments per voice
        // PHASE 1: CPU ASSEMBLY (batched before GPU write)
        // Assemble all GPU data on CPU first, leveraging internal Rayon parallelism in segment assembly
        // Note: We can't use Rayon at the voice level due to Sync constraints, but segment assembly
        // already uses Rayon internally for per-particle parallelism (see write_segments_for_voice)
        let start_assembly = Instant::now();

        let drone_physics_views: Vec<_> = [VoiceId::Voice0, VoiceId::Voice3]
            .iter()
            .filter_map(|&voice_id| {
                model.voice_manager.voices.get(&voice_id).and_then(|voice| {
                    voice.as_drone().map(|drone| {
                        // Assemble particle GPU data (CPU work)
                        let particles = model
                            .particle_system
                            .assemble_particles_for_voice(voice_id, model.engine_debug);

                        // Assemble segment GPU data (CPU work - uses Rayon internally for particles)
                        let segments = model.particle_system.assemble_segments_for_voice(
                            voice_id,
                            drone.params.segment_length,
                            drone.params.segment_line_width,
                            model.engine_debug,
                        );

                        let mask = drone.mask;

                        (voice_id, particles, segments, mask)
                    })
                })
            })
            .collect();

        if model.engine_debug {
            let assembly_time = start_assembly.elapsed();
            println!(
                "[CPU ASSEMBLY] Total assembly time: {:.3}ms",
                assembly_time.as_secs_f64() * 1000.0
            );
        }

        // PHASE 2: SEQUENTIAL GPU WRITE
        // Write pre-assembled data to GPU buffers (fast memcpy, must be sequential)
        let start_gpu_write = Instant::now();

        for (voice_id, particles, segments, mask) in drone_physics_views {
            let Some(texture_name) = model.render_state.get_texture_name(voice_id) else {
                continue;
            };

            let texture = rendering
                .get_named_texture(&texture_name)
                .expect("Fatal Error: Missing particle texture");

            // Encode particles and segments to particle texture (straight alpha)
            let (renderer, seg_renderer, texture) = match voice_id {
                VoiceId::Voice0 => (
                    &model.render_state.particle_renderer_voice0,
                    &model.render_state.segment_renderer_voice0,
                    texture,
                ),
                VoiceId::Voice3 => (
                    &model.render_state.particle_renderer_voice3,
                    &model.render_state.segment_renderer_voice3,
                    texture,
                ),
                _ => continue,
            };

            let particle_count = renderer.update_buffer(queue, &particles);
            let (_, segment_instance_count) = seg_renderer.update_buffer(queue, &segments);

            if particle_count > 0 {
                renderer.encode_only(&mut encoder, particle_count, texture);
                seg_renderer.encode_only(&mut encoder, segment_instance_count, texture);
            }

            // Draw mask to the same particle texture (both use straight alpha)
            // Masks and particles can share a texture because they both use standard alpha blending
            mask.draw(&rendering.draw, model.render_state.render_rect);
            rendering.encode_draw_commands_into(device, &mut encoder, &texture_name);
        }

        if model.engine_debug {
            let gpu_write_time = start_gpu_write.elapsed();
            println!(
                "[GPU WRITE] Total GPU buffer write time: {:.3}ms",
                gpu_write_time.as_secs_f64() * 1000.0
            );
        }

        /************ Executing the pipelines ************* */

        // Combine voice textures (straight alpha inputs → premultiplied output)
        // The composite shader handles straight→premultiplied conversion internally
        if let Err(e) = rendering.execute_named_pipeline("combine_voices", device, &mut encoder) {
            eprintln!("Error executing combine_voices pipeline: {}", e);
        }

        // Encode heatmap (still uses legacy buffer for now)
        // will not work in the current ZERO-COPY implementation because buffer will
        // remain empty.
        /*
        model.render_state.heatmap_renderer.encode_into(
            device,
            &mut encoder,
            queue,
            &model.render_state.gpu_particle_buffer,
            model.render_state.render_rect,
            rendering.get_named_texture("heatmap").unwrap(),
        );


        //Encode post processing
        if let Err(e) = rendering.execute_named_pipeline("heatmap_effects", device, &mut encoder) {
            eprintln!("Error executing heatmap_effects pipeline: {}", e);
        }

        if let Err(e) = rendering.execute_named_pipeline("composite_step", device, &mut encoder) {
            eprintln!("Error executing composite_step pipeline: {}", e);
        }
        */

        if let Err(e) = rendering.execute_named_pipeline("effects", device, &mut encoder) {
            eprintln!("Error executing effects pipeline: {}", e);
        }

        // Draw rhythm formations to rhythm_alpha texture
        model
            .rhythm_view
            .draw_alpha_elements(&rendering.draw, model.ui_state.show_debug_geometry);

        rendering.encode_draw_commands_into(device, &mut encoder, "rhythm_alpha");

        model.rhythm_view.draw_activations(&rendering.draw);
        rendering.encode_draw_commands_into(device, &mut encoder, "rhythm_activations");

        // Unified text overlay (new system)
        let now = Instant::now();
        {
            let mut overlay = model.ui_state.text_overlay.borrow_mut();

            // Always route AI status text into Voice2's history (not live text).
            let (ai_status_text, do_fade) = model.voice_manager.current_ai_status_text();
            overlay.push_ai_status_history_if_changed(ai_status_text, do_fade, now);

            overlay.update_and_draw_all(&rendering.draw, now);

            // Draw the central dividing line
            rendering
                .draw
                .line()
                .start(vec2(0.0, rendering.output_texture.height() as f32 / 2.0))
                .end(vec2(0.0, rendering.output_texture.height() as f32 / -2.0))
                .color(rgba(1.0, 1.0, 1.0, 1.0))
                .stroke_weight(2.0);
        }

        // Encode text overlay to terminal texture
        rendering.encode_draw_commands_into(device, &mut encoder, "terminal");

        // Composite rhythm formations onto particles (with alpha 0.5)
        // particles → rhythm_alpha (α=0.5)
        if let Err(e) = rendering.execute_named_pipeline("rhythm composite", device, &mut encoder) {
            eprintln!("Error executing rhythm composite pipeline: {}", e);
        }

        // Add rhythm activations (should cover layers beneath, not blend)
        // rhythm_composited → rhythm_activations
        if let Err(e) = rendering.execute_named_pipeline("add activations", device, &mut encoder) {
            eprintln!("Error executing add activations pipeline: {}", e);
        }

        // Final composite: add terminal on top (with full opacity)
        // rhythm_with_activations → terminal (α=1.0)
        if let Err(e) = rendering.execute_named_pipeline("final composite", device, &mut encoder) {
            eprintln!("Error executing final composite pipeline: {}", e);
        }

        rendering.submit_command_encoder(device, queue, encoder);

        // Update reshaper if needed (could be cached in Model)
        rendering.draw_to_frame(&model.render_state.audience_reshaper, &frame);
    }
    // End Rendering context

    // Show screen bounds if enabled
    if model.ui_state.show_bounds {
        draw_bounds(app, model);
    }

    // Draw over the texture
    let _ = model.render_state.audience_draw.to_frame(app, &frame);
}

// ************************ Debug display  *************************************

fn draw_bounds(app: &App, model: &Model) {
    let draw = &model.render_state.audience_draw;
    let rect = app
        .window(model.render_state.audience_window_id)
        .unwrap()
        .rect();

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
        .stroke(rgba(0.4, 1.0, 0.4, 0.75)) // Green outline
        .stroke_weight(5.0)
        .no_fill();
}
