// src/text/line_breaker.rs
//
// Split TextBlock into visual lines.
//
// This is intentionally simple for now. It can be upgraded later to use
// glyph-metric measurement.

use crate::text::{TextBlock, TextFadeMode, TextLine, TextStyle, WrapPolicy};
use std::time::Instant;

/// Estimate how many characters fit on a line for a given width and font size.
///
/// This is a heuristic; the goal is deterministic, stable wrapping.
pub fn estimate_max_chars_per_line(width: f32, font_size: u32) -> usize {
    if width <= 0.0 || font_size == 0 {
        return 0;
    }

    // Rough average glyph width.
    let avg_char_w = font_size as f32 * 0.6;
    ((width / avg_char_w).floor() as usize).max(8)
}

pub fn break_block(
    block: &TextBlock,
    now: Instant,
    max_chars_per_line: usize,
) -> Vec<TextLine> {
    // Optional JSON pretty print.
    let mut text = block.text.clone();
    if matches!(block.wrap, WrapPolicy::JsonPrettyPrintIfValid) {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(text.trim()) {
            if let Ok(pretty) = serde_json::to_string_pretty(&val) {
                text = pretty;
            }
        }
    }

    // First split on explicit newlines.
    let mut raw_lines: Vec<String> = text
        .split('\n')
        .map(|s| s.replace('\t', "    "))
        .collect();

    // Apply wrapping within each raw line.
    let mut wrapped: Vec<String> = Vec::new();
    for raw in raw_lines.drain(..) {
        match block.wrap {
            WrapPolicy::HardWrap | WrapPolicy::JsonPrettyPrintIfValid => {
                wrapped.extend(wrap_line(&raw, max_chars_per_line));
            }
            WrapPolicy::WordWrap => {
                wrapped.extend(wrap_line(&raw, max_chars_per_line));
            }
            WrapPolicy::TruncateTail { .. }
            | WrapPolicy::TruncateHead { .. }
            | WrapPolicy::TruncateMiddle { .. } => {
                // Truncation is applied after wrapping.
                wrapped.extend(wrap_line(&raw, max_chars_per_line));
            }
        }
    }

    // Apply truncation policies after wrapping.
    let truncated: Vec<String> = match block.wrap {
        WrapPolicy::TruncateTail { max_lines } => truncate_tail(wrapped, max_lines),
        WrapPolicy::TruncateHead { max_lines } => truncate_head(wrapped, max_lines),
        WrapPolicy::TruncateMiddle { head, tail } => truncate_middle(wrapped, head, tail),
        _ => wrapped,
    };

    truncated
        .into_iter()
        .map(|s| TextLine::new(s, block.style, block.fade, now))
        .collect()
}

fn wrap_line(line: &str, max_chars: usize) -> Vec<String> {
    if max_chars == 0 {
        return Vec::new();
    }

    let line = line.trim_end_matches('\r');
    if line.len() <= max_chars {
        return vec![line.to_string()];
    }

    // Prefer whitespace breaks. This is not perfect for unicode width, but stable.
    let mut out = Vec::new();
    let mut current = String::new();

    for word in line.split_whitespace() {
        let needed = if current.is_empty() {
            word.len()
        } else {
            current.len() + 1 + word.len()
        };

        if needed > max_chars {
            if !current.is_empty() {
                out.push(current);
                current = String::new();
            }

            // Word itself too long: hard-split.
            if word.len() > max_chars {
                out.extend(hard_split(word, max_chars));
            } else {
                current.push_str(word);
            }
        } else {
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(word);
        }
    }

    if !current.is_empty() {
        out.push(current);
    }

    if out.is_empty() {
        vec![String::new()]
    } else {
        out
    }
}

fn hard_split(s: &str, max_chars: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut chunk = String::new();

    for ch in s.chars() {
        chunk.push(ch);
        if chunk.chars().count() >= max_chars {
            out.push(chunk);
            chunk = String::new();
        }
    }

    if !chunk.is_empty() {
        out.push(chunk);
    }

    out
}

fn truncate_tail(mut lines: Vec<String>, max_lines: usize) -> Vec<String> {
    if max_lines == 0 {
        return Vec::new();
    }
    if lines.len() <= max_lines {
        return lines;
    }
    lines.drain(0..lines.len() - max_lines);
    lines
}

fn truncate_head(lines: Vec<String>, max_lines: usize) -> Vec<String> {
    if max_lines == 0 {
        return Vec::new();
    }
    if lines.len() <= max_lines {
        return lines;
    }
    lines.into_iter().take(max_lines).collect()
}

fn truncate_middle(lines: Vec<String>, head: usize, tail: usize) -> Vec<String> {
    if lines.len() <= head + tail + 1 {
        return lines;
    }

    let mut out: Vec<String> = Vec::new();
    out.extend(lines.iter().take(head).cloned());
    out.push("…".to_string());
    out.extend(lines.iter().skip(lines.len() - tail).cloned());
    out
}

// Silence unused imports if this module is used without some variants yet.
#[allow(dead_code)]
fn _touch(_s: TextStyle, _f: TextFadeMode) {}
