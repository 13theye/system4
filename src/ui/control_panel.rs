use super::params::DroneVoiceParams;
use super::voice_panel;
use crate::command_engine::Command;
use crate::groups::VoiceId;
use crate::model::Model;
use crate::terminals::{command_input::CommandInput, commands::TerminalCommand};
use nannou::App;

pub fn update_control_ui(app: &App, model: &mut Model) {
    let Some(control_window) = app.window(model.render_state.control_window_id) else {
        eprintln!("Control window not found. Exiting app.");
        std::process::exit(1);
    };
    let rect = control_window.rect();
    let height = rect.h() - 5.0;
    let width = rect.w() - 5.0;

    // Extract all parameters before creating egui context to avoid borrowing conflicts
    let voice0_params = DroneVoiceParams::extract(model, VoiceId::Voice0);
    let voice3_params = DroneVoiceParams::extract(model, VoiceId::Voice3);

    let ctx = model.ui_state.egui.begin_frame();

    // Set text style settings
    let style = (*ctx.style()).clone();
    ctx.set_style(adjust_style_from(style));

    let mut show_forces_changed = false;
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
                    ui.set_min_size(egui::vec2(150.0, height));
                    // Status info
                    ui.label(format!(
                        "Particles: {}",
                        model.particle_system.get_particle_count()
                    ));
                    // FPS
                    ui.label(format!("FPS: {:.1}", model.ui_state.fps.fps()));
                    ui.add_space(15.0);

                    show_forces_changed = ui
                        .checkbox(&mut model.ui_state.show_forces, "Show Forces")
                        .changed();
                    ui.add_space(30.0);

                    // Instructions section
                    ui.label("...");
                    ui.label("P: Debug view");

                    // Push tab selector to bottom with expanding space
                    ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                        // Tab bar at bottom
                        ui.add_space(20.0);
                        ui.horizontal(|ui| {
                            if ui
                                .selectable_label(model.ui_state.active_tab == 0, "Terminal")
                                .clicked()
                            {
                                model.ui_state.active_tab = 0;
                            }
                            if ui
                                .selectable_label(model.ui_state.active_tab == 1, "Voices")
                                .clicked()
                            {
                                model.ui_state.active_tab = 1;
                            }
                        });
                    });
                });

                ui.separator();

                // Tab content (top-aligned)
                ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    match model.ui_state.active_tab {
                        1 => {
                            // Voices tab content with scrollable columns
                            ui.horizontal(|ui| {
                                // Voice 0 (col 2) - column with scrollable content
                                ui.vertical(|ui| {
                                    ui.set_width(320.0);
                                    ui.set_min_height(height);
                                    if model.voice_manager.has_voice(VoiceId::Voice0) {
                                        let voice0_commands = voice_panel::render_drone_voice_panel(
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

                                // Voice 1 (col 3) - column with scrollable content
                                ui.vertical(|ui| {
                                    ui.set_width(320.0);
                                    ui.set_min_height(height);
                                    ui.heading("Voice 1: Rhythm");
                                    ui.add_space(2.0);
                                    egui::ScrollArea::vertical()
                                        .id_source("voice1_scroll")
                                        .auto_shrink([false, false])
                                        .show(ui, |_ui| {}); // end Voice 2 scroll area
                                }); // end Voice 2 column

                                // Voice 2 - column with scrollable content
                                ui.vertical(|ui| {
                                    ui.set_width(320.0);
                                    ui.set_min_height(height);
                                    ui.heading("Voice 2: Rhythm");
                                    ui.add_space(2.0);
                                    egui::ScrollArea::vertical()
                                        .id_source("voice2_scroll")
                                        .auto_shrink([false, false])
                                        .show(ui, |ui| {
                                            // Voice 3 rhythm controls placeholder
                                            ui.label("Rhythm controls");
                                            ui.label("coming soon...");
                                        }); // end Voice 2 scroll area
                                }); // end Voice 2 column

                                // Voice 3: Column 5 - column with scrollable content
                                ui.vertical(|ui| {
                                    ui.set_width(320.0);
                                    ui.set_min_height(height);
                                    if model.voice_manager.has_voice(VoiceId::Voice3) {
                                        let voice3_commands = voice_panel::render_drone_voice_panel(
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
                        0 => {
                            // NTerminal tab content - three column layout with scrollbars
                            ui.horizontal(|ui| {
                                // Left column: Command input and status - column with scrollable content
                                ui.vertical(|ui| {
                                    ui.set_width(480.0);
                                    ui.set_min_height(height);
                                    ui.heading("Terminal Interface");
                                    ui.add_space(2.0);

                                    egui::ScrollArea::vertical()
                                        .id_source("terminal_input_scroll")
                                        .auto_shrink([false, false])
                                        .show(ui, |ui| {
                                            // Multi-line text input using TextBuffer implementation
                                            ui.label("Input:");
                                            ui.add_space(5.0);

                                            // Editable text area using CommandInput as TextBuffer
                                            egui::Frame::none()
                                                .fill(egui::Color32::BLACK)
                                                .stroke(egui::Stroke::new(
                                                    1.0,
                                                    egui::Color32::WHITE,
                                                ))
                                                .inner_margin(egui::style::Margin::symmetric(
                                                    8.0, 8.0,
                                                ))
                                                .show(ui, |ui| {
                                                    ui.set_min_size(egui::vec2(280.0, 150.0));
                                                    ui.set_max_height(150.0);

                                                    egui::ScrollArea::vertical()
                                                        .max_width(380.0)
                                                        .max_height(150.0)
                                                        .show(ui, |ui| {
                                                            // Use TextEdit with proper Enter key handling
                                                            let response = ui.add(
                                                                egui::TextEdit::multiline(
                                                                    &mut model
                                                                        .ui_state
                                                                        .command_input,
                                                                )
                                                                .font(egui::TextStyle::Body)
                                                                .frame(false)
                                                                .min_size(egui::vec2(280.0, 150.0))
                                                                .interactive(true)
                                                                .desired_width(f32::INFINITY)
                                                                .lock_focus(true)
                                                                .hint_text("Type command here..."),
                                                            );

                                                            // Update terminal display with live command text
                                                            if response.changed() {
                                                                model
                                                                    .ui_state
                                                                    .terminal_manager
                                                                    .borrow_mut()
                                                                    .update_from_command_input(
                                                                        "main",
                                                                        &model
                                                                            .ui_state
                                                                            .command_input,
                                                                    );
                                                            }

                                                            // Handle Enter key press through egui input system
                                                            // Check for Enter key pressed while the text field has focus
                                                            if response.has_focus()
                                                                && ui.input(|i| {
                                                                    i.key_pressed(egui::Key::Enter)
                                                                })
                                                                && model
                                                                    .ui_state
                                                                    .command_input
                                                                    .is_ready_for_execution()
                                                            {
                                                                if let Some(command) = model
                                                                    .ui_state
                                                                    .command_input
                                                                    .try_execute()
                                                                {
                                                                    println!(
                                                                        "Executing command: {:?}",
                                                                        command
                                                                    );

                                                                    // Collect terminal command for processing after egui context is dropped
                                                                    terminal_commands_to_process
                                                                        .push(command);

                                                                    // Clear the input after successful execution
                                                                    model
                                                                        .ui_state
                                                                        .command_input
                                                                        .clear();

                                                                    // Clear the terminal display
                                                                    model
                                                                        .ui_state
                                                                        .terminal_manager
                                                                        .borrow_mut()
                                                                        .clear_terminal_view(
                                                                            "main",
                                                                        );
                                                                }
                                                            }
                                                        });
                                                });

                                            ui.add_space(10.0);
                                            ui.separator();

                                            // Command status display
                                            ui.horizontal(|ui| {
                                                ui.label("Status:");

                                                // Priority: Show execution results first
                                                if let Some(success) =
                                                    model.ui_state.command_input.last_success()
                                                {
                                                    ui.colored_label(
                                                        egui::Color32::GREEN,
                                                        format!("✅ {}", success),
                                                    );
                                                } else if let Some(error) =
                                                    model.ui_state.command_input.last_error()
                                                {
                                                    ui.colored_label(
                                                        egui::Color32::RED,
                                                        format!("❌ Error: {}", error),
                                                    );
                                                } else if model
                                                    .ui_state
                                                    .command_input
                                                    .is_ready_for_execution()
                                                {
                                                    ui.colored_label(
                                                        egui::Color32::LIGHT_GREEN,
                                                        "Ready to execute (press Enter)",
                                                    );
                                                } else if model.ui_state.command_input.is_empty() {
                                                    ui.colored_label(
                                                        egui::Color32::GRAY,
                                                        "Ready for input",
                                                    );
                                                } else {
                                                    ui.colored_label(
                                                        egui::Color32::YELLOW,
                                                        "Add semicolon (;) to execute",
                                                    );
                                                }
                                            });

                                            // Show formatted display preview
                                            ui.add_space(5.0);
                                            ui.label("Preview:");
                                            ui.add_space(2.0);

                                            egui::Frame::none()
                                                .fill(egui::Color32::DARK_GRAY)
                                                .stroke(egui::Stroke::new(1.0, egui::Color32::GRAY))
                                                .inner_margin(egui::style::Margin::symmetric(
                                                    6.0, 6.0,
                                                ))
                                                .show(ui, |ui| {
                                                    let display_text =
                                                        model.ui_state.command_input.display();
                                                    if display_text.is_empty() {
                                                        ui.colored_label(
                                                            egui::Color32::GRAY,
                                                            "Command preview will appear here...",
                                                        );
                                                    } else {
                                                        ui.label(display_text);
                                                    }
                                                });
                                        }); // end left column scroll area
                                }); // end left column

                                ui.separator();

                                // Middle column: AI controls
                                ui.vertical(|ui| {
                                    ui.set_width(260.0);
                                    ui.set_min_height(height);
                                    ui.heading("AI Controls");
                                    ui.add_space(8.0);

                                    ui.checkbox(
                                        &mut model.ui_state.auto_ai_from_voice1,
                                        "Auto-generate AI rhythm when Voice1 changes",
                                    );

                                    ui.add_space(12.0);

                                    if ui.button("Generate Voice2").clicked() {
                                        // Equivalent of typing: voice(2).generate();
                                        terminal_commands_to_process
                                            .push(TerminalCommand::GenerateRhythm { voice_id: 2 });
                                    }

                                    ui.add_space(8.0);

                                    if ui.button("Clear Voice2").clicked() {
                                        // Equivalent of typing: voice(2).clear();
                                        terminal_commands_to_process
                                            .push(TerminalCommand::Clear { voice_id: 2 });
                                    }
                                }); // end middle column

                                ui.separator();

                                // Right column: Examples and help - column with scrollable content
                                ui.vertical(|ui| {
                                    ui.set_width(550.0);
                                    ui.set_min_height(height);
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
                                        }); // end right column scroll area
                                }); // end right column
                            }); // end terminal horizontal layout
                        }
                        _ => {}
                    }
                }); // end top-aligned layout
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
