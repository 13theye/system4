//! src/view/mask.rs
//!
//! This module defines a view mask that can be used with the ParticleSystem
//!

use crate::{groups::VoiceId, utils::tween};

use nannou::prelude::*;
use std::time::{Duration, Instant};

/// A `Mask` is a view mask that can be used with the ParticleSystem
/// Particles within the Mask are rendered. Particles outside of the mask are invisible
/// but continue to be simulated.
#[derive(Debug, Copy, Clone)]
pub struct Mask {
    pub parent_voice: VoiceId,
    pub params: MaskParams,
    pub state: MaskState,
}

/// The parameters of a mask
#[derive(Debug, Copy, Clone)]
pub struct MaskParams {
    pub origin: Vec2,
    pub size: Vec2,
}

impl MaskParams {
    pub fn init(origin: Vec2, size: Vec2) -> Self {
        Self { origin, size }
    }

    /// Convenience function to init a default mask parameters for a specific voice
    pub fn init_for_voice(voice_id: VoiceId) -> Option<Self> {
        let origin = match voice_id {
            VoiceId::Voice0 => vec2(-950.0, 0.0),
            VoiceId::Voice3 => vec2(950.0, 0.0),
            _ => return None,
        };

        let size = vec2(900.0, 1300.0);
        Some(Self { origin, size })
    }
}

/// The state of a mask
/// - Active: The mask is active and can be used by the ParticleSystem
/// - Inactive: The mask is inactive and does nothing
/// - Animating: The mask is animating between two sets of parameters
#[derive(Debug, Copy, Clone)]
pub enum MaskState {
    Active,
    Inactive,
    Animating(MaskAnimation),
}

/// An animation defining a transition between one set of mask parameters and another
#[derive(Debug, Copy, Clone)]
pub struct MaskAnimation {
    pub start_time: Instant,
    pub duration: Duration,
    pub start_params: MaskParams,
    pub end_params: MaskParams,
}

impl Mask {
    /*************** Mask initilization  *************** */
    pub fn init(parent_voice: VoiceId, origin: Vec2, size: Vec2) -> Self {
        Self {
            parent_voice,
            params: MaskParams::init(origin, size),
            state: MaskState::Active,
        }
    }

    /// Convenience function to init a default mask for a specific voice
    pub fn init_for_voice(voice_id: VoiceId) -> Option<Self> {
        let params = MaskParams::init_for_voice(voice_id)?;
        Some(Self {
            parent_voice: voice_id,
            params,
            state: MaskState::Active,
        })
    }

    pub fn with_state(self, state: MaskState) -> Self {
        Self { state, ..self }
    }

    /**************** Basic utility methods ********************** */
    pub fn contains_point(&self, point: Vec2) -> bool {
        self.rect().contains(point)
    }

    pub fn rect(&self) -> Rect {
        Rect::from_x_y_w_h(
            self.params.origin.x,
            self.params.origin.y,
            self.params.size.x,
            self.params.size.y,
        )
    }

    /************** Animation methods ************************** */
    pub fn animate_to(&mut self, end_params: MaskParams, duration: Duration) {
        self.state = MaskState::Animating(MaskAnimation {
            start_time: Instant::now(),
            duration,
            start_params: self.params,
            end_params,
        })
    }

    pub fn update_animation(&mut self, now: Instant) {
        if let MaskState::Animating(animation) = &mut self.state {
            let elapsed = now.duration_since(animation.start_time);

            // Complete the animation if full time has elapsed
            if elapsed >= animation.duration {
                self.params = animation.end_params;
                self.state = MaskState::Active;
            } else {
                // Interpolate between start and end parameters
                let progress = elapsed.as_secs_f32() / animation.duration.as_secs_f32();
                self.params = MaskParams {
                    origin: tween::lerp(
                        animation.start_params.origin,
                        animation.end_params.origin,
                        progress,
                    ),
                    size: tween::lerp(
                        animation.start_params.size,
                        animation.end_params.size,
                        progress,
                    ),
                }
            }
        }
    }

    /*************** Drawing methods ************************** */

    /// Split the screen into rectangles that exclude the Mask rect
    pub fn dissect_screen(&self, texture_rect: Rect, scale_x: f32, scale_y: f32) -> Vec<Rect> {
        let mask_rect = self.rect();

        // Between the left edge of screen and the Mask
        let left_rect = Rect::from_corners(
            texture_rect.top_left(),
            vec2(mask_rect.bottom_left().x, texture_rect.bottom_left().y),
        );

        // Between the right edge of screen and the Mask
        let right_rect = Rect::from_corners(
            texture_rect.top_right(),
            vec2(mask_rect.bottom_right().x, texture_rect.bottom_right().y),
        );

        // Between the top edge of the screen and the Mask
        let top_rect = Rect::from_corners(left_rect.top_right(), mask_rect.top_right());

        // Between the bottom edge of the screen and the Mask
        let bottom_rect = Rect::from_corners(left_rect.bottom_right(), mask_rect.bottom_right());

        vec![left_rect, right_rect, top_rect, bottom_rect]
    }

    pub fn draw(&self, draw: &Draw, texture_rect: Rect, scale_x: f32, scale_y: f32) {
        let rects = self.dissect_screen(texture_rect, scale_x, scale_y);
        for rect in rects.iter() {
            draw.rect().xy(rect.xy()).wh(rect.wh()).color(BLACK);
        }
    }
}
