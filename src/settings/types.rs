// src/config/types.rs
//
// Config types for the app

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct OscSendConfig {
    pub target_addr: String,
    pub target_port: u16,
}

#[derive(Debug, Deserialize)]
pub struct OscLoopConfig {
    pub target_addr: String,
    pub target_port: u16,
}

#[derive(Debug, Deserialize)]
pub struct OscReceiveConfig {
    pub receive_port: u16,
}

#[derive(Debug, Deserialize)]
pub struct RenderConfig {
    pub texture_width: u32,
    pub texture_height: u32,
    pub texture_samples: u32,
    pub arc_resolution: u32,
    pub dpi_scale: f32,
}

#[derive(Debug, Deserialize)]
pub struct ParticleConfig {
    pub limit: u32,
}

#[derive(Debug, Deserialize)]
pub struct SpeedConfig {
    pub bpm: f32,
}

#[derive(Debug, Deserialize)]
pub struct AudienceWindowConfig {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Deserialize)]
pub struct PerformerWindowConfig {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Deserialize)]
pub struct ControlWindowConfig {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Deserialize)]
pub struct OpenAIServiceConfig {
    pub url: String,
    pub model: String,
    pub system_prompt: String,
    pub schema_description: String,
    pub strict_object_adherence: bool,
    pub api_key: Option<String>,
}
