// src/model/mod.rs
//
// The main App Model

pub mod command_builder;
pub mod controller;
pub mod terminal_processor;

use crate::{
    command_engine::context::ExecutionContext,
    groups::{Rhythm, Voice, VoiceId},
    managers::{RhythmManager, VoiceManager},
    model::controller::Command,
    osc::{OscController, OscSender},
    particle::ParticleSystem,
    rendering::{GpuSegmentBuffer, RenderState},
    services::sequencer::SequencerService,
    terminals::commands::rhythm::RhythmParamModification,
    ui::UiState,
    utils::IdGenerator,
    view::RhythmView,
};

use prat::clockservice::ClockService;
use rand::rngs::ThreadRng;

use std::collections::HashMap;

pub struct Model {
    pub particle_system: ParticleSystem,

    // State managers
    pub voice_manager: VoiceManager,
    pub rhythm_manager: RhythmManager,
    pub rhythm_view: RhythmView,

    // Clock and Sequencers
    pub clock: ClockService,
    pub sequencer_service: SequencerService,

    // OSC
    pub osc: OscController,
    pub osc_send: OscSender,
    pub osc_loop: OscSender,

    // Rendering state
    pub render_state: RenderState,

    // UI state
    pub ui_state: UiState,

    // Simple ID counter
    pub id_generator: IdGenerator,

    // Random
    pub rng: ThreadRng,

    // Unified command queue with priority resolution
    pub command_queue: Vec<Command>,
}

impl Drop for Model {
    fn drop(&mut self) {
        println!("\n\nApp terminating, sending kill drone signals...");
        erase_drone(self, 1);
        erase_drone(self, 4);
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

fn erase_drone(model: &mut Model, id: i32) {
    let voice_id = VoiceId::from_i32(id);
    model.kill_voice(voice_id);
    model.particle_system.forces.recalculate_once();
    model.osc_send.send_drone_on_off(id, 0);
}

// ExecutionContext implementation for Model
impl ExecutionContext for Model {
    fn log_command(&mut self, command: &crate::commands::Command) {
        self.ui_state
            .terminal_manager
            .borrow_mut()
            .process_command(command);
    }

    // Voice state access - delegate to voice_manager
    fn has_voice(&self, voice_id: VoiceId) -> bool {
        self.voice_manager.has_voice(voice_id)
    }

    fn get_voice(&self, voice_id: VoiceId) -> Option<&Voice> {
        self.voice_manager.get_voice(voice_id)
    }

    fn get_voice_mut(&mut self, voice_id: VoiceId) -> Option<&mut Voice> {
        self.voice_manager.get_voice_mut(voice_id)
    }

    fn insert_voice(&mut self, voice_id: VoiceId, voice: Voice) {
        self.voice_manager.insert_voice(voice_id, voice);
    }

    fn remove_voice(&mut self, voice_id: VoiceId) -> Option<Voice> {
        self.voice_manager.remove_voice(voice_id)
    }

    fn voices(&self) -> &HashMap<VoiceId, Voice> {
        self.voice_manager.voices()
    }

    fn voices_mut(&mut self) -> &mut HashMap<VoiceId, Voice> {
        self.voice_manager.voices_mut()
    }

    // Rhythm state access - delegate to rhythm_manager
    fn has_rhythm(&self, voice_id: VoiceId) -> bool {
        self.rhythm_manager.has_rhythm(voice_id)
    }

    fn get_rhythm(&self, voice_id: VoiceId) -> Option<&Rhythm> {
        self.rhythm_manager.get_rhythm(voice_id)
    }

    fn get_rhythm_mut(&mut self, voice_id: VoiceId) -> Option<&mut Rhythm> {
        self.rhythm_manager.get_rhythm_mut(voice_id)
    }

    fn insert_rhythm(&mut self, voice_id: VoiceId, rhythm: Rhythm) {
        self.rhythm_manager.insert_rhythm(voice_id, rhythm);
    }

    fn remove_rhythm(&mut self, voice_id: VoiceId) -> Option<Rhythm> {
        self.rhythm_manager.remove_rhythm(voice_id)
    }

    fn rhythms(&self) -> &HashMap<VoiceId, Rhythm> {
        self.rhythm_manager.rhythms()
    }

    fn rhythms_mut(&mut self) -> &mut HashMap<VoiceId, Rhythm> {
        self.rhythm_manager.rhythms_mut()
    }

    // Rhythm view access
    fn rhythm_view(&self) -> &RhythmView {
        &self.rhythm_view
    }

    fn rhythm_view_mut(&mut self) -> &mut RhythmView {
        &mut self.rhythm_view
    }

    // Wind field access
    fn wind_field(&mut self) -> &mut crate::forces::WindField {
        &mut self.particle_system.forces.wind_field
    }

    // Particle system defaults
    fn default_particle_color(&self) -> nannou::color::Rgb {
        self.particle_system.default_particle_color
    }

    fn global_max_spawn_rate(&self) -> f32 {
        self.particle_system.global_max_spawn_rate
    }

    // GPU segment buffer access - delegate to voice_manager
    fn get_segment_buffer(&self, voice_id: VoiceId) -> Option<&GpuSegmentBuffer> {
        self.voice_manager.get_segment_buffer(voice_id)
    }

    fn insert_segment_buffer(
        &mut self,
        voice_id: VoiceId,
        buffer: crate::rendering::GpuSegmentBuffer,
    ) {
        self.voice_manager.insert_segment_buffer(voice_id, buffer);
    }

    fn remove_segment_buffer(
        &mut self,
        voice_id: VoiceId,
    ) -> Option<crate::rendering::GpuSegmentBuffer> {
        self.voice_manager.remove_segment_buffer(voice_id)
    }

    // Sequencer service access
    fn sequencer_service(&mut self) -> &mut SequencerService {
        &mut self.sequencer_service
    }

    // OSC communication
    fn osc_send(&mut self) -> &mut crate::osc::OscSender {
        &mut self.osc_send
    }

    // Random number generator
    fn rng(&mut self) -> &mut rand::rngs::ThreadRng {
        &mut self.rng
    }

    // Command queue
    fn queue_command(&mut self, command: crate::commands::Command) {
        self.command_queue.push(command);
    }

    // Validation - delegate to managers
    fn validate_voice_exists(&self, voice_id: VoiceId) -> bool {
        self.voice_manager.validate_voice_exists(voice_id)
    }

    fn validate_rhythm_exists(&self, voice_id: VoiceId) -> bool {
        self.rhythm_manager.validate_rhythm_exists(voice_id)
    }

    fn validate_circle_exists(&self, voice_id: VoiceId, circle_id: usize) -> bool {
        self.voice_manager
            .validate_circle_exists(voice_id, circle_id)
    }

    // Composite operations - delegate to voice_manager with wind field access
    fn remove_circle_from_voice(&mut self, voice_id: VoiceId, circle_id: usize) -> bool {
        let wind_field = &mut self.particle_system.forces.wind_field;
        self.voice_manager
            .remove_circle_from_voice(voice_id, circle_id, wind_field)
    }

    fn remove_all_circles_from_voice(&mut self, voice_id: VoiceId) {
        let wind_field = &mut self.particle_system.forces.wind_field;
        self.voice_manager
            .remove_all_circles_from_voice(voice_id, wind_field);
    }

    // Rhythm composite operations - delegate to rhythm_manager with service access
    fn update_rhythm_sequencer(&mut self, voice_id: VoiceId) {
        self.rhythm_manager
            .update_rhythm_sequencer(voice_id, &mut self.sequencer_service);
    }

    fn rhythm_reroll_wings(&mut self, voice_id: VoiceId) {
        self.rhythm_manager.rhythm_reroll_wings(
            voice_id,
            &mut self.rng,
            &mut self.sequencer_service,
        );
    }

    fn rhythm_add_wings(&mut self, voice_id: VoiceId, count: usize) {
        self.rhythm_manager
            .rhythm_add_wings(voice_id, count, &mut self.rng);
    }

    fn rhythm_stop_sequencer(&mut self, voice_id: VoiceId) {
        self.rhythm_manager
            .rhythm_stop_sequencer(voice_id, &mut self.sequencer_service);
    }

    fn rhythm_modify_all_slots_length(
        &mut self,
        voice_id: VoiceId,
        modification: RhythmParamModification,
    ) {
        self.rhythm_manager
            .rhythm_modify_all_slots_length(voice_id, modification, &mut self.rng);
    }

    fn rhythm_modify_all_slots_velocity(
        &mut self,
        voice_id: VoiceId,
        modification: RhythmParamModification,
    ) {
        self.rhythm_manager
            .rhythm_modify_all_slots_velocity(voice_id, modification, &mut self.rng);
    }

    fn rhythm_modify_all_slots_cutoff(
        &mut self,
        voice_id: VoiceId,
        modification: RhythmParamModification,
    ) {
        self.rhythm_manager
            .rhythm_modify_all_slots_cutoff(voice_id, modification, &mut self.rng);
    }
}
