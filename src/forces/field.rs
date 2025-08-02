/// src/force/field.rs
///
/// Force field for field-based forces
use nannou::prelude::*;
use std::collections::BTreeMap;

use crate::{
    forces::{
        wind::{WindCircle, WindCircleParams, WindField},
        Gravity,
    },
    particle::Particle,
    view::Voice,
};

/// The ForceField tracks the forces that are acting on the particles.
/// It provides a coordinate space to align forces to screen locations.
pub struct ForceFields {
    // Force fields
    pub wind_field: WindField,
    pub gravity_field: Vec<Gravity>,

    // Force objects
    pub wind_circles: BTreeMap<usize, WindCircle>,
    pub gravity_sources: Vec<Gravity>,

    // Origin in the World Coordinate Space
    origin: Vec2,
    bounds_size: Vec2,
    grid_cols: usize,
    grid_rows: usize,
    cell_size: Vec2,
}

impl ForceFields {
    pub fn new(origin: Vec2, bounds_size: Vec2, grid_cols: usize, grid_rows: usize) -> Self {
        let cell_size = Vec2::new(
            bounds_size.x / grid_cols as f32,
            bounds_size.y / grid_rows as f32,
        );

        Self {
            wind_field: WindField::new(origin, bounds_size, grid_cols, grid_rows),
            gravity_field: Vec::new(),
            wind_circles: BTreeMap::new(),
            gravity_sources: Vec::new(),
            origin,
            bounds_size,
            grid_cols,
            grid_rows,
            cell_size,
        }
    }

    /// Update all circles in this ForceField
    pub fn update(&mut self, show_forces: bool) {
        for circle in self.wind_circles.values_mut() {
            circle.update(&mut self.wind_field, show_forces);
        }
    }

    /// Update all Winds in this ForceField
    pub fn force_update_all(&mut self) {
        self.wind_field.force_update_all();
    }

    /// Apply all applicable forces to a particle
    pub fn apply_forces_to_particle(&mut self, particle: &mut Particle) {
        // Apply wind
        self.wind_field.apply(particle);

        // Apply gravity
        for gravity_source in &self.gravity_field {
            gravity_source.apply(particle);
        }
    }

    /// Recalculate all applicable forces in this ForceField
    pub fn recalculate_once(&mut self) {
        self.update(true);
    }

    /// Add a WindCircle to this ForceField
    pub fn add_wind_circle(&mut self, circle: WindCircle) {
        println!("Added wind circle {}", circle.id);
        self.wind_circles.entry(circle.id).or_insert(circle);
    }

    /// Add a GravitySource to this ForceField
    pub fn add_gravity_source(&mut self, origin: Vec2, mass: f32) {
        let gravity = Gravity::new(origin, mass);
        self.gravity_field.push(gravity);
    }

    /// Returns a BTreeMap of all WindCircleParms by WindCircle ID
    pub fn get_circle_params_all(&self) -> BTreeMap<usize, WindCircleParams> {
        self.wind_circles
            .iter()
            .map(|(id, circle)| (*id, circle.with_params_read(|params| params.clone())))
            .collect()
    }

    /// Returns a BTreeMap of all WindCircle's WindCircleParams for a given Voice
    pub fn get_circle_params_by_voice(&self, voice: Voice) -> BTreeMap<usize, WindCircleParams> {
        self.wind_circles
            .iter()
            .filter(|(_, circle)| circle.parent_voice == voice)
            .map(|(id, circle)| (*id, circle.with_params_read(|params| params.clone())))
            .collect()
    }

    /// Returns a WindCircleParams for a given WindCircle ID
    pub fn get_circle_params_by_id(&self, id: usize) -> Option<WindCircleParams> {
        self.wind_circles
            .get(&id)
            .map(|circle| circle.with_params_read(|params| params.clone()))
    }

    /// Returns a Vec of all WindCircle IDs for a given Voice
    pub fn get_circle_ids_by_voice(&self, voice: &Voice) -> Vec<usize> {
        self.wind_circles
            .iter()
            .filter(|(_, circle)| circle.parent_voice == *voice)
            .map(|(id, _)| *id)
            .collect()
    }

    /******************* OSC command compatibility methods ********************* */

    /// Remove all WindCircles for a given Voice
    pub fn remove_wind_by_voice(&mut self, voice: &Voice) {
        let circle_ids: Vec<usize> = self.get_circle_ids_by_voice(voice);

        if circle_ids.is_empty() {
            println!("Wind circles not found for {}", voice);
            return;
        }

        for id in circle_ids {
            let Some(circle) = self.wind_circles.get_mut(&id) else {
                return;
            };

            circle.remove_from_field(&mut self.wind_field, true);
            self.wind_circles.remove(&id);
        }
    }

    /// Set the center bias of all WindCircles for a given Voice
    pub fn set_circle_center_bias_by_voice(&mut self, voice: &Voice, bias: f32) {
        let circle_ids: Vec<usize> = self.get_circle_ids_by_voice(voice);

        if circle_ids.is_empty() {
            println!("Wind circles not found for {}", voice);
            return;
        }

        for id in circle_ids {
            let Some(circle) = self.wind_circles.get_mut(&id) else {
                return;
            };

            circle.with_params_write(|p| {
                p.center_bias(bias);
            });
        }
    }

    /// Currently unused. Needs to be reworked if used.
    pub fn set_circle_volume_by_voice(&mut self, voice: &Voice, alpha: f32) {
        let circle_ids: Vec<usize> = self.get_circle_ids_by_voice(voice);

        if circle_ids.is_empty() {
            println!("Wind circles not found for {}", voice);
            return;
        }

        for id in circle_ids {
            let Some(params) = self.get_circle_params_by_id(id) else {
                return;
            };

            let radius = params.outer_radius;
            let new_width = radius * alpha + 200.0;

            self.set_circle_dims(id, radius, new_width);
        }
    }

    /// Set the strength of all WindCircles for a given Voice
    pub fn set_circle_strength_by_voice(&mut self, voice: &Voice, strength: f32) {
        let circle_ids: Vec<usize> = self.get_circle_ids_by_voice(voice);

        if circle_ids.is_empty() {
            println!("Wind circles not found for {}", voice);
            return;
        }

        for id in circle_ids {
            let Some(circle) = self.wind_circles.get_mut(&id) else {
                return;
            };

            circle.with_params_write(|p| {
                p.strength(strength);
            });
        }
    }

    /// Set the outer radius of all WindCircles for a given Voice
    pub fn set_circle_outer_radius_by_voice(&mut self, voice: &Voice, radius: f32) {
        let circle_ids: Vec<usize> = self.get_circle_ids_by_voice(voice);

        if circle_ids.is_empty() {
            println!("Wind circles not found for {}", voice);
            return;
        }

        for id in circle_ids {
            let Some(circle) = self.wind_circles.get_mut(&id) else {
                return;
            };

            circle.with_params_write(|p| {
                p.outer_radius(radius);
            });
        }
    }

    /// Set the inner radius of all WindCircles for a given Voice
    pub fn set_circle_inner_radius_by_voice(&mut self, voice: &Voice, val: f32) {
        let circle_ids: Vec<usize> = self.get_circle_ids_by_voice(voice);

        if circle_ids.is_empty() {
            println!("Wind circles not found for {}", voice);
            return;
        }

        for id in circle_ids {
            let Some(circle) = self.wind_circles.get_mut(&id) else {
                return;
            };

            let outer_radius = circle.with_params_read(|p| p.outer_radius);
            let inner_radius = outer_radius * val;

            circle.with_params_write(|p| {
                p.inner_radius(inner_radius);
            });
        }
    }

    /// Set the outer and inner radius of a WindCircle by width
    pub fn set_circle_dims(&mut self, id: usize, radius: f32, width: f32) {
        if let Some(circle) = self.wind_circles.get_mut(&id) {
            circle.with_params_write(|p| {
                p.outer_radius(radius);
                p.inner_radius(width);
            });
        } else {
            println!("Wind circle {} not found", id);
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct CellIdx {
    pub x: usize,
    pub y: usize,
}
