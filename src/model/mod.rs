// src/model/mod.rs
//
// The main App Model

use crate::{
    config::*,
    forces::WindCircle,
    fps::FpsManager,
    osc::{OscCommand, OscController, OscSender},
    particle::ParticleSystem,
    terminals::{TerminalParams, TerminalSystem},
    utils::IdGenerator,
    voice::Voice,
};
use nannou::{prelude::*, rand::rngs::ThreadRng, text::Font, wgpu::TextureReshaper};
use nannou_egui::Egui;
use nnpipe::renderers::{
    HeatmapRenderer, ParticleGpu, ParticleRenderer, SegmentGpu, SegmentRenderer,
};
use nnpipe::*;

use std::{cell::RefCell, collections::HashMap, time::Instant};

pub type GpuBuffers = (Vec<ParticleGpu>, Vec<SegmentGpu>);

pub struct Model {
    pub particle_system: ParticleSystem,

    pub terminal_system: TerminalSystem,

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
    pub gpu_buffers: HashMap<Voice, GpuBuffers>,
    pub rendering: RefCell<Nnpipe>,
    pub heatmap_renderer: HeatmapRenderer,
    pub particle_renderer1: ParticleRenderer,
    pub particle_renderer4: ParticleRenderer,

    pub segment_renderer1: SegmentRenderer,
    pub segment_renderer4: SegmentRenderer,

    pub dpi_scale: f32,
    pub font: Font,

    // Simple ID counter
    pub id_generator: IdGenerator,

    // Egui
    pub egui: Egui,

    // Random
    pub rng: ThreadRng,

    // FPS display
    pub fps: FpsManager,

    // Timing for render and updates
    pub frame_count: u64,
    pub update_ticks: u64,
    pub last_update: Instant,

    // Debug stuff
    pub show_bounds: bool,
    pub show_forces: bool,
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
    let voice = Voice::from_i32(id);
    model.particle_system.kill_voice(&voice);
    model.particle_system.forces.update(model.show_forces);
    model.osc_send.send_drone_on_off(id, 0);
}
