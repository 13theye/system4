use crate::{
    groups::{Rhythm, RhythmParams, VoiceId},
    managers::AIRhythm,
    sequencer::SequencerService,
    settings::OpenAIServiceConfig,
    terminals::commands::rhythm::{RhythmConfig, RhythmParamModification},
    view::rhythm::{RhythmFormationType, RhythmView},
};
use rand::rngs::ThreadRng;
use std::collections::HashMap;
use std::time::Instant;

/// RhythmManager handles all rhythm-related state and operations.
/// This includes:
/// - Rhythm lifecycle (creation, removal)
/// - Rhythm state access (mutable and immutable)
/// - Composite operations requiring coordinated access to sequencer service and RNG
pub struct RhythmManager {
    rhythms: HashMap<VoiceId, Rhythm>,
    ai_rhythm: AIRhythm,
}

impl RhythmManager {
    pub fn new(config: &OpenAIServiceConfig) -> Self {
        Self {
            rhythms: HashMap::new(),
            ai_rhythm: AIRhythm::new(config),
        }
    }

    /// Send the current rhythm state for Voice1 to the AI service.
    ///
    /// The AI-generated rhythm will be applied to Voice2.
    pub fn request_ai_rhythm(&mut self) {
        let sample_voice = VoiceId::Voice1;
        let target_voice = VoiceId::Voice2;

        let Some(current_rhythm) = self.rhythms.get(&sample_voice) else {
            println!(
                "RhythmManager: (warning) no rhythm found for {:?}; skipping AI send",
                sample_voice
            );
            return;
        };

        println!(
            "RhythmManager: sending rhythm {:?} from {:?} to AI for target {:?}",
            current_rhythm.as_string_representation(),
            sample_voice,
            target_voice
        );

        // Delegate AI request construction and sending to AIRhythm.
        self.ai_rhythm
            .request_rhythm_for_voice(current_rhythm.get_params(), target_voice);
    }

    /// Poll the AI service for completed responses, extract the first valid
    /// rhythm pattern (there should only be one), and apply it to the voice that initiated the request.
    pub fn update_ai(
        &mut self,
        now: Instant,
        sequencer_service: &mut SequencerService,
        rhythm_view: &mut RhythmView,
        rng: &mut ThreadRng,
    ) {
        let Some(results) = self.ai_rhythm.poll_stream() else {
            return;
        };

        if results.is_empty() {
            return;
        }

        for result in results {
            self.apply_rhythm_params_to_voice(
                result.target_voice,
                result.params,
                now,
                sequencer_service,
                rhythm_view,
                rng,
            );
        }
    }

    /// Expose current AI reasoning text for UI rendering.
    pub fn current_ai_reasoning_text(&self) -> Option<&str> {
        self.ai_rhythm.current_reasoning_text()
    }

    /// Ensure a Rhythm exists for the target voice, then apply AI-derived
    /// `RhythmParams` and keep sequencer and view in sync.
    fn apply_rhythm_params_to_voice(
        &mut self,
        voice_id: VoiceId,
        rhythm_params: RhythmParams,
        now: Instant,
        sequencer_service: &mut SequencerService,
        rhythm_view: &mut RhythmView,
        rng: &mut ThreadRng,
    ) {
        use std::collections::hash_map::Entry;

        let rhythm = match self.rhythms.entry(voice_id) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                let config = RhythmConfig::get_defaults_for_voice(voice_id);
                let resolved = config.merge_with_defaults();
                let params = resolved.to_rhythm_params();
                let mut rhythm = Rhythm::new_with_params(voice_id, params);

                // Initialize default slots and wings so the rhythm is fully
                // functional before we override wings via the AI pattern.
                rhythm.initialize_slots(rng);
                rhythm.randomize_wings(rng);

                // Hook up sequencer and callbacks.
                rhythm.add_sequencer(sequencer_service);
                if let Some(data_rx) = sequencer_service.get_data_rx(voice_id) {
                    rhythm.set_sequencer_data_rx(data_rx);
                }

                // Create a default formation for this voice.
                let radius = if voice_id == VoiceId::Voice1 {
                    800.0
                } else {
                    450.0
                };

                rhythm_view.add_formation(
                    voice_id,
                    RhythmFormationType::Circle { radius },
                    rhythm.get_params(),
                    now,
                );

                entry.insert(rhythm)
            }
        };

        println!(
            "RhythmManager: applying AI rhythm {} to {:?}",
            rhythm_params.as_test_ai_rhythm(),
            voice_id
        );

        rhythm.set_params(rhythm_params);
        rhythm.update_sequencer(sequencer_service);
        rhythm_view.reinitialize_formation(voice_id, rhythm.get_params(), now);

        // Schedule the target voice to start when Voice1 hits slot 0 on the
        // next whole-note boundary. This keeps both voices time- and
        // sequence-aligned without restarting Voice1.
        sequencer_service.sync_start_to_voice(voice_id, VoiceId::Voice1);
    }

    // Rhythm state access
    pub fn has_rhythm(&self, voice_id: VoiceId) -> bool {
        self.rhythms.contains_key(&voice_id)
    }

    pub fn get_rhythm(&self, voice_id: VoiceId) -> Option<&Rhythm> {
        self.rhythms.get(&voice_id)
    }

    pub fn get_rhythm_mut(&mut self, voice_id: VoiceId) -> Option<&mut Rhythm> {
        self.rhythms.get_mut(&voice_id)
    }

    pub fn insert_rhythm(&mut self, voice_id: VoiceId, rhythm: Rhythm) {
        self.rhythms.insert(voice_id, rhythm);
    }

    pub fn remove_rhythm(&mut self, voice_id: VoiceId) -> Option<Rhythm> {
        self.rhythms.remove(&voice_id)
    }

    pub fn rhythms(&self) -> &HashMap<VoiceId, Rhythm> {
        &self.rhythms
    }

    pub fn rhythms_mut(&mut self) -> &mut HashMap<VoiceId, Rhythm> {
        &mut self.rhythms
    }

    // Validation
    pub fn validate_rhythm_exists(&self, voice_id: VoiceId) -> bool {
        self.rhythms.contains_key(&voice_id)
    }

    // Composite operations requiring coordinated access to services
    /// Update the sequencer for a rhythm
    /// Requires coordinated access to rhythm and sequencer service
    pub fn update_rhythm_sequencer(
        &self,
        voice_id: VoiceId,
        sequencer_service: &mut SequencerService,
    ) {
        if let Some(rhythm) = self.rhythms.get(&voice_id) {
            rhythm.update_sequencer(sequencer_service);
        }
    }

    /// Reroll wings for a rhythm
    /// Requires coordinated access to rhythm, RNG, and sequencer service
    pub fn rhythm_reroll_wings(
        &mut self,
        voice_id: VoiceId,
        rng: &mut ThreadRng,
        sequencer_service: &mut SequencerService,
    ) {
        if let Some(rhythm) = self.rhythms.get_mut(&voice_id) {
            rhythm.reroll_wings(rng, sequencer_service);
        }
    }

    /// Add wings to a rhythm
    /// Requires coordinated access to rhythm and RNG
    pub fn rhythm_add_wings(&mut self, voice_id: VoiceId, count: usize, rng: &mut ThreadRng) {
        if let Some(rhythm) = self.rhythms.get_mut(&voice_id) {
            rhythm.add_wings(count, rng);
        }
    }

    /// Stop the sequencer for a rhythm
    /// Requires coordinated access to rhythm and sequencer service
    pub fn rhythm_stop_sequencer(
        &mut self,
        voice_id: VoiceId,
        sequencer_service: &mut SequencerService,
    ) {
        if let Some(rhythm) = self.rhythms.get_mut(&voice_id) {
            rhythm.stop_sequencer(sequencer_service);
        }
    }

    /// Modify all slot lengths for a rhythm
    /// Requires coordinated access to rhythm and RNG
    pub fn rhythm_modify_all_slots_length(
        &mut self,
        voice_id: VoiceId,
        modification: RhythmParamModification,
        rng: &mut ThreadRng,
    ) {
        if let Some(rhythm) = self.rhythms.get_mut(&voice_id) {
            rhythm.modify_all_slots_length(modification, rng);
        }
    }

    /// Modify all slot velocities for a rhythm
    /// Requires coordinated access to rhythm and RNG
    pub fn rhythm_modify_all_slots_velocity(
        &mut self,
        voice_id: VoiceId,
        modification: RhythmParamModification,
        rng: &mut ThreadRng,
    ) {
        if let Some(rhythm) = self.rhythms.get_mut(&voice_id) {
            rhythm.modify_all_slots_velocity(modification, rng);
        }
    }

    /// Modify all slot cutoffs for a rhythm
    /// Requires coordinated access to rhythm and RNG
    pub fn rhythm_modify_all_slots_cutoff(
        &mut self,
        voice_id: VoiceId,
        modification: RhythmParamModification,
        rng: &mut ThreadRng,
    ) {
        if let Some(rhythm) = self.rhythms.get_mut(&voice_id) {
            rhythm.modify_all_slots_cutoff(modification, rng);
        }
    }
}
