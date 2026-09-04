//! Ephemeral toast notifications.
use super::super::*;

impl App {
    pub(in crate::tui) fn push_toast(&mut self, level: ToastLevel, message: impl Into<String>) {
        self.toasts.push(Toast {
            level,
            message: message.into(),
            shown_at: std::time::Instant::now(),
        });
        if self.toasts.len() > 4 {
            self.toasts.remove(0);
        }
    }

    pub(in crate::tui) fn prune_toasts(&mut self) {
        let now = std::time::Instant::now();
        self.toasts
            .retain(|t| now.duration_since(t.shown_at) < std::time::Duration::from_secs(5));
    }

    pub(in crate::tui) fn render_toasts(&self, frame: &mut ratatui::Frame, area: Rect) {
        if self.toasts.is_empty() {
            return;
        }
        let toast_w: u16 = 60;
        let gap: u16 = 1;
        let max_width = area.width.saturating_sub(2);
        let width = toast_w.min(max_width);
        if width < 10 {
            return;
        }
        let right_x = area.x + area.width.saturating_sub(width + 1);
        let toast_h: u16 = 3;
        let mut y = area.y + area.height.saturating_sub(toast_h);
        for toast in self.toasts.iter().rev() {
            if y + toast_h > area.y + area.height {
                break;
            }
            let rect = Rect {
                x: right_x,
                y,
                width,
                height: toast_h,
            };
            let (glyph, accent) = match toast.level {
                ToastLevel::Success => ("✓", self.theme.success),
                ToastLevel::Info => ("ℹ", self.theme.info),
                ToastLevel::Warning => ("⚠", self.theme.warning),
                ToastLevel::Error => ("✗", self.theme.error),
            };
            frame.render_widget(Clear, rect);
            let block = Block::default()
                .borders(Borders::ALL)
                .style(Style::default().bg(self.theme.popup_background))
                .border_style(Style::default().fg(accent));
            let inner_x = rect.x + 2;
            let inner_y = rect.y + 1;
            let inner_w = rect.width.saturating_sub(4);
            frame.render_widget(block, rect);
            let text = format!("{} {}", glyph, toast.message);
            let body = Paragraph::new(text).style(
                Style::default()
                    .fg(accent)
                    .bg(self.theme.popup_background),
            );
            frame.render_widget(
                body,
                Rect {
                    x: inner_x,
                    y: inner_y,
                    width: inner_w,
                    height: 1,
                },
            );
            if y < area.y + toast_h + gap {
                break;
            }
            y = y.saturating_sub(toast_h + gap);
        }
    }
}
