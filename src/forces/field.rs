// src/force/field.rs
//
// Force field for field-based forces

use nannou::prelude::*;
use std::collections::BTreeMap;

use crate::{
    forces::{
        wind::{WindCircle, WindCircleParams, WindField},
        Gravity,
    },
    particle::Particle,
};

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

// The ForceField tracks the forces that are acting on the particles.
// It contains a WindField and a GravityField, as well as various
// Force objects like WindCircles, and GravitySources.

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

    pub fn update(&mut self, show_forces: bool) {
        for circle in self.wind_circles.values_mut() {
            circle.update(&mut self.wind_field, show_forces);
        }
    }

    pub fn force_update_all(&mut self) {
        self.wind_field.force_update_all();
    }

    pub fn apply(&mut self, particle: &mut Particle) {
        // Apply wind
        self.wind_field.apply(particle);

        // Apply gravity
        for gravity_source in &self.gravity_field {
            gravity_source.apply(particle);
        }
    }

    pub fn recalculate_once(&mut self) {
        self.update(true);
    }

    pub fn add_wind_circle(&mut self, circle: WindCircle) {
        println!("Added wind circle {}", circle.id);
        self.wind_circles.entry(circle.id).or_insert(circle);
    }

    pub fn add_gravity_source(&mut self, origin: Vec2, mass: f32) {
        let gravity = Gravity::new(origin, mass);
        self.gravity_field.push(gravity);
    }

    pub fn get_circle_params(&self) -> BTreeMap<usize, WindCircleParams> {
        self.wind_circles
            .iter()
            .map(|(id, circle)| (*id, circle.with_params_read(|params| params.clone())))
            .collect()
    }

    /******************* OSC command compatibility methods ********************* */

    pub fn update_wind_circle_center_bias(&mut self, id: usize, bias: f32) {
        if let Some(circle) = self.wind_circles.get_mut(&id) {
            circle.with_params_write(|p| {
                p.center_bias(bias);
            });
        } else {
            println!("Wind circle {} not found", id);
        }
    }

    pub fn update_wind_circle_strength(&mut self, id: usize, strength: f32) {
        if let Some(circle) = self.wind_circles.get_mut(&id) {
            circle.with_params_write(|p| {
                p.strength(strength);
            });
        } else {
            println!("Wind circle {} not found", id);
        }
    }

    pub fn set_circle_dims(&mut self, id: usize, radius: f32, width: f32) {
        if let Some(circle) = self.wind_circles.get_mut(&id) {
            circle.with_params_write(|p| {
                p.radius(radius);
                p.width(width);
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
