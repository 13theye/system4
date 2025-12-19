use nannou::{prelude::*, text::Font, App};

use std::fs;

use system4::{
    groups::VoiceId,
    text::{
        layout::{
            HorizontalJustify, TextBoxAnchor, TextBoxLayout, TextBoxLayoutParams, VerticalFlow,
        },
        overlay::TextOverlay,
        params_dashboard,
        view::{TextPaneView, TextPaneViewParams, TextTheme},
        TextPane, TextPaneId, TextSlot,
    },
};

pub fn load_font(app: &App) -> Font {
    // Assumes "assets/terminal_font.ttf" exists relative to the executable
    // or relative to the project root if running with `cargo run`
    let assets = app.assets_path().expect("Could not find assets directory");
    let font_path = assets.join("terminal_font.ttf");
    let font_bytes =
        fs::read(&font_path).unwrap_or_else(|_| panic!("Failed to read font file at {:?}", font_path));
    Font::from_bytes(font_bytes).unwrap_or_else(|_| panic!("Failed to load font at {:?}", font_path))
}

pub fn init_text_overlay(render_size: Vec2, font: &Font) -> TextOverlay {
    // Create new unified text overlay for per-voice panes.
    let mut text_overlay = TextOverlay::new();

    // Ensure panes can display all pinned parameter lines + one command input line.
    let params_line_count = params_dashboard::default_tracked_keys().len();
    let min_pane_lines = params_line_count + 1;

    // Helper to create a text pane + view.
    let mut add_voice_pane = |voice: VoiceId,
                              anchor: TextBoxAnchor,
                              anchor_pos: Vec2,
                              width: f32,
                              num_lines: usize,
                              font_size: u32,
                              justify: HorizontalJustify,
                              theme: TextTheme| {
        let num_lines = num_lines.max(min_pane_lines);

        let line_spacing = 5.0;
        let line_height = font_size as f32 + line_spacing * 2.0;

        let layout = TextBoxLayout::new(TextBoxLayoutParams {
            anchor,
            anchor_pos,
            width,
            num_lines,
            line_height,
            vertical_flow: VerticalFlow::TopDown,
            horizontal_justify: justify,
        });

        let view = TextPaneView::new(TextPaneViewParams {
            layout,
            font: font.clone(),
            font_size,
            line_spacing,
            fade_delay_secs: 1.0,
            color_fade_secs: 1.5,
            chars_per_second: 0.0, // default: show immediately for overlay panes
            theme,
        });

        let mut pane = TextPane::new(num_lines);
        pane.set_slot_line_budget(TextSlot::Params, params_line_count);
        pane.set_slot_line_budget(TextSlot::CommandInput, 1);
        pane.set_slot_line_budget(TextSlot::AiStream, 2);

        text_overlay.insert_pane(TextPaneId::Voice(voice), pane, view);
    };

    // Arrange voice panes as four columns across the full render width.
    let gutter_x = 40.0;
    let gutter_y = 40.0;
    let columns = 4.0;

    let col_w = render_size.x / columns;
    let pane_w = (col_w - gutter_x).max(200.0);

    let left_edge = -render_size.x / 2.0;
    let top_y = render_size.y / 2.0 - gutter_y;

    for (i, voice) in [
        VoiceId::Voice0,
        VoiceId::Voice1,
        VoiceId::Voice2,
        VoiceId::Voice3,
    ]
    .iter()
    .copied()
    .enumerate()
    {
        let x0 = left_edge + i as f32 * col_w + gutter_x / 2.0;
        add_voice_pane(
            voice,
            TextBoxAnchor::TopLeft,
            vec2(x0, top_y),
            pane_w,
            8,
            28,
            HorizontalJustify::Left,
            TextTheme::default(),
        );
    }

    text_overlay
}
