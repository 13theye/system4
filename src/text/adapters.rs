// src/text/adapters.rs
//
// Convert various domain events into display-oriented text blocks.

use crate::{
    command_engine::{Command, CommandInner, CompositeCommand, SimpleCommand},
    groups::VoiceId,
    text::{TextBlock, TextFadeMode, TextStyle, WrapPolicy},
};

/// Convert an executed command into 0..N text blocks, each routed to a voice.
///
/// This is intentionally conservative: it only formats a subset of commands.
pub fn blocks_for_command(command: &Command) -> Vec<(VoiceId, TextBlock)> {
    let mut out = Vec::new();

    match &command.command {
        CommandInner::Simple(simple) => {
            if let Some((voice, text)) = format_simple(simple) {
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

fn format_simple(cmd: &SimpleCommand) -> Option<(VoiceId, String)> {
    match cmd {
        SimpleCommand::Alpha { voice_id, value } => {
            Some((*voice_id, format!("brightness({:.2})", value)))
        }
        SimpleCommand::Volume { voice_id, value } => Some((*voice_id, format!("volume({:.2})", value))),
        SimpleCommand::Feedback { voice_id, value } => {
            Some((*voice_id, format!("feedback({:.2})", value)))
        }
        SimpleCommand::Vibration { voice_id, value } => {
            Some((*voice_id, format!("vibration({:.2})", value)))
        }

        SimpleCommand::OuterRadius {
            voice_id,
            circle_id,
            value,
        } => Some((
            *voice_id,
            format!("circle({}).outerRadius({:.2})", circle_id, value),
        )),
        SimpleCommand::InnerRadius {
            voice_id,
            circle_id,
            value,
        } => Some((
            *voice_id,
            format!("circle({}).innerRadius({:.2})", circle_id, value),
        )),
        SimpleCommand::Force {
            voice_id,
            circle_id,
            value,
        } => Some((
            *voice_id,
            format!("circle({}).force({:.2})", circle_id, value),
        )),
        SimpleCommand::Gravity {
            voice_id,
            circle_id,
            value,
        } => Some((
            *voice_id,
            format!("circle({}).gravity({:.2})", circle_id, value),
        )),
        SimpleCommand::Noise {
            voice_id,
            circle_id,
            value,
        } => Some((
            *voice_id,
            format!("circle({}).noise({:.2})", circle_id, value),
        )),
        SimpleCommand::CenterX {
            voice_id,
            circle_id,
            value,
        } => Some((
            *voice_id,
            format!("circle({}).centerX({:.2})", circle_id, value),
        )),
        SimpleCommand::CenterY {
            voice_id,
            circle_id,
            value,
        } => Some((
            *voice_id,
            format!("circle({}).centerY({:.2})", circle_id, value),
        )),

        SimpleCommand::RemoveCircle { voice_id, circle_id } => Some((
            *voice_id,
            format!("removeCircle(circle {})", circle_id),
        )),

        // Not currently displayed.
        _ => None,
    }
}

fn format_composite(cmd: &CompositeCommand) -> Option<(VoiceId, String)> {
    match cmd {
        CompositeCommand::CreateDrone { config } => {
            Some((config.voice, format!("makeDrone(voice {})", config.voice.to_i32())))
        }
        CompositeCommand::ModifyDrone { voice_id, .. } => {
            Some((*voice_id, format!("modifyDrone(voice {})", voice_id.to_i32())))
        }
        CompositeCommand::NewCircle { voice_id, .. } => {
            Some((*voice_id, format!("newCircle(voice {})", voice_id.to_i32())))
        }
        CompositeCommand::Clear { voice_id } => {
            Some((*voice_id, format!("clear(voice {})", voice_id.to_i32())))
        }

        // Rhythms: can be enabled once Voice1/2 panes exist.
        _ => None,
    }
}
