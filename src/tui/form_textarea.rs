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

/// Glyph shown at column `col` of `line`. Space if that cell is empty
/// (caret at end of the line, or the line is shorter than `col`).
fn overlay_glyph_at(line: &str, col: usize) -> char {
    line.chars().nth(col).filter(|c| *c != '\n').unwrap_or(' ')
}

/// Map a FormEdit text caret to a 1-cell overlay inside the bordered `box_rect`.
/// Paragraph has no horizontal scroll, so a caret past the inner width overlays
/// the last visible glyph rather than painting the logical (often space) char.
pub(super) fn form_input_cursor_cell(
    kind: &editform::FieldKind,
    value: &str,
    cursor_pos: usize,
    box_rect: Rect,
) -> Option<(u16, u16, char)> {
    if box_rect.width < 3 || box_rect.height < 3 {
        return None;
    }
    let inner_x = box_rect.x.saturating_add(1);
    let inner_y = box_rect.y.saturating_add(1);
    let inner_w = box_rect.width.saturating_sub(2);
    let inner_h = box_rect.height.saturating_sub(2);
    if inner_w == 0 || inner_h == 0 {
        return None;
    }

    let pos = cursor_pos.min(value.chars().count());

    match kind {
        editform::FieldKind::Text { .. } | editform::FieldKind::Url { .. } => {
            let col = (pos as u16).min(inner_w.saturating_sub(1));
            let ch = overlay_glyph_at(value, col as usize);
            Some((inner_x.saturating_add(col), inner_y, ch))
        }
        editform::FieldKind::Textarea { .. } => {
            let visible_rows = inner_h as usize;
            let (_display, first_visible_row, total_rows) =
                render_textarea_display_window(value, pos, true, visible_rows);
            let lines = input_lines_preserve(value);
            let cursor_row =
                textarea_cursor_row(value, pos).min(lines.len().saturating_sub(1));
            if cursor_row < first_visible_row {
                return None;
            }
            let row_in_view = (cursor_row - first_visible_row) as u16;
            if row_in_view >= inner_h {
                return None;
            }
            let text_w = if total_rows > visible_rows {
                inner_w.saturating_sub(1)
            } else {
                inner_w
            };
            if text_w == 0 {
                return None;
            }
            let cursor_col = textarea_cursor_col(value, pos);
            let col = (cursor_col as u16).min(text_w.saturating_sub(1));
            let line = lines.get(cursor_row).map(|s| s.as_str()).unwrap_or("");
            let ch = overlay_glyph_at(line, col as usize);
            Some((
                inner_x.saturating_add(col),
                inner_y.saturating_add(row_in_view),
                ch,
            ))
        }
        _ => None,
    }
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
    for line in lines.iter().take(end).skip(start) {
        display.push(line.clone());
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
    paint_scrollbar(
        frame,
        area,
        first_visible_row,
        total_rows,
        visible_rows,
        scrollbar_color,
        scrollbar_color,
        background,
    );
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

