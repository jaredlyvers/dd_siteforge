//! Component / image / page picker events and commits.
use super::super::*;

impl App {
    pub(in crate::tui) fn handle_image_picker_event(&mut self, key: event::KeyEvent) -> Option<ModalResult> {
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

    pub(in crate::tui) fn image_picker_descend_or_pick(&mut self) {
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

    pub(in crate::tui) fn commit_image_pick(&mut self, value: String, binding: ImagePickBinding) {
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

    pub(in crate::tui) fn handle_page_picker_event(
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

    pub(in crate::tui) fn commit_page_pick(&mut self) {
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

    pub(in crate::tui) fn handle_component_picker_event_unified(
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
}
