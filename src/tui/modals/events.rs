//! Modal event router plus save/template/rename/confirm/validation.
use super::super::*;

impl App {
    pub(in crate::tui) fn handle_validation_errors_event(&mut self, key: event::KeyEvent) -> Option<ModalResult> {
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

    pub(in crate::tui) fn handle_modal_event(&mut self, evt: Event) -> Option<ModalResult> {
        let _ = self.modal.as_ref()?;

        if let Event::Key(key) = &evt {
            if key.code == KeyCode::F(1) {
                self.overlay = Some(Overlay::Help { scroll: 0 });
                return Some(ModalResult::Continue);
            }
            if key.code == KeyCode::F(2) {
                self.overlay = Some(Overlay::Theme { scroll: 0 });
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
            let (col, row) = (m.column, m.row);

            if matches!(kind, MouseEventKind::Up(_)) {
                self.scrollbar_drag = None;
                return Some(ModalResult::Continue);
            }

            if matches!(kind, MouseEventKind::Drag(MouseButton::Left))
                && self.scrollbar_drag == Some(ScrollbarDrag::FormEdit)
            {
                let sb = *self.form_scrollbar_track.borrow();
                if let Some(Modal::FormEdit { scroll_offset, .. }) = self.modal.as_mut() {
                    *scroll_offset = sb.offset_at(row).min(u16::MAX as usize) as u16;
                }
                return Some(ModalResult::Continue);
            }

            // FormEdit scrollbar before field click-to-focus so a track click
            // jumps instead of focusing the adjacent input.
            if matches!(kind, MouseEventKind::Down(MouseButton::Left)) {
                let expand_hit = self
                    .form_expand_hits
                    .borrow()
                    .iter()
                    .find(|(_, r)| contains(*r, col, row))
                    .map(|(idx, _)| *idx);
                if let Some(idx) = expand_hit {
                    if let Some(Modal::FormEdit { state, cursor_pos, .. }) = self.modal.as_mut() {
                        state.focused_field = idx;
                        let field_id = state.form.fields.get(idx).map(|f| f.id);
                        if let Some(field_id) = field_id {
                            *cursor_pos = state.get(field_id).len();
                        }
                    }
                    self.form_textarea_expanded = true;
                    return Some(ModalResult::Continue);
                }
                if self.form_textarea_expanded {
                    return Some(ModalResult::Continue);
                }
                let sb = *self.form_scrollbar_track.borrow();
                if contains(sb.rect, col, row) {
                    if let Some(Modal::FormEdit { scroll_offset, .. }) = self.modal.as_mut() {
                        *scroll_offset = sb.offset_at(row).min(u16::MAX as usize) as u16;
                    }
                    self.scrollbar_drag = Some(ScrollbarDrag::FormEdit);
                    return Some(ModalResult::Continue);
                }
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
                    if self.form_textarea_expanded {
                        if let Some(Modal::FormEdit {
                            state, cursor_pos, ..
                        }) = self.modal.as_mut()
                        {
                            let field_id = match state.form.fields.get(state.focused_field) {
                                Some(f)
                                    if matches!(f.kind, editform::FieldKind::Textarea { .. }) =>
                                {
                                    Some(f.id)
                                }
                                _ => None,
                            };
                            if let Some(field_id) = field_id {
                                *cursor_pos = textarea_move_cursor_vertical(
                                    state.get(field_id),
                                    *cursor_pos,
                                    delta as isize,
                                );
                            }
                        }
                        return Some(ModalResult::Continue);
                    }
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

    pub(in crate::tui) fn handle_save_prompt_event_unified(&mut self, key: event::KeyEvent) -> Option<ModalResult> {
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
                        self.push_toast(ToastLevel::Error, format!("Failed to save: {}", e));
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

    pub(in crate::tui) fn handle_template_picker_event(&mut self, key: event::KeyEvent) -> Option<ModalResult> {
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

    pub(in crate::tui) fn handle_new_page_title_prompt_event(
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

    pub(in crate::tui) fn handle_rename_page_prompt_event(&mut self, key: event::KeyEvent) -> Option<ModalResult> {
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

    pub(in crate::tui) fn commit_rename_page(&mut self, title: String, page_idx: usize) -> Option<ModalResult> {
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

    pub(in crate::tui) fn handle_confirm_prompt_event(&mut self, key: event::KeyEvent) -> Option<ModalResult> {
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

    pub(in crate::tui) fn commit_delete_page(&mut self) {
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
    pub(in crate::tui) fn open_validation_modal(&mut self) {
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
