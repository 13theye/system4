use crate::{
    groups::{Voice, VoiceId},
    rendering::GpuSegmentBuffer,
};
use std::collections::HashMap;

/// VoiceManager handles all voice-related state and operations.
/// This includes:
/// - Voice lifecycle (creation, removal)
/// - Voice state access (mutable and immutable)
/// - GPU segment buffer management (tied to voice lifecycle)
/// - Wind circle operations that need coordinated access to wind field
pub struct VoiceManager {
    voices: HashMap<VoiceId, Voice>,
    gpu_segment_buffers: HashMap<VoiceId, GpuSegmentBuffer>,
}

impl VoiceManager {
    pub fn new() -> Self {
        Self {
            voices: HashMap::new(),
            gpu_segment_buffers: HashMap::new(),
        }
    }

    // Voice state access
    pub fn has_voice(&self, voice_id: VoiceId) -> bool {
        self.voices.contains_key(&voice_id)
    }

    pub fn get_voice(&self, voice_id: VoiceId) -> Option<&Voice> {
        self.voices.get(&voice_id)
    }

    pub fn get_voice_mut(&mut self, voice_id: VoiceId) -> Option<&mut Voice> {
        self.voices.get_mut(&voice_id)
    }

    pub fn insert_voice(&mut self, voice_id: VoiceId, voice: Voice) {
        self.voices.insert(voice_id, voice);
    }

    pub fn remove_voice(&mut self, voice_id: VoiceId) -> Option<Voice> {
        self.voices.remove(&voice_id)
    }

    pub fn voices(&self) -> &HashMap<VoiceId, Voice> {
        &self.voices
    }

    pub fn voices_mut(&mut self) -> &mut HashMap<VoiceId, Voice> {
        &mut self.voices
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
    pub fn validate_voice_exists(&self, voice_id: VoiceId) -> bool {
        self.voices.contains_key(&voice_id)
    }

    pub fn validate_circle_exists(&self, voice_id: VoiceId, circle_id: usize) -> bool {
        self.voices
            .get(&voice_id)
            .map(|voice| voice.wind_circles.contains_key(&circle_id))
            .unwrap_or(false)
    }

    // Composite operations that need coordinated access to wind field
    /// Remove a specific circle from a voice's wind circles
    /// This requires coordinated access to both the wind field and the voice
    pub fn remove_circle_from_voice(&mut self, voice_id: VoiceId, circle_id: usize) -> bool {
        if let Some(voice) = self.voices.get_mut(&voice_id) {
            if voice.wind_circles.get_mut(&circle_id).is_some() {
                voice.remove_wind_circle(circle_id);
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Remove all circles from a voice's wind circles
    /// This requires coordinated access to both the wind field and the voice
    pub fn remove_all_circles_from_voice(&mut self, voice_id: VoiceId) {
        if let Some(voice) = self.voices.get_mut(&voice_id) {
            voice.remove_all_circles();
        }
    }
}

impl Default for VoiceManager {
    fn default() -> Self {
        Self::new()
    }
}
