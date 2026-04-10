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
    let font_bytes = fs::read(&font_path)
        .unwrap_or_else(|_| panic!("Failed to read font file at {:?}", font_path));
    Font::from_bytes(font_bytes)
        .unwrap_or_else(|_| panic!("Failed to load font at {:?}", font_path))
}

struct VoicePaneConfig {
    voice: VoiceId,
    anchor: TextBoxAnchor,
    anchor_pos: Vec2,
    width: f32,
    num_lines: usize,
    font_size: u32,
    justify: HorizontalJustify,
    theme: TextTheme,
}

impl Default for VoicePaneConfig {
    fn default() -> Self {
        Self {
            voice: VoiceId::Voice0,
            anchor: TextBoxAnchor::TopLeft,
            anchor_pos: Vec2::ZERO,
            width: 200.0,
            num_lines: 8,
            font_size: 28,
            justify: HorizontalJustify::Left,
            theme: TextTheme::default(),
        }
    }
}

impl VoicePaneConfig {
    /// Get presets for each voice
    pub fn get(voice_id: VoiceId, anchor_pos: Vec2, width: f32) -> Self {
        match voice_id {
            VoiceId::Voice0 => VoicePaneConfig {
                voice: VoiceId::Voice0,
                anchor: TextBoxAnchor::TopLeft,
                anchor_pos,
                width,
                justify: HorizontalJustify::Left,
                ..Default::default()
            },
            VoiceId::Voice1 => VoicePaneConfig {
                voice: VoiceId::Voice1,
                anchor: TextBoxAnchor::TopLeft,
                anchor_pos,
                width,
                justify: HorizontalJustify::Right,
                ..Default::default()
            },
            VoiceId::Voice2 => VoicePaneConfig {
                voice: VoiceId::Voice2,
                anchor: TextBoxAnchor::TopLeft,
                anchor_pos,
                width,
                justify: HorizontalJustify::Left,
                ..Default::default()
            },
            VoiceId::Voice3 => VoicePaneConfig {
                voice: VoiceId::Voice3,
                anchor: TextBoxAnchor::TopLeft,
                anchor_pos,
                width,
                justify: HorizontalJustify::Right,
                ..Default::default()
            },
            VoiceId::Invalid => VoicePaneConfig::default(),
        }
    }
}

pub fn init_text_overlay(render_size: Vec2, font: &Font) -> TextOverlay {
    // Create new unified text overlay for per-voice panes.
    let mut text_overlay = TextOverlay::new();

    // Arrange voice panes as four columns across the full render width.
    let gutter_x = 100.0;
    let gutter_y = 100.0;
    let columns = 4.0;

    let col_w = render_size.x / columns;
    let pane_w = col_w - (gutter_x * 2.0);

    let left_edge = -render_size.x / 2.0;
    let top_y = render_size.y / 2.0 - gutter_y;

    VoiceId::all().iter().enumerate().for_each(|(i, voice_id)| {
        let x0 = left_edge + i as f32 * col_w + gutter_x;
        let voice_pane_config = VoicePaneConfig::get(*voice_id, vec2(x0, top_y), pane_w);
        add_voice_pane(&mut text_overlay, font, voice_pane_config);
    });

    text_overlay
}

fn add_voice_pane(text_overlay: &mut TextOverlay, font: &Font, config: VoicePaneConfig) {
    let params_line_count = params_dashboard::default_tracked_keys().len();
    let command_input_budget = 3;
    let ai_stream_budget = 2;
    // Ensure the pane has room for params + AiStream + CommandInput + ≥1 history slot.
    let num_lines = config
        .num_lines
        .max(params_line_count + ai_stream_budget + command_input_budget + 1);

    let line_spacing = 5.0;
    let line_height = config.font_size as f32 + line_spacing * 2.0;

    let layout = TextBoxLayout::new(TextBoxLayoutParams {
        anchor: config.anchor,
        anchor_pos: config.anchor_pos,
        width: config.width,
        num_lines,
        line_height,
        vertical_flow: VerticalFlow::TopDown,
        horizontal_justify: config.justify,
    });

    let view = TextPaneView::new(TextPaneViewParams {
        layout,
        font: font.clone(),
        font_size: config.font_size,
        line_spacing,
        fade_delay_secs: 1.0,
        color_fade_secs: 1.5,
        chars_per_second: 0.0, // default: show immediately for overlay panes
        theme: config.theme,
    });

    let mut pane = TextPane::new(num_lines);
    pane.set_slot_line_budget(TextSlot::Params, params_line_count);
    pane.set_slot_line_budget(TextSlot::CommandInput, command_input_budget);
    pane.set_slot_line_budget(TextSlot::AiStream, ai_stream_budget);

    text_overlay.insert_pane(TextPaneId::Voice(config.voice), pane, view);
}
