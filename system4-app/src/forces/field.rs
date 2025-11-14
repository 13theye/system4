/// src/force/field.rs
///
/// Force field for field-based forces (app-level wrapper)
use nannou::prelude::*;
use std::collections::HashMap;

use crate::{
    groups::{Voice, VoiceId},
    physics_ext::to_core_vec2,
};

// Re-export core ForceFields
pub use system4_core::physics::forces::ForceFields;

/// Extension trait for app-specific ForceFields methods
pub trait ForceFieldsExt {
    /// Update ForceField with all Voices' WindCircles (app-level convenience method)
    fn update(&mut self, voices: &mut HashMap<VoiceId, Voice>, rng: &mut rand::rngs::ThreadRng);
}

impl ForceFieldsExt for ForceFields {
    fn update(&mut self, voices: &mut HashMap<VoiceId, Voice>, rng: &mut rand::rngs::ThreadRng) {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        use system4_core::physics::VoiceId as CoreVoiceId;

        // Helper to create hash from voice_id and circle_id
        fn hash_voice_circle(voice_id: CoreVoiceId, circle_id: usize) -> u64 {
            let mut hasher = DefaultHasher::new();
            (voice_id, circle_id).hash(&mut hasher);
            hasher.finish()
        }

        // Use callback pattern to update circles and collect noise values
        self.update_with_circles(
            |wind_field| {
                let mut circle_noise_values = HashMap::new();

                for voice in voices.values_mut() {
                    let core_voice_id = voice.id.to_core();
                    for circle in voice.wind_circles.values_mut() {
                        circle.update(wind_field);
                        let hash_key = hash_voice_circle(core_voice_id, circle.id);
                        circle_noise_values.insert(hash_key, circle.params().noise);
                    }
                }

                circle_noise_values
            },
            rng,
        );
    }
}

/// Helper to create ForceFields from Nannou types
pub fn new_force_fields(origin: Vec2, bounds_size: Vec2, grid_cols: usize, grid_rows: usize) -> ForceFields {
    ForceFields::new(
        to_core_vec2(origin),
        to_core_vec2(bounds_size),
        grid_cols,
        grid_rows,
    )
}
