/// src/force/mod.rs
///
/// Modules defining the behavior of forces
pub mod field;
pub use field::{CellIdx, ForceFields};

pub mod force;
pub use force::Force;

pub mod wind;
pub use wind::{Wind, WindCircle, WindCircleParams, WindField};
