use crate::{
    commands::Command,
    forces::WindField,
    groups::{Rhythm, Voice, VoiceId},
    osc::OscSender,
    rendering::GpuSegmentBuffer,
    services::sequencer::SequencerService,
    terminals::commands::rhythm::RhythmParamModification,
    view::RhythmView,
};
use rand::rngs::ThreadRng;
use std::collections::HashMap;

/// ExecutionContext provides command handlers with access to application state
/// without requiring full Model ownership. This enables:
/// - Independent testing of command handlers with mock contexts
/// - Clear separation between state (Model) and behavior (CommandEngine)
/// - Explicit dependencies for each command handler
pub trait ExecutionContext {
    // Logging
    fn log_command(&mut self, command: &Command);

    // Voice state access
    fn has_voice(&self, voice_id: VoiceId) -> bool;
    fn get_voice(&self, voice_id: VoiceId) -> Option<&Voice>;
    fn get_voice_mut(&mut self, voice_id: VoiceId) -> Option<&mut Voice>;
    fn insert_voice(&mut self, voice_id: VoiceId, voice: Voice);
    fn remove_voice(&mut self, voice_id: VoiceId) -> Option<Voice>;
    fn voices(&self) -> &HashMap<VoiceId, Voice>;
    fn voices_mut(&mut self) -> &mut HashMap<VoiceId, Voice>;

    // Rhythm state access
    fn has_rhythm(&self, voice_id: VoiceId) -> bool;
    fn get_rhythm(&self, voice_id: VoiceId) -> Option<&Rhythm>;
    fn get_rhythm_mut(&mut self, voice_id: VoiceId) -> Option<&mut Rhythm>;
    fn insert_rhythm(&mut self, voice_id: VoiceId, rhythm: Rhythm);
    fn remove_rhythm(&mut self, voice_id: VoiceId) -> Option<Rhythm>;
    fn rhythms(&self) -> &HashMap<VoiceId, Rhythm>;
    fn rhythms_mut(&mut self) -> &mut HashMap<VoiceId, Rhythm>;

    // Rhythm view access
    fn rhythm_view(&self) -> &RhythmView;
    fn rhythm_view_mut(&mut self) -> &mut RhythmView;

    // Wind field access
    fn wind_field(&mut self) -> &mut WindField;

    // Particle system defaults (for drone creation)
    fn default_particle_color(&self) -> nannou::color::Rgb;
    fn global_max_spawn_rate(&self) -> f32;

    // GPU segment buffer access
    fn get_segment_buffer(&self, voice_id: VoiceId) -> Option<&GpuSegmentBuffer>;
    fn insert_segment_buffer(&mut self, voice_id: VoiceId, buffer: GpuSegmentBuffer);
    fn remove_segment_buffer(&mut self, voice_id: VoiceId) -> Option<GpuSegmentBuffer>;

    // Sequencer service access
    fn sequencer_service(&mut self) -> &mut SequencerService;

    // OSC communication
    fn osc_send(&mut self) -> &mut OscSender;

    // Random number generator
    fn rng(&mut self) -> &mut ThreadRng;

    // Command queue (for deferred command generation)
    fn queue_command(&mut self, command: Command);

    // Validation
    fn validate_voice_exists(&self, voice_id: VoiceId) -> bool;
    fn validate_rhythm_exists(&self, voice_id: VoiceId) -> bool;
    fn validate_circle_exists(&self, voice_id: VoiceId, circle_id: usize) -> bool;

    // Composite operations that need multiple mutable borrows
    fn remove_circle_from_voice(&mut self, voice_id: VoiceId, circle_id: usize) -> bool;
    fn remove_all_circles_from_voice(&mut self, voice_id: VoiceId);
    fn update_rhythm_sequencer(&mut self, voice_id: VoiceId);
    fn rhythm_reroll_wings(&mut self, voice_id: VoiceId);
    fn rhythm_add_wings(&mut self, voice_id: VoiceId, count: usize);
    fn rhythm_stop_sequencer(&mut self, voice_id: VoiceId);
    fn rhythm_modify_all_slots_length(
        &mut self,
        voice_id: VoiceId,
        modification: RhythmParamModification,
    );
    fn rhythm_modify_all_slots_velocity(
        &mut self,
        voice_id: VoiceId,
        modification: RhythmParamModification,
    );
    fn rhythm_modify_all_slots_cutoff(
        &mut self,
        voice_id: VoiceId,
        modification: RhythmParamModification,
    );
}
