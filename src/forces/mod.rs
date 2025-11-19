/// src/force/mod.rs
///
/// Modules defining the behavior of forces
pub mod field;
pub use field::{CellIdx, ForceFields};

pub mod force;
pub use force::Force;

//pub mod wind;
//pub use wind::{WindCircle, WindCircleParams, WindField};

pub mod wind_new;
pub use wind_new::WindField;

pub mod wind_circle;
pub use wind_circle::{WindCircle, WindCircleParams};
