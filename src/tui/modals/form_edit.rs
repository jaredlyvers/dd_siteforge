//! FormEdit key handling, drill-down, and field edits.
use super::super::*;

impl App {
    pub(in crate::tui) fn handle_form_edit_event(&mut self, key: event::KeyEvent) -> Option<ModalResult> {
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
}
