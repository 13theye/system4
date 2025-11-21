// src/model/mod.rs
//
// The main App Model

pub mod command_builder;
pub mod controller;
pub mod terminal_processor;

use crate::{
    groups::{Rhythm, Voice, VoiceId},
    model::controller::Command,
    osc::{OscController, OscSender},
    particle::ParticleSystem,
    services::sequencer::SequencerService,
    terminals::{command_input::CommandInput, terminal_view::TerminalViewManager},
    utils::IdGenerator,
    view::RhythmView,
};

use fps::FpsManager;
use nannou::{prelude::*, text::Font, wgpu::TextureReshaper};
use nannou_egui::Egui;
use nnpipe::renderers::{
    HeatmapRenderer, ParticleGpu, ParticleRenderer, SegmentGpu, SegmentRenderer,
};
use nnpipe::*;
use prat::clockservice::ClockService;
use rand::rngs::ThreadRng;

use std::{cell::RefCell, collections::HashMap};

pub type GpuParticleBuffer = Vec<ParticleGpu>;
pub type GpuSegmentBuffer = Vec<SegmentGpu>;

pub struct Model {
    pub particle_system: ParticleSystem,

    pub voices: HashMap<VoiceId, Voice>,
    pub rhythms: HashMap<VoiceId, Rhythm>,
    pub rhythm_view: RhythmView,

    // Clock and Sequencers
    pub clock: ClockService,
    pub sequencer_service: SequencerService,

    // OSC
    pub osc: OscController,
    pub osc_send: OscSender,
    pub osc_loop: OscSender,

    // Windows' texture reshapers
    pub render_size: Vec2,
    pub render_rect: Rect,
    pub audience_window_id: WindowId,
    pub performer_window_id: WindowId,
    pub control_window_id: WindowId,
    pub audience_reshaper: TextureReshaper,
    pub performer_reshaper: TextureReshaper,

    // Nannou API
    /// Draw context for UI elements to audience_window only
    pub audience_draw: nannou::Draw,
    /// Draw context for UI elements to performer_window only
    pub performer_draw: nannou::Draw,
    /// Draw context for drawing UI elements to ui_window only
    pub control_draw: nannou::Draw,

    // Rendering engine
    pub gpu_particle_buffer: GpuParticleBuffer,
    pub gpu_segment_buffers: HashMap<VoiceId, GpuSegmentBuffer>,
    pub rendering: RefCell<Nnpipe>,
    pub heatmap_renderer: HeatmapRenderer,
    pub particle_renderer: ParticleRenderer,
    pub segment_renderer: SegmentRenderer,
    pub dpi_scale: f32,
    pub font: Font,

    // Zero-copy particle rendering counts
    pub particle_count: usize,
    pub segment_instance_count: usize,

    // Simple ID counter
    pub id_generator: IdGenerator,

    // Egui
    pub egui: Egui,

    // Random
    pub rng: ThreadRng,

    // FPS display
    pub fps: FpsManager,

    // Debug stuff
    pub show_bounds: bool,
    pub show_forces: bool,

    // Command input for NTerminal
    pub command_input: CommandInput,

    // Terminal view manager for on-screen display
    pub terminal_manager: RefCell<TerminalViewManager>,

    // Unified command queue with priority resolution
    pub command_queue: Vec<Command>,

    // UI state
    pub active_tab: usize, // 0 = Voices, 1 = NTerminal
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
impl crate::command_engine::context::ExecutionContext for Model {
    fn log_command(&mut self, command: &crate::commands::Command) {
        self.terminal_manager.borrow_mut().process_command(command);
    }

    // Voice state access
    fn has_voice(&self, voice_id: VoiceId) -> bool {
        self.voices.contains_key(&voice_id)
    }

    fn get_voice(&self, voice_id: VoiceId) -> Option<&Voice> {
        self.voices.get(&voice_id)
    }

    fn get_voice_mut(&mut self, voice_id: VoiceId) -> Option<&mut Voice> {
        self.voices.get_mut(&voice_id)
    }

    fn insert_voice(&mut self, voice_id: VoiceId, voice: Voice) {
        self.voices.insert(voice_id, voice);
    }

    fn remove_voice(&mut self, voice_id: VoiceId) -> Option<Voice> {
        self.voices.remove(&voice_id)
    }

    fn voices(&self) -> &HashMap<VoiceId, Voice> {
        &self.voices
    }

    fn voices_mut(&mut self) -> &mut HashMap<VoiceId, Voice> {
        &mut self.voices
    }

    // Rhythm state access
    fn has_rhythm(&self, voice_id: VoiceId) -> bool {
        self.rhythms.contains_key(&voice_id)
    }

    fn get_rhythm(&self, voice_id: VoiceId) -> Option<&Rhythm> {
        self.rhythms.get(&voice_id)
    }

    fn get_rhythm_mut(&mut self, voice_id: VoiceId) -> Option<&mut Rhythm> {
        self.rhythms.get_mut(&voice_id)
    }

    fn insert_rhythm(&mut self, voice_id: VoiceId, rhythm: Rhythm) {
        self.rhythms.insert(voice_id, rhythm);
    }

    fn remove_rhythm(&mut self, voice_id: VoiceId) -> Option<Rhythm> {
        self.rhythms.remove(&voice_id)
    }

    fn rhythms(&self) -> &HashMap<VoiceId, Rhythm> {
        &self.rhythms
    }

    fn rhythms_mut(&mut self) -> &mut HashMap<VoiceId, Rhythm> {
        &mut self.rhythms
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

    // GPU segment buffer access
    fn get_segment_buffer(&self, voice_id: VoiceId) -> Option<&crate::rendering::GpuSegmentBuffer> {
        self.gpu_segment_buffers.get(&voice_id)
    }

    fn insert_segment_buffer(&mut self, voice_id: VoiceId, buffer: crate::rendering::GpuSegmentBuffer) {
        self.gpu_segment_buffers.insert(voice_id, buffer);
    }

    fn remove_segment_buffer(&mut self, voice_id: VoiceId) -> Option<crate::rendering::GpuSegmentBuffer> {
        self.gpu_segment_buffers.remove(&voice_id)
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

    // Validation
    fn validate_voice_exists(&self, voice_id: VoiceId) -> bool {
        self.voices.contains_key(&voice_id)
    }

    fn validate_rhythm_exists(&self, voice_id: VoiceId) -> bool {
        self.rhythms.contains_key(&voice_id)
    }

    fn validate_circle_exists(&self, voice_id: VoiceId, circle_id: usize) -> bool {
        self.voices
            .get(&voice_id)
            .map(|voice| voice.wind_circles.contains_key(&circle_id))
            .unwrap_or(false)
    }

    fn remove_circle_from_voice(&mut self, voice_id: VoiceId, circle_id: usize) -> bool {
        let wind_field = &mut self.particle_system.forces.wind_field;

        if let Some(voice) = self.voices.get_mut(&voice_id) {
            if let Some(circle) = voice.wind_circles.get_mut(&circle_id) {
                circle.remove_from_field(wind_field);
                voice.remove_wind_circle(circle_id);
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    fn remove_all_circles_from_voice(&mut self, voice_id: VoiceId) {
        let wind_field = &mut self.particle_system.forces.wind_field;

        if let Some(voice) = self.voices.get_mut(&voice_id) {
            for (_id, circle) in voice.wind_circles.iter_mut() {
                circle.remove_from_field(wind_field);
            }
        }
    }

    fn update_rhythm_sequencer(&mut self, voice_id: VoiceId) {
        if let Some(rhythm) = self.rhythms.get(&voice_id) {
            rhythm.update_sequencer(&mut self.sequencer_service);
        }
    }

    fn rhythm_reroll_wings(&mut self, voice_id: VoiceId) {
        if let Some(rhythm) = self.rhythms.get_mut(&voice_id) {
            rhythm.reroll_wings(&mut self.rng, &mut self.sequencer_service);
        }
    }

    fn rhythm_add_wings(&mut self, voice_id: VoiceId, count: usize) {
        if let Some(rhythm) = self.rhythms.get_mut(&voice_id) {
            rhythm.add_wings(count, &mut self.rng);
        }
    }

    fn rhythm_stop_sequencer(&mut self, voice_id: VoiceId) {
        if let Some(rhythm) = self.rhythms.get_mut(&voice_id) {
            rhythm.stop_sequencer(&mut self.sequencer_service);
        }
    }

    fn rhythm_modify_all_slots_length(&mut self, voice_id: VoiceId, modification: crate::terminals::commands::rhythm::ParameterModification) {
        if let Some(rhythm) = self.rhythms.get_mut(&voice_id) {
            rhythm.modify_all_slots_length(modification, &mut self.rng);
        }
    }

    fn rhythm_modify_all_slots_velocity(&mut self, voice_id: VoiceId, modification: crate::terminals::commands::rhythm::ParameterModification) {
        if let Some(rhythm) = self.rhythms.get_mut(&voice_id) {
            rhythm.modify_all_slots_velocity(modification, &mut self.rng);
        }
    }

    fn rhythm_modify_all_slots_cutoff(&mut self, voice_id: VoiceId, modification: crate::terminals::commands::rhythm::ParameterModification) {
        if let Some(rhythm) = self.rhythms.get_mut(&voice_id) {
            rhythm.modify_all_slots_cutoff(modification, &mut self.rng);
        }
    }
}
