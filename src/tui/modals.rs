//! Modal types, rendering, and event handling.
use super::*;
// UNIFIED MODAL SYSTEM
// ============================================================================

/// All modal types in the application
pub(super) enum Modal {
    /// Component picker for inserting components
    ComponentPicker { query: String, selected: usize },
    /// Save file dialog
    SavePrompt { path: String },
    /// Template picker for adding a new page.
    TemplatePicker {
        /// Index within the template option list that is currently highlighted.
        selected: usize,
    },
    /// Title entry prompt shown before the TemplatePicker when adding a new page.
    NewPageTitlePrompt {
        title: String,
    },
    /// Path entry prompt shown when exporting the site to a local directory.
    ExportPathPrompt {
        path: String,
    },
    /// Path entry prompt shown when previewing the site in a browser.
    PreviewPathPrompt {
        path: String,
    },
    /// Title-edit prompt shown when renaming an existing page.
    RenamePagePrompt {
        title: String,
        page_idx: usize,
    },
    /// Generic yes/no confirmation prompt.
    ConfirmPrompt {
        message: String,
        on_confirm: ConfirmKind,
    },
    /// Scrollable list of validation errors.
    ValidationErrors {
        errors: Vec<String>,
        scroll_offset: usize,
    },
    /// File picker rooted at `./source/images/`.
    ImagePicker {
        state: ImagePickerState,
    },
    /// Page picker — lists site pages and writes `/<slug>` to a URL field.
    PagePicker {
        state: PagePickerState,
    },
    /// Unified form editor: all fields of a component rendered together,
    /// Tab moves between fields, Left/Right cycles enums, Ctrl+S saves via
    /// `cursor::apply_edit_form_to_component`.
    ///
    /// When `drill_stack` is non-empty, the editor is currently inside a
    /// nested SubForm item; Ctrl+S/Esc return to the outer parent rather
    /// than committing to the model.
    FormEdit {
        state: editform::EditFormState,
        cursor: cursor::Cursor,
        cursor_pos: usize, // text cursor within focused field's string
        drill_stack: Vec<DrillFrame>,
        scroll_offset: u16, // vertical row scroll within the form content
    },
}

/// One frame of drill-down context: parent form state plus the (subform id,
/// item idx) we entered from. When we return, we copy the current state into
/// `parent_state.sub_state[subform_field_id][item_idx]` and make the parent
/// the active state again.
pub(super) struct DrillFrame {
    pub(super) parent_state: editform::EditFormState,
    pub(super) parent_cursor_pos: usize,
    pub(super) parent_scroll_offset: u16,
    pub(super) subform_field_id: String,
    pub(super) item_idx: usize,
}

/// Common modal result returned from event handling
pub(super) enum ModalResult {
    /// Stay open, continue handling events
    Continue,
    /// Close modal with success
    CloseSuccess,
    /// Close modal with cancel
    CloseCancel,
}

/// The action to execute when a ConfirmPrompt is confirmed.
#[derive(Debug, Clone)]
pub(super) enum ConfirmKind {
    DeletePage,
    QuitUnsaved,
}

/// Live state of an open image picker. `root` and `cwd` are absolute
/// paths; `cwd` is always equal to or a descendant of `root`.
#[derive(Debug, Clone)]
pub(super) struct ImagePickerState {
    pub(super) root: std::path::PathBuf,
    pub(super) cwd: std::path::PathBuf,
    pub(super) filter: String,
    pub(super) selected: usize,
    pub(super) binding: ImagePickBinding,
}

#[derive(Debug, Clone)]
pub(super) enum ImagePickBinding {
    /// Write back into the FormEdit modal's currently-focused URL field.
    FormEditField { field_id: String },
}

/// Live state of an open page picker. Lists site pages by title; on Enter
/// writes `/<slug>` into the bound URL field.
#[derive(Debug, Clone)]
pub(super) struct PagePickerState {
    /// Snapshot of (slug, title) pairs at modal-open time. The picker
    /// doesn't track site mutations while open — it operates on a frozen
    /// list and the underlying site is back-burnered while paused.
    pub(super) pages: Vec<(String, String)>,
    pub(super) filter: String,
    pub(super) selected: usize,
    pub(super) binding: PagePickBinding,
}

#[derive(Debug, Clone)]
pub(super) enum PagePickBinding {
    /// Write back into the FormEdit modal's currently-focused URL field.
    FormEditField { field_id: String },
}

/// Visual/semantic class of a toast notification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ToastLevel {
    Success,
    Info,
    Warning,
}

/// A transient bottom-right notification. Expires ~5s after `shown_at`.
#[derive(Debug, Clone)]
pub(super) struct Toast {
    pub(super) level: ToastLevel,
    pub(super) message: String,
    pub(super) shown_at: std::time::Instant,
}

/// Unified modal configuration
pub(super) struct ModalConfig {
    pub(super) width_percent: u16,
    pub(super) height_percent: u16,
    pub(super) footer_text: String,
}

impl Default for ModalConfig {
    fn default() -> Self {
        Self {
            width_percent: 80,
            height_percent: 80,
            footer_text: "Tab/Up/Down: navigate | Ctrl+S: save | Esc: cancel".to_string(),
        }
    }
}

impl App {
    /// Check if any modal is currently open
    #[allow(dead_code)]
    pub(super) fn is_modal_open(&self) -> bool {
        self.modal.is_some()
    }
    pub(super) fn render_modal(&self, frame: &mut ratatui::Frame) {
        if let Some(modal) = &self.modal {
            self.render_unified_modal(frame, modal);
        }
    }
    pub(super) fn render_unified_modal(&self, frame: &mut ratatui::Frame, modal: &Modal) {
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
    pub(super) fn render_form_edit_modal(
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
    pub(super) fn render_form_field_value(
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
    pub(super) fn render_scrollbar(
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
    pub(super) fn render_component_picker_unified(
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
    pub(super) fn render_save_prompt_unified(&self, frame: &mut ratatui::Frame, path: &str) {
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
    pub(super) fn render_template_picker_modal(&self, frame: &mut ratatui::Frame, selected: usize) {
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
    pub(super) fn render_new_page_title_prompt(&self, frame: &mut ratatui::Frame, title: &str) {
        self.render_single_input_modal(
            frame,
            " New page — title ",
            "Title",
            title,
            "Enter or Ctrl+S: continue  |  Esc: cancel",
        );
    }
    pub(super) fn render_export_path_prompt(&self, frame: &mut ratatui::Frame, path: &str) {
        self.render_single_input_modal(
            frame,
            " Export — output directory ",
            "Path (relative to site JSON)",
            path,
            "Enter or Ctrl+S: export  |  Esc: cancel",
        );
    }
    pub(super) fn render_preview_path_prompt(&self, frame: &mut ratatui::Frame, path: &str) {
        self.render_single_input_modal(
            frame,
            " Preview — output directory ",
            "Path (relative to site JSON)",
            path,
            "Enter or Ctrl+S: preview  |  Esc: cancel",
        );
    }
    pub(super) fn render_rename_page_prompt(&self, frame: &mut ratatui::Frame, title: &str, _page_idx: usize) {
        self.render_single_input_modal(
            frame,
            " Rename page ",
            "Title",
            title,
            "Enter or Ctrl+S: save  |  Esc: cancel",
        );
    }
    pub(super) fn render_single_input_modal(
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
    pub(super) fn render_confirm_prompt(&self, frame: &mut ratatui::Frame, message: &str) {
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
    pub(super) fn render_validation_errors_modal(
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
    pub(super) fn push_toast(&mut self, level: ToastLevel, message: impl Into<String>) {
        self.toasts.push(Toast {
            level,
            message: message.into(),
            shown_at: std::time::Instant::now(),
        });
        if self.toasts.len() > 4 {
            self.toasts.remove(0);
        }
    }
    pub(super) fn prune_toasts(&mut self) {
        let now = std::time::Instant::now();
        self.toasts
            .retain(|t| now.duration_since(t.shown_at) < std::time::Duration::from_secs(5));
    }
    pub(super) fn render_toasts(&self, frame: &mut ratatui::Frame, area: Rect) {
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
    pub(super) fn wrap_validation_lines(&self, errors: &[String], width: usize) -> Vec<String> {
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
    pub(super) fn handle_validation_errors_event(&mut self, key: event::KeyEvent) -> Option<ModalResult> {
        use crossterm::event::KeyCode;
        let (errors_len, scroll) = match &self.modal {
            Some(Modal::ValidationErrors { errors, scroll_offset }) => {
                (errors.len(), *scroll_offset)
            }
            _ => return Some(ModalResult::CloseCancel),
        };
        match key.code {
            KeyCode::Enter | KeyCode::Esc => {
                self.modal = None;
                Some(ModalResult::CloseSuccess)
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(Modal::ValidationErrors { scroll_offset, .. }) = self.modal.as_mut() {
                    *scroll_offset = scroll_offset.saturating_sub(1);
                }
                Some(ModalResult::Continue)
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(Modal::ValidationErrors { scroll_offset, .. }) = self.modal.as_mut() {
                    if scroll + 1 < errors_len.max(1) {
                        *scroll_offset += 1;
                    }
                }
                Some(ModalResult::Continue)
            }
            KeyCode::PageUp => {
                if let Some(Modal::ValidationErrors { scroll_offset, .. }) = self.modal.as_mut() {
                    *scroll_offset = scroll_offset.saturating_sub(5);
                }
                Some(ModalResult::Continue)
            }
            KeyCode::PageDown => {
                if let Some(Modal::ValidationErrors { scroll_offset, .. }) = self.modal.as_mut() {
                    *scroll_offset = (scroll + 5).min(errors_len.saturating_sub(1));
                }
                Some(ModalResult::Continue)
            }
            _ => Some(ModalResult::Continue),
        }
    }
    pub(super) fn render_image_picker_modal(
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
    pub(super) fn handle_image_picker_event(&mut self, key: event::KeyEvent) -> Option<ModalResult> {
        use crossterm::event::{KeyCode, KeyModifiers};
        let Some(Modal::ImagePicker { state }) = self.modal.as_mut() else {
            return Some(ModalResult::CloseCancel);
        };

        match key.code {
            KeyCode::Esc => {
                self.modal = self.paused_form_edit_modal.take();
                self.push_toast(ToastLevel::Info, "Image pick cancelled.");
                Some(ModalResult::CloseCancel)
            }
            KeyCode::Up if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                state.selected = state.selected.saturating_sub(1);
                Some(ModalResult::Continue)
            }
            KeyCode::Down if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                let entries = list_dir_entries(&state.cwd);
                let filtered = filter_entries(&entries, &state.filter);
                if !filtered.is_empty() {
                    state.selected = (state.selected + 1).min(filtered.len() - 1);
                }
                Some(ModalResult::Continue)
            }
            KeyCode::Left if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                if state.cwd != state.root {
                    if let Some(parent) = state.cwd.parent() {
                        state.cwd = parent.to_path_buf();
                        state.filter.clear();
                        state.selected = 0;
                    }
                }
                Some(ModalResult::Continue)
            }
            KeyCode::Right if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.image_picker_descend_or_pick();
                Some(ModalResult::Continue)
            }
            KeyCode::Enter => {
                self.image_picker_descend_or_pick();
                Some(ModalResult::Continue)
            }
            KeyCode::Backspace => {
                state.filter.pop();
                state.selected = 0;
                Some(ModalResult::Continue)
            }
            KeyCode::Char(c)
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && (c.is_alphanumeric() || c == '-' || c == '_' || c == '.') =>
            {
                state.filter.push(c);
                state.selected = 0;
                Some(ModalResult::Continue)
            }
            _ => Some(ModalResult::Continue),
        }
    }
    pub(super) fn image_picker_descend_or_pick(&mut self) {
        let (cwd, root, selected_name, is_dir, binding) = {
            let Some(Modal::ImagePicker { state }) = self.modal.as_ref() else {
                return;
            };
            let entries = list_dir_entries(&state.cwd);
            let filtered = filter_entries(&entries, &state.filter);
            let Some(entry) = filtered.get(state.selected) else {
                return;
            };
            (
                state.cwd.clone(),
                state.root.clone(),
                entry.name.clone(),
                entry.is_dir,
                state.binding.clone(),
            )
        };

        if is_dir {
            if let Some(Modal::ImagePicker { state }) = self.modal.as_mut() {
                state.cwd = cwd.join(&selected_name);
                state.filter.clear();
                state.selected = 0;
            }
            return;
        }

        // File pick: build the output-relative path under assets/images/.
        let target_full = cwd.join(&selected_name);
        let rel_under_root = target_full
            .strip_prefix(&root)
            .unwrap_or(&target_full)
            .to_string_lossy()
            .replace('\\', "/");
        let stored = format!("assets/images/{}", rel_under_root);

        self.commit_image_pick(stored, binding);
    }
    pub(super) fn commit_image_pick(&mut self, value: String, binding: ImagePickBinding) {
        match binding {
            ImagePickBinding::FormEditField { field_id } => {
                self.modal = self.paused_form_edit_modal.take();
                if let Some(Modal::FormEdit { state, cursor_pos, .. }) = self.modal.as_mut() {
                    state.set(&field_id, value.clone());
                    *cursor_pos = state.get(&field_id).len();
                    self.push_toast(
                        ToastLevel::Success,
                        format!("Picked image: {}", value),
                    );
                } else {
                    self.push_toast(
                        ToastLevel::Warning,
                        "Image pick lost: parent form modal closed.",
                    );
                }
            }
        }
    }
    pub(super) fn render_page_picker_modal(
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
    pub(super) fn handle_page_picker_event(
        &mut self,
        key: event::KeyEvent,
    ) -> Option<ModalResult> {
        use crossterm::event::{KeyCode, KeyModifiers};
        let Some(Modal::PagePicker { state }) = self.modal.as_mut() else {
            return Some(ModalResult::CloseCancel);
        };
        match key.code {
            KeyCode::Esc => {
                self.modal = self.paused_form_edit_modal.take();
                self.push_toast(ToastLevel::Info, "Page pick cancelled.");
                Some(ModalResult::CloseCancel)
            }
            KeyCode::Up if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                state.selected = state.selected.saturating_sub(1);
                Some(ModalResult::Continue)
            }
            KeyCode::Down if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                let filtered = filter_pages(&state.pages, &state.filter);
                if !filtered.is_empty() {
                    state.selected =
                        (state.selected + 1).min(filtered.len() - 1);
                }
                Some(ModalResult::Continue)
            }
            KeyCode::Enter => {
                self.commit_page_pick();
                Some(ModalResult::CloseSuccess)
            }
            KeyCode::Backspace => {
                state.filter.pop();
                state.selected = 0;
                Some(ModalResult::Continue)
            }
            KeyCode::Char(c)
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && (c.is_alphanumeric() || c == '-' || c == '_' || c == ' ') =>
            {
                state.filter.push(c);
                state.selected = 0;
                Some(ModalResult::Continue)
            }
            _ => Some(ModalResult::Continue),
        }
    }
    pub(super) fn commit_page_pick(&mut self) {
        let (slug, binding) = {
            let Some(Modal::PagePicker { state }) = self.modal.as_ref() else {
                return;
            };
            let filtered = filter_pages(&state.pages, &state.filter);
            let Some((slug, _)) = filtered.get(state.selected).cloned() else {
                return;
            };
            (slug, state.binding.clone())
        };
        match binding {
            PagePickBinding::FormEditField { field_id } => {
                self.modal = self.paused_form_edit_modal.take();
                if let Some(Modal::FormEdit {
                    state, cursor_pos, ..
                }) = self.modal.as_mut()
                {
                    let value = crate::model::page_href(&slug);
                    state.set(&field_id, value.clone());
                    *cursor_pos = state.get(&field_id).len();
                    self.push_toast(
                        ToastLevel::Success,
                        format!("Picked page: {}", value),
                    );
                } else {
                    self.push_toast(
                        ToastLevel::Warning,
                        "Page pick lost: parent form modal closed.",
                    );
                }
            }
        }
    }
    pub(super) fn handle_modal_event(&mut self, evt: Event) -> Option<ModalResult> {
        let _ = self.modal.as_ref()?;

        if let Event::Key(key) = &evt {
            if key.code == KeyCode::F(1) {
                self.show_help = true;
                self.help_scroll = 0;
                return Some(ModalResult::Continue);
            }
            let key = *key;
            return match self.modal.as_ref()? {
                Modal::ComponentPicker { .. } => {
                    self.handle_component_picker_event_unified(key)
                }
                Modal::SavePrompt { .. } => self.handle_save_prompt_event_unified(key),
                Modal::FormEdit { .. } => self.handle_form_edit_event(key),
                Modal::TemplatePicker { .. } => self.handle_template_picker_event(key),
                Modal::NewPageTitlePrompt { .. } => self.handle_new_page_title_prompt_event(key),
                Modal::ExportPathPrompt { .. } => self.handle_export_path_prompt_event(key),
                Modal::PreviewPathPrompt { .. } => self.handle_preview_path_prompt_event(key),
                Modal::RenamePagePrompt { .. } => self.handle_rename_page_prompt_event(key),
                Modal::ConfirmPrompt { .. } => self.handle_confirm_prompt_event(key),
                Modal::ValidationErrors { .. } => self.handle_validation_errors_event(key),
                Modal::ImagePicker { .. } => self.handle_image_picker_event(key),
                Modal::PagePicker { .. } => self.handle_page_picker_event(key),
            };
        }

        if let Event::Mouse(m) = &evt {
            let kind = m.kind;
            // Click-to-focus inside multi-field modals: pick the input box
            // whose cached rect contains the click.
            if matches!(kind, MouseEventKind::Down(MouseButton::Left)) {
                let (col, row) = (m.column, m.row);
                let hit = self
                    .modal_field_areas
                    .borrow()
                    .iter()
                    .find(|(_, r)| {
                        col >= r.x
                            && col < r.x + r.width
                            && row >= r.y
                            && row < r.y + r.height
                    })
                    .map(|(idx, _)| *idx);
                if let Some(idx) = hit {
                    if let Some(modal) = self.modal.as_mut() {
                        match modal {
                            Modal::FormEdit { state, .. } => {
                                state.focused_field = idx;
                            }
                            _ => {}
                        }
                    }
                    return Some(ModalResult::Continue);
                }
            }
            match kind {
                MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                    let delta: i32 = if matches!(kind, MouseEventKind::ScrollUp) { -3 } else { 3 };
                    if let Some(modal) = self.modal.as_mut() {
                        match modal {
                            Modal::ValidationErrors { errors, scroll_offset } => {
                                let max = errors.len().saturating_sub(1);
                                let next = (*scroll_offset as i32 + delta).max(0) as usize;
                                *scroll_offset = next.min(max);
                            }
                            Modal::FormEdit { scroll_offset, .. } => {
                                let next = (*scroll_offset as i32 + delta).max(0) as u16;
                                *scroll_offset = next;
                            }
                            _ => {}
                        }
                    }
                    return Some(ModalResult::Continue);
                }
                _ => {}
            }
        }

        Some(ModalResult::Continue)
    }
    pub(super) fn handle_form_edit_event(&mut self, key: event::KeyEvent) -> Option<ModalResult> {
        use crossterm::event::{KeyCode, KeyModifiers};

        // Ctrl+P: open image picker on image_url fields, page picker on
        // link_url fields. Heuristic on field id since both kinds are
        // FieldKind::Url today.
        if matches!(key.code, KeyCode::Char('p'))
            && key.modifiers.contains(KeyModifiers::CONTROL)
        {
            let Some(Modal::FormEdit { state, .. }) = self.modal.as_ref() else {
                return Some(ModalResult::Continue);
            };
            let field_opt = state.form.fields.get(state.focused_field);
            let field_id = match field_opt {
                Some(f) if matches!(f.kind, editform::FieldKind::Url { .. }) => f.id.to_string(),
                _ => return Some(ModalResult::Continue),
            };

            if field_id.contains("image") {
                let base = self
                    .path
                    .as_ref()
                    .and_then(|p| p.parent().map(std::path::PathBuf::from))
                    .unwrap_or_else(|| std::path::PathBuf::from("."));
                let root = base.join("source").join("images");
                if !root.exists() {
                    self.push_toast(
                        ToastLevel::Warning,
                        format!("Source folder not found: {}", root.display()),
                    );
                    return Some(ModalResult::Continue);
                }
                let paused = self.modal.take();
                self.paused_form_edit_modal = paused;
                self.modal = Some(Modal::ImagePicker {
                    state: ImagePickerState {
                        root: root.clone(),
                        cwd: root,
                        filter: String::new(),
                        selected: 0,
                        binding: ImagePickBinding::FormEditField { field_id },
                    },
                });
                return Some(ModalResult::Continue);
            }

            if field_id.contains("link") {
                let pages: Vec<(String, String)> = self
                    .site
                    .pages
                    .iter()
                    .map(|p| (p.slug.clone(), p.head.title.clone()))
                    .collect();
                if pages.is_empty() {
                    self.push_toast(
                        ToastLevel::Warning,
                        "No pages to pick from.".to_string(),
                    );
                    return Some(ModalResult::Continue);
                }
                let paused = self.modal.take();
                self.paused_form_edit_modal = paused;
                self.modal = Some(Modal::PagePicker {
                    state: PagePickerState {
                        pages,
                        filter: String::new(),
                        selected: 0,
                        binding: PagePickBinding::FormEditField { field_id },
                    },
                });
                return Some(ModalResult::Continue);
            }

            // URL field with neither "image" nor "link" in its id — no
            // picker fits. Fall through silently so Ctrl+P is a no-op.
            return Some(ModalResult::Continue);
        }

        // Ctrl+S: drilled-down form returns to its parent; top-level form
        // commits to the model.
        if matches!(key.code, KeyCode::Char('s')) && key.modifiers.contains(KeyModifiers::CONTROL) {
            let taken = self.modal.take();
            if let Some(Modal::FormEdit {
                state,
                cursor,
                cursor_pos,
                mut drill_stack,
                scroll_offset: _,
            }) = taken
            {
                if let Some(frame) = drill_stack.pop() {
                    // Returning from a drilled-in item — write current state back
                    // into the parent's sub_state and make the parent the active form.
                    let mut parent = frame.parent_state;
                    let items = parent
                        .sub_state
                        .entry(frame.subform_field_id.clone())
                        .or_default();
                    if frame.item_idx < items.len() {
                        items[frame.item_idx] = state;
                    } else {
                        items.push(state);
                    }
                    self.push_toast(ToastLevel::Success, "Item saved — editing parent.");
                    self.modal = Some(Modal::FormEdit {
                        state: parent,
                        cursor,
                        cursor_pos: frame.parent_cursor_pos,
                        drill_stack,
                        scroll_offset: frame.parent_scroll_offset,
                    });
                    return Some(ModalResult::Continue);
                }
                // Top-level save: commit to the model.
                match cursor::apply_edit_form_to_component(&mut self.site, &cursor, &state) {
                    Ok(()) => {
                        let msg = format!("Saved {}.", state.form.title);
                        self.push_toast(ToastLevel::Success, msg);
                        return Some(ModalResult::CloseSuccess);
                    }
                    Err(e) => {
                        self.push_toast(ToastLevel::Warning, format!("Save failed: {e}"));
                        self.modal = Some(Modal::FormEdit {
                            state,
                            cursor,
                            cursor_pos,
                            drill_stack,
                            scroll_offset: 0,
                        });
                        return Some(ModalResult::Continue);
                    }
                }
            }
            return Some(ModalResult::CloseCancel);
        }
        // Esc: drilled-down discards and returns; top-level closes.
        if matches!(key.code, KeyCode::Esc) {
            let taken = self.modal.take();
            if let Some(Modal::FormEdit {
                state: _,
                cursor,
                cursor_pos: _,
                mut drill_stack,
                scroll_offset: _,
            }) = taken
            {
                if let Some(frame) = drill_stack.pop() {
                    self.push_toast(ToastLevel::Info, "Item edit cancelled.");
                    self.modal = Some(Modal::FormEdit {
                        state: frame.parent_state,
                        cursor,
                        cursor_pos: frame.parent_cursor_pos,
                        drill_stack,
                        scroll_offset: frame.parent_scroll_offset,
                    });
                    return Some(ModalResult::Continue);
                }
            }
            self.modal = None;
            return Some(ModalResult::CloseCancel);
        }

        let Some(Modal::FormEdit {
            state,
            cursor_pos,
            scroll_offset,
            ..
        }) = self.modal.as_mut()
        else {
            return Some(ModalResult::CloseCancel);
        };

        // Snapshot the focused field's id and kind (to satisfy borrow rules before mutation).
        let focused_idx = state.focused_field;
        let (field_id, is_enum, is_textarea, is_subform, accepts_text) = match state
            .form
            .fields
            .get(focused_idx)
        {
            Some(f) => (
                f.id,
                matches!(f.kind, editform::FieldKind::Enum { .. }),
                matches!(f.kind, editform::FieldKind::Textarea { .. }),
                matches!(f.kind, editform::FieldKind::SubForm { .. }),
                matches!(
                    f.kind,
                    editform::FieldKind::Text { .. }
                        | editform::FieldKind::Url { .. }
                        | editform::FieldKind::Textarea { .. }
                ),
            ),
            None => return Some(ModalResult::CloseCancel),
        };

        // SubForm collection handling: A/X/Enter/Up/Down operate on items list.
        if is_subform {
            match key.code {
                KeyCode::Char('A') => {
                    if let Some(new_item) = state.new_sub_item(field_id) {
                        let items = state.sub_state.entry(field_id.to_string()).or_default();
                        let selected = state
                            .selected_sub_item
                            .get(field_id)
                            .copied()
                            .unwrap_or(0);
                        let insert_at = if items.is_empty() {
                            0
                        } else {
                            (selected + 1).min(items.len())
                        };
                        items.insert(insert_at, new_item);
                        state
                            .selected_sub_item
                            .insert(field_id.to_string(), insert_at);
                        self.push_toast(ToastLevel::Success, "Item added.");
                    }
                    return Some(ModalResult::Continue);
                }
                KeyCode::Char('X') => {
                    let min_items = match state.form.fields[focused_idx].kind {
                        editform::FieldKind::SubForm { min_items, .. } => min_items,
                        _ => 0,
                    };
                    let items = state.sub_state.entry(field_id.to_string()).or_default();
                    if items.len() > min_items {
                        let selected = state
                            .selected_sub_item
                            .get(field_id)
                            .copied()
                            .unwrap_or(0);
                        if selected < items.len() {
                            items.remove(selected);
                            let new_sel = selected.min(items.len().saturating_sub(1));
                            state
                                .selected_sub_item
                                .insert(field_id.to_string(), new_sel);
                            self.push_toast(ToastLevel::Info, "Item removed.");
                        }
                    } else {
                        self.push_toast(ToastLevel::Warning, format!("Must keep at least {min_items} item(s)."));
                    }
                    return Some(ModalResult::Continue);
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    let selected = state
                        .selected_sub_item
                        .get(field_id)
                        .copied()
                        .unwrap_or(0);
                    let items_len = state
                        .sub_state
                        .get(field_id)
                        .map(|v| v.len())
                        .unwrap_or(0);
                    if items_len == 0 {
                        state.focus_prev();
                    *scroll_offset = auto_scroll_for_focus(state, *scroll_offset);
                        *cursor_pos =
                            state.get(state.form.fields[state.focused_field].id).len();
                    } else if selected == 0 {
                        state.focus_prev();
                    *scroll_offset = auto_scroll_for_focus(state, *scroll_offset);
                        *cursor_pos =
                            state.get(state.form.fields[state.focused_field].id).len();
                    } else {
                        state
                            .selected_sub_item
                            .insert(field_id.to_string(), selected - 1);
                    }
                    return Some(ModalResult::Continue);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    let selected = state
                        .selected_sub_item
                        .get(field_id)
                        .copied()
                        .unwrap_or(0);
                    let items_len = state
                        .sub_state
                        .get(field_id)
                        .map(|v| v.len())
                        .unwrap_or(0);
                    if selected + 1 < items_len {
                        state
                            .selected_sub_item
                            .insert(field_id.to_string(), selected + 1);
                    } else {
                        state.focus_next();
                    *scroll_offset = auto_scroll_for_focus(state, *scroll_offset);
                        *cursor_pos =
                            state.get(state.form.fields[state.focused_field].id).len();
                    }
                    return Some(ModalResult::Continue);
                }
                KeyCode::Enter => {
                    // Drill into the selected item by taking ownership of the modal.
                    let taken = self.modal.take();
                    if let Some(Modal::FormEdit {
                        mut state,
                        cursor,
                        cursor_pos,
                        mut drill_stack,
                        scroll_offset,
                    }) = taken
                    {
                        let selected = state
                            .selected_sub_item
                            .get(field_id)
                            .copied()
                            .unwrap_or(0);
                        let items_len = state
                            .sub_state
                            .get(field_id)
                            .map(|v| v.len())
                            .unwrap_or(0);
                        if selected < items_len {
                            let template = match &state.form.fields[focused_idx].kind {
                                editform::FieldKind::SubForm { template, .. } => *template,
                                _ => unreachable!(
                                    "is_subform was true but kind is not SubForm"
                                ),
                            };
                            let placeholder = editform::EditFormState::new(template);
                            let items = state
                                .sub_state
                                .get_mut(field_id)
                                .expect("sub_state present for SubForm field");
                            let item_state = std::mem::replace(&mut items[selected], placeholder);
                            let item_cursor_pos = item_state
                                .get(item_state.form.fields[item_state.focused_field].id)
                                .len();
                            drill_stack.push(DrillFrame {
                                parent_state: state,
                                parent_cursor_pos: cursor_pos,
                                parent_scroll_offset: scroll_offset,
                                subform_field_id: field_id.to_string(),
                                item_idx: selected,
                            });
                            self.modal = Some(Modal::FormEdit {
                                state: item_state,
                                cursor,
                                cursor_pos: item_cursor_pos,
                                drill_stack,
                                scroll_offset: 0,
                            });
                            self.push_toast(ToastLevel::Info, "Editing item. Ctrl+S returns to parent.");
                        } else {
                            // Nothing to drill into; restore modal unchanged.
                            self.modal = Some(Modal::FormEdit {
                                state,
                                cursor,
                                cursor_pos,
                                drill_stack,
                                scroll_offset,
                            });
                        }
                    }
                    return Some(ModalResult::Continue);
                }
                _ => {}
            }
        }

        match key.code {
            KeyCode::Tab => {
                state.focus_next();
                    *scroll_offset = auto_scroll_for_focus(state, *scroll_offset);
                *cursor_pos = state.get(state.form.fields[state.focused_field].id).len();
            }
            KeyCode::BackTab => {
                state.focus_prev();
                    *scroll_offset = auto_scroll_for_focus(state, *scroll_offset);
                *cursor_pos = state.get(state.form.fields[state.focused_field].id).len();
            }
            KeyCode::Left => {
                if is_enum {
                    state.cycle_enum(false);
                } else if *cursor_pos > 0 {
                    *cursor_pos -= 1;
                }
            }
            KeyCode::Right => {
                if is_enum {
                    state.cycle_enum(true);
                } else {
                    let len = state.get(field_id).len();
                    if *cursor_pos < len {
                        *cursor_pos += 1;
                    }
                }
            }
            KeyCode::Up => {
                if is_textarea {
                    *cursor_pos =
                        textarea_move_cursor_vertical(state.get(field_id), *cursor_pos, -1);
                } else {
                    state.focus_prev();
                    *scroll_offset = auto_scroll_for_focus(state, *scroll_offset);
                    *cursor_pos = state.get(state.form.fields[state.focused_field].id).len();
                }
            }
            KeyCode::Down => {
                if is_textarea {
                    *cursor_pos =
                        textarea_move_cursor_vertical(state.get(field_id), *cursor_pos, 1);
                } else {
                    state.focus_next();
                    *scroll_offset = auto_scroll_for_focus(state, *scroll_offset);
                    *cursor_pos = state.get(state.form.fields[state.focused_field].id).len();
                }
            }
            KeyCode::PageUp if is_textarea => {
                *cursor_pos =
                    textarea_move_cursor_vertical(state.get(field_id), *cursor_pos, -10);
            }
            KeyCode::PageDown if is_textarea => {
                *cursor_pos =
                    textarea_move_cursor_vertical(state.get(field_id), *cursor_pos, 10);
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                if accepts_text {
                    let current = state.get(field_id).to_string();
                    let pos = (*cursor_pos).min(current.len());
                    let mut new = String::with_capacity(current.len() + 1);
                    new.push_str(&current[..pos]);
                    new.push(c);
                    new.push_str(&current[pos..]);
                    state.set(field_id, new);
                    *cursor_pos = pos + 1;
                }
            }
            KeyCode::Backspace => {
                if accepts_text {
                    let current = state.get(field_id).to_string();
                    let pos = (*cursor_pos).min(current.len());
                    if pos > 0 {
                        let mut new = String::with_capacity(current.len() - 1);
                        new.push_str(&current[..pos - 1]);
                        new.push_str(&current[pos..]);
                        state.set(field_id, new);
                        *cursor_pos = pos - 1;
                    }
                }
            }
            KeyCode::Enter => {
                if is_textarea {
                    let current = state.get(field_id).to_string();
                    let pos = (*cursor_pos).min(current.len());
                    let mut new = String::with_capacity(current.len() + 1);
                    new.push_str(&current[..pos]);
                    new.push('\n');
                    new.push_str(&current[pos..]);
                    state.set(field_id, new);
                    *cursor_pos = pos + 1;
                } else {
                    state.focus_next();
                    *scroll_offset = auto_scroll_for_focus(state, *scroll_offset);
                    *cursor_pos = state.get(state.form.fields[state.focused_field].id).len();
                }
            }
            _ => {}
        }

        Some(ModalResult::Continue)
    }
    pub(super) fn handle_component_picker_event_unified(
        &mut self,
        key: event::KeyEvent,
    ) -> Option<ModalResult> {
        use crossterm::event::KeyCode;

        let (query, selected) =
            if let Some(Modal::ComponentPicker { query, selected }) = self.modal.take() {
                (query, selected)
            } else {
                return Some(ModalResult::CloseCancel);
            };

        match key.code {
            KeyCode::Esc => {
                self.push_toast(ToastLevel::Info, "Component picker cancelled.");
                return Some(ModalResult::CloseCancel);
            }
            KeyCode::Up => {
                let new_selected = selected.saturating_sub(1);
                self.modal = Some(Modal::ComponentPicker {
                    query,
                    selected: new_selected,
                });
            }
            KeyCode::Down => {
                let filtered = self.filtered_component_kinds(&query);
                let total = filtered.len();
                let new_selected = if total == 0 {
                    0
                } else {
                    (selected + 1).min(total - 1)
                };
                self.modal = Some(Modal::ComponentPicker {
                    query,
                    selected: new_selected,
                });
            }
            KeyCode::Backspace => {
                let mut new_query = query;
                new_query.pop();
                self.modal = Some(Modal::ComponentPicker {
                    query: new_query,
                    selected,
                });
                self.normalize_component_picker_selection();
            }
            KeyCode::Enter => {
                let filtered = self.filtered_component_kinds(&query);
                if let Some(kind) = filtered.get(selected.min(filtered.len().saturating_sub(1))) {
                    self.component_kind = *kind;
                    self.insert_selected_component_kind();
                    return Some(ModalResult::CloseSuccess);
                }
                self.push_toast(ToastLevel::Warning, "No component selected.");
                return Some(ModalResult::CloseCancel);
            }
            KeyCode::Char(c) => {
                let mut new_query = query;
                new_query.push(c);
                self.modal = Some(Modal::ComponentPicker {
                    query: new_query,
                    selected,
                });
                self.normalize_component_picker_selection();
            }
            _ => {
                // Restore modal if we didn't handle the key
                self.modal = Some(Modal::ComponentPicker { query, selected });
            }
        }

        self.sync_tree_row_with_selection();
        Some(ModalResult::Continue)
    }
    pub(super) fn handle_save_prompt_event_unified(&mut self, key: event::KeyEvent) -> Option<ModalResult> {
        use crossterm::event::KeyCode;

        let path = if let Some(Modal::SavePrompt { path }) = self.modal.take() {
            path
        } else {
            return Some(ModalResult::CloseCancel);
        };

        match key.code {
            KeyCode::Esc => {
                self.push_toast(ToastLevel::Info, "Save cancelled.");
                Some(ModalResult::CloseCancel)
            }
            KeyCode::Enter => {
                let raw = path.trim();
                if raw.is_empty() {
                    self.push_toast(ToastLevel::Warning, "Save path cannot be empty.");
                    self.modal = Some(Modal::SavePrompt { path });
                    Some(ModalResult::Continue)
                } else {
                    let path_buf = std::path::PathBuf::from(raw);
                    if let Err(e) = self.commit_save_with_backup(&path_buf) {
                        self.push_toast(ToastLevel::Warning, format!("Failed to save: {}", e));
                        self.modal = Some(Modal::SavePrompt { path });
                        Some(ModalResult::Continue)
                    } else {
                        let msg = format!("Saved {}", path_buf.display());
                        self.push_toast(ToastLevel::Success, msg);
                        Some(ModalResult::CloseSuccess)
                    }
                }
            }
            KeyCode::Backspace => {
                let mut new_path = path;
                new_path.pop();
                self.modal = Some(Modal::SavePrompt { path: new_path });
                Some(ModalResult::Continue)
            }
            KeyCode::Char(c) => {
                let mut new_path = path;
                new_path.push(c);
                self.modal = Some(Modal::SavePrompt { path: new_path });
                Some(ModalResult::Continue)
            }
            _ => {
                self.modal = Some(Modal::SavePrompt { path });
                Some(ModalResult::Continue)
            }
        }
    }
    pub(super) fn handle_template_picker_event(&mut self, key: event::KeyEvent) -> Option<ModalResult> {
        use crossterm::event::KeyCode;
        let Some(Modal::TemplatePicker { selected }) = self.modal.as_mut() else {
            return Some(ModalResult::CloseCancel);
        };
        match key.code {
            KeyCode::Esc => {
                self.modal = None;
                self.push_toast(ToastLevel::Info, "Add page cancelled.");
                Some(ModalResult::CloseCancel)
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if *selected > 0 {
                    *selected -= 1;
                }
                Some(ModalResult::Continue)
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if *selected < 3 {
                    *selected += 1;
                }
                Some(ModalResult::Continue)
            }
            KeyCode::Enter => {
                let picked = *selected;
                let title = self.pending_new_page_title.take().unwrap_or_default();
                if title.is_empty() {
                    self.modal = None;
                    self.push_toast(ToastLevel::Info, "Cancelled — no title.");
                    return Some(ModalResult::CloseCancel);
                }
                let mut new_page = match picked {
                    0 => crate::model::Page::from_template(
                        &title,
                        crate::model::PageTemplate::Blank,
                    ),
                    1 => crate::model::Page::from_template(
                        &title,
                        crate::model::PageTemplate::HeroOnly,
                    ),
                    2 => crate::model::Page::from_template(
                        &title,
                        crate::model::PageTemplate::HeroPlusSection,
                    ),
                    3 => {
                        let src_idx =
                            self.selected_page.min(self.site.pages.len().saturating_sub(1));
                        let src = &self.site.pages[src_idx];
                        crate::model::Page::duplicate_from(src)
                    }
                    _ => crate::model::Page::from_template(
                        &title,
                        crate::model::PageTemplate::Blank,
                    ),
                };
                // Dedup id/slug to avoid collisions.
                if self.site.pages.iter().any(|p| p.id == new_page.id) {
                    let base_id = new_page.id.clone();
                    let base_slug = new_page.slug.clone();
                    for n in 2.. {
                        let candidate_id = format!("{}-{}", base_id, n);
                        if !self.site.pages.iter().any(|p| p.id == candidate_id) {
                            new_page.id = candidate_id;
                            new_page.slug = format!("{}-{}", base_slug, n);
                            break;
                        }
                    }
                }
                self.site.pages.push(new_page);
                self.selected_page = self.site.pages.len() - 1;
                self.selected_node = 0;
                self.selected_column = 0;
                self.selected_component = 0;
                self.selected_nested_item = 0;
                self.modal = None;
                let msg = format!(
                    "Added page: {}",
                    self.site.pages[self.selected_page].head.title
                );
                self.push_toast(ToastLevel::Success, msg);
                Some(ModalResult::CloseSuccess)
            }
            _ => Some(ModalResult::Continue),
        }
    }
    pub(super) fn handle_new_page_title_prompt_event(
        &mut self,
        key: event::KeyEvent,
    ) -> Option<ModalResult> {
        use crossterm::event::KeyCode;

        let title = if let Some(Modal::NewPageTitlePrompt { title }) = self.modal.take() {
            title
        } else {
            return Some(ModalResult::CloseCancel);
        };

        match key.code {
            KeyCode::Esc => {
                self.push_toast(ToastLevel::Info, "Add page cancelled.");
                Some(ModalResult::CloseCancel)
            }
            KeyCode::Enter => {
                let trimmed = title.trim().to_string();
                if trimmed.is_empty() {
                    self.push_toast(ToastLevel::Warning, "Title required.");
                    self.modal = Some(Modal::NewPageTitlePrompt { title });
                    Some(ModalResult::Continue)
                } else {
                    self.pending_new_page_title = Some(trimmed);
                    self.modal = Some(Modal::TemplatePicker { selected: 0 });
                    Some(ModalResult::Continue)
                }
            }
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                let trimmed = title.trim().to_string();
                if trimmed.is_empty() {
                    self.push_toast(ToastLevel::Warning, "Title required.");
                    self.modal = Some(Modal::NewPageTitlePrompt { title });
                    Some(ModalResult::Continue)
                } else {
                    self.pending_new_page_title = Some(trimmed);
                    self.modal = Some(Modal::TemplatePicker { selected: 0 });
                    Some(ModalResult::Continue)
                }
            }
            KeyCode::Backspace => {
                let mut new_title = title;
                new_title.pop();
                self.modal = Some(Modal::NewPageTitlePrompt { title: new_title });
                Some(ModalResult::Continue)
            }
            KeyCode::Char(c) => {
                let mut new_title = title;
                new_title.push(c);
                self.modal = Some(Modal::NewPageTitlePrompt { title: new_title });
                Some(ModalResult::Continue)
            }
            _ => {
                self.modal = Some(Modal::NewPageTitlePrompt { title });
                Some(ModalResult::Continue)
            }
        }
    }
    pub(super) fn handle_export_path_prompt_event(&mut self, key: event::KeyEvent) -> Option<ModalResult> {
        use crossterm::event::KeyCode;
        let path = if let Some(Modal::ExportPathPrompt { path }) = self.modal.take() {
            path
        } else {
            return Some(ModalResult::CloseCancel);
        };
        match key.code {
            KeyCode::Esc => {
                self.push_toast(ToastLevel::Info, "Export cancelled.");
                Some(ModalResult::CloseCancel)
            }
            KeyCode::Enter => self.commit_export_path_from_prompt(path),
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.commit_export_path_from_prompt(path)
            }
            KeyCode::Backspace => {
                let mut new_path = path;
                new_path.pop();
                self.modal = Some(Modal::ExportPathPrompt { path: new_path });
                Some(ModalResult::Continue)
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                let mut new_path = path;
                new_path.push(c);
                self.modal = Some(Modal::ExportPathPrompt { path: new_path });
                Some(ModalResult::Continue)
            }
            _ => {
                self.modal = Some(Modal::ExportPathPrompt { path });
                Some(ModalResult::Continue)
            }
        }
    }
    pub(super) fn handle_preview_path_prompt_event(&mut self, key: event::KeyEvent) -> Option<ModalResult> {
        use crossterm::event::KeyCode;
        let path = if let Some(Modal::PreviewPathPrompt { path }) = self.modal.take() {
            path
        } else {
            return Some(ModalResult::CloseCancel);
        };
        match key.code {
            KeyCode::Esc => {
                self.push_toast(ToastLevel::Info, "Preview cancelled.");
                Some(ModalResult::CloseCancel)
            }
            KeyCode::Enter => self.commit_preview_path_from_prompt(path),
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.commit_preview_path_from_prompt(path)
            }
            KeyCode::Backspace => {
                let mut new_path = path;
                new_path.pop();
                self.modal = Some(Modal::PreviewPathPrompt { path: new_path });
                Some(ModalResult::Continue)
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                let mut new_path = path;
                new_path.push(c);
                self.modal = Some(Modal::PreviewPathPrompt { path: new_path });
                Some(ModalResult::Continue)
            }
            _ => {
                self.modal = Some(Modal::PreviewPathPrompt { path });
                Some(ModalResult::Continue)
            }
        }
    }
    pub(super) fn commit_preview_path_from_prompt(&mut self, path: String) -> Option<ModalResult> {
        let trimmed = path.trim();
        if trimmed.is_empty() {
            self.push_toast(ToastLevel::Warning, "Preview path required.");
            self.modal = Some(Modal::PreviewPathPrompt { path });
            Some(ModalResult::Continue)
        } else {
            self.commit_preview_to(trimmed.to_string());
            Some(ModalResult::CloseSuccess)
        }
    }
    pub(super) fn commit_preview_to(&mut self, rel: String) {
        use std::path::{Path, PathBuf};
        let normalized = normalize_relative_path(&rel);
        let base = self
            .path
            .as_ref()
            .and_then(|p| p.parent().map(PathBuf::from))
            .unwrap_or_else(|| PathBuf::from("."));
        let out = base.join(Path::new(&normalized));

        if let Err(e) = crate::export::export_site(&self.site, &out, Some(&base)) {
            let msg = format!("Preview failed: {}", e);
            self.push_toast(ToastLevel::Warning, msg);
            return;
        }
        self.site.export_dir = Some(normalized.clone());

        let display = display_relative_path(&base, &out, &normalized);
        let count = self.site.pages.len();
        self.push_toast(
            ToastLevel::Success,
            format!("Exported {} page(s) to {}", count, display),
        );
        match self.ensure_preview_server(out.clone()) {
            Ok(url) => match open_in_browser(&url) {
                Ok(()) => {
                    self.push_toast(
                        ToastLevel::Info,
                        format!("Opening {} in browser…", url),
                    );
                }
                Err(e) => {
                    self.push_toast(
                        ToastLevel::Warning,
                        format!("Browser open failed: {}", e),
                    );
                }
            },
            Err(e) => {
                self.push_toast(
                    ToastLevel::Warning,
                    format!("Preview server failed: {}", e),
                );
            }
        }
    }
    pub(super) fn ensure_preview_server(&mut self, out: PathBuf) -> anyhow::Result<String> {
        let slug = self.current_page_slug_for_preview();
        if let Some(server) = self.preview_server.as_ref() {
            server.set_root(out);
            return Ok(server.url_for(&slug));
        }
        let server = crate::serve::StaticServer::start(out)?;
        let url = server.url_for(&slug);
        self.preview_server = Some(server);
        Ok(url)
    }
    pub(super) fn current_page_slug_for_preview(&self) -> String {
        let idx = self.selected_page.min(self.site.pages.len().saturating_sub(1));
        self.site
            .pages
            .get(idx)
            .map(|p| p.slug.clone())
            .unwrap_or_else(|| "index".to_string())
    }
    pub(super) fn begin_preview_flow(&mut self) {
        let root = self.path.as_ref().and_then(|p| p.parent().map(std::path::Path::to_path_buf));
        let errors = crate::validate::validate_site_with_root(&self.site, root.as_deref());
        if !errors.is_empty() {
            self.modal = Some(Modal::ValidationErrors {
                errors,
                scroll_offset: 0,
            });
            return;
        }
        match self.site.export_dir.clone() {
            Some(dir) if !dir.trim().is_empty() => {
                self.commit_preview_to(dir);
            }
            _ => {
                self.modal = Some(Modal::PreviewPathPrompt {
                    path: "./web/".to_string(),
                });
            }
        }
    }
    pub(super) fn begin_export_flow(&mut self) {
        let root = self.path.as_ref().and_then(|p| p.parent().map(std::path::Path::to_path_buf));
        let errors = crate::validate::validate_site_with_root(&self.site, root.as_deref());
        if !errors.is_empty() {
            self.modal = Some(Modal::ValidationErrors {
                errors,
                scroll_offset: 0,
            });
            return;
        }
        match self.site.export_dir.clone() {
            Some(dir) if !dir.trim().is_empty() => {
                self.commit_export_to(dir);
            }
            _ => {
                self.modal = Some(Modal::ExportPathPrompt {
                    path: "./web/".to_string(),
                });
            }
        }
    }
    pub(super) fn commit_export_path_from_prompt(&mut self, path: String) -> Option<ModalResult> {
        let trimmed = path.trim();
        if trimmed.is_empty() {
            self.push_toast(ToastLevel::Warning, "Export path required.");
            self.modal = Some(Modal::ExportPathPrompt { path });
            Some(ModalResult::Continue)
        } else {
            self.commit_export_to(trimmed.to_string());
            Some(ModalResult::CloseSuccess)
        }
    }
    pub(super) fn commit_export_to(&mut self, rel: String) {
        use std::path::{Path, PathBuf};
        let normalized = normalize_relative_path(&rel);
        let base = self
            .path
            .as_ref()
            .and_then(|p| p.parent().map(PathBuf::from))
            .unwrap_or_else(|| PathBuf::from("."));
        let out = base.join(Path::new(&normalized));

        match crate::export::export_site(&self.site, &out, Some(&base)) {
            Ok(report) => {
                self.site.export_dir = Some(normalized.clone());
                let display = display_relative_path(&base, &out, &normalized);
                let msg = format!("Exported {} page(s) to {}", report.pages, display);
                self.push_toast(ToastLevel::Success, msg);
            }
            Err(e) => {
                let msg = format!("Export failed: {}", e);
                self.push_toast(ToastLevel::Warning, msg);
            }
        }
    }
    pub(super) fn handle_rename_page_prompt_event(&mut self, key: event::KeyEvent) -> Option<ModalResult> {
        use crossterm::event::KeyCode;
        let (title, page_idx) = match &self.modal {
            Some(Modal::RenamePagePrompt { title, page_idx }) => (title.clone(), *page_idx),
            _ => return Some(ModalResult::CloseCancel),
        };
        match key.code {
            KeyCode::Esc => {
                self.modal = None;
                self.push_toast(ToastLevel::Info, "Rename cancelled.");
                Some(ModalResult::CloseCancel)
            }
            KeyCode::Enter => self.commit_rename_page(title, page_idx),
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.commit_rename_page(title, page_idx)
            }
            KeyCode::Backspace => {
                let mut new_title = title;
                new_title.pop();
                self.modal = Some(Modal::RenamePagePrompt {
                    title: new_title,
                    page_idx,
                });
                Some(ModalResult::Continue)
            }
            KeyCode::Char(c) => {
                let mut new_title = title;
                new_title.push(c);
                self.modal = Some(Modal::RenamePagePrompt {
                    title: new_title,
                    page_idx,
                });
                Some(ModalResult::Continue)
            }
            _ => Some(ModalResult::Continue),
        }
    }
    pub(super) fn commit_rename_page(&mut self, title: String, page_idx: usize) -> Option<ModalResult> {
        let trimmed = title.trim();
        if trimmed.is_empty() {
            self.push_toast(ToastLevel::Warning, "Title required.");
            self.modal = Some(Modal::RenamePagePrompt { title, page_idx });
            return Some(ModalResult::Continue);
        }
        if let Some(page) = self.site.pages.get_mut(page_idx) {
            page.head.title = trimmed.to_string();
            if !page.slug_locked {
                page.slug = crate::model::slug_from_title(trimmed);
            }
            let msg = format!("Renamed page: {}", page.head.title);
            self.push_toast(ToastLevel::Success, msg);
        } else {
            self.push_toast(ToastLevel::Warning, "Page no longer exists.");
        }
        self.modal = None;
        Some(ModalResult::CloseSuccess)
    }
    pub(super) fn handle_confirm_prompt_event(&mut self, key: event::KeyEvent) -> Option<ModalResult> {
        use crossterm::event::KeyCode;
        let kind = match &self.modal {
            Some(Modal::ConfirmPrompt { on_confirm, .. }) => on_confirm.clone(),
            _ => return Some(ModalResult::CloseCancel),
        };
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                match kind {
                    ConfirmKind::DeletePage => self.commit_delete_page(),
                    ConfirmKind::QuitUnsaved => self.should_quit = true,
                }
                self.modal = None;
                Some(ModalResult::CloseSuccess)
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.modal = None;
                self.push_toast(ToastLevel::Info, "Cancelled.");
                Some(ModalResult::CloseCancel)
            }
            _ => Some(ModalResult::Continue),
        }
    }
    pub(super) fn commit_delete_page(&mut self) {
        if self.site.pages.len() <= 1 {
            self.push_toast(ToastLevel::Warning, "Cannot delete last page.");
            return;
        }
        let idx = self.selected_page.min(self.site.pages.len() - 1);
        let removed = self.site.pages.remove(idx);
        let msg = format!("Deleted page: {}", removed.head.title);
        self.push_toast(ToastLevel::Success, msg);
        self.deleted_pages.push(removed);
        // Cap trash at 20 (oldest dropped).
        if self.deleted_pages.len() > 20 {
            self.deleted_pages.remove(0);
        }
        self.selected_page = idx.min(self.site.pages.len() - 1);
        self.selected_node = 0;
        self.selected_column = 0;
        self.selected_component = 0;
        self.selected_nested_item = 0;
    }

    /// Run `validate_site` on the current site. Open `Modal::ValidationErrors`
    /// if any errors; otherwise set a green status and leave no modal open.
    pub(super) fn open_validation_modal(&mut self) {
        let root = self.path.as_ref().and_then(|p| p.parent().map(std::path::Path::to_path_buf));
        let errors = crate::validate::validate_site_with_root(&self.site, root.as_deref());
        if errors.is_empty() {
            self.push_toast(ToastLevel::Success, "No validation errors.");
        } else {
            self.modal = Some(Modal::ValidationErrors {
                errors,
                scroll_offset: 0,
            });
        }
    }
}

impl Modal {
    #[allow(dead_code)]
    pub(super) fn variant_name(&self) -> &'static str {
        match self {
            Modal::ComponentPicker { .. } => "ComponentPicker",
            Modal::SavePrompt { .. } => "SavePrompt",
            Modal::FormEdit { .. } => "FormEdit",
            Modal::TemplatePicker { .. } => "TemplatePicker",
            Modal::NewPageTitlePrompt { .. } => "NewPageTitlePrompt",
            Modal::ExportPathPrompt { .. } => "ExportPathPrompt",
            Modal::PreviewPathPrompt { .. } => "PreviewPathPrompt",
            Modal::RenamePagePrompt { .. } => "RenamePagePrompt",
            Modal::ConfirmPrompt { .. } => "ConfirmPrompt",
            Modal::ValidationErrors { .. } => "ValidationErrors",
            Modal::ImagePicker { .. } => "ImagePicker",
            Modal::PagePicker { .. } => "PagePicker",
        }
    }
}
