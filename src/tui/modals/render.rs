//! Paint every modal variant, form fields, and toasts.
use super::super::*;

impl App {
    /// Check if any modal is currently open
    #[allow(dead_code)]
    pub(in crate::tui) fn is_modal_open(&self) -> bool {
        self.modal.is_some()
    }

    pub(in crate::tui) fn render_modal(&self, frame: &mut ratatui::Frame) {
        if let Some(modal) = &self.modal {
            self.render_unified_modal(frame, modal);
        }
    }

    pub(in crate::tui) fn render_unified_modal(&self, frame: &mut ratatui::Frame, modal: &Modal) {
        match modal {
            Modal::ComponentPicker { query, selected } => {
                self.render_component_picker_unified(frame, query, *selected);
            }
            Modal::SavePrompt { path } => {
                self.render_save_prompt_unified(frame, path);
            }
            Modal::FormEdit {
                state,
                cursor_pos,
                scroll_offset,
                ..
            } => {
                self.render_form_edit_modal(frame, state, *cursor_pos, *scroll_offset);
            }
            Modal::TemplatePicker { selected } => {
                self.render_template_picker_modal(frame, *selected);
            }
            Modal::NewPageTitlePrompt { title } => {
                self.render_new_page_title_prompt(frame, title);
            }
            Modal::ExportPathPrompt { path } => {
                self.render_export_path_prompt(frame, path);
            }
            Modal::PreviewPathPrompt { path } => {
                self.render_preview_path_prompt(frame, path);
            }
            Modal::RenamePagePrompt { title, page_idx } => {
                self.render_rename_page_prompt(frame, title, *page_idx);
            }
            Modal::ConfirmPrompt { message, .. } => {
                self.render_confirm_prompt(frame, message);
            }
            Modal::ValidationErrors { errors, scroll_offset } => {
                self.render_validation_errors_modal(frame, errors, *scroll_offset);
            }
            Modal::ImagePicker { state } => self.render_image_picker_modal(frame, state),
            Modal::PagePicker { state } => self.render_page_picker_modal(frame, state),
        }
    }

    pub(in crate::tui) fn render_form_edit_modal(
        &self,
        frame: &mut ratatui::Frame,
        state: &editform::EditFormState,
        cursor_pos: usize,
        scroll_offset: u16,
    ) {
        let area = centered_rect(70, 80, frame.area());
        frame.render_widget(Clear, area);

        // Outer border + title with solid modal background.
        let outer = Block::default()
            .title(format!(" Edit Item -- {} ", state.form.title))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.modal_labels))
            .title_style(
                Style::default()
                    .fg(self.theme.modal_labels)
                    .add_modifier(Modifier::BOLD),
            )
            .style(Style::default().bg(self.theme.popup_background));
        let inner = outer.inner(area);
        frame.render_widget(outer, area);
        if inner.height < 3 || inner.width < 6 {
            return;
        }

        // Help row at the very top of the content area.
        let help_rect = Rect::new(inner.x, inner.y, inner.width, 1);
        let help_text = "Tab/Up/Down: navigate | Ctrl+S: save | Esc: cancel";
        frame.render_widget(
            Paragraph::new(help_text).style(
                Style::default()
                    .fg(self.theme.modal_labels)
                    .bg(self.theme.popup_background)
                    .add_modifier(Modifier::BOLD),
            ),
            help_rect,
        );

        // Content area begins 2 rows below (help + spacer). Reserve 1 col for scrollbar.
        if inner.height < 4 {
            return;
        }
        let content_top = inner.y.saturating_add(2);
        let content_height = inner.height.saturating_sub(2);
        let scrollbar_col = inner
            .x
            .saturating_add(inner.width.saturating_sub(1));
        let content_rect = Rect::new(inner.x, content_top, inner.width.saturating_sub(1), content_height);

        // Build virtual field layout: each entry holds (field_idx, label_y, box_y, box_height).
        #[derive(Clone, Copy)]
        struct Slot {
            idx: usize,
            label_y: u16,
            box_y: u16,
            box_height: u16,
        }
        let mut slots: Vec<Slot> = Vec::new();
        let mut virt_y: u16 = 0;
        for (idx, field) in state.form.fields.iter().enumerate() {
            if !state.field_visible(field) {
                continue;
            }
            let content_rows: u16 = match &field.kind {
                editform::FieldKind::Textarea { rows, .. } => {
                    let max_rows = textarea_max_rows_for_window(content_height);
                    textarea_display_rows(
                        state.get(field.id),
                        (*rows).max(1),
                        Some(content_rect.width.saturating_sub(2)),
                        max_rows,
                    )
                }
                editform::FieldKind::SubForm { .. } => {
                    let items_len = state
                        .sub_state
                        .get(field.id)
                        .map(|v| v.len())
                        .unwrap_or(0);
                    // header line + one row per item (at least 1 placeholder row)
                    (1 + items_len.max(1)) as u16
                }
                _ => 1,
            };
            let box_height = content_rows.saturating_add(2); // +2 for borders
            let label_y = virt_y;
            let box_y = virt_y.saturating_add(1);
            slots.push(Slot {
                idx,
                label_y,
                box_y,
                box_height,
            });
            virt_y = virt_y.saturating_add(1 + box_height + 1); // label + box + blank separator
        }
        let total_height = virt_y;
        let max_scroll = total_height.saturating_sub(content_height);
        let scroll = scroll_offset.min(max_scroll);

        // Refresh the per-frame click-to-focus cache for this modal.
        self.modal_field_areas.borrow_mut().clear();

        for slot in &slots {
            let field = &state.form.fields[slot.idx];
            let focused = slot.idx == state.focused_field;
            let label_screen = slot.label_y as i32 - scroll as i32;
            let box_top_screen = slot.box_y as i32 - scroll as i32;
            let box_bottom_screen = box_top_screen + slot.box_height as i32;
            // Skip entries entirely outside the content window.
            if box_bottom_screen <= 0 || label_screen >= content_height as i32 {
                continue;
            }

            // Label row.
            if label_screen >= 0 && label_screen < content_height as i32 {
                let label_rect = Rect::new(
                    content_rect.x,
                    content_rect.y + label_screen as u16,
                    content_rect.width,
                    1,
                );
                let label_color = if focused {
                    self.theme.text_active_focus
                } else {
                    self.theme.text_labels
                };
                let label_mod = if focused {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                };
                frame.render_widget(
                    Paragraph::new(format!("{}:", field.label)).style(
                        Style::default()
                            .fg(label_color)
                            .bg(self.theme.popup_background)
                            .add_modifier(label_mod),
                    ),
                    label_rect,
                );
            }

            // Input box. Textareas may be taller than the current viewport, so
            // clamp the drawn box instead of dropping it entirely.
            if box_top_screen >= 0 && box_top_screen < content_height as i32 {
                let border_color = if focused {
                    self.theme.input_border_focus
                } else {
                    self.theme.input_border_default
                };
                let visible_box_height = slot
                    .box_height
                    .min(content_height.saturating_sub(box_top_screen as u16));
                if visible_box_height < 3 {
                    continue;
                }
                let box_rect = Rect::new(
                    content_rect.x,
                    content_rect.y + box_top_screen as u16,
                    content_rect.width,
                    visible_box_height,
                );
                let field_block = Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color).bg(self.theme.popup_background))
                    .style(Style::default().bg(self.theme.popup_background));
                let inner_rect = field_block.inner(box_rect);
                frame.render_widget(field_block, box_rect);
                self.modal_field_areas
                    .borrow_mut()
                    .push((slot.idx, box_rect));
                self.render_form_field_value(
                    frame,
                    field,
                    state,
                    cursor_pos,
                    focused,
                    inner_rect,
                );
            }
        }

        // Scrollbar on the right column when content exceeds window.
        if total_height > content_height {
            let track_bg = Block::default().style(Style::default().bg(self.theme.popup_background));
            frame.render_widget(
                track_bg,
                Rect::new(scrollbar_col, content_top, 1, content_height),
            );
            let thumb_height = ((content_height as u32 * content_height as u32
                / total_height.max(1) as u32) as u16)
                .max(1);
            let travel = content_height.saturating_sub(thumb_height);
            let thumb_y = if max_scroll == 0 {
                0
            } else {
                ((scroll as u32 * travel as u32) / max_scroll.max(1) as u32) as u16
            };
            let thumb = Paragraph::new(vec!["█".to_string(); thumb_height as usize].join("\n"))
                .style(Style::default().fg(self.theme.scrollbar).bg(self.theme.popup_background));
            frame.render_widget(
                thumb,
                Rect::new(scrollbar_col, content_top + thumb_y, 1, thumb_height),
            );
        }
    }

    pub(in crate::tui) fn render_form_field_value(
        &self,
        frame: &mut ratatui::Frame,
        field: &editform::FormField,
        state: &editform::EditFormState,
        cursor_pos: usize,
        focused: bool,
        rect: Rect,
    ) {
        let text_color = if focused {
            self.theme.input_text_focus
        } else {
            self.theme.input_text_default
        };
        let value_style = Style::default()
            .fg(text_color)
            .bg(self.theme.popup_background);

        match &field.kind {
            editform::FieldKind::Text { .. } | editform::FieldKind::Url { .. } => {
                let value = state.get(field.id);
                let display = if focused {
                    render_cursor_line(value, cursor_pos)
                } else {
                    value.to_string()
                };
                frame.render_widget(Paragraph::new(display).style(value_style), rect);
            }
            editform::FieldKind::Textarea { .. } => {
                let value = state.get(field.id);
                let visible_rows = rect.height as usize;
                let (display, first_visible_row, total_rows) =
                    render_textarea_display_window(value, cursor_pos, focused, visible_rows);
                let text_rect = if total_rows > visible_rows {
                    Rect {
                        width: rect.width.saturating_sub(1),
                        ..rect
                    }
                } else {
                    rect
                };
                frame.render_widget(
                    Paragraph::new(display)
                        .style(value_style),
                    text_rect,
                );
                if total_rows > visible_rows {
                    render_textarea_scrollbar(
                        frame,
                        Rect {
                            x: rect.x + rect.width.saturating_sub(1),
                            y: rect.y,
                            width: 1,
                            height: rect.height,
                        },
                        first_visible_row,
                        visible_rows,
                        total_rows,
                        self.theme.scrollbar,
                        self.theme.popup_background,
                    );
                }
            }
            editform::FieldKind::Enum { options, .. } => {
                let value = state.get(field.id);
                let display = format!("< {} >", value);
                let mut style = value_style;
                if !options.iter().any(|o| *o == value) {
                    style = Style::default()
                        .fg(self.theme.error)
                        .bg(self.theme.popup_background);
                }
                frame.render_widget(Paragraph::new(display).style(style), rect);
            }
            editform::FieldKind::OptionalLinkTriple { .. } => {
                // Reserved — hero migration uses 3 flat fields instead.
            }
            editform::FieldKind::SubForm {
                summary_field_id, ..
            } => {
                let items = state.sub_state.get(field.id).cloned().unwrap_or_default();
                let selected = state
                    .selected_sub_item
                    .get(field.id)
                    .copied()
                    .unwrap_or(0);
                let mut lines: Vec<String> = Vec::new();
                lines.push(format!(
                    "{} item(s) — A add · X remove · Enter edit",
                    items.len()
                ));
                if items.is_empty() {
                    lines.push("  (no items; press A to add)".to_string());
                } else {
                    for (i, item) in items.iter().enumerate() {
                        let summary = item
                            .values
                            .get(*summary_field_id)
                            .cloned()
                            .unwrap_or_default();
                        let summary = if summary.trim().is_empty() {
                            "(untitled)".to_string()
                        } else {
                            summary
                        };
                        let marker = if focused && i == selected { ">" } else { " " };
                        lines.push(format!("  {} {}. {}", marker, i + 1, summary));
                    }
                }
                frame.render_widget(Paragraph::new(lines.join("\n")).style(value_style), rect);
            }
        }
    }

    pub(in crate::tui) fn render_scrollbar(
        &self,
        frame: &mut ratatui::Frame,
        inner: Rect,
        scroll_offset: usize,
        visible_count: usize,
        total_count: usize,
        header_height: u16,
        footer_height: u16,
    ) {
        let scrollbar_x = inner.x + inner.width.saturating_sub(2);
        let scrollbar_top = header_height + 1;
        let scrollbar_height = inner
            .height
            .saturating_sub(header_height + footer_height + 2);

        // Track
        for y_offset in 0..scrollbar_height {
            frame.render_widget(
                Paragraph::new("│").style(Style::default().fg(self.theme.border)),
                Rect {
                    x: scrollbar_x,
                    y: inner.y + scrollbar_top + y_offset,
                    width: 1,
                    height: 1,
                },
            );
        }

        // Thumb
        let thumb_size = ((visible_count * scrollbar_height as usize) / total_count).max(1);
        let thumb_pos = if total_count > visible_count {
            ((scroll_offset * (scrollbar_height as usize - thumb_size))
                / (total_count - visible_count)) as u16
        } else {
            0
        };

        for i in 0..(thumb_size as u16) {
            let y = scrollbar_top + thumb_pos + i;
            if y < scrollbar_top + scrollbar_height {
                frame.render_widget(
                    Paragraph::new("█").style(Style::default().fg(self.theme.active)),
                    Rect {
                        x: scrollbar_x,
                        y: inner.y + y,
                        width: 1,
                        height: 1,
                    },
                );
            }
        }
    }

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
