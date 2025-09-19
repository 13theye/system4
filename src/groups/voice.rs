// src/groups/voice.rs

use crate::{
    forces::WindCircle,
    groups::VoiceId,
    particle::{EmitDirection, Emitter, FullScreenRandomEmitter, LinearEmitter},
    terminals::commands::drone::DroneConfig,
    utils::IdGenerator,
};
use nannou::prelude::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct VoiceParams {
    pub particle_limit: usize,
    pub volume: f32,

    pub alpha_limit: f32,
    pub color_limit: Rgb,
    pub feedback: f32,
    pub vibration: f32,
}

impl Default for VoiceParams {
    fn default() -> Self {
        Self {
            particle_limit: 5000,
            volume: 0.0,
            alpha_limit: 0.0,
            color_limit: Rgb::new(0.0, 0.0, 0.0),
            feedback: 0.0,
            vibration: 0.0,
        }
    }
}

pub struct Voice {
    pub id: VoiceId,
    pub params: VoiceParams,
    pub bounds_rect: Rect,

    pub emitters: Vec<Box<dyn Emitter>>,
    pub wind_circles: HashMap<usize, WindCircle>,
    wind_circle_idx: usize,
}

impl Voice {
    pub fn new_with_id(id: VoiceId) -> Self {
        Self {
            id,
            params: VoiceParams::default(),
            bounds_rect: Rect::from_x_y_w_h(0.0, 0.0, 0.0, 0.0),
            emitters: Vec::new(),
            wind_circles: HashMap::new(),
            wind_circle_idx: 0,
        }
    }

    fn issue_wind_circle_idx(&mut self) -> usize {
        let idx = self.wind_circle_idx;
        self.wind_circle_idx += 1;
        idx
    }

    pub fn begin_drone(
        &mut self,
        config: &DroneConfig,
        default_particle_color: Rgb,
        default_spawn_rate: f32,
        id_generator: &mut IdGenerator,
    ) {
        // Apply defaults for create operations
        let (default_center_x, default_center_y) = match config.voice {
            1 => (-1280.0, 0.0),
            4 => (1280.0, 0.0),
            _ => (0.0, 0.0),
        };

        // Extract config or use defaults
        let brightness = config.brightness.unwrap_or(0.7);
        let volume = config.volume.unwrap_or(0.5);
        let gravity = config.gravity.unwrap_or(0.0);
        let force = config.force.unwrap_or(10.0);
        let feedback = config.feedback.unwrap_or(0.0);
        let outer_radius = config.outer_radius.unwrap_or(800.0);
        let inner_radius = config.inner_radius.unwrap_or(200.0);
        let center_x = config.center_x.unwrap_or(default_center_x);
        let center_y = config.center_y.unwrap_or(default_center_y);
        let noise = config.noise.unwrap_or(0.0);
        let vibration = config.vibration.unwrap_or(0.0);

        // WindCircle creation
        let center = vec2(center_x, center_y);
        let circle = WindCircle::new(
            self.issue_wind_circle_idx(),
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

        let emitter_left = LinearEmitter::new(
            id_generator.generate(),
            self.id,
            self.bounds_rect.top_left(),
            self.bounds_rect.mid_left(),
            EmitDirection::East,
            default_spawn_rate,
        );

        let emitter_right = LinearEmitter::new(
            id_generator.generate(),
            self.id,
            self.bounds_rect.mid_right(),
            self.bounds_rect.bottom_right(),
            EmitDirection::West,
            default_spawn_rate,
        );

        /*
        let emitter_center = PointEmitter::new(
            id_generator.generate(),
            voice,
            mask.origin,
            self.global_max_spawn_rate,
            spawn_rate_factor,
        );
         */

        let fullscreen_rect = Rect::from_x_y_w_h(0.0, 0.0, 3840.0, 2160.0);
        let emitter_full = FullScreenRandomEmitter::new(
            id_generator.generate(),
            self.id,
            fullscreen_rect,
            default_spawn_rate,
        );

        // Add the emitters
        self.emitters.push(Box::new(emitter_left)); //emitter_left);
        self.emitters.push(Box::new(emitter_right));
        //self.emitters.push(Box::new(emitter_center));
        self.emitters.push(Box::new(emitter_full));

        // Set the particle system params
        self.set_color_limit(default_particle_color);
        self.set_alpha_limit(brightness);
        self.set_volume(volume);
        self.set_feedback(feedback);
        self.set_vibration(vibration);
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

    /********** Wind Circle methods ********************* */

    /// Add a WindCircle to this Voice
    pub fn add_wind_circle(&mut self, circle: WindCircle) {
        println!("{}: Added wind circle {}", self.id, circle.id);
        self.wind_circles.insert(circle.id, circle);
        self.bounds_rect = self.calculate_bounds();
    }

    /// Set the outer radius of a WindCircle
    pub fn set_circle_outer_radius(&mut self, id: usize, value: f32) {
        if let Some(circle) = self.wind_circles.get_mut(&id) {
            circle.params_mut().set_outer_radius(value);
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
