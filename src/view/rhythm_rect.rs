use nannou::prelude::*;

#[derive(Debug)]
pub struct RhythmRect {
    /// Width and Length
    pub(crate) dims: Vec2,
    /// Color in Nannou Rgb
    pub(crate) color: Rgb,
    /// Alpha value
    pub(crate) alpha: f32,
}

impl RhythmRect {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }
}
