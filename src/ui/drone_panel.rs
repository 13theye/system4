// Voice panel rendering functions

use super::params::DroneVoiceParams;
use crate::command_engine::{Command, CommandInner, CommandSource, SimpleCommand};

/// Renders a drone voice panel (for Voice 0 and Voice 3)
/// Returns a vector of commands that should be queued
pub fn render_drone_voice_panel(
    ui: &mut egui::Ui,
    params: &DroneVoiceParams,
    voice_name: &str,
    scroll_id: &str,
    circles_scroll_id: &str,
) -> Vec<Command> {
    let mut commands = Vec::new();
    let voice_id = params.voice_id;

    ui.heading(voice_name);
    ui.add_space(2.0);

    egui::ScrollArea::vertical()
        .id_source(scroll_id)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // Voice-level parameters (always shown)
            ui.add_space(5.0);

            // Alpha slider
            let mut alpha = params.voice_params.alpha_limit;
            if ui
                .add(
                    egui::Slider::new(&mut alpha, 0.0..=1.0)
                        .text("Brightness")
                        .custom_formatter(|n, _| format!("{:.3}", n)),
                )
                .changed()
            {
                commands.push(Command::new(
                    CommandInner::Simple(SimpleCommand::Alpha {
                        voice_id,
                        value: alpha,
                    }),
                    CommandSource::UI,
                ));
            }

            // Volume slider
            let mut volume = params.voice_params.volume;
            if ui
                .add(
                    egui::Slider::new(&mut volume, 0.0..=1.0)
                        .text("Volume")
                        .custom_formatter(|n, _| format!("{:.3}", n)),
                )
                .changed()
            {
                commands.push(Command::new(
                    CommandInner::Simple(SimpleCommand::Volume {
                        voice_id,
                        value: volume,
                    }),
                    CommandSource::UI,
                ));
            }

            // Vibration offset slider
            let mut vibration_offset = params.voice_params.vibration;
            if ui
                .add(
                    egui::Slider::new(&mut vibration_offset, 0.0..=1.0)
                        .text("Vibration")
                        .custom_formatter(|n, _| format!("{:.3}", n)),
                )
                .changed()
            {
                commands.push(Command::new(
                    CommandInner::Simple(SimpleCommand::Vibration {
                        voice_id,
                        value: vibration_offset,
                    }),
                    CommandSource::UI,
                ));
            }

            // Feedback slider
            let mut feedback = params.voice_params.feedback;
            if ui
                .add(
                    egui::Slider::new(&mut feedback, 0.0..=1.0)
                        .text("Feedback")
                        .custom_formatter(|n, _| format!("{:.3}", n)),
                )
                .changed()
            {
                commands.push(Command::new(
                    CommandInner::Simple(SimpleCommand::Feedback {
                        voice_id,
                        value: feedback,
                    }),
                    CommandSource::UI,
                ));
            }

            // Emitter Position slider
            let mut emitter_position = params.voice_params.emitter_position;
            if ui
                .add(
                    egui::Slider::new(&mut emitter_position, 0.0..=1.0)
                        .text("Emitter Pos")
                        .custom_formatter(|n, _| format!("{:.3}", n)),
                )
                .changed()
            {
                commands.push(Command::new(
                    CommandInner::Simple(SimpleCommand::MoveEmitters {
                        voice_id,
                        value: emitter_position,
                    }),
                    CommandSource::UI,
                ));
            }

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(5.0);
            ui.label("Wind Circles:");

            if !params.circles.is_empty() {
                // Horizontal scroll area for multiple circles
                egui::ScrollArea::horizontal()
                    .id_source(circles_scroll_id)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            for (circle_id, circle_params) in &params.circles {
                                // Each circle gets its own vertical column
                                ui.vertical(|ui| {
                                    ui.set_width(140.0);
                                    ui.label(format!("Circle {}", circle_id));
                                    ui.add_space(5.0);

                                    // Outer Radius slider
                                    let mut radius = circle_params.outer_radius;
                                    if ui
                                        .add(
                                            egui::Slider::new(&mut radius, 0.0..=1100.0)
                                                .text("OR")
                                                .custom_formatter(|n, _| format!("{:.1}", n)),
                                        )
                                        .changed()
                                    {
                                        commands.push(Command::new(
                                            CommandInner::Simple(SimpleCommand::OuterRadius {
                                                voice_id,
                                                circle_id: *circle_id,
                                                value: radius,
                                            }),
                                            CommandSource::UI,
                                        ));
                                    }

                                    // Inner Radius slider
                                    let mut inner_radius = circle_params.inner_radius;
                                    if ui
                                        .add(
                                            egui::Slider::new(&mut inner_radius, 0.0..=1100.0)
                                                .text("IR")
                                                .custom_formatter(|n, _| format!("{:.1}", n)),
                                        )
                                        .changed()
                                    {
                                        commands.push(Command::new(
                                            CommandInner::Simple(SimpleCommand::InnerRadius {
                                                voice_id,
                                                circle_id: *circle_id,
                                                value: inner_radius,
                                            }),
                                            CommandSource::UI,
                                        ));
                                    }

                                    // Force slider
                                    let mut strength = circle_params.force;
                                    if ui
                                        .add(
                                            egui::Slider::new(&mut strength, 0.0..=30.0)
                                                .text("Force")
                                                .custom_formatter(|n, _| format!("{:.1}", n)),
                                        )
                                        .changed()
                                    {
                                        commands.push(Command::new(
                                            CommandInner::Simple(SimpleCommand::Force {
                                                voice_id,
                                                circle_id: *circle_id,
                                                value: strength,
                                            }),
                                            CommandSource::UI,
                                        ));
                                    }

                                    // Gravity slider
                                    let mut center_bias = circle_params.gravity;
                                    if ui
                                        .add(
                                            egui::Slider::new(&mut center_bias, 0.0..=2.0)
                                                .text("Gravity")
                                                .custom_formatter(|n, _| format!("{:.2}", n)),
                                        )
                                        .changed()
                                    {
                                        commands.push(Command::new(
                                            CommandInner::Simple(SimpleCommand::Gravity {
                                                voice_id,
                                                circle_id: *circle_id,
                                                value: center_bias,
                                            }),
                                            CommandSource::UI,
                                        ));
                                    }

                                    // Noise slider
                                    let mut angle_variation = circle_params.noise;
                                    if ui
                                        .add(
                                            egui::Slider::new(&mut angle_variation, 0.0..=1.0)
                                                .text("Noise")
                                                .custom_formatter(|n, _| format!("{:.2}", n)),
                                        )
                                        .changed()
                                    {
                                        commands.push(Command::new(
                                            CommandInner::Simple(SimpleCommand::Noise {
                                                voice_id,
                                                circle_id: *circle_id,
                                                value: angle_variation,
                                            }),
                                            CommandSource::UI,
                                        ));
                                    }

                                    // Center X slider
                                    let mut center_x = circle_params.center.x;
                                    if ui
                                        .add(
                                            egui::Slider::new(&mut center_x, -1920.0..=1920.0)
                                                .text("Ctr X")
                                                .custom_formatter(|n, _| format!("{:.0}", n)),
                                        )
                                        .changed()
                                    {
                                        commands.push(Command::new(
                                            CommandInner::Simple(SimpleCommand::CenterX {
                                                voice_id,
                                                circle_id: *circle_id,
                                                value: center_x,
                                            }),
                                            CommandSource::UI,
                                        ));
                                    }

                                    // Center Y slider
                                    let mut center_y = circle_params.center.y;
                                    if ui
                                        .add(
                                            egui::Slider::new(&mut center_y, -1080.0..=1080.0)
                                                .text("Ctr Y")
                                                .custom_formatter(|n, _| format!("{:.0}", n)),
                                        )
                                        .changed()
                                    {
                                        commands.push(Command::new(
                                            CommandInner::Simple(SimpleCommand::CenterY {
                                                voice_id,
                                                circle_id: *circle_id,
                                                value: center_y,
                                            }),
                                            CommandSource::UI,
                                        ));
                                    }
                                    ui.add_space(10.0);
                                });
                            }
                        });
                    });
            } else {
                ui.label("No circles");
            }
        });

    commands
}
