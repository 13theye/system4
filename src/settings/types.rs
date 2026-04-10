// src/config/types.rs
//
// Config types for the app

use serde::{Deserialize, Serialize};

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
pub struct PathConfig {
    pub intro_image: String,
}

#[derive(Debug, Deserialize)]
pub struct ParticleConfig {
    pub per_voice_limit: u32,
}

#[derive(Debug, Deserialize)]
pub struct TempoConfig {
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

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct RhythmColorConfig {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl RhythmColorConfig {
    pub fn to_array(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

#[derive(Debug, Deserialize)]
pub struct RhythmVisConfig {
    pub base_color: RhythmColorConfig,
    pub gradient_1: RhythmColorConfig,
    pub gradient_2: RhythmColorConfig,
}

#[derive(Debug, Deserialize)]
pub struct OpenAIServiceConfig {
    pub url: String,
    pub model: String,
    pub system_prompt: String,
    pub schema_description: Option<String>,
    pub strict_request_object_adherence: bool,
    pub api_key: Option<String>,
}
