use nannou::prelude::*;

use crate::{
    groups::rhythm::{self, RhythmParams},
    view::RhythmViewUpdateParams,
};

#[derive(Debug)]
pub enum RhythmFormationState {
    Inactive,
    Initializing,
    Active,
    Reinitializing,
    Clearing,
}

pub trait RhythmFormation {
    fn center(&self) -> Vec2;
    fn capacity(&self) -> usize;

    fn initialize_rhythm(&mut self, rhythm_params: &RhythmParams, time: f32);
    fn reinitialize_rhythm(&mut self, rhythm_params: &RhythmParams, time: f32);
    fn clear_rhythm(&mut self, time: f32);

    fn update_transitions(&mut self, time: f32);
    fn update_active(
        &mut self,
        rhythm_params: &RhythmParams,
        update_params: &RhythmViewUpdateParams,
        time: f32,
    );

    fn draw(&self, draw: &Draw);
}

pub trait RhythmElement {
    fn set_position(&mut self, pos: Vec2);
    fn position(&self) -> Vec2;
    fn target_position(&self) -> Vec2;
    fn start_position(&self) -> Vec2;
    fn set_animation_positions(&mut self, start: Vec2, target: Vec2);
    fn set_last_active_time(&mut self, time: f32);
    fn last_update_time(&self) -> f32;
    fn set_is_wing(&mut self, is_wing: bool);
    fn update(
        &mut self,
        rhythm_params: &RhythmParams,
        update_params: &RhythmViewUpdateParams,
        slot: usize,
        time: f32,
    );
    fn draw(&self, draw: &Draw);
}
