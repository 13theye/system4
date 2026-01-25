use super::drone_panel;
use super::params::DroneVoiceParams;
use crate::command_engine::Command;
use crate::groups::VoiceId;
use crate::model::Model;
use crate::terminals::{command_input::CommandInput, commands::TerminalCommand};
use crate::text::{TextBlock, TextFadeMode, TextPaneId, TextSlot, TextStyle, WrapPolicy};
use crate::ui::UIActiveTab;
use nannou::App;
use std::time::Instant;

pub fn update_control_ui(app: &App, model: &mut Model) {
    let Some(control_window) = app.window(model.render_state.control_window_id) else {
        eprintln!("Control window not found. Exiting app.");
        std::process::exit(1);
    };
    let rect = control_window.rect();
    let height = rect.h() - 25.0;
    let width = rect.w() - 5.0;

    // Extract all parameters before creating egui context to avoid borrowing conflicts
    let voice0_params = DroneVoiceParams::extract(model, VoiceId::Voice0);
    let voice3_params = DroneVoiceParams::extract(model, VoiceId::Voice3);

    let ctx = model.ui_state.egui.begin_frame();

    // Set text style settings
    let style = (*ctx.style()).clone();
    ctx.set_style(adjust_style_from(style));

    let mut command_queue = Vec::<Command>::new();
    let mut terminal_commands_to_process = Vec::new();

    egui::Window::new("Control Panel")
        .fixed_pos(egui::pos2(0.0, 0.0))
        .default_size(egui::vec2(width, height))
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .frame(egui::Frame {
            fill: egui::Color32::from_rgba_unmultiplied(0, 0, 0, 0), // Dark background
            stroke: egui::Stroke::new(0.0, egui::Color32::from_rgb(10, 10, 10)), // Subtle border
            inner_margin: egui::style::Margin::symmetric(10.0, 10.0), // Padding
            outer_margin: egui::style::Margin::same(0.0),            // No margin
            rounding: egui::Rounding::same(1.0),                     // Slightly rounded corners
            shadow: egui::epaint::Shadow::NONE,
        })
        .show(&ctx, |ui| {
            ui.horizontal(|ui| {
                // Vertical 1: Instructions and status info (always visible)
                ui.vertical(|ui| {
                    ui.set_min_size(egui::vec2(180.0, height));
                    // Status info
                    ui.label(format!(
                        "Particles: {}",
                        model.particle_system.get_total_particle_count()
                    ));
                    // FPS
                    ui.label(format!("FPS: {:.1}", model.ui_state.fps.fps()));
                    ui.add_space(15.0);

                    ui
                        .checkbox(&mut model.ui_state.show_forces, "Show Forces")
                        .changed();

                    let mut particle_system_mode_is_combined = model.particle_system.mode.is_combined();
                    if ui
                        .checkbox(
                            &mut particle_system_mode_is_combined,
                            "Combine voices' forces",
                        )
                        .changed() {
                            model.particle_system.mode.toggle();
                        }
                    ui.add_space(30.0);

                    // Push tab selector to bottom with expanding space
                    ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                        // Tab bar at bottom
                        ui.add_space(20.0);
                        ui.horizontal(|ui| {
                            if ui
                                .selectable_label(model.ui_state.active_tab == UIActiveTab::Debug, UIActiveTab::Debug.display_text())
                                .clicked()
                            {
                                model.ui_state.active_tab = UIActiveTab::Debug;
                            }
                        });
                        ui.horizontal(|ui| {
                            if ui
                                .selectable_label(model.ui_state.active_tab == UIActiveTab::Terminal, UIActiveTab::Terminal.display_text())
                                .clicked()
                            {
                                model.ui_state.active_tab = UIActiveTab::Terminal;
                            }
                            if ui
                                .selectable_label(model.ui_state.active_tab == UIActiveTab::Drones, UIActiveTab::Drones.display_text())
                                .clicked()
                            {
                                model.ui_state.active_tab = UIActiveTab::Drones;
                            }

                        });
                    });
                });

                ui.separator();

                // Everything to the right of the status column should be vertically scrollable.
                // (Left column stays fixed for particles/FPS/tab selection.)
                egui::ScrollArea::vertical()
                    .id_source("performer_control_right_scroll")
                    .auto_shrink([false, false])
                    .max_height(height)
                    .show(ui, |ui| {
                        // Tab content (top-aligned)
                        ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                            match model.ui_state.active_tab {
                                UIActiveTab::Drones => {
                                    // Voices tab content with scrollable columns
                                    ui.horizontal(|ui| {
                                        // Voice 0 - column with scrollable content
                                        ui.vertical(|ui| {
                                            ui.set_width(640.0);
                                            ui.set_min_height(height);
                                            if model.voice_manager.has_drone(VoiceId::Voice0) {
                                                let voice0_commands = drone_panel::render_drone_voice_panel(
                                                    ui,
                                                    &voice0_params,
                                                    "Voice 0: Drone",
                                                    "voice0_scroll",
                                                    "voice0_circles_scroll",
                                                );
                                                command_queue.extend(voice0_commands);
                                            } else {
                                                ui.heading("Voice 0: Drone");
                                                ui.add_space(10.0);
                                                ui.label("Voice not active");
                                            }
                                        }); // end Voice 0 column


                                        // Voice 3 - column with scrollable content
                                        ui.vertical(|ui| {
                                            ui.set_width(640.0);
                                            ui.set_min_height(height);
                                            if model.voice_manager.has_drone(VoiceId::Voice3) {
                                                let voice3_commands = drone_panel::render_drone_voice_panel(
                                                    ui,
                                                    &voice3_params,
                                                    "Voice 3: Drone",
                                                    "voice3_scroll",
                                                    "voice3_circles_scroll",
                                                );
                                                command_queue.extend(voice3_commands);
                                            } else {
                                                ui.heading("Voice 3: Drone");
                                                ui.add_space(10.0);
                                                ui.label("Voice not active");
                                            }
                                        }); // end Voice 3 column
                                    }); // end voices horizontal layout
                                }
                                UIActiveTab::Terminal => {
                                    // Terminal tab content - 4 per-voice terminals in columns
                                    ui.vertical(|ui| {
                                        ui.heading("Terminals (per voice)");
                                        ui.add_space(6.0);

                                        ui.horizontal(|ui| {
                                            let voices = [
                                                VoiceId::Voice0,
                                                VoiceId::Voice1,
                                                VoiceId::Voice2,
                                                VoiceId::Voice3,
                                            ];

                                            for (i, voice) in voices.iter().copied().enumerate() {
                                                ui.vertical(|ui| {
                                                    ui.set_width(300.0);
                                                    ui.set_min_height(height * 0.55);

                                                    egui::Frame::none()
                                                        .fill(egui::Color32::from_rgb(10, 10, 10))
                                                        .stroke(egui::Stroke::new(
                                                            1.0,
                                                            egui::Color32::from_rgb(60, 60, 60),
                                                        ))
                                                        .inner_margin(egui::style::Margin::symmetric(
                                                            8.0, 8.0,
                                                        ))
                                                        .show(ui, |ui| {
                                                            ui.label(format!(
                                                                "Voice {} terminal",
                                                                voice.to_i32()
                                                            ));
                                                            ui.add_space(5.0);

                                                            // Render editor and capture edit/execute events.
                                                            let (changed, display_text, executed_cmd) = {
                                                                let input = model
                                                                    .ui_state
                                                                    .command_inputs
                                                                    .get_mut(&voice)
                                                                    .expect("missing command input for voice");

                                                                let response = ui.add(
                                                                    egui::TextEdit::multiline(input)
                                                                        .id_source(format!(
                                                                            "terminal_input_{}",
                                                                            voice.to_i32()
                                                                        ))
                                                                        .font(egui::TextStyle::Body)
                                                                        .frame(true)
                                                                        .min_size(egui::vec2(220.0, 120.0))
                                                                        .interactive(true)
                                                                        .desired_width(f32::INFINITY)
                                                                        .lock_focus(true)
                                                                        .hint_text(
                                                                            "Type command (no voice() prefix)...",
                                                                        ),
                                                                );

                                                                let should_execute = response.has_focus()
                                                                    && ui.input(|i| {
                                                                        i.key_pressed(egui::Key::Enter)
                                                                    })
                                                                    && input.is_ready_for_execution();

                                                                let executed = if should_execute {
                                                                    input.try_execute_as_voice(voice)
                                                                } else {
                                                                    None
                                                                };

                                                                let display = input.display().to_string();

                                                                // Clear editor after a successful execution.
                                                                if executed.is_some() {
                                                                    input.clear();
                                                                }

                                                                (response.changed(), display, executed)
                                                            };

                                                            // Live overlay update.
                                                            if changed {
                                                                let now = Instant::now();
                                                                if display_text.trim().is_empty() {
                                                                    model
                                                                        .ui_state
                                                                        .text_overlay
                                                                        .borrow_mut()
                                                                        .clear_live_slot(
                                                                            TextPaneId::Voice(voice),
                                                                            TextSlot::CommandInput,
                                                                        );
                                                                } else {
                                                                    model
                                                                        .ui_state
                                                                        .text_overlay
                                                                        .borrow_mut()
                                                                        .set_live_block(
                                                                            TextPaneId::Voice(voice),
                                                                            TextSlot::CommandInput,
                                                                            TextBlock::new(display_text)
                                                                                .style(TextStyle::Normal)
                                                                                .fade(TextFadeMode::NoFade)
                                                                                .wrap(WrapPolicy::HardWrap),
                                                                            now,
                                                                        );
                                                                }
                                                            }

                                                            // If we executed a command, queue it and clear the live slot.
                                                            if let Some(command) = executed_cmd {
                                                                terminal_commands_to_process.push(command);
                                                                model
                                                                    .ui_state
                                                                    .text_overlay
                                                                    .borrow_mut()
                                                                    .clear_live_slot(
                                                                        TextPaneId::Voice(voice),
                                                                        TextSlot::CommandInput,
                                                                    );
                                                            }

                                                            // Status line.
                                                            if let Some(input) =
                                                                model.ui_state.command_inputs.get(&voice)
                                                            {
                                                                ui.horizontal(|ui| {
                                                                    ui.label("Status:");

                                                                    if let Some(success) = input.last_success() {
                                                                        ui.colored_label(
                                                                            egui::Color32::GREEN,
                                                                            success.to_string(),
                                                                        );
                                                                    } else if let Some(error) = input.last_error() {
                                                                        ui.colored_label(
                                                                            egui::Color32::RED,
                                                                            format!("Error: {}", error),
                                                                        );
                                                                    } else if input.is_ready_for_execution() {
                                                                        ui.colored_label(
                                                                            egui::Color32::LIGHT_GREEN,
                                                                            "Ready (press Enter)",
                                                                        );
                                                                    } else {
                                                                        ui.colored_label(
                                                                            egui::Color32::GRAY,
                                                                            "Editing",
                                                                        );
                                                                    }
                                                                });
                                                            }
                                                        });
                                                });

                                                if i + 1 < voices.len() {
                                                    ui.separator();
                                                }
                                            }
                                        });

                                        ui.add_space(12.0);
                                        ui.separator();

                                        // Keep existing AI controls + Examples below.
                                        ui.horizontal(|ui| {
                                            // Middle column: AI controls
                                            ui.vertical(|ui| {
                                                ui.set_width(260.0);
                                                ui.set_min_height(height * 0.35);
                                                ui.heading("AI Controls");
                                                ui.add_space(8.0);

                                                ui.checkbox(
                                                    &mut model.ui_state.auto_ai_from_voice1,
                                                    "Auto-generate AI rhythm when Voice1 changes",
                                                );

                                                ui.add_space(12.0);

                                                if ui.button("Generate Voice2").clicked() {
                                                    terminal_commands_to_process.push(
                                                        TerminalCommand::GenerateRhythm { voice_id: 2 },
                                                    );
                                                }

                                                ui.add_space(8.0);

                                                if ui.button("Clear Voice2").clicked() {
                                                    terminal_commands_to_process
                                                        .push(TerminalCommand::Clear { voice_id: 2 });
                                                }
                                            });

                                            ui.separator();

                                            // Right column: Examples and help
                                            ui.vertical(|ui| {
                                                ui.set_width(650.0);
                                                ui.set_min_height(height * 0.35);
                                                ui.heading("Examples");
                                                ui.add_space(2.0);
                                                egui::ScrollArea::vertical()
                                                    .id_source("terminal_help_scroll")
                                                    .auto_shrink([false, false])
                                                    .show(ui, |ui| {
                                                        for example in CommandInput::get_examples() {
                                                            ui.label(format!("• {}", example));
                                                            ui.add_space(2.0);
                                                        }

                                                        ui.add_space(20.0);
                                                        ui.heading("Controls");
                                                        ui.add_space(5.0);
                                                        ui.label(
                                                            "• Type commands and press Enter to add lines",
                                                        );
                                                        ui.label("• Commands ending with ';' will execute");
                                                        ui.label("• Backspace to edit, Escape to clear");
                                                    });
                                            });
                                        });
                                    });
                                }

                                // Debug UI panel
                                UIActiveTab::Debug => {
                                    ui.vertical(|ui| {
                                        ui
                                            .checkbox(&mut model.ui_state.show_bounds, "Bounding Box")
                                            .changed();
                                        let show_engine_debug_changed = ui
                                            .checkbox(&mut model.engine_debug, "Particle Engine Logs")
                                            .changed();

                                        if show_engine_debug_changed {
                                            model.render_state.set_render_engines_debug(model.engine_debug);
                                        }
                                        
                                        ui.add_space(12.0);
                                    });

                                    ui.vertical(|ui| {
                                        let audience_window = app.window(model.render_state.audience_window_id).unwrap().rect();

                                        ui.label(format!("Render size:           {} x {}", model.render_state.render_size.x, model.render_state.render_size.y));
                                        ui.label(format!("Audience window size:  {} x {}", audience_window.w(), audience_window.h()));
                                        ui.label(format!("Particle system size:  {} x {}", model.particle_system.size().x, model.particle_system.size().y));
                                        ui.label(format!("Wind field size:       {} x {}", model.particle_system.force_fields.wind_field.size().x, model.particle_system.force_fields.wind_field.size().y));


                                    });
                                }
                            }
                        }); // end top-aligned layout
                    });
            }); // end main horizontal layout
        });

    drop(ctx);

    // Process any terminal commands that were collected during the UI update
    for terminal_command in terminal_commands_to_process {
        model.process_terminal_command(terminal_command);
    }

    // Queue all UI voice commands for priority processing
    for command in command_queue {
        model.queue_command(command);
    }
}

fn adjust_style_from(style: egui::Style) -> egui::Style {
    let mut style = style;
    // Set font sizes for different text styles
    style.text_styles = [
        (
            egui::TextStyle::Heading,
            egui::FontId::new(15.0, egui::FontFamily::Monospace),
        ),
        (
            egui::TextStyle::Body,
            egui::FontId::new(13.0, egui::FontFamily::Monospace),
        ),
        (
            egui::TextStyle::Button,
            egui::FontId::new(13.0, egui::FontFamily::Monospace),
        ),
        (
            egui::TextStyle::Small,
            egui::FontId::new(11.0, egui::FontFamily::Monospace),
        ),
    ]
    .into_iter()
    .collect();

    // Increase spacing for better usability
    style.spacing.item_spacing = egui::vec2(10.0, 5.0);
    style.spacing.button_padding = egui::vec2(8.0, 2.0);
    style.spacing.slider_width = 150.0; // Make sliders wider

    // Set colors
    let mut visuals = style.visuals.clone();
    visuals.widgets.active.bg_fill = egui::Color32::from_rgb(120, 120, 120); // Active button color
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(70, 70, 70); // Hover color
    visuals.window_fill = egui::Color32::from_rgba_unmultiplied(0, 0, 0, 0); // Window background
    visuals.window_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(10, 10, 10));

    style.visuals = visuals;

    style
}
