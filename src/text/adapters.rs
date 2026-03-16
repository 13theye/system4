// src/text/adapters.rs
//
// Convert various domain events into display-oriented text blocks.

use crate::{
    command_engine::{Command, CommandInner, CompositeCommand, SimpleCommand},
    groups::VoiceId,
    text::{params_dashboard::ParamKey, TextBlock, TextFadeMode, TextStyle, WrapPolicy},
};

/// Extract parameter updates from an executed command.
///
/// These updates are intended for the pinned params slot (`TextSlot::Params`),
/// not the scrolling history.
pub fn param_updates_for_command(command: &Command) -> Vec<(VoiceId, ParamKey, f32)> {
    let mut out = Vec::new();

    let CommandInner::Simple(simple) = &command.command else {
        return out;
    };

    match simple {
        // Voice-level
        SimpleCommand::Alpha { voice_id, value } => {
            out.push((*voice_id, ParamKey::Voice("brightness"), *value));
        }
        SimpleCommand::Volume { voice_id, value } => {
            out.push((*voice_id, ParamKey::Voice("volume"), *value));
        }
        SimpleCommand::Feedback { voice_id, value } => {
            out.push((*voice_id, ParamKey::Voice("feedback"), *value));
        }
        SimpleCommand::Vibration { voice_id, value } => {
            out.push((*voice_id, ParamKey::Voice("vibration"), *value));
        }

        // Circle-level
        SimpleCommand::OuterRadius {
            voice_id,
            circle_id,
            value,
        } => out.push((
            *voice_id,
            ParamKey::Circle {
                circle_id: *circle_id,
                name: "outerRadius",
            },
            *value,
        )),
        SimpleCommand::InnerRadius {
            voice_id,
            circle_id,
            value,
        } => out.push((
            *voice_id,
            ParamKey::Circle {
                circle_id: *circle_id,
                name: "innerRadius",
            },
            *value,
        )),
        SimpleCommand::Force {
            voice_id,
            circle_id,
            value,
        } => out.push((
            *voice_id,
            ParamKey::Circle {
                circle_id: *circle_id,
                name: "force",
            },
            *value,
        )),
        SimpleCommand::Gravity {
            voice_id,
            circle_id,
            value,
        } => out.push((
            *voice_id,
            ParamKey::Circle {
                circle_id: *circle_id,
                name: "gravity",
            },
            *value,
        )),
        SimpleCommand::Noise {
            voice_id,
            circle_id,
            value,
        } => out.push((
            *voice_id,
            ParamKey::Circle {
                circle_id: *circle_id,
                name: "noise",
            },
            *value,
        )),
        SimpleCommand::CenterX {
            voice_id,
            circle_id,
            value,
        } => out.push((
            *voice_id,
            ParamKey::Circle {
                circle_id: *circle_id,
                name: "centerX",
            },
            *value,
        )),
        SimpleCommand::CenterY {
            voice_id,
            circle_id,
            value,
        } => out.push((
            *voice_id,
            ParamKey::Circle {
                circle_id: *circle_id,
                name: "centerY",
            },
            *value,
        )),
        _ => {}
    }

    out
}

/// Convert an executed command into 0..N text blocks, each routed to a voice.
///
/// Note: parameter updates are intentionally excluded; they belong in the pinned
/// params slot rather than the scrolling history.
pub fn blocks_for_command(command: &Command) -> Vec<(VoiceId, TextBlock)> {
    let mut out = Vec::new();

    match &command.command {
        CommandInner::Simple(simple) => {
            if let Some((voice, text)) = format_simple_for_history(simple) {
                out.push((
                    voice,
                    TextBlock::new(text)
                        .style(TextStyle::Normal)
                        .fade(TextFadeMode::Fade)
                        .wrap(WrapPolicy::WordWrap),
                ));
            }
        }
        CommandInner::Composite(comp) => {
            if let Some((voice, text)) = format_composite(comp) {
                out.push((
                    voice,
                    TextBlock::new(text)
                        .style(TextStyle::Bright)
                        .fade(TextFadeMode::NoFade)
                        .wrap(WrapPolicy::WordWrap),
                ));
            }
        }
    }

    out
}

fn format_simple_for_history(cmd: &SimpleCommand) -> Option<(VoiceId, String)> {
    match cmd {
        // Keep non-parameter simple commands that are useful as history.
        SimpleCommand::RemoveCircle {
            voice_id,
            circle_id,
        } => Some((*voice_id, format!("removeCircle(circle {})", circle_id))),

        // Parameter updates are handled by `param_updates_for_command`.
        _ => None,
    }
}

fn format_composite(cmd: &CompositeCommand) -> Option<(VoiceId, String)> {
    match cmd {
        CompositeCommand::CreateDrone { config } => Some((
            config.voice,
            format!("makeDrone(voice {})", config.voice.to_i32()),
        )),
        CompositeCommand::ModifyDrone { voice_id, .. } => Some((
            *voice_id,
            format!("modifyDrone(voice {})", voice_id.to_i32()),
        )),
        CompositeCommand::NewFormation {
            voice_id,
            formation_type,
            ..
        } => Some((
            *voice_id,
            format!("new{:?}(voice {})", formation_type, voice_id.to_i32()),
        )),
        CompositeCommand::Clear { voice_id } => {
            Some((*voice_id, format!("clear(voice {})", voice_id.to_i32())))
        }

        // Rhythms: can be enabled once Voice1/2 panes exist.
        _ => None,
    }
}
