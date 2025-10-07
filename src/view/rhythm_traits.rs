use nannou::prelude::*;
use std::collections::HashMap;

use crate::{groups::rhythm::RhythmParams, view::RhythmViewUpdateParams};

pub trait RhythmFormation {
    fn center(&self) -> Vec2;
    fn capacity(&self) -> usize;

    fn initialize_rhythm(&mut self, rhythm_params: &RhythmParams);
    fn update(
        &mut self,
        rhythm_params: &RhythmParams,
        update_params: &RhythmViewUpdateParams,
        time: f32,
    );

    fn draw(&self, draw: &Draw);
}

pub trait RhythmElement {
    fn position(&self) -> Vec2;
    fn dims(&self) -> Vec2;
    fn color(&self) -> Rgb;
    fn alpha(&self) -> f32;
    fn last_update_time(&self) -> f32;
}
