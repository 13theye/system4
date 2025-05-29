// src/force/field.rs
//
// Force field for field-based forces

use nannou::prelude::*;
use std::collections::HashMap;

use crate::{
    forces::{Gravity, WindCircle, WindField},
    particle::Particle,
};

pub struct ForceFields {
    // Force fields
    pub wind_field: WindField,
    pub gravity_field: Vec<Gravity>,

    // Force objects
    pub wind_circles: HashMap<usize, WindCircle>,
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
            wind_circles: HashMap::new(),
            gravity_sources: Vec::new(),
            origin,
            bounds_size,
            grid_cols,
            grid_rows,
            cell_size,
        }
    }

    pub fn update(&mut self) {
        for circle in self.wind_circles.values_mut() {
            circle.update(&mut self.wind_field);
        }
    }

    pub fn apply(&mut self, particle: &mut Particle) {
        // Apply wind
        self.wind_field.apply(particle);

        // Apply gravity
        for gravity_source in &self.gravity_field {
            gravity_source.apply(particle);
        }
    }

    pub fn add_wind_circle(&mut self, circle: WindCircle) {
        self.wind_circles.insert(circle.id, circle);
    }

    pub fn add_gravity_source(&mut self, origin: Vec2, mass: f32) {
        let gravity = Gravity::new(origin, mass);
        self.gravity_field.push(gravity);
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct CellIdx {
    pub x: usize,
    pub y: usize,
}
