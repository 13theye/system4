// src/text/layout.rs
//
// General text layout utilities.

use nannou::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextBoxAnchor {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Center,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerticalFlow {
    /// First line is at the top; subsequent lines go downward.
    TopDown,
    /// First line is at the bottom; subsequent lines go upward.
    BottomUp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HorizontalJustify {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy)]
pub struct TextBoxLayoutParams {
    pub anchor: TextBoxAnchor,
    pub anchor_pos: Vec2,
    pub width: f32,
    pub num_lines: usize,
    pub line_height: f32,
    pub vertical_flow: VerticalFlow,
    pub horizontal_justify: HorizontalJustify,
}

#[derive(Debug, Clone)]
pub struct TextBoxLayout {
    pub rect: Rect,
    pub line_positions: Vec<Vec2>,
    pub width: f32,
    pub horizontal_justify: HorizontalJustify,
}

impl TextBoxLayout {
    pub fn new(params: TextBoxLayoutParams) -> Self {
        let height = params.line_height * params.num_lines as f32;
        let rect = rect_from_anchor(params.anchor, params.anchor_pos, params.width, height);
        let line_positions = generate_line_positions(rect, params.num_lines, params.line_height, params.vertical_flow);

        Self {
            rect,
            line_positions,
            width: params.width,
            horizontal_justify: params.horizontal_justify,
        }
    }

    pub fn num_lines(&self) -> usize {
        self.line_positions.len()
    }

    pub fn line_position(&self, idx: usize) -> Option<Vec2> {
        self.line_positions.get(idx).copied()
    }
}

fn rect_from_anchor(anchor: TextBoxAnchor, anchor_pos: Vec2, width: f32, height: f32) -> Rect {
    match anchor {
        TextBoxAnchor::TopLeft => {
            Rect::from_x_y_w_h(anchor_pos.x + width / 2.0, anchor_pos.y - height / 2.0, width, height)
        }
        TextBoxAnchor::TopRight => {
            Rect::from_x_y_w_h(anchor_pos.x - width / 2.0, anchor_pos.y - height / 2.0, width, height)
        }
        TextBoxAnchor::BottomLeft => {
            Rect::from_x_y_w_h(anchor_pos.x + width / 2.0, anchor_pos.y + height / 2.0, width, height)
        }
        TextBoxAnchor::BottomRight => {
            Rect::from_x_y_w_h(anchor_pos.x - width / 2.0, anchor_pos.y + height / 2.0, width, height)
        }
        TextBoxAnchor::Center => Rect::from_x_y_w_h(anchor_pos.x, anchor_pos.y, width, height),
    }
}

fn generate_line_positions(rect: Rect, num_lines: usize, line_height: f32, flow: VerticalFlow) -> Vec<Vec2> {
    let mut out = Vec::with_capacity(num_lines);

    match flow {
        VerticalFlow::TopDown => {
            // Position lines starting from top edge going down.
            for i in 0..num_lines {
                let y = rect.top() - (i as f32 + 0.5) * line_height;
                out.push(vec2(rect.left(), y));
            }
        }
        VerticalFlow::BottomUp => {
            // Position lines starting from bottom edge going up.
            for i in 0..num_lines {
                let y = rect.bottom() + (i as f32 + 0.5) * line_height;
                out.push(vec2(rect.left(), y));
            }
        }
    }

    out
}
