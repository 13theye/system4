// src/groups/voice.rs

use crate::{
    forces::wind::WindCircle,
    groups::VoiceId,
    particle::emitter::{EmitDirection, Emitter, LinearEmitter},
    terminals::commands::drone::DroneConfig,
};
use nannou::prelude::*;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DroneState {
    Active,
    Clearing, // Particles fading out, voice will be removed when all particles are dead
}

#[derive(Debug, Clone)]
pub struct DroneParams {
    pub particle_limit: usize,
    pub volume: f32,
    pub default_spawn_rate: f32,

    pub alpha_limit: f32,
    pub color_limit: Rgb,
    pub feedback: f32,
    pub vibration: f32,
    pub segment_length: f32,
    pub segment_line_width: f32,
    pub emitter_position: f32,
}

impl Default for DroneParams {
    fn default() -> Self {
        Self {
            particle_limit: 15000,
            volume: 0.0,
            default_spawn_rate: 30.0,
            alpha_limit: 0.0,
            color_limit: Rgb::new(0.0, 0.0, 0.0),
            feedback: 0.0,
            vibration: 0.0,
            segment_length: 1.0,
            segment_line_width: 2.0,
            emitter_position: 0.0,
        }
    }
}

pub struct Drone {
    pub id: VoiceId,
    pub state: DroneState,
    pub params: DroneParams,
    // smallest bounding box containing all wind_circles
    pub bounds_rect: Rect,

    pub emitters: Vec<Box<dyn Emitter>>,
    pub wind_circles: HashMap<usize, WindCircle>,
    current_wind_circle_idx: usize,
}

impl Drone {
    pub fn new_with_id(id: VoiceId) -> Self {
        Self {
            id,
            state: DroneState::Active,
            params: DroneParams::default(),
            bounds_rect: Rect::from_x_y_w_h(0.0, 0.0, 0.0, 0.0),
            emitters: Vec::new(),
            wind_circles: HashMap::new(),
            current_wind_circle_idx: 0,
        }
    }

    pub fn with_particle_limit(mut self, limit: usize) -> Self {
        self.params.particle_limit = limit;
        self
    }

    pub fn issue_wind_circle_idx(&mut self) -> usize {
        if let Some(i) =
            (0..self.current_wind_circle_idx).find(|&i| !self.wind_circles.contains_key(&i))
        {
            i
        } else {
            let idx = self.current_wind_circle_idx;
            self.current_wind_circle_idx += 1;
            idx
        }
    }

    /// Initialize drone structure (WindCircle and emitters) without setting parameters
    /// Parameters should be set through the command pipeline after this call
    /// DroneConfig should have been merged with defaults as necessary
    pub fn initialize_drone(
        &mut self,
        resolved_config: &DroneConfig,
        default_particle_color: Rgb,
        default_spawn_rate: f32,
    ) -> usize {
        // Extract resolved values for WindCircle creation
        let gravity = resolved_config.gravity.unwrap();
        let force = resolved_config.force.unwrap();
        let outer_radius = resolved_config.outer_radius.unwrap();
        let inner_radius = resolved_config.inner_radius.unwrap();
        let center_x = resolved_config.center_x.unwrap();
        let center_y = resolved_config.center_y.unwrap();
        let noise = resolved_config.noise.unwrap();

        self.params.default_spawn_rate = default_spawn_rate;

        // WindCircle creation
        let center = vec2(center_x, center_y);
        let circle_id = self.issue_wind_circle_idx();
        let circle = WindCircle::new(
            circle_id,
            self.id,
            center,
            outer_radius,
            inner_radius,
            force,
            gravity,
            noise,
        );

        // Add the wind circle to the forces FIRST
        self.add_wind_circle(circle);

        // Create particle emitters (now that bounds can be calculated correctly)
        self.bounds_rect = self.calculate_bounds();
        self.add_linear_emitters();

        // Set the default color (this is structural, not a DroneConfig parameter)
        self.set_color_limit(default_particle_color);

        // Return the circle ID for use in parameter commands
        circle_id
    }

    fn calculate_bounds(&self) -> Rect {
        if self.wind_circles.is_empty() {
            return Rect::from_x_y_w_h(0.0, 0.0, 0.0, 0.0);
        }

        let mut min_x = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_y = f32::NEG_INFINITY;

        for circle in self.wind_circles.values() {
            let circle_rect = circle.rect();

            min_x = min_x.min(circle_rect.left());
            max_x = max_x.max(circle_rect.right());
            min_y = min_y.min(circle_rect.bottom());
            max_y = max_y.max(circle_rect.top());
        }

        let width = max_x - min_x;
        let height = max_y - min_y;
        let center_x = min_x + width / 2.0;
        let center_y = min_y + height / 2.0;

        Rect::from_x_y_w_h(center_x, center_y, width, height)
    }

    fn add_linear_emitters(&mut self) {
        let p = self.params.emitter_position;
        let center_y = self.bounds_rect.y();
        let offset = self.bounds_rect.h() / 2.0 - 100.0;

        // Left emitter (slides downward as p increases)
        let left_start = vec2(self.bounds_rect.left(), center_y + offset * (1.0 - p));
        let left_end = vec2(self.bounds_rect.left(), center_y - offset * p);

        // Right emitter (slides upward as p increases)
        let right_start = vec2(self.bounds_rect.right(), center_y + offset * p);
        let right_end = vec2(self.bounds_rect.right(), center_y - offset * (1.0 - p));

        let emitter_left = LinearEmitter::new(
            self.id,
            left_start,
            left_end,
            EmitDirection::East,
            self.params.default_spawn_rate,
        );

        let emitter_right = LinearEmitter::new(
            self.id,
            right_start,
            right_end,
            EmitDirection::West,
            self.params.default_spawn_rate,
        );

        // Add the emitters
        self.emitters.push(Box::new(emitter_left));
        self.emitters.push(Box::new(emitter_right));

        self.set_is_spawning(true);
    }

    pub fn is_active(&self) -> bool {
        self.state == DroneState::Active
    }

    pub fn set_alpha_limit(&mut self, value: f32) {
        self.params.alpha_limit = value;
    }

    pub fn set_color_limit(&mut self, value: Rgb) {
        self.params.color_limit = value;
    }

    pub fn set_is_spawning(&mut self, is_spawning: bool) {
        for emitter in self.emitters.iter_mut() {
            emitter.set_enabled(is_spawning);
        }
    }

    pub fn set_volume(&mut self, value: f32) {
        self.params.volume = value;
    }

    pub fn set_feedback(&mut self, value: f32) {
        self.params.feedback = value;
    }

    pub fn set_vibration(&mut self, value: f32) {
        self.params.vibration = value;
    }

    pub fn set_segment_length(&mut self, value: f32) {
        self.params.segment_length = value;
    }

    pub fn set_segment_line_width(&mut self, value: f32) {
        self.params.segment_line_width = value;
    }

    pub fn set_emitter_position(&mut self, value: f32) {
        self.params.emitter_position = value.clamp(0.0, 1.0);
        self.recalculate_emitters();
    }

    fn recalculate_emitters(&mut self) {
        self.bounds_rect = self.calculate_bounds();
        self.emitters.clear();
        self.add_linear_emitters();
    }

    /********** Wind Circle methods ********************* */

    /// Returns `true` if a WindCircle exists for the given `circle_id`
    pub fn has_wind_circle(&self, circle_id: usize) -> bool {
        self.wind_circles.contains_key(&circle_id)
    }

    /// Add a WindCircle to this Voice
    pub fn add_wind_circle(&mut self, circle: WindCircle) {
        self.wind_circles.insert(circle.id, circle);
        self.recalculate_emitters();
    }

    /// Remove a WindCircle from this voice
    pub fn remove_wind_circle(&mut self, id: usize) {
        self.wind_circles.remove(&id);
        self.recalculate_emitters();
    }

    pub fn remove_all_circles(&mut self) {
        self.wind_circles.clear();
        self.recalculate_emitters();
    }

    /// Set the outer radius of a WindCircle
    pub fn set_circle_outer_radius(&mut self, id: usize, value: f32) {
        if let Some(circle) = self.wind_circles.get_mut(&id) {
            circle.params_mut().set_outer_radius(value);
            self.recalculate_emitters();
        } else {
            println!(
                "Voice {} set OR: Wind circle not found for id: {}",
                self.id, id
            );
        }
    }

    /// Set the inner radius of a WindCircle
    pub fn set_circle_inner_radius(&mut self, id: usize, value: f32) {
        if let Some(circle) = self.wind_circles.get_mut(&id) {
            circle.params_mut().set_inner_radius(value);
            self.recalculate_emitters();
        } else {
            println!(
                "Voice {} set IR: Wind circle not found for id: {}",
                self.id, id
            );
        }
    }

    pub fn set_circle_center_x(&mut self, id: usize, value: f32) {
        if let Some(circle) = self.wind_circles.get_mut(&id) {
            circle.params_mut().set_center_x(value);
            self.recalculate_emitters();
        } else {
            println!(
                "Voice {} set CX: Wind circle not found for id: {}",
                self.id, id
            );
        }
    }

    pub fn set_circle_center_y(&mut self, id: usize, value: f32) {
        if let Some(circle) = self.wind_circles.get_mut(&id) {
            circle.params_mut().set_center_y(value);
            self.recalculate_emitters();
        } else {
            println!(
                "Voice {} set CY: Wind circle not found for id: {}",
                self.id, id
            );
        }
    }

    /// Set the gravity of a WindCircle
    pub fn set_circle_gravity(&mut self, id: usize, value: f32) {
        if let Some(circle) = self.wind_circles.get_mut(&id) {
            circle.params_mut().set_gravity(value);
        } else {
            println!(
                "Voice {} set gravity: Wind circle not found for id: {}",
                self.id, id
            );
        }
    }

    /// Set the force of a WindCircle
    pub fn set_circle_force(&mut self, id: usize, value: f32) {
        if let Some(circle) = self.wind_circles.get_mut(&id) {
            circle.params_mut().set_force(value);
        } else {
            println!(
                "Voice {} set force: Wind circle not found for id: {}",
                self.id, id
            );
        }
    }

    /// Set the outer radius of a WindCircle
    pub fn set_circle_noise(&mut self, id: usize, value: f32) {
        if let Some(circle) = self.wind_circles.get_mut(&id) {
            circle.params_mut().set_noise(value);
        } else {
            println!(
                "Voice {} set noise: Wind circle not found for id: {}",
                self.id, id
            );
        }
    }
}
