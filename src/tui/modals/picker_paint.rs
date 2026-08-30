//! Component, image, and page picker painting.
use super::super::*;

impl App {
    pub(in crate::tui) fn render_component_picker_unified(
        &self,
        frame: &mut ratatui::Frame,
        query: &str,
        selected: usize,
    ) {
        let config = ModalConfig {
            width_percent: 70,
            height_percent: 70,
            footer_text: "Type to filter | Up/Down: select | Enter: insert | Esc: cancel"
                .to_string(),
        };

        let area = centered_rect(config.width_percent, config.height_percent, frame.area());
        frame.render_widget(Clear, area);

        let modal_block = Block::default()
            .title("Insert Component")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.border))
            .title_style(
                Style::default()
                    .fg(self.theme.title)
                    .add_modifier(Modifier::BOLD),
            );

        frame.render_widget(modal_block.clone(), area);
        let inner = modal_block.inner(area);

        // Search box
        let search_text = format!("Search: {}", query);
        let search = Paragraph::new(search_text).style(Style::default().fg(self.theme.foreground));
        frame.render_widget(
            search,
            Rect {
                x: inner.x,
                y: inner.y,
                width: inner.width,
                height: 1,
            },
        );

        // Filtered list
        let filtered = self.filtered_component_kinds(query);
        let items: Vec<ListItem> = filtered
            .iter()
            .enumerate()
            .map(|(idx, kind)| {
                let style = if idx == selected {
                    Style::default()
                        .fg(self.theme.selected_foreground)
                        .bg(self.theme.selected_background)
                } else {
                    Style::default().fg(self.theme.foreground)
                };
                ListItem::new(kind.label()).style(style)
            })
            .collect();

        let list = List::new(items)
            .block(Block::default())
            .highlight_symbol("> ");

        frame.render_widget(
            list,
            Rect {
                x: inner.x,
                y: inner.y + 2,
                width: inner.width,
                height: inner.height.saturating_sub(3),
            },
        );

        // Footer
        let footer =
            Paragraph::new(&config.footer_text[..]).style(Style::default().fg(self.theme.muted));
        frame.render_widget(
            footer,
            Rect {
                x: inner.x,
                y: inner.y + inner.height.saturating_sub(1),
                width: inner.width,
                height: 1,
            },
        );
    }

    pub(in crate::tui) fn render_image_picker_modal(
        &self,
        frame: &mut ratatui::Frame,
        state: &ImagePickerState,
    ) {
        let area = centered_rect(70, 70, frame.area());
        frame.render_widget(Clear, area);

        let outer = Block::default()
            .title(" Pick image ")
            .borders(Borders::ALL)
            .style(Style::default().bg(self.theme.popup_background))
            .border_style(Style::default().fg(self.theme.border_active))
            .title_style(
                Style::default()
                    .fg(self.theme.title)
                    .add_modifier(Modifier::BOLD),
            );
        let inner = outer.inner(area);
        frame.render_widget(outer, area);
        if inner.height < 5 || inner.width < 10 {
            return;
        }

        let pad: u16 = 2;
        let content_x = inner.x + pad;
        let content_w = inner.width.saturating_sub(pad * 2);

        // Row 0: cwd path (relative to root).
        let rel = state.cwd.strip_prefix(&state.root).unwrap_or(&state.cwd);
        let rel_str = rel.to_string_lossy();
        let cwd_label = if rel_str.is_empty() {
            "Folder: ./source/images/".to_string()
        } else {
            format!("Folder: ./source/images/{}", rel_str)
        };
        frame.render_widget(
            Paragraph::new(cwd_label).style(
                Style::default()
                    .fg(self.theme.muted)
                    .bg(self.theme.popup_background),
            ),
            Rect::new(content_x, inner.y, content_w, 1),
        );

        // Row 1: filter input (with a trailing underscore as a fake cursor).
        let filter_label = format!("Filter: {}_", state.filter);
        frame.render_widget(
            Paragraph::new(filter_label).style(
                Style::default()
                    .fg(self.theme.text_active_focus)
                    .bg(self.theme.popup_background),
            ),
            Rect::new(content_x, inner.y + 1, content_w, 1),
        );

        // Body: filtered entry list, with vertical scroll keeping selection in view.
        let entries = list_dir_entries(&state.cwd);
        let filtered = filter_entries(&entries, &state.filter);
        let body_y = inner.y + 3;
        let body_h = inner.height.saturating_sub(5);
        let visible = body_h as usize;
        let start = if filtered.is_empty() {
            0
        } else if state.selected >= visible {
            state.selected + 1 - visible
        } else {
            0
        };

        if filtered.is_empty() {
            frame.render_widget(
                Paragraph::new("(no matches)").style(
                    Style::default()
                        .fg(self.theme.muted)
                        .bg(self.theme.popup_background),
                ),
                Rect::new(content_x, body_y, content_w, 1),
            );
        } else {
            for (i, entry) in filtered.iter().skip(start).take(visible).enumerate() {
                let row = body_y + i as u16;
                let is_selected = (start + i) == state.selected;
                let glyph = if entry.is_dir { "/" } else { " " };
                let line = format!("{} {}", glyph, entry.name);
                let (fg, bg) = if is_selected {
                    (
                        self.theme.selected_foreground,
                        self.theme.selected_background,
                    )
                } else if entry.is_dir {
                    (self.theme.folders, self.theme.popup_background)
                } else {
                    (self.theme.files, self.theme.popup_background)
                };
                frame.render_widget(
                    Paragraph::new(line).style(Style::default().fg(fg).bg(bg)),
                    Rect::new(content_x, row, content_w, 1),
                );
            }
        }

        // Footer hint.
        let footer_y = inner.y + inner.height.saturating_sub(1);
        frame.render_widget(
            Paragraph::new(
                "↑/↓: move  |  →/Enter: descend or pick  |  ←: parent  |  type: filter  |  Esc: cancel",
            )
            .style(
                Style::default()
                    .fg(self.theme.muted)
                    .bg(self.theme.popup_background),
            ),
            Rect::new(content_x, footer_y, content_w, 1),
        );
    }

    pub(in crate::tui) fn render_page_picker_modal(
        &self,
        frame: &mut ratatui::Frame,
        state: &PagePickerState,
    ) {
        let area = centered_rect(60, 60, frame.area());
        frame.render_widget(Clear, area);
        let outer = Block::default()
            .title(" Pick page ")
            .borders(Borders::ALL)
            .style(Style::default().bg(self.theme.popup_background))
            .border_style(Style::default().fg(self.theme.border_active))
            .title_style(
                Style::default()
                    .fg(self.theme.title)
                    .add_modifier(Modifier::BOLD),
            );
        let inner = outer.inner(area);
        frame.render_widget(outer, area);
        if inner.height < 5 || inner.width < 10 {
            return;
        }
        let pad: u16 = 2;
        let content_x = inner.x + pad;
        let content_w = inner.width.saturating_sub(pad * 2);

        // Filter row.
        let filter_label = format!("Filter: {}_", state.filter);
        frame.render_widget(
            Paragraph::new(filter_label).style(
                Style::default()
                    .fg(self.theme.text_active_focus)
                    .bg(self.theme.popup_background),
            ),
            Rect::new(content_x, inner.y, content_w, 1),
        );

        // Filtered list.
        let filtered = filter_pages(&state.pages, &state.filter);
        let body_y = inner.y + 2;
        let body_h = inner.height.saturating_sub(3);
        let visible = body_h as usize;
        let start = if filtered.is_empty() {
            0
        } else if state.selected >= visible {
            state.selected + 1 - visible
        } else {
            0
        };

        if filtered.is_empty() {
            frame.render_widget(
                Paragraph::new("(no matches)").style(
                    Style::default()
                        .fg(self.theme.muted)
                        .bg(self.theme.popup_background),
                ),
                Rect::new(content_x, body_y, content_w, 1),
            );
        } else {
            for (i, (slug, title)) in
                filtered.iter().skip(start).take(visible).enumerate()
            {
                let row = body_y + i as u16;
                let is_selected = (start + i) == state.selected;
                let line = format!("{}  /{}", title, slug);
                let (fg, bg) = if is_selected {
                    (
                        self.theme.selected_foreground,
                        self.theme.selected_background,
                    )
                } else {
                    (self.theme.foreground, self.theme.popup_background)
                };
                frame.render_widget(
                    Paragraph::new(line).style(Style::default().fg(fg).bg(bg)),
                    Rect::new(content_x, row, content_w, 1),
                );
            }
        }

        let footer_y = inner.y + inner.height.saturating_sub(1);
        frame.render_widget(
            Paragraph::new(
                "↑/↓: move  |  Enter: pick  |  type: filter  |  Esc: cancel",
            )
            .style(
                Style::default()
                    .fg(self.theme.muted)
                    .bg(self.theme.popup_background),
            ),
            Rect::new(content_x, footer_y, content_w, 1),
        );
    }
}
