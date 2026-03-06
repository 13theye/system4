use crate::{
    groups::{Drone, VoiceId},
    rendering::GpuSegmentBuffer,
};
use std::collections::HashMap;

/// DroneManager handles all drone-related state and operations.
/// This includes:
/// - Drone lifecycle (creation, removal)
/// - Drone state access (mutable and immutable)
/// - GPU segment buffer management (tied to drone lifecycle)
/// - Wind circle operations that need coordinated access to wind field
pub struct DroneManager {
    gpu_segment_buffers: HashMap<VoiceId, GpuSegmentBuffer>,
    voice_particle_limit: u32,
}

impl DroneManager {
    pub fn init(voice_particle_limit: u32) -> Self {
        Self {
            gpu_segment_buffers: HashMap::new(),
            voice_particle_limit,
        }
    }

    pub fn drone_particle_limit(&self) -> u32 {
        self.voice_particle_limit
    }

    // GPU segment buffer access
    pub fn get_segment_buffer(&self, voice_id: VoiceId) -> Option<&GpuSegmentBuffer> {
        self.gpu_segment_buffers.get(&voice_id)
    }

    pub fn insert_segment_buffer(&mut self, voice_id: VoiceId, buffer: GpuSegmentBuffer) {
        self.gpu_segment_buffers.insert(voice_id, buffer);
    }

    pub fn remove_segment_buffer(&mut self, voice_id: VoiceId) -> Option<GpuSegmentBuffer> {
        self.gpu_segment_buffers.remove(&voice_id)
    }

    pub fn segment_buffers(&self) -> &HashMap<VoiceId, GpuSegmentBuffer> {
        &self.gpu_segment_buffers
    }

    pub fn segment_buffers_mut(&mut self) -> &mut HashMap<VoiceId, GpuSegmentBuffer> {
        &mut self.gpu_segment_buffers
    }

    // Validation
    pub fn validate_circle_exists_for_drone(&self, drone: Drone, circle_id: usize) -> bool {
        drone.wind_circle_formations.contains_key(&circle_id)
    }

    // Composite operations that need coordinated access to wind field
    /// Remove a specific circle from a voice's wind circles
    /// This requires coordinated access to both the wind field and the voice
    pub fn remove_circle_from_drone(&self, drone: &mut Drone, circle_id: usize) -> bool {
        if drone.wind_circle_formations.get_mut(&circle_id).is_some() {
            drone.remove_wind_circle(circle_id);
            true
        } else {
            false
        }
    }

    /// Remove all circles from a voice's wind circles
    /// This requires coordinated access to both the wind field and the voice
    pub fn remove_all_circles_from_drone(&self, drone: &mut Drone) {
        drone.remove_all_circles();
    }
}
