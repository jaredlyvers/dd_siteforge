//! Shared │/█ scrollbar painter and click-to-offset mapping.
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::Paragraph;

/// Track rect plus the content metrics captured at paint, used for hit-testing.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct ScrollbarTrack {
    pub rect: Rect,
    pub total: usize,
    pub visible: usize,
}

/// Which painted scrollbar currently owns a mouse drag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ScrollbarDrag {
    Details,
    Layout,
    Help,
    Theme,
    FormEdit,
}

/// Paint a `│` track and `█` thumb.
///
/// Thumb height is the Details formula `(track_h * visible) / total`, clamped
/// to `1..=track_h` — not Help/Theme's old `visible² / total` (which only
/// matched when track height equalled the visible row count).
///
/// No-op when `total <= visible` or the track has no area.
pub(super) fn paint_scrollbar(
    frame: &mut ratatui::Frame,
    track: Rect,
    offset: usize,
    total: usize,
    visible: usize,
    track_fg: Color,
    thumb_fg: Color,
    bg: Color,
) {
    if track.width == 0 || track.height == 0 || total <= visible {
        return;
    }

    let track_h = track.height as usize;
    for row in 0..track.height {
        frame.render_widget(
            Paragraph::new("│").style(Style::default().fg(track_fg).bg(bg)),
            Rect {
                x: track.x,
                y: track.y + row,
                width: 1,
                height: 1,
            },
        );
    }

    let thumb_h = ((track_h * visible) / total).max(1).min(track_h);
    let scroll_range = total.saturating_sub(visible).max(1);
    let offset = offset.min(total.saturating_sub(visible));
    let thumb_top = (offset * track_h.saturating_sub(thumb_h)) / scroll_range;
    let thumb_top = thumb_top.min(track_h.saturating_sub(thumb_h));
    for i in 0..thumb_h {
        frame.render_widget(
            Paragraph::new("█").style(Style::default().fg(thumb_fg).bg(bg)),
            Rect {
                x: track.x,
                y: track.y + (thumb_top + i) as u16,
                width: 1,
                height: 1,
            },
        );
    }
}

/// Map a click `y` on `track` to a scroll offset in `0..=total.saturating_sub(visible)`.
///
/// Proportional: position along the track × max scroll. Clicks above the track
/// map to 0; clicks at or below the last row map to max.
pub(super) fn scrollbar_click_offset(track: Rect, y: u16, total: usize, visible: usize) -> usize {
    let max_scroll = total.saturating_sub(visible);
    if max_scroll == 0 || track.height == 0 {
        return 0;
    }
    let last = track.y.saturating_add(track.height.saturating_sub(1));
    let y = y.max(track.y).min(last);
    let rel = (y - track.y) as usize;
    let denom = (track.height as usize).saturating_sub(1).max(1);
    (rel * max_scroll) / denom
}

impl ScrollbarTrack {
    pub(super) fn offset_at(self, y: u16) -> usize {
        scrollbar_click_offset(self.rect, y, self.total, self.visible)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(y: u16, h: u16) -> Rect {
        Rect {
            x: 10,
            y,
            width: 1,
            height: h,
        }
    }

    #[test]
    fn click_offset_at_track_top_is_zero() {
        let t = track(5, 10);
        assert_eq!(scrollbar_click_offset(t, 5, 100, 10), 0);
    }

    #[test]
    fn click_offset_at_track_bottom_is_max() {
        let t = track(5, 10);
        // last row of a 10-tall track starting at y=5 is y=14; max = 100-10=90
        assert_eq!(scrollbar_click_offset(t, 14, 100, 10), 90);
    }

    #[test]
    fn click_offset_mid_is_in_range() {
        let t = track(5, 10);
        // 5 rows from top: rel=5, denom=9, max=90 → 50
        let off = scrollbar_click_offset(t, 10, 100, 10);
        assert!(off > 0 && off < 90, "{off}");
        assert_eq!(off, 50);
    }

    #[test]
    fn click_offset_when_total_fits_is_zero() {
        let t = track(5, 10);
        assert_eq!(scrollbar_click_offset(t, 14, 10, 10), 0);
        assert_eq!(scrollbar_click_offset(t, 14, 5, 10), 0);
        assert_eq!(scrollbar_click_offset(t, 5, 8, 10), 0);
    }

    #[test]
    fn click_offset_clamps_y_outside_track() {
        let t = track(5, 10);
        assert_eq!(scrollbar_click_offset(t, 0, 100, 10), 0);
        assert_eq!(scrollbar_click_offset(t, 99, 100, 10), 90);
    }
}
