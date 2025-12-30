/// src/force/mod.rs
///
/// Modules defining the behavior of forces
pub mod field;
pub use field::{CellIdx, ForceFields};

pub mod force_type;
pub use force_type::ForceType;

//pub mod wind;
//pub use wind::{WindCircle, WindCircleParams, WindField};

pub mod wind_new;
pub use wind_new::WindField;

pub mod wind_circle;
pub use wind_circle::{WindCircle, WindCircleParams};
