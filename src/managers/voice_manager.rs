//! src/managers/voice_manager.rs
//!
//! Top-level gateway to all voice-specific state.

use crate::{
    groups::{Drone, Rhythm, Voice, VoiceId},
    managers::{AIRhythmManager, AiStreamEvent, DroneManager},
    rendering::GpuSegmentBuffer,
    sequencer::SequencerService,
    settings::Settings,
    terminals::commands::rhythm::RhythmParamModification,
    view::rhythm::{RhythmView, RhythmViewUpdateParams},
};

use std::{collections::HashMap, time::Instant};

const AI_VOICE_ID: VoiceId = VoiceId::Voice2;

pub struct VoiceManager {
    // Voice state
    pub voices: HashMap<VoiceId, Voice>,

    // Managers of Drone and AIRhythm specific states
    drone_manager: DroneManager,
    ai_rhythm_manager: AIRhythmManager,
}

impl VoiceManager {
    pub fn init(settings: &Settings) -> Self {
        let voice_particle_limit = settings.particles.per_voice_limit;
        let openai_service_config = &settings.openai_service;

        Self {
            voices: HashMap::new(),
            drone_manager: DroneManager::init(voice_particle_limit),
            ai_rhythm_manager: AIRhythmManager::init(openai_service_config),
        }
    }

    /// Returns `true` if a Voice exists for the given `VoiceId`
    pub fn validate_voice_exists(&self, voice_id: VoiceId) -> bool {
        self.voices.contains_key(&voice_id)
    }

    /// Insert a `Voice` with the key `VoiceId`
    pub fn insert_voice(&mut self, voice_id: VoiceId, voice: Voice) {
        self.voices.insert(voice_id, voice);
    }

    /// Remove a `Voice` with the key `VoiceId`, returning the value if it exists
    pub fn remove_voice(&mut self, voice_id: VoiceId) -> Option<Voice> {
        self.voices.remove(&voice_id)
    }

    /**************** Drone methods ************************* */

    /// Check if a `Voice` exists for a `VoiceId` and is a `Drone`.
    pub fn has_drone(&self, voice_id: VoiceId) -> bool {
        self.voices
            .get(&voice_id)
            .is_some_and(|voice| voice.is_drone())
    }

    /// Attempt to retrieve a `Drone` from a `VoiceId`. Returns `None` if the `VoiceId` does not exist or is not a `Drone`.
    pub fn get_drone(&self, voice_id: VoiceId) -> Option<&Drone> {
        let voice = self.voices.get(&voice_id)?;
        voice.as_drone()
    }

    /// Attempt to retrieve a mutable reference to a `Drone` from a `VoiceId`. Returns `None` if the `VoiceId` does not exist or is not a `Drone`.
    pub fn get_mut_drone(&mut self, voice_id: VoiceId) -> Option<&mut Drone> {
        let voice = self.voices.get_mut(&voice_id)?;
        voice.as_drone_mut()
    }

    /// Return the per-voice particle limit for drones
    pub fn drone_particle_limit(&self) -> u32 {
        self.drone_manager.drone_particle_limit()
    }

    /// Check that a particular `WindCircle` exists for a `VoiceId`
    pub fn voice_has_wind_circle(&self, voice_id: VoiceId, circle_id: usize) -> bool {
        self.get_drone(voice_id)
            .is_some_and(|drone| drone.has_circle_formation(circle_id))
    }

    /// Remove a `WindCircle` from a `VoiceId`
    pub fn remove_circle_from_voice(&mut self, voice_id: VoiceId, circle_id: usize) -> bool {
        let Some(drone) = self.get_mut_drone(voice_id) else {
            return false;
        };
        drone.remove_circle_formation(circle_id);
        true
    }

    /// Remove all `WindCircles` from a `VoiceId`
    pub fn remove_all_circles_from_voice(&mut self, voice_id: VoiceId) {
        let Some(drone) = self.get_mut_drone(voice_id) else {
            return;
        };

        drone.remove_all_circle_formations();
    }

    /// Return a reference to the DroneManager's segment buffer for a `VoiceId`
    pub fn get_segment_buffer(&self, voice_id: VoiceId) -> Option<&GpuSegmentBuffer> {
        self.drone_manager.get_segment_buffer(voice_id)
    }

    /// Update all Drone's masks
    pub fn update_drone_masks(&mut self, now: Instant) {
        for (_, voice) in self.voices.iter_mut() {
            let Some(drone) = voice.as_drone_mut() else {
                continue;
            };

            drone.mask.update_animation(now);
        }
    }

    /**************** Rhythm methods ************************* */

    /// Check if a `Voice` exists for a `VoiceId` and is a `Rhythm`.
    pub fn has_rhythm(&self, voice_id: VoiceId) -> bool {
        self.voices
            .get(&voice_id)
            .is_some_and(|voice| voice.is_rhythm())
    }

    /// Attempt to retrieve a `Rhythm` from a `VoiceId`. Returns `None` if the `VoiceId` does not exist or is not a `Rhythm`.
    pub fn get_rhythm(&self, voice_id: VoiceId) -> Option<&Rhythm> {
        let voice = self.voices.get(&voice_id)?;
        voice.as_rhythm()
    }

    /// Attempt to retrieve a mutable reference to a `Rhythm` from a `VoiceId`. Returns `None` if the `VoiceId` does not exist or is not a `Rhythm`.
    pub fn get_rhythm_mut(&mut self, voice_id: VoiceId) -> Option<&mut Rhythm> {
        let voice = self.voices.get_mut(&voice_id)?;
        voice.as_rhythm_mut()
    }

    /// Apply `rhythm` changes to the sequencer
    pub fn apply_rhythm_to_sequencer(
        &self,
        voice_id: VoiceId,
        sequencer_service: &mut SequencerService,
    ) {
        if let Some(rhythm) = self.get_rhythm(voice_id) {
            rhythm.update_sequencer(sequencer_service);
        }
    }

    pub fn rhythm_reroll_wings(
        &mut self,
        voice_id: VoiceId,
        rng: &mut rand::rngs::ThreadRng,
        sequencer_service: &mut SequencerService,
    ) {
        if let Some(rhythm) = self.get_rhythm_mut(voice_id) {
            rhythm.reroll_wings(rng, sequencer_service);
        }
    }

    pub fn rhythm_add_wings(
        &mut self,
        voice_id: VoiceId,
        count: usize,
        rng: &mut rand::rngs::ThreadRng,
    ) {
        if let Some(rhythm) = self.get_rhythm_mut(voice_id) {
            rhythm.add_wings(count, rng);
        }
    }

    pub fn rhythm_stop_sequencer(
        &mut self,
        voice_id: VoiceId,
        sequencer_service: &mut SequencerService,
    ) {
        if let Some(rhythm) = self.get_rhythm_mut(voice_id) {
            rhythm.stop_sequencer(sequencer_service);
        }
    }

    pub fn rhythm_modify_all_slots_length(
        &mut self,
        voice_id: VoiceId,
        modification: RhythmParamModification,
        rng: &mut rand::rngs::ThreadRng,
    ) {
        if let Some(rhythm) = self.get_rhythm_mut(voice_id) {
            rhythm.modify_all_slots_length(modification, rng);
        }
    }

    pub fn rhythm_modify_all_slots_velocity(
        &mut self,
        voice_id: VoiceId,
        modification: RhythmParamModification,
        rng: &mut rand::rngs::ThreadRng,
    ) {
        if let Some(rhythm) = self.get_rhythm_mut(voice_id) {
            rhythm.modify_all_slots_velocity(modification, rng);
        }
    }

    pub fn rhythm_modify_all_slots_cutoff(
        &mut self,
        voice_id: VoiceId,
        modification: RhythmParamModification,
        rng: &mut rand::rngs::ThreadRng,
    ) {
        if let Some(rhythm) = self.get_rhythm_mut(voice_id) {
            rhythm.modify_all_slots_cutoff(modification, rng);
        }
    }

    /// Update all the rhythm voices and their views. Returns a list of flags to indicate if we are on an active slot
    pub fn update_rhythms(
        &mut self,
        tempo: f64,
        rhythm_view: &mut RhythmView,
        now: std::time::Instant,
    ) -> Vec<bool> {
        let mut events = Vec::new();

        for (voice_id, voice) in self.voices.iter_mut() {
            let Some(rhythm) = voice.as_rhythm_mut() else {
                continue;
            };

            let (current_slot, current_wing) = rhythm.update();
            let rhythm_params = rhythm.get_params();

            // Push a flag if we are on an active slot of the sequence
            // This is part of the prototype for particle-based events for rhythms
            if let Some(current_wing) = current_wing {
                if rhythm_params.wings.contains(&current_wing) {
                    events.push(true);
                }
            }

            let update_params = RhythmViewUpdateParams {
                current_slot,
                current_wing,
                tempo,
                subdivision: rhythm.get_subdivision().to_owned(),
            };

            rhythm_view.update_voice(voice_id, &update_params, now);
        }

        events
    }

    /****************** AIRhythm methods ******************** */

    pub fn current_ai_status_text(&self) -> (Option<&str>, bool) {
        self.ai_rhythm_manager.current_ai_status_text()
    }

    pub fn is_ai_request_pending(&self) -> bool {
        self.ai_rhythm_manager.is_ai_request_pending()
    }

    pub fn request_ai_rhythm_from(&mut self, sample_voice_id: VoiceId) {
        let Some(sample_rhythm) = self.get_rhythm(sample_voice_id) else {
            return;
        };

        let target_voice = AI_VOICE_ID;

        println!(
            "VoiceManager: sending rhythm [{}] from {:?} to AI for target {:?}",
            sample_rhythm.to_rhythm_string(),
            sample_voice_id,
            target_voice
        );

        self.ai_rhythm_manager
            .request_ai_rhythm(sample_rhythm.get_params().clone(), target_voice);
    }

    pub fn update_ai(
        &mut self,
        now: std::time::Instant,
        sequencer_service: &mut SequencerService,
        rhythm_view: &mut RhythmView,
        rng: &mut rand::rngs::ThreadRng,
    ) -> Option<Vec<AiStreamEvent>> {
        use crate::terminals::commands::rhythm::RhythmConfig;

        let (results, events) = self.ai_rhythm_manager.poll_ai();

        let Some(results) = results else {
            return events;
        };

        for result in results {
            let voice_id = result.target_voice;

            // If we don't have a rhythm for this voice, create one.
            if !self.has_rhythm(voice_id) {
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

                rhythm_view.add_formation(voice_id, rhythm.get_params(), now);

                self.insert_voice(voice_id, Voice::new_from_rhythm(rhythm));
            }

            // Retrieve the pre-existing rhythm or the new one if just created
            let Some(rhythm) = self.get_rhythm_mut(voice_id) else {
                println!(
                    "VoiceManager: failed to retrieve rhythm for voice {}",
                    voice_id
                );
                return events;
            };

            // Extract parameters from the AI result
            let rhythm_params = result.params;

            println!(
                "RhythmManager: applying AI rhythm [{}] to {:?}",
                rhythm_params.to_rhythm_string(),
                voice_id
            );

            rhythm.set_params(rhythm_params);
            rhythm.update_sequencer(sequencer_service);
            rhythm_view.reinitialize_formation(voice_id, rhythm.get_params(), now);
            rhythm_view.update_voice_element_radii(voice_id, now);

            // Schedule the target voice to start when Voice1 hits slot 0 on the
            // next whole-note boundary. This keeps both voices time- and
            // sequence-aligned without restarting Voice1.
            sequencer_service.sync_start_to_voice(voice_id, VoiceId::Voice1);
        }

        events
    }
}
