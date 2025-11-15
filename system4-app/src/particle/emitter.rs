/// src/particle/emitter.rs
///
/// Re-exports of core emitter types plus app-specific drawing extensions

use nannou::prelude::*;

// Re-export all emitter types from core
pub use system4_core::physics::particles::{
    EmitDirection, Emitter, FullScreenRandomEmitter, LinearEmitter, PointEmitter,
};

use crate::physics_ext::to_nannou_vec2;

/// Extension trait for Emitter drawing
pub trait EmitterDrawExt {
    fn draw(&self, draw: &Draw, scale_x: f32, scale_y: f32);
}

impl EmitterDrawExt for FullScreenRandomEmitter {
    fn draw(&self, draw: &Draw, scale_x: f32, scale_y: f32) {
        let center = to_nannou_vec2(self.spawn_area_center());
        let size = to_nannou_vec2(self.spawn_area_size());

        draw.rect()
            .xy(center * vec2(scale_x, scale_y))
            .wh(size * vec2(scale_x, scale_y))
            .stroke_color(rgba(1.0, 0.0, 0.0, 0.2))
            .stroke_weight(10.0)
            .no_fill();
    }
}

impl EmitterDrawExt for PointEmitter {
    fn draw(&self, draw: &Draw, scale_x: f32, scale_y: f32) {
        let origin = to_nannou_vec2(self.origin());

        draw.ellipse()
            .radius(2.0)
            .xy(origin * vec2(scale_x, scale_y))
            .color(rgba(1.0, 0.0, 0.0, 0.2))
            .stroke_weight(0.0);
    }
}

impl EmitterDrawExt for LinearEmitter {
    fn draw(&self, draw: &Draw, scale_x: f32, scale_y: f32) {
        let start = to_nannou_vec2(self.start());
        let end = to_nannou_vec2(self.end());

        draw.line()
            .start(start * vec2(scale_x, scale_y))
            .end(end * vec2(scale_x, scale_y))
            .color(rgba(1.0, 0.0, 0.0, 0.2))
            .stroke_weight(4.0);
    }
}

/// Extension trait for dyn Emitter trait objects
pub trait DynEmitterDrawExt {
    fn draw_dyn(&self, draw: &Draw, scale_x: f32, scale_y: f32);
}

impl DynEmitterDrawExt for dyn Emitter {
    fn draw_dyn(&self, draw: &Draw, scale_x: f32, scale_y: f32) {
        // Try to downcast to each concrete type and draw
        if let Some(emitter) = (self as &dyn std::any::Any).downcast_ref::<FullScreenRandomEmitter>() {
            emitter.draw(draw, scale_x, scale_y);
        } else if let Some(emitter) = (self as &dyn std::any::Any).downcast_ref::<PointEmitter>() {
            emitter.draw(draw, scale_x, scale_y);
        } else if let Some(emitter) = (self as &dyn std::any::Any).downcast_ref::<LinearEmitter>() {
            emitter.draw(draw, scale_x, scale_y);
        }
    }
}
