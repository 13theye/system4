//! Preset values for masks

use crate::groups::VoiceId;
use nannou::prelude::*;

pub enum MaskPreset {
    Small(VoiceId),
    Halfscreen(VoiceId),
    Fullscreen(VoiceId),
}

pub fn mask_size(preset: &MaskPreset) -> Option<Vec2> {
    match preset {
        MaskPreset::Small(voice_id) => mask_small_size(voice_id),
        MaskPreset::Halfscreen(voice_id) => mask_halfscreen_size(voice_id),
        MaskPreset::Fullscreen(voice_id) => mask_fullscreen_size(voice_id),
    }
}

pub fn mask_origin(preset: &MaskPreset) -> Option<Vec2> {
    match preset {
        MaskPreset::Small(voice_id) => mask_small_origin(voice_id),
        MaskPreset::Halfscreen(voice_id) => mask_halfscreen_origin(voice_id),
        MaskPreset::Fullscreen(voice_id) => mask_fullscreen_origin(voice_id),
    }
}

fn mask_small_size(voice_id: &VoiceId) -> Option<Vec2> {
    match voice_id {
        VoiceId::Voice0 => Some(vec2(900.0, 1300.0)),
        VoiceId::Voice3 => Some(vec2(900.0, 1300.0)),
        _ => None,
    }
}

fn mask_small_origin(voice_id: &VoiceId) -> Option<Vec2> {
    match voice_id {
        VoiceId::Voice0 => Some(vec2(-950.0, 0.0)),
        VoiceId::Voice3 => Some(vec2(950.0, 0.0)),
        _ => None,
    }
}

fn mask_halfscreen_size(voice_id: &VoiceId) -> Option<Vec2> {
    match voice_id {
        VoiceId::Voice0 => Some(vec2(1900.0, 2140.0)),
        VoiceId::Voice3 => Some(vec2(1900.0, 2140.0)),
        _ => None,
    }
}

fn mask_halfscreen_origin(voice_id: &VoiceId) -> Option<Vec2> {
    match voice_id {
        VoiceId::Voice0 => Some(vec2(-950.0, 0.0)),
        VoiceId::Voice3 => Some(vec2(950.0, 0.0)),
        _ => None,
    }
}

fn mask_fullscreen_size(_voice_id: &VoiceId) -> Option<Vec2> {
    Some(vec2(3840.0, 2160.0))
}

fn mask_fullscreen_origin(_voice_id: &VoiceId) -> Option<Vec2> {
    Some(vec2(0.0, 0.0))
}
