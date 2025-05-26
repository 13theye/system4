// src/fps/fps_manager.rs
//
// The FPS calculating module for apps that use Nannou

use nannou::prelude::*;
use std::time::Instant;

pub struct FpsManager {
    fps: f32,
    fps_update_interval: f32,
    last_update: Instant,
    frame_count: usize,
    frame_time_accumulator: f32,

    counting: bool,

    // The place where the FPS will be drawn
    draw_position: Option<Point2>,
}

impl Default for FpsManager {
    fn default() -> Self {
        Self::new()
    }
}

impl FpsManager {
    pub fn new() -> Self {
        Self {
            fps: 0.0,
            fps_update_interval: 0.3,
            last_update: Instant::now(),
            frame_count: 0,
            frame_time_accumulator: 0.0,
            counting: false,

            draw_position: None,
        }
    }

    // Start counting frames
    pub fn initialize(&mut self) {
        self.fps = 0.0;
        self.frame_count = 0;
        self.frame_time_accumulator = 0.0;
        self.last_update = Instant::now();
    }

    // Update the FPS
    pub fn update(&mut self, draw: &Draw) {
        // Don't update if not counting
        if !self.counting {
            return;
        }

        self.calculate_fps();
        self.draw(draw);
    }

    fn calculate_fps(&mut self) {
        let now = Instant::now();
        let dt = now - self.last_update;
        let elapsed = dt.as_secs_f32();

        self.frame_count += 1;
        self.frame_time_accumulator += elapsed;

        if elapsed >= self.fps_update_interval {
            if self.frame_count > 0 {
                let avg_frame_time = self.frame_time_accumulator / self.frame_count as f32;
                self.fps = if avg_frame_time > 0.0 {
                    1.0 / avg_frame_time
                } else {
                    0.0
                };
            }

            // Reset accumulators
            self.frame_count = 0;
            self.frame_time_accumulator = 0.0;
            self.last_update = Instant::now();
        }
    }

    pub fn fps(&self) -> f32 {
        self.fps
    }

    /***************** Drawing functionality ***************************/

    pub fn set_draw_position(&mut self, position: Point2) {
        self.draw_position = Some(position);
    }

    pub fn draw(&self, draw: &Draw) {
        // Don't draw if no position is set
        let Some(pos) = self.draw_position else {
            return;
        };

        draw.text(&format!("FPS: {:.1}", self.fps))
            .x_y(pos.x, pos.y)
            .color(RED)
            .font_size(10);
    }
}
