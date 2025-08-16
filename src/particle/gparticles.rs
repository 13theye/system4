// src/components/particles/gparticles.rs
//
// Core GPU particle data structures and parameters
// based on the CPU implementation

use bytemuck::{Pod, Zeroable};

// currently 128 bytes
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
/// The GPU particle data structure
pub struct GParticle {
    pub parent_emitter: u32, // the emitter that spawned this particle: 4B
    pub position: [f32; 2],  // current world position: 8B
    pub feedback_positions: [[f32; 2]; 8], // previous world positions: 64B
    pub velocity: [f32; 2],  // current velocity vector: 8B
    pub acceleration: [f32; 2], // current acceleration vector (reset each frame): 4B

    pub age: f32,                 // age in ticks: 4B
    pub remaining_life_span: f32, // remaining ticks: 4B
    pub age_per_frame: f32,       // ticks per frame: 4B

    pub size: f32,         // particle size: 4B
    pub mass: f32,         // particle mass: 4B
    pub rgba: [f32; 4],    // particle color: 16B
    pub _padding: [u8; 0], // 16-byte-alignment end padding
}
