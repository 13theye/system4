use super::commands::Command;
use crate::{
    forces::wind_field::WindField,
    groups::{Drone, Rhythm, Voice, VoiceId},
    osc::OscSender,
    sequencer::SequencerService,
    terminals::commands::rhythm::RhythmParamModification,
    view::rhythm::RhythmView,
};
use rand::rngs::ThreadRng;

/// ExecutionContext provides command handlers with access to application state
/// without requiring full Model ownership. This enables:
/// - Independent testing of command handlers with mock contexts
/// - Clear separation between state (Model) and behavior (CommandEngine)
/// - Explicit dependencies for each command handler
pub trait ExecutionContext {
    // Logging
    fn log_command(&mut self, command: &Command);

    // Voice state access
    fn insert_voice(&mut self, voice_id: VoiceId, voice: Voice);
    fn remove_voice(&mut self, voice_id: VoiceId) -> Option<Voice>;

    // Drone state access
    fn has_drone(&self, voice_id: VoiceId) -> bool;
    fn get_drone(&self, voice_id: VoiceId) -> Option<&Drone>;
    fn get_drone_mut(&mut self, voice_id: VoiceId) -> Option<&mut Drone>;
    fn voice_particle_limit(&self) -> u32;

    // Rhythm state access
    fn has_rhythm(&self, voice_id: VoiceId) -> bool;
    fn get_rhythm(&self, voice_id: VoiceId) -> Option<&Rhythm>;
    fn get_rhythm_mut(&mut self, voice_id: VoiceId) -> Option<&mut Rhythm>;

    // Rhythm view access
    fn rhythm_view(&self) -> &RhythmView;
    fn rhythm_view_mut(&mut self) -> &mut RhythmView;

    // Wind field access
    fn wind_field(&mut self) -> &mut WindField;

    // Particle system defaults (for drone creation)
    fn default_particle_color(&self) -> nannou::color::Rgb;
    fn global_max_spawn_rate(&self) -> f32;

    // Sequencer service access
    fn sequencer_service(&mut self) -> &mut SequencerService;

    // OSC communication
    fn osc_send(&mut self) -> &mut OscSender;

    // Random number generator
    fn rng(&mut self) -> &mut ThreadRng;

    // Command queue (for deferred command generation)
    fn queue_command(&mut self, command: Command);

    // Validation
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
