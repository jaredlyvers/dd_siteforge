//! FormEdit textarea layout helpers.
use super::*;
pub(super) fn focused_field_virtual_rows(state: &editform::EditFormState) -> (u16, u16) {
    let mut y: u16 = 0;
    for (idx, field) in state.form.fields.iter().enumerate() {
        if !state.field_visible(field) {
            continue;
        }
        let content_rows: u16 = match &field.kind {
            editform::FieldKind::Textarea { rows, .. } => {
                textarea_display_rows(
                    state.get(field.id),
                    (*rows).max(1),
                    None,
                    TEXTAREA_MAX_DISPLAY_ROWS,
                )
            }
            editform::FieldKind::SubForm { .. } => {
                let items_len = state
                    .sub_state
                    .get(field.id)
                    .map(|v| v.len())
                    .unwrap_or(0);
                (1 + items_len.max(1)) as u16
            }
            _ => 1,
        };
        let box_height = content_rows.saturating_add(2);
        let entry_height = 1u16.saturating_add(box_height).saturating_add(1);
        if idx == state.focused_field {
            return (y, y.saturating_add(1).saturating_add(box_height));
        }
        y = y.saturating_add(entry_height);
    }
    (0, 0)
}

pub(super) fn textarea_display_rows(
    value: &str,
    base_rows: u16,
    wrap_width: Option<u16>,
    max_rows: u16,
) -> u16 {
    let content_rows = textarea_visual_line_count(value, wrap_width).min(u16::MAX as usize) as u16;
    base_rows
        .max(content_rows.max(1))
        .min(max_rows.max(1))
}

pub(super) fn textarea_max_rows_for_window(content_height: u16) -> u16 {
    content_height
        .saturating_sub(3)
        .max(1)
        .min(TEXTAREA_MAX_DISPLAY_ROWS)
}

pub(super) fn textarea_visual_line_count(value: &str, wrap_width: Option<u16>) -> usize {
    let Some(width) = wrap_width.map(|w| w.max(1) as usize) else {
        return input_lines_preserve(value).len().max(1);
    };

    input_lines_preserve(value)
        .iter()
        .map(|line| {
            let chars = line.chars().count();
            chars.div_ceil(width).max(1)
        })
        .sum::<usize>()
        .max(1)
}

#[cfg(test)]
pub(super) fn render_textarea_display(
    value: &str,
    cursor_pos: usize,
    focused: bool,
    visible_rows: usize,
) -> String {
    render_textarea_display_window(value, cursor_pos, focused, visible_rows).0
}

pub(super) fn render_textarea_display_window(
    value: &str,
    cursor_pos: usize,
    focused: bool,
    visible_rows: usize,
) -> (String, usize, usize) {
    let visible_rows = visible_rows.max(1);
    let mut lines = input_lines_preserve(value);
    if lines.is_empty() {
        lines.push(String::new());
    }

    let cursor_row = textarea_cursor_row(value, cursor_pos).min(lines.len().saturating_sub(1));
    let start = if focused {
        cursor_row.saturating_sub(visible_rows.saturating_sub(1))
    } else {
        0
    };
    let end = (start + visible_rows).min(lines.len());

    let mut display = Vec::with_capacity(visible_rows);
    for (idx, line) in lines.iter().enumerate().take(end).skip(start) {
        if focused && idx == cursor_row {
            let cursor_col = textarea_cursor_col(value, cursor_pos);
            display.push(render_cursor_line(line, cursor_col));
        } else {
            display.push(line.clone());
        }
    }
    while display.len() < visible_rows {
        display.push(String::new());
    }
    (display.join("\n"), start, lines.len())
}

pub(super) fn render_textarea_scrollbar(
    frame: &mut ratatui::Frame,
    area: Rect,
    first_visible_row: usize,
    visible_rows: usize,
    total_rows: usize,
    scrollbar_color: Color,
    background: Color,
) {
    if area.height == 0 || total_rows <= visible_rows {
        return;
    }

    for y in 0..area.height {
        frame.render_widget(
            Paragraph::new(" ").style(Style::default().bg(background)),
            Rect {
                x: area.x,
                y: area.y + y,
                width: 1,
                height: 1,
            },
        );
    }

    let track_height = area.height as usize;
    let thumb_height = ((visible_rows.max(1) * track_height) / total_rows.max(1))
        .max(1)
        .min(track_height);
    let max_scroll = total_rows.saturating_sub(visible_rows.max(1));
    let travel = track_height.saturating_sub(thumb_height);
    let thumb_top = if max_scroll == 0 {
        0
    } else {
        (first_visible_row.min(max_scroll) * travel) / max_scroll
    };

    for y in thumb_top..thumb_top + thumb_height {
        frame.render_widget(
            Paragraph::new("█").style(Style::default().fg(scrollbar_color).bg(background)),
            Rect {
                x: area.x,
                y: area.y + y as u16,
                width: 1,
                height: 1,
            },
        );
    }
}

pub(super) fn textarea_cursor_row(value: &str, cursor_pos: usize) -> usize {
    value
        .chars()
        .take(cursor_pos.min(value.chars().count()))
        .filter(|c| *c == '\n')
        .count()
}

pub(super) fn textarea_cursor_col(value: &str, cursor_pos: usize) -> usize {
    let mut col = 0;
    for c in value.chars().take(cursor_pos.min(value.chars().count())) {
        if c == '\n' {
            col = 0;
        } else {
            col += 1;
        }
    }
    col
}

pub(super) fn textarea_move_cursor_vertical(value: &str, cursor_pos: usize, row_delta: isize) -> usize {
    let lines = input_lines_preserve(value);
    let current_row = textarea_cursor_row(value, cursor_pos).min(lines.len().saturating_sub(1));
    let current_col = textarea_cursor_col(value, cursor_pos);
    let target_row = current_row
        .saturating_add_signed(row_delta)
        .min(lines.len().saturating_sub(1));

    cursor_from_row_col(&lines, target_row, current_col)
}

/// Compute a new scroll offset that keeps the focused field in view given
/// a conservative estimate of the content window height. 16 rows covers the
/// common case of an 80% / 80% modal on a standard terminal.
pub(super) fn auto_scroll_for_focus(state: &editform::EditFormState, current_scroll: u16) -> u16 {
    const ESTIMATED_VISIBLE: u16 = 16;
    let (top, bottom) = focused_field_virtual_rows(state);
    if top < current_scroll {
        top
    } else if bottom > current_scroll.saturating_add(ESTIMATED_VISIBLE) {
        bottom.saturating_sub(ESTIMATED_VISIBLE)
    } else {
        current_scroll
    }
}

/// Insert a block cursor `▋` at `cursor_pos` in `value`. Used by the form
/// editor to show where typing will land in a single-line text field.
pub(super) fn render_cursor_line(value: &str, cursor_pos: usize) -> String {
    let chars: Vec<char> = value.chars().collect();
    let pos = cursor_pos.min(chars.len());
    let mut out = String::with_capacity(value.len() + 3);
    for (i, ch) in chars.iter().enumerate() {
        if i == pos {
            out.push('▋');
        }
        out.push(*ch);
    }
    if pos >= chars.len() {
        out.push('▋');
    }
    out
}

