//! Dispatch and FormEdit painting.
use super::super::*;

impl App {
    /// Check if any modal is currently open
    #[allow(dead_code)]
    pub(in crate::tui) fn is_modal_open(&self) -> bool {
        self.modal.is_some()
    }

    pub(in crate::tui) fn render_modal(&self, frame: &mut ratatui::Frame) {
        *self.form_scrollbar_track.borrow_mut() = ScrollbarTrack::default();
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
            .style(Style::default().bg(self.theme.modal_background));
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
                    .bg(self.theme.modal_background)
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
                            .bg(self.theme.modal_background)
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
                    .border_style(Style::default().fg(border_color).bg(self.theme.modal_background))
                    .style(Style::default().bg(self.theme.modal_background));
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
            let track = Rect::new(scrollbar_col, content_top, 1, content_height);
            *self.form_scrollbar_track.borrow_mut() = ScrollbarTrack {
                rect: track,
                total: total_height as usize,
                visible: content_height as usize,
            };
            paint_scrollbar(
                frame,
                track,
                scroll as usize,
                total_height as usize,
                content_height as usize,
                self.theme.scrollbar,
                self.theme.scrollbar_hover,
                self.theme.modal_background,
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
            .bg(self.theme.modal_background);

        match &field.kind {
            editform::FieldKind::Text { .. } | editform::FieldKind::Url { .. } => {
                let value = state.get(field.id);
                frame.render_widget(Paragraph::new(value).style(value_style), rect);
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
                        self.theme.modal_background,
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
                        .bg(self.theme.modal_background);
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
}
