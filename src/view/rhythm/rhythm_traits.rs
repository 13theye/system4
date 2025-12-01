use nannou::prelude::*;
use std::time::Instant;

use crate::{groups::RhythmParams, view::rhythm::RhythmViewUpdateParams};

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

    fn initialize_rhythm(&mut self, rhythm_params: &RhythmParams, now: Instant);
    fn reinitialize_rhythm(&mut self, rhythm_params: &RhythmParams, now: Instant);
    fn clear_rhythm(&mut self, now: Instant);

    fn update_transitions(&mut self, now: Instant);
    fn update_active(
        &mut self,
        rhythm_params: &RhythmParams,
        update_params: &RhythmViewUpdateParams,
        now: Instant,
    );

    fn draw(&self, draw: &Draw);
}

pub trait RhythmElement {
    fn set_position(&mut self, pos: Vec2);
    fn position(&self) -> Vec2;
    fn target_position(&self) -> Vec2;
    fn start_position(&self) -> Vec2;
    fn set_animation_positions(&mut self, start: Vec2, target: Vec2);
    fn set_last_active_instant(&mut self, now: Instant);
    fn last_update_instant(&self) -> Instant;
    fn set_is_wing(&mut self, is_wing: bool);
    fn update(
        &mut self,
        rhythm_params: &RhythmParams,
        update_params: &RhythmViewUpdateParams,
        slot: usize,
        now: Instant,
    );
    fn draw(&self, draw: &Draw);
}
