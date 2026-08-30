//! Single-input, confirm, and validation modal painting.
use super::super::*;

impl App {
    pub(in crate::tui) fn render_save_prompt_unified(&self, frame: &mut ratatui::Frame, path: &str) {
        let config = ModalConfig {
            width_percent: 70,
            height_percent: 35,
            footer_text: "Enter: save | Esc: cancel".to_string(),
        };

        let area = centered_rect(config.width_percent, config.height_percent, frame.area());
        frame.render_widget(Clear, area);

        let modal_block = Block::default()
            .title("Save Page")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.border))
            .title_style(
                Style::default()
                    .fg(self.theme.title)
                    .add_modifier(Modifier::BOLD),
            );

        frame.render_widget(modal_block.clone(), area);
        let inner = modal_block.inner(area);

        let content = format!("Save file path:\n{}\n\n{}", path, config.footer_text);
        let prompt = Paragraph::new(content).style(
            Style::default()
                .fg(self.theme.foreground)
                .bg(self.theme.popup_background),
        );

        frame.render_widget(prompt, inner);
    }

    pub(in crate::tui) fn render_template_picker_modal(&self, frame: &mut ratatui::Frame, selected: usize) {
        use ratatui::widgets::{List, ListItem, ListState};
        let area = centered_rect(60, 30, frame.area());
        frame.render_widget(Clear, area);
        let options = ["Blank", "Hero only", "Hero + Section", "Duplicate current"];
        let items: Vec<ListItem> = options.iter().map(|s| ListItem::new(*s)).collect();
        let mut state = ListState::default();
        state.select(Some(selected.min(options.len() - 1)));
        let list = List::new(items)
            .block(
                Block::default()
                    .title(" New page — choose template ")
                    .borders(Borders::ALL)
                    .style(
                        Style::default()
                            .fg(self.theme.modal_text)
                            .bg(self.theme.popup_background),
                    )
                    .border_style(Style::default().fg(self.theme.border_active))
                    .title_style(Style::default().fg(self.theme.modal_labels)),
            )
            .highlight_style(
                Style::default()
                    .fg(self.theme.selected_foreground)
                    .bg(self.theme.selected_background)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");
        frame.render_stateful_widget(list, area, &mut state);
    }

    pub(in crate::tui) fn render_new_page_title_prompt(&self, frame: &mut ratatui::Frame, title: &str) {
        self.render_single_input_modal(
            frame,
            " New page — title ",
            "Title",
            title,
            "Enter or Ctrl+S: continue  |  Esc: cancel",
        );
    }

    pub(in crate::tui) fn render_export_path_prompt(&self, frame: &mut ratatui::Frame, path: &str) {
        self.render_single_input_modal(
            frame,
            " Export — output directory ",
            "Path (relative to site JSON)",
            path,
            "Enter or Ctrl+S: export  |  Esc: cancel",
        );
    }

    pub(in crate::tui) fn render_preview_path_prompt(&self, frame: &mut ratatui::Frame, path: &str) {
        self.render_single_input_modal(
            frame,
            " Preview — output directory ",
            "Path (relative to site JSON)",
            path,
            "Enter or Ctrl+S: preview  |  Esc: cancel",
        );
    }

    pub(in crate::tui) fn render_rename_page_prompt(&self, frame: &mut ratatui::Frame, title: &str, _page_idx: usize) {
        self.render_single_input_modal(
            frame,
            " Rename page ",
            "Title",
            title,
            "Enter or Ctrl+S: save  |  Esc: cancel",
        );
    }

    pub(in crate::tui) fn render_single_input_modal(
        &self,
        frame: &mut ratatui::Frame,
        outer_title: &str,
        label: &str,
        value: &str,
        footer_text: &str,
    ) {
        let area = centered_rect(60, 30, frame.area());
        frame.render_widget(Clear, area);

        let modal_block = Block::default()
            .title(outer_title.to_string())
            .borders(Borders::ALL)
            .style(Style::default().bg(self.theme.popup_background))
            .border_style(Style::default().fg(self.theme.border_active))
            .title_style(
                Style::default()
                    .fg(self.theme.title)
                    .add_modifier(Modifier::BOLD),
            );
        frame.render_widget(modal_block.clone(), area);
        let inner = modal_block.inner(area);

        if inner.width < 6 || inner.height < 6 {
            return;
        }

        let padding_x: u16 = 2;
        let content_x = inner.x + padding_x;
        let content_w = inner.width.saturating_sub(padding_x * 2);

        // Label row
        let label_area = Rect {
            x: content_x,
            y: inner.y + 1,
            width: content_w,
            height: 1,
        };
        // Single-input modal has exactly one field always focused, so label
        // uses the text_active_focus token.
        let label_para = Paragraph::new(format!("{}:", label)).style(
            Style::default()
                .fg(self.theme.text_active_focus)
                .bg(self.theme.popup_background),
        );
        frame.render_widget(label_para, label_area);

        // Bordered input box (3 rows tall: border + content + border)
        let input_area = Rect {
            x: content_x,
            y: inner.y + 2,
            width: content_w,
            height: 3,
        };
        let input = Paragraph::new(value)
            .style(
                Style::default()
                    .fg(self.theme.input_text_focus)
                    .bg(self.theme.popup_background),
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .style(Style::default().bg(self.theme.popup_background))
                    .border_style(Style::default().fg(self.theme.input_border_focus)),
            );
        frame.render_widget(input, input_area);

        // Cursor inside the input box (one row below top border, after last char).
        let max_x = input_area.x + input_area.width.saturating_sub(2);
        let cursor_x = (input_area.x + 1 + value.chars().count() as u16).min(max_x);
        let cursor_y = input_area.y + 1;
        // Paint a themed cursor cell so the cursor color follows the theme.
        let cursor_cell = Paragraph::new(" ").style(
            Style::default()
                .fg(self.theme.popup_background)
                .bg(self.theme.cursor),
        );
        frame.render_widget(
            cursor_cell,
            Rect {
                x: cursor_x,
                y: cursor_y,
                width: 1,
                height: 1,
            },
        );
        frame.set_cursor_position((cursor_x, cursor_y));

        // Footer hint row at the bottom of inner
        let footer_y = inner.y + inner.height.saturating_sub(2);
        let footer_area = Rect {
            x: content_x,
            y: footer_y,
            width: content_w,
            height: 1,
        };
        let footer = Paragraph::new(footer_text).style(
            Style::default()
                .fg(self.theme.muted)
                .bg(self.theme.popup_background),
        );
        frame.render_widget(footer, footer_area);
    }

    pub(in crate::tui) fn render_confirm_prompt(&self, frame: &mut ratatui::Frame, message: &str) {
        let area = centered_rect(70, 35, frame.area());
        frame.render_widget(Clear, area);

        let modal_block = Block::default()
            .title(" Confirm ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.border_active))
            .title_style(
                Style::default()
                    .fg(self.theme.title)
                    .add_modifier(Modifier::BOLD),
            );

        frame.render_widget(modal_block.clone(), area);
        let inner = modal_block.inner(area);

        let content = format!("{}\n\ny = confirm, n / Esc = cancel", message);
        let prompt = Paragraph::new(content).style(
            Style::default()
                .fg(self.theme.foreground)
                .bg(self.theme.popup_background),
        );

        frame.render_widget(prompt, inner);
    }

    pub(in crate::tui) fn render_validation_errors_modal(
        &self,
        frame: &mut ratatui::Frame,
        errors: &[String],
        scroll_offset: usize,
    ) {
        let area = centered_rect(70, 60, frame.area());
        frame.render_widget(Clear, area);

        let outer_title = format!(" Validation — {} error(s) ", errors.len());
        let modal_block = Block::default()
            .title(outer_title)
            .borders(Borders::ALL)
            .style(Style::default().bg(self.theme.popup_background))
            .border_style(Style::default().fg(self.theme.border_active))
            .title_style(
                Style::default()
                    .fg(self.theme.title)
                    .add_modifier(Modifier::BOLD),
            );
        frame.render_widget(modal_block.clone(), area);
        let inner = modal_block.inner(area);

        if inner.width < 4 || inner.height < 3 {
            return;
        }

        let padding_x: u16 = 2;
        let content_x = inner.x + padding_x;
        let content_w = inner.width.saturating_sub(padding_x * 2);
        let footer_height: u16 = 1;
        let list_height = inner.height.saturating_sub(footer_height);

        let wrapped_lines = self.wrap_validation_lines(errors, content_w as usize);
        let visible: Vec<String> = wrapped_lines
            .iter()
            .skip(scroll_offset)
            .take(list_height as usize)
            .cloned()
            .collect();

        let body = Paragraph::new(visible.join("\n")).style(
            Style::default()
                .fg(self.theme.foreground)
                .bg(self.theme.popup_background),
        );
        frame.render_widget(
            body,
            Rect {
                x: content_x,
                y: inner.y,
                width: content_w,
                height: list_height,
            },
        );

        let footer_y = inner.y + inner.height.saturating_sub(footer_height);
        let footer_area = Rect {
            x: content_x,
            y: footer_y,
            width: content_w,
            height: 1,
        };
        let footer_text = if wrapped_lines.len() > list_height as usize {
            "j / k or \u{2191} / \u{2193} to scroll  |  Enter or Esc to dismiss"
        } else {
            "Enter or Esc to dismiss"
        };
        let footer = Paragraph::new(footer_text).style(
            Style::default()
                .fg(self.theme.muted)
                .bg(self.theme.popup_background),
        );
        frame.render_widget(footer, footer_area);
    }

    pub(in crate::tui) fn wrap_validation_lines(&self, errors: &[String], width: usize) -> Vec<String> {
        let mut out = Vec::with_capacity(errors.len());
        for (i, err) in errors.iter().enumerate() {
            let prefix = format!("{}. ", i + 1);
            let indent = " ".repeat(prefix.len());
            let body_w = width.saturating_sub(prefix.len()).max(1);
            let mut first = true;
            let mut remaining = err.as_str();
            while !remaining.is_empty() {
                let take = remaining.chars().take(body_w).count();
                let split_byte = remaining
                    .char_indices()
                    .nth(take)
                    .map(|(i, _)| i)
                    .unwrap_or(remaining.len());
                let (chunk, rest) = remaining.split_at(split_byte);
                let line = if first {
                    format!("{}{}", prefix, chunk)
                } else {
                    format!("{}{}", indent, chunk)
                };
                out.push(line);
                remaining = rest;
                first = false;
            }
        }
        out
    }
}
