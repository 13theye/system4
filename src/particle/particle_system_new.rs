/// particle_system_new.rs
///
/// Faster particles using Structs-of-Arrays
///
use nannou::prelude::*;

const PARTICLE_MASS: f32 = 11.0;
const PARTICLE_LIFE_SPAN: f32 = 3600.0;
const FADE_IN_DURATION: f32 = 400.0; // frames to fade in
const FADE_OUT_DURATION: f32 = 100.0;

#[derive(Debug, Default)]
pub struct Particles {
    // Positioning and motion
    positions: Vec<Point2>,
    velocities: Vec<Vec2>,
    accelerations: Vec<Vec2>,
    feedback_positions: Vec<[Option<Point2>; 4]>,

    // Lifespan
    ages: Vec<f32>,
    remaining_life_spans: Vec<f32>,

    // Size
    sizes: Vec<f32>,
    masses: Vec<f32>,

    // Color and alpha
    colors: Vec<Rgb>,
    alphas: Vec<f32>,

    // Metadata
    parent_ids: Vec<usize>,
}

impl Particles {
    pub fn new() -> Self {
        Self {
            positions: Vec::new(),
            velocities: Vec::new(),
            accelerations: Vec::new(),
            feedback_positions: Vec::new(),
            ages: Vec::new(),
            remaining_life_spans: Vec::new(),
            sizes: Vec::new(),
            masses: Vec::new(),
            colors: Vec::new(),
            alphas: Vec::new(),
            parent_ids: Vec::new(),
        }
    }

    /// Add a moving particle. Used by Emitters.
    /// - default parameters implied:
    /// - particle life span (via constant)
    /// - particle mass (via constant)
    #[allow(clippy::too_many_arguments)]
    pub fn add_new_particle_with_motion(
        &mut self,
        parent_id: usize,
        position: Point2,
        size: f32,
        color: Rgb,
        alpha: f32,
        acceleration: Vec2,
        velocity: Vec2,
    ) {
        self.positions.push(position);
        self.accelerations.push(acceleration);
        self.velocities.push(velocity);
        self.feedback_positions.push([None; 4]);
        self.ages.push(0.0);
        self.remaining_life_spans.push(PARTICLE_LIFE_SPAN);
        self.sizes.push(size);
        self.masses.push(PARTICLE_MASS);
        self.colors.push(color);
        self.alphas.push(alpha);
        self.parent_ids.push(parent_id);
    }

    /// Draw all particles
    pub fn draw_all(&self, draw: &Draw, feedback: f32, dpi_scale: f32) {
        self.positions
            .iter()
            .enumerate()
            .for_each(|(index, &position)| {
                let scaled_position = position / dpi_scale;
                let scaled_size = self.sizes[index] / dpi_scale;

                draw.line()
                    .xy(scaled_position)
                    .start(scaled_position + vec2(scaled_size / 2.0, 0.0))
                    .end(scaled_position + vec2(0.0, scaled_size / 2.0))
                    .stroke_weight(scaled_size)
                    .color(Rgba {
                        color: self.colors[index],
                        alpha: self.alphas[index],
                    });

                if feedback > 0.1 {
                    for i in 1..(feedback * 3.0).round().min(3.0) as usize {
                        if let Some(position) = self.feedback_positions[index][i] {
                            let scaled_position = position / dpi_scale;
                            let color = Rgba {
                                color: self.colors[index],
                                alpha: (self.alphas[index] - (i as f32 / 3.0)).max(0.1),
                            };
                            draw.line()
                                .xy(scaled_position)
                                .start(scaled_position + vec2(scaled_size / 2.0, 0.0))
                                .end(scaled_position + vec2(0.0, scaled_size / 2.0))
                                .stroke_weight(scaled_size)
                                .color(color);
                        }
                    }
                }
            });
    }

    /******** Particle accessors: lifespan  **************/

    /// Returns true if the particle has a remaining life span > 0
    pub fn is_alive(&self, index: usize) -> bool {
        self.remaining_life_spans[index] > 0.0
    }

    /// Sets remaining life of a particle to 0
    pub fn kill(&mut self, index: usize) {
        self.remaining_life_spans[index] = 0.0;
    }

    /// Tells a particle to begin fading out
    pub fn fade_out(&mut self, index: usize) {
        self.remaining_life_spans[index] = FADE_OUT_DURATION;
    }

    /// Get a the fade out duration of particles
    pub fn fade_out_duration(&self) -> f32 {
        FADE_OUT_DURATION
    }

    /******** Particle accessors: motion & position ********/

    /// Returns true if the particle is outside of the bounds_rect area
    /// - buffer is 1000 pixels, allowing for a particle to leave the area
    ///   and then return
    pub fn is_out_of_bounds(&self, index: usize, bounds_rect: Rect) -> bool {
        let buffer = 1000.0;
        self.positions[index].x < bounds_rect.left() - buffer
            || self.positions[index].x > bounds_rect.right() + buffer
            || self.positions[index].y < bounds_rect.bottom() - buffer
            || self.positions[index].y > bounds_rect.top() + buffer
    }
}
