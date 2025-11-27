use crate::{
    groups::{Rhythm, RhythmParams, VoiceId},
    managers::AIRhythm,
    services::{
        openai::schema::{
            response::{OpenAIOutputContent, OpenAIOutputItem},
            RhythmObject,
        },
        sequencer::SequencerService,
    },
    settings::OpenAIServiceConfig,
    terminals::commands::rhythm::{RhythmConfig, RhythmParamModification},
    view::rhythm_view::RhythmView,
    view::RhythmFormationType,
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
    /// Voice for which we most recently sent an AI rhythm request.
    /// We assume a single in-flight AI request at a time.
    pending_ai_voice: Option<VoiceId>,
}

impl RhythmManager {
    pub fn new(config: &OpenAIServiceConfig) -> Self {
        Self {
            rhythms: HashMap::new(),
            ai_rhythm: AIRhythm::new(config),
            pending_ai_voice: None,
        }
    }

    /************ AI Rhythm *************************** */
    /// Send the current rhythm state for Voice1 to the AI service.
    ///
    /// The AI-generated rhythm will be applied to Voice2.
    pub fn send_to_ai(&mut self) {
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

        let rhythm_object = current_rhythm.as_serializable_object();
        println!("RhythmManager: sending object: \n{:#?}", rhythm_object);

        // Record which voice the AI result should be applied to.
        self.pending_ai_voice = Some(target_voice);
        self.ai_rhythm.send_openai(rhythm_object);
    }

    /// Poll the AI service for completed responses, extract the first valid
    /// rhythm pattern, and apply it to the voice that initiated the request.
    pub fn poll_ai(
        &mut self,
        now: Instant,
        sequencer_service: &mut SequencerService,
        rhythm_view: &mut RhythmView,
        rng: &mut ThreadRng,
    ) {
        let output_items = self.ai_rhythm.poll_openai();
        if output_items.is_empty() {
            return;
        }

        // Determine which voice this AI result belongs to.
        let Some(voice_id) = self.pending_ai_voice.take() else {
            println!(
                "RhythmManager: (warning) received AI rhythm text but no pending voice; ignoring"
            );
            return;
        };

        // Find the first instance of the Message type of OpenAIOutputItem
        let mut rhythm_params: Option<RhythmParams> = None;
        for output_item in &output_items {
            print_output_item(output_item);
            if let Some(pattern) = extract_rhythm_params(output_item) {
                rhythm_params = Some(pattern);
                break;
            }
        }

        let Some(rhythm_params) = rhythm_params else {
            println!(
                "RhythmManager: AI response for {:?} did not contain a valid RhythmParams: {:#?}",
                voice_id, output_items
            );
            return;
        };

        println!(
            "RhythmManager: applying AI rhythm {} to {:?}",
            &rhythm_params.as_test_ai_rhythm(),
            voice_id
        );

        // Ensure a Rhythm exists for the target voice; if not, create one
        // with default values before applying the AI pattern.
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

        // Apply the AI-derived pattern, then keep sequencer and view in sync.
        rhythm.set_params(rhythm_params);
        rhythm.update_sequencer(sequencer_service);
        rhythm_view.reinitialize_formation(voice_id, rhythm.get_params(), now);

        // Schedule Voice2 to start when Voice1 hits slot 0 on the next
        // whole-note boundary. This keeps both voices time- and
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

/// Extracts a RhythmParams from an AI response text.
///
/// The OpenAI model sometimes wraps the JSON object in Markdown code fences
/// (e.g. ```json ... ```). This helper is tolerant of such wrappers by
/// extracting the first JSON object found in the text.
fn extract_rhythm_params(output_item: &OpenAIOutputItem) -> Option<RhythmParams> {
    match output_item {
        OpenAIOutputItem::Message(message) => {
            for output_content in &message.content {
                if let OpenAIOutputContent::OutputText { text } = output_content {
                    // Some models return the JSON wrapped in Markdown code
                    // fences (```json ... ```). To be robust, we extract the
                    // substring from the first '{' to the last '}' and attempt
                    // to parse that as JSON.
                    let trimmed = text.trim();

                    let json_candidate =
                        if let (Some(start), Some(end)) = (trimmed.find('{'), trimmed.rfind('}')) {
                            // SAFETY: `start` and `end` are valid byte indices
                            // returned by `find`/`rfind` on the same &str.
                            &trimmed[start..=end]
                        } else {
                            trimmed
                        };

                    match serde_json::from_str::<RhythmObject>(json_candidate) {
                        Ok(object) => {
                            return Some(RhythmParams::from_rhythm_response_object(object));
                        }
                        Err(e) => {
                            println!(
                                "Failed to parse RhythmResponseObject from text. Candidate JSON: '{}'. Error: {}",
                                json_candidate,
                                e
                            );
                        }
                    }
                }
            }
        }
        OpenAIOutputItem::Reasoning(_) => {}
    }

    None
}

fn print_output_item(output_item: &OpenAIOutputItem) {
    match output_item {
        OpenAIOutputItem::Message(message) => {
            println!("RhythmManager: AI output message received");
            for (idx, output_content) in message.content.iter().enumerate() {
                if let OpenAIOutputContent::OutputText { text } = output_content {
                    println!("\n--- OutputText #{idx} raw ---");
                    println!("{}", text); // shows real newlines if they’re actual '\n' chars
                }
            }
        }
        OpenAIOutputItem::Reasoning(reasoning) => {
            println!("RhythmManager: AI output reasoning received");
            for (idx, text) in reasoning.content.iter().enumerate() {
                let inner_text = &text.text;
                println!("\n--- Reasoning Text #{idx} raw ---");
                println!("{}", inner_text); // shows real newlines if they’re actual '\n' chars
            }
        }
    }
}
