// src/forces/wind.rs
//
// Re-exports of core wind types plus app-specific drawing extensions

use nannou::prelude::*;

// Re-export all wind types from core
pub use system4_core::physics::forces::{
    CellIdx, Wind, WindCell, WindCircle, WindCircleParams, WindField,
};

/// Extension trait for WindField drawing
pub trait WindFieldDrawExt {
    fn draw(&self, draw: &Draw, scale_x: f32, scale_y: f32, div_factor: usize);
    fn draw_grid(&self, draw: &Draw, scale_x: f32, scale_y: f32);
    fn draw_grid_rect(&self, draw: &Draw, scale_x: f32, scale_y: f32);
}

impl WindFieldDrawExt for WindField {
    fn draw(&self, draw: &Draw, scale_x: f32, scale_y: f32, div_factor: usize) {
        draw_origin(self, draw, scale_x, scale_y);
        self.draw_grid(draw, scale_x, scale_y);
        draw_vectors(self, draw, scale_x, scale_y, div_factor);
    }

    fn draw_grid(&self, draw: &Draw, scale_x: f32, scale_y: f32) {
        let cols = self.grid_cols();
        let rows = self.grid_rows();
        let cell_w = self.cell_size().x * scale_x;
        let cell_h = self.cell_size().y * scale_y;

        let total_w = cols as f32 * cell_w;
        let total_h = rows as f32 * cell_h;

        let offset_x = -total_w / 2.0;
        let offset_y = -total_h / 2.0;

        // Vertical lines
        for col in 0..=cols {
            let x = offset_x + col as f32 * cell_w;
            draw.line()
                .start(pt2(x, offset_y))
                .end(pt2(x, offset_y + total_h))
                .color(rgba(0.3, 0.3, 0.3, 0.3))
                .weight(1.0);
        }

        // Horizontal lines
        for row in 0..=rows {
            let y = offset_y + row as f32 * cell_h;
            draw.line()
                .start(pt2(offset_x, y))
                .end(pt2(offset_x + total_w, y))
                .color(rgba(0.3, 0.3, 0.3, 0.3))
                .weight(1.0);
        }
    }

    fn draw_grid_rect(&self, draw: &Draw, scale_x: f32, scale_y: f32) {
        use crate::physics_ext::to_nannou_vec2;

        for row in 0..self.grid_rows() {
            for col in 0..self.grid_cols() {
                let idx = CellIdx { col, row };
                let cell = self.get_cell(idx).expect("cell should exist");

                let color: Rgba = if let Some(wind) = cell.combined_wind() {
                    let dir = to_nannou_vec2(wind.direction());
                    let h = dir.y.atan2(dir.x) / (2.0 * PI);
                    let s = wind.strength() / 30.0;
                    Rgba::from(hsv(h, s, 1.0))
                } else {
                    rgba(0.0, 0.0, 0.0, 0.0)
                };

                let origin = to_nannou_vec2(cell.origin());
                let cell_size = to_nannou_vec2(self.cell_size());

                draw.rect()
                    .xy(origin * vec2(scale_x, scale_y))
                    .w_h(cell_size.x * scale_x, cell_size.y * scale_y)
                    .stroke_color(rgba(0.3, 0.3, 0.3, 0.3))
                    .stroke_weight(1.0)
                    .color(color);
            }
        }
    }
}

/// Draw all the Wind vectors
fn draw_vectors(field: &WindField, draw: &Draw, scale_x: f32, scale_y: f32, div_factor: usize) {
    use crate::physics_ext::to_nannou_vec2;

    for row in 0..field.grid_rows() {
        for col in 0..field.grid_cols() {
            // Sample based on div_factor using deterministic pattern
            if !(col + row).is_multiple_of(div_factor) {
                continue;
            }

            let idx = CellIdx { col, row };
            let cell = field.get_cell(idx).expect("cell should exist");

            let Some(wind) = cell.combined_wind() else {
                continue;
            };

            // Draw wind vector from cell origin
            let vector_scale = 3.0;
            let origin = to_nannou_vec2(cell.origin());
            let direction = to_nannou_vec2(wind.direction());
            let vector_end = origin + direction * wind.strength() * vector_scale;

            draw.line()
                .start(origin * vec2(scale_x, scale_y))
                .end(vector_end * vec2(scale_x, scale_y))
                .color(rgba(0.0, 0.8, 1.0, 0.2))
                .stroke_weight(5.0);

            draw.rect()
                .xy(vector_end * vec2(scale_x, scale_y))
                .wh(vec2(20.0 * scale_x, 20.0 * scale_y))
                .color(rgba(1.0, 0.0, 0.0, 0.2))
                .stroke_weight(0.0);
        }
    }
}

/// Draw the origin of the WindField
fn draw_origin(field: &WindField, draw: &Draw, scale_x: f32, scale_y: f32) {
    use crate::physics_ext::to_nannou_vec2;
    let origin = to_nannou_vec2(field.origin());
    draw.ellipse()
        .xy(origin * vec2(scale_x, scale_y))
        .w_h(20.0 * scale_x, 20.0 * scale_y)
        .color(rgba(0.0, 1.0, 0.0, 0.2));
}

/// Extension trait for WindCircle drawing and bounds
pub trait WindCircleDrawExt {
    fn draw_center(&self, draw: &Draw, scale_x: f32, scale_y: f32);
    fn draw(&self, draw: &Draw, scale_x: f32, scale_y: f32);
    fn rect(&self) -> Rect;
}

impl WindCircleDrawExt for WindCircle {
    fn draw_center(&self, draw: &Draw, scale_x: f32, scale_y: f32) {
        use crate::physics_ext::to_nannou_vec2;
        let center = to_nannou_vec2(self.params().center);
        draw.ellipse()
            .xy(center * vec2(scale_x, scale_y))
            .w_h(40.0 * scale_x, 40.0 * scale_y)
            .color(rgba(1.0, 0.2, 0.0, 0.2));
    }

    fn draw(&self, draw: &Draw, scale_x: f32, scale_y: f32) {
        use crate::physics_ext::to_nannou_vec2;
        let center = to_nannou_vec2(self.params().center) * vec2(scale_x, scale_y);
        let outer_radius = self.params().outer_radius;
        let inner_radius = self.params().inner_radius;

        // Draw outer radius circle
        draw.ellipse()
            .xy(center)
            .radius(outer_radius * scale_x.min(scale_y))
            .stroke_color(rgba(0.8, 0.4, 0.0, 0.6))
            .stroke_weight(2.0)
            .no_fill();

        // Draw inner radius circle
        draw.ellipse()
            .xy(center)
            .radius(inner_radius * scale_x.min(scale_y))
            .stroke_color(rgba(0.8, 0.4, 0.0, 0.4))
            .stroke_weight(1.0)
            .no_fill();
    }

    fn rect(&self) -> Rect {
        use crate::physics_ext::to_nannou_vec2;
        let center = to_nannou_vec2(self.params().center);
        let radius = self.params().outer_radius;
        Rect::from_xy_wh(center, vec2(radius * 2.0, radius * 2.0))
    }
}
