// src/osc/gpu_osc_mapper.rs
//
// Efficient OSC-to-GPU parameter mapping system
// Batches parameter changes and minimizes GPU buffer uploads

use std::collections::{HashMap, HashSet};
use crate::view::Voice;

/// Tracks which voice parameters have changed and need GPU sync
#[derive(Default)]
pub struct OscGpuMapper {
    // Track which voice groups need parameter updates
    dirty_voices: HashSet<Voice>,
    
    // Batch parameter changes for efficiency
    pending_updates: HashMap<Voice, VoiceParameterUpdate>,
    
    // Performance metrics
    sync_count: u32,
    batch_count: u32,
}

/// Represents a batched parameter update for a voice
#[derive(Default, Clone)]
pub struct VoiceParameterUpdate {
    pub alpha_multiplier: Option<f32>,
    pub size_multiplier: Option<f32>,
    pub color_tint: Option<[f32; 3]>,
    pub trail_amount: Option<f32>,
    pub physics_scale: Option<f32>,
    pub spawn_rate_factor: Option<f32>,
    pub particle_limit: Option<u32>,
    pub is_visible: Option<bool>,
    pub is_spawning: Option<bool>,
}

impl OscGpuMapper {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Mark a voice as needing parameter sync
    pub fn mark_voice_dirty(&mut self, voice: Voice) {
        self.dirty_voices.insert(voice);
    }
    
    /// Add a parameter change to the batch for a voice
    pub fn add_parameter_change(&mut self, voice: Voice, update: VoiceParameterUpdate) {
        self.dirty_voices.insert(voice);
        
        // Merge with existing pending updates
        let pending = self.pending_updates.entry(voice).or_default();
        if update.alpha_multiplier.is_some() {
            pending.alpha_multiplier = update.alpha_multiplier;
        }
        if update.size_multiplier.is_some() {
            pending.size_multiplier = update.size_multiplier;
        }
        if update.color_tint.is_some() {
            pending.color_tint = update.color_tint;
        }
        if update.trail_amount.is_some() {
            pending.trail_amount = update.trail_amount;
        }
        if update.physics_scale.is_some() {
            pending.physics_scale = update.physics_scale;
        }
        if update.spawn_rate_factor.is_some() {
            pending.spawn_rate_factor = update.spawn_rate_factor;
        }
        if update.particle_limit.is_some() {
            pending.particle_limit = update.particle_limit;
        }
        if update.is_visible.is_some() {
            pending.is_visible = update.is_visible;
        }
        if update.is_spawning.is_some() {
            pending.is_spawning = update.is_spawning;
        }
    }
    
    /// Apply all pending parameter changes to a particle system
    pub fn apply_pending_changes<F>(&mut self, mut apply_fn: F) 
    where 
        F: FnMut(Voice, &VoiceParameterUpdate)
    {
        for (voice, update) in self.pending_updates.drain() {
            apply_fn(voice, &update);
            self.batch_count += 1;
        }
        
        self.dirty_voices.clear();
        self.sync_count += 1;
    }
    
    /// Get all voices that need parameter updates
    pub fn get_dirty_voices(&self) -> &HashSet<Voice> {
        &self.dirty_voices
    }
    
    /// Check if there are pending parameter changes
    pub fn has_pending_changes(&self) -> bool {
        !self.pending_updates.is_empty()
    }
    
    /// Get the number of voices with pending changes
    pub fn pending_voice_count(&self) -> usize {
        self.pending_updates.len()
    }
    
    /// Clear all pending changes (use when switching systems)
    pub fn clear_pending(&mut self) {
        self.pending_updates.clear();
        self.dirty_voices.clear();
    }
    
    /// Get performance statistics
    pub fn get_stats(&self) -> (u32, u32, f32) {
        let avg_batch_size = if self.sync_count > 0 {
            self.batch_count as f32 / self.sync_count as f32
        } else {
            0.0
        };
        (self.sync_count, self.batch_count, avg_batch_size)
    }
    
    /// Reset performance counters
    pub fn reset_stats(&mut self) {
        self.sync_count = 0;
        self.batch_count = 0;
    }
}

/// Helper functions for creating parameter updates from OSC commands
impl VoiceParameterUpdate {
    pub fn alpha(alpha: f32) -> Self {
        Self {
            alpha_multiplier: Some(alpha),
            ..Default::default()
        }
    }
    
    pub fn size_multiplier(size: f32) -> Self {
        Self {
            size_multiplier: Some(size),
            ..Default::default()
        }
    }
    
    pub fn color_tint(r: f32, g: f32, b: f32) -> Self {
        Self {
            color_tint: Some([r, g, b]),
            ..Default::default()
        }
    }
    
    pub fn trail(trail: f32) -> Self {
        Self {
            trail_amount: Some(trail),
            ..Default::default()
        }
    }
    
    pub fn physics_scale(scale: f32) -> Self {
        Self {
            physics_scale: Some(scale),
            ..Default::default()
        }
    }
    
    pub fn spawn_rate_factor(factor: f32) -> Self {
        Self {
            spawn_rate_factor: Some(factor),
            ..Default::default()
        }
    }
    
    pub fn visible(visible: bool) -> Self {
        Self {
            is_visible: Some(visible),
            ..Default::default()
        }
    }
    
    pub fn spawning(spawning: bool) -> Self {
        Self {
            is_spawning: Some(spawning),
            ..Default::default()
        }
    }
}