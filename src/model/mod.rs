// src/model/mod.rs
//
// The main App Model

pub mod command_flow;
pub mod command_helpers;
pub mod queries;
pub mod terminal_processor;
pub mod validation;

use crate::{
    command_engine::{
        context::ExecutionContext, Command, CommandInner, CommandSource, CompositeCommand,
        SimpleCommand,
    },
    groups::{Rhythm, Voice, VoiceId},
    managers::{RhythmManager, VoiceManager},
    osc::{OscController, OscSender},
    particle::ParticleSystem,
    rendering::{GpuSegmentBuffer, RenderState},
    sequencer::SequencerService,
    terminals::commands::rhythm::RhythmParamModification,
    ui::UiState,
    utils::IdGenerator,
    view::rhythm::RhythmView,
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

    // Pending commands to execute.
    pub command_queue: Vec<Command>,

    // Flag indicating that Voice1's rhythm was created/modified this frame
    // and, if auto-AI is enabled, we should trigger a single AI rhythm
    // request after all commands have been applied.
    pub auto_ai_pending_for_voice1: bool,
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
    fn log_command(&mut self, command: &Command) {
        // Prefer the unified text overlay system.
        let now = std::time::Instant::now();

        // If we receive OSC param updates for a voice that doesn't exist, ignore.
        // (Prevents params slot noise when no voice is active.)
        let should_ignore_osc_params_for_voice = |voice_id: VoiceId, source: CommandSource| {
            source == CommandSource::OSC
                && !(self.voice_manager.has_voice(voice_id)
                    || self.rhythm_manager.has_rhythm(voice_id))
        };

        // Track per-frame auto-AI triggers for Voice1 rhythms.
        if let Some(voice_id) = voice_id_for_command(command) {
            if voice_id == VoiceId::Voice1 && is_rhythm_shape_or_param_command(command) {
                self.auto_ai_pending_for_voice1 = true;
            }
        }

        // Clear pinned params when a voice is cleared.
        if let Some(voice_id) = voice_id_for_command(command) {
            if matches!(
                command.command,
                CommandInner::Simple(SimpleCommand::ClearDrone { .. })
                    | CommandInner::Simple(SimpleCommand::ClearRhythm { .. })
                    | CommandInner::Composite(CompositeCommand::Clear { .. })
            ) {
                self.ui_state
                    .text_overlay
                    .borrow_mut()
                    .clear_params_dashboard(voice_id);
            }
        }

        // Parameter updates go to the pinned params slot.
        for (voice_id, key, value) in crate::text::adapters::param_updates_for_command(command) {
            if should_ignore_osc_params_for_voice(voice_id, command.source) {
                continue;
            }

            self.ui_state
                .text_overlay
                .borrow_mut()
                .apply_param_update(voice_id, key, value, now);
        }

        // Non-parameter messages go to history.
        for (voice_id, block) in crate::text::adapters::blocks_for_command(command) {
            self.ui_state.text_overlay.borrow_mut().push_history_block(
                crate::text::TextPaneId::Voice(voice_id),
                block,
                now,
            );
        }
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
    fn queue_command(&mut self, command: crate::command_engine::Command) {
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

fn voice_id_for_command(command: &Command) -> Option<VoiceId> {
    match &command.command {
        CommandInner::Simple(simple) => match simple {
            SimpleCommand::Alpha { voice_id, .. }
            | SimpleCommand::Volume { voice_id, .. }
            | SimpleCommand::Feedback { voice_id, .. }
            | SimpleCommand::Vibration { voice_id, .. }
            | SimpleCommand::MoveEmitters { voice_id, .. }
            | SimpleCommand::OuterRadius { voice_id, .. }
            | SimpleCommand::InnerRadius { voice_id, .. }
            | SimpleCommand::Force { voice_id, .. }
            | SimpleCommand::Gravity { voice_id, .. }
            | SimpleCommand::Noise { voice_id, .. }
            | SimpleCommand::CenterX { voice_id, .. }
            | SimpleCommand::CenterY { voice_id, .. }
            | SimpleCommand::ListCircles { voice_id }
            | SimpleCommand::AddWings { voice_id, .. }
            | SimpleCommand::RemoveWings { voice_id, .. }
            | SimpleCommand::ClearRhythm { voice_id }
            | SimpleCommand::ClearDrone { voice_id }
            | SimpleCommand::RemoveCircle { voice_id, .. }
            | SimpleCommand::RhythmCapacity { voice_id, .. }
            | SimpleCommand::RhythmNumWings { voice_id, .. }
            | SimpleCommand::RhythmSubdivision { voice_id, .. }
            | SimpleCommand::RhythmLengthRange { voice_id, .. }
            | SimpleCommand::RhythmVelocityRange { voice_id, .. }
            | SimpleCommand::RhythmCutoffRange { voice_id, .. }
            | SimpleCommand::RhythmModifyLength { voice_id, .. }
            | SimpleCommand::RhythmModifyVelocity { voice_id, .. }
            | SimpleCommand::RhythmModifyCutoff { voice_id, .. } => Some(*voice_id),
        },
        CommandInner::Composite(comp) => match comp {
            CompositeCommand::CreateDrone { config } => Some(config.voice),
            CompositeCommand::CreateRhythm { config } => Some(config.voice),
            CompositeCommand::ModifyDrone { voice_id, .. }
            | CompositeCommand::ModifyRhythm { voice_id, .. }
            | CompositeCommand::NewCircle { voice_id, .. }
            | CompositeCommand::Clear { voice_id } => Some(*voice_id),
        },
    }
}

fn is_rhythm_shape_or_param_command(command: &Command) -> bool {
    match &command.command {
        CommandInner::Composite(comp) => matches!(
            comp,
            CompositeCommand::CreateRhythm { .. } | CompositeCommand::ModifyRhythm { .. }
        ),
        CommandInner::Simple(simple) => matches!(
            simple,
            SimpleCommand::RhythmCapacity { .. }
                | SimpleCommand::RhythmNumWings { .. }
                | SimpleCommand::RhythmSubdivision { .. }
                | SimpleCommand::RhythmLengthRange { .. }
                | SimpleCommand::RhythmVelocityRange { .. }
                | SimpleCommand::RhythmCutoffRange { .. }
                | SimpleCommand::RhythmModifyLength { .. }
                | SimpleCommand::RhythmModifyVelocity { .. }
                | SimpleCommand::RhythmModifyCutoff { .. }
                | SimpleCommand::AddWings { .. }
                | SimpleCommand::RemoveWings { .. }
        ),
    }
}
