// system4-core/src/physics/color.rs
// Simple color types for physics simulation

/// RGB color with f32 components (0.0-1.0)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgb {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl Rgb {
    pub const fn new(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b }
    }
}

/// RGBA color with f32 components (0.0-1.0)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgba {
    pub color: Rgb,
    pub alpha: f32,
}

impl Rgba {
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            color: Rgb::new(r, g, b),
            alpha: a,
        }
    }

    pub const fn from_rgb(color: Rgb, alpha: f32) -> Self {
        Self { color, alpha }
    }
}

// Convenience constructors matching Nannou's API
pub const fn rgb(r: f32, g: f32, b: f32) -> Rgb {
    Rgb::new(r, g, b)
}

pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Rgba {
    Rgba::new(r, g, b, a)
}
