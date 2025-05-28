// src/force/field.rs
//
// Force field for field-based forces

use nannou::prelude::*;

use crate::{
    forces::{Force, Gravity, WindField},
    particle::Particle,
};

pub struct ForceField {
    // Forces
    pub wind: WindField,
    pub gravity: Vec<Gravity>,

    // Origin in the World Coordinate Space
    origin: Vec2,
    bounds_size: Vec2,
    grid_cols: usize,
    grid_rows: usize,
    cell_size: Vec2,
}

impl ForceField {
    pub fn new(origin: Vec2, bounds_size: Vec2, grid_cols: usize, grid_rows: usize) -> Self {
        let cell_size = Vec2::new(
            bounds_size.x / grid_cols as f32,
            bounds_size.y / grid_rows as f32,
        );

        Self {
            wind: WindField::new(origin, bounds_size, grid_cols, grid_rows),
            gravity: Vec::new(),
            origin,
            bounds_size,
            grid_cols,
            grid_rows,
            cell_size,
        }
    }

    pub fn apply(&self, particle: &mut Particle) {
        // Apply wind
        self.wind.apply(particle);

        // Apply gravity
        for gravity_source in &self.gravity {
            gravity_source.apply(particle);
        }
    }

    pub fn add_gravity_source(&mut self, origin: Vec2, mass: f32) {
        let gravity = Gravity::new(origin, mass);
        self.gravity.push(gravity);
    }
}
