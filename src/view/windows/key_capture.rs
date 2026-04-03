//! src/view/windows/key_capture.rs
//!
//! Input capture for Control window

use crate::{
    groups::VoiceId,
    model::Model,
    text::{TextPaneId, TextSlot},
};

use nannou::prelude::*;

// ******************************* Input Capture *****************************

/// Keypress events for Control window
pub fn raw_window_event(_app: &App, model: &mut Model, event: &nannou::winit::event::WindowEvent) {
    model.ui_state.egui.handle_raw_event(event);

    // Handle keyboard input for command terminal
    if let nannou::winit::event::WindowEvent::KeyboardInput { input, .. } = event {
        if input.state == nannou::winit::event::ElementState::Pressed {
            if let Some(key) = input.virtual_keycode {
                use nannou::winit::event::VirtualKeyCode;

                match key {
                    VirtualKeyCode::Escape => {
                        for voice in [
                            VoiceId::Voice0,
                            VoiceId::Voice1,
                            VoiceId::Voice2,
                            VoiceId::Voice3,
                        ] {
                            if let Some(input) = model.ui_state.command_inputs.get_mut(&voice) {
                                input.clear();
                            }
                            model
                                .ui_state
                                .text_overlay
                                .borrow_mut()
                                .clear_live_slot(TextPaneId::Voice(voice), TextSlot::CommandInput);
                        }
                    }

                    VirtualKeyCode::P => {
                        /*
                        // Toggle debug and FPS display - disabled, now handled
                        // via a checkbox
                        model.ui_state.show_bounds = !model.ui_state.show_bounds;
                         */
                    }

                    /*
                    VirtualKeyCode::M => {
                        let command = CommandInner::Simple(SimpleCommand::MaskAnimation {
                            voice_id: VoiceId::Voice0,
                            new_origin: vec2(0.0, 0.0),
                            new_size: vec2(3840.0, 2160.0),
                            duration: Duration::from_secs(3),
                        });

                        model.queue_command(Command::new(command, CommandSource::Terminal));
                    }

                    VirtualKeyCode::N => {
                        let command = CommandInner::Simple(SimpleCommand::MaskAnimation {
                            voice_id: VoiceId::Voice0,
                            new_origin: vec2(-950.0, 0.0),
                            new_size: vec2(900.0, 1300.0),
                            duration: Duration::from_secs(1),
                        });

                        model.queue_command(Command::new(command, CommandSource::Terminal));
                    }

                    VirtualKeyCode::J => {
                        let command = CommandInner::Simple(SimpleCommand::MaskAnimation {
                            voice_id: VoiceId::Voice3,
                            new_origin: vec2(0.0, 0.0),
                            new_size: vec2(3840.0, 2160.0),
                            duration: Duration::from_secs(3),
                        });

                        model.queue_command(Command::new(command, CommandSource::Terminal));
                    }

                    VirtualKeyCode::K => {
                        let command = CommandInner::Simple(SimpleCommand::MaskAnimation {
                            voice_id: VoiceId::Voice3,
                            new_origin: vec2(950.0, 0.0),
                            new_size: vec2(900.0, 1300.0),
                            duration: Duration::from_secs(1),
                        });

                        model.queue_command(Command::new(command, CommandSource::Terminal));
                    }
                     */
                    _ => {}
                }
            }
        }
    }

    // Note: Text input is now handled directly by egui TextEdit widget
    // through the TextBuffer trait implementation
}

/// Key pressed handler for Performer window
pub fn performer_key_pressed(_app: &App, model: &mut Model, key: nannou::prelude::Key) {
    if key == nannou::prelude::Key::I {
        model.intro_image.toggle_visible();
    }
}
