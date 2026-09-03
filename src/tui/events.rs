//! Keyboard, mouse, and Pages-panel dispatch.
use super::*;

impl App {
    /// Try to handle a key as a Pages-panel-scoped action.
    /// Returns `true` if the key was consumed — caller should short-circuit.
    pub(super) fn try_handle_pages_panel_key(&mut self, key: &event::KeyEvent) -> bool {
        use crossterm::event::{KeyCode, KeyModifiers};
        match key.code {
            KeyCode::Char('A') if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.modal = Some(Modal::NewPageTitlePrompt {
                    title: String::new(),
                });
                self.push_toast(ToastLevel::Info, "New page: type a title, Enter to continue.");
                true
            }
            KeyCode::Char('X') if key.modifiers.contains(KeyModifiers::SHIFT) => {
                if self.site.pages.len() <= 1 {
                    self.push_toast(ToastLevel::Warning, "Cannot delete last page.");
                } else {
                    let title = self.site.pages[self.selected_page].head.title.clone();
                    self.modal = Some(Modal::ConfirmPrompt {
                        message: format!("Delete \"{}\"? y/n", title),
                        on_confirm: ConfirmKind::DeletePage,
                    });
                }
                true
            }
            KeyCode::Char('u') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                if let Some(page) = self.deleted_pages.pop() {
                    let msg = format!("Restored page: {}", page.head.title);
                    self.push_toast(ToastLevel::Success, msg);
                    self.site.pages.push(page);
                    self.selected_page = self.site.pages.len() - 1;
                    self.selected_node = 0;
                    self.selected_column = 0;
                    self.selected_component = 0;
                    self.selected_nested_item = 0;
                } else {
                    self.push_toast(ToastLevel::Warning, "No deleted pages to restore.");
                }
                true
            }
            KeyCode::Char('J') if key.modifiers.contains(KeyModifiers::SHIFT) => {
                let idx = self.selected_page;
                if idx + 1 < self.site.pages.len() {
                    self.site.pages.swap(idx, idx + 1);
                    self.selected_page = idx + 1;
                    self.push_toast(ToastLevel::Success, "Moved page down.");
                }
                true
            }
            KeyCode::Char('K') if key.modifiers.contains(KeyModifiers::SHIFT) => {
                let idx = self.selected_page;
                if idx > 0 {
                    self.site.pages.swap(idx, idx - 1);
                    self.selected_page = idx - 1;
                    self.push_toast(ToastLevel::Success, "Moved page up.");
                }
                true
            }
            KeyCode::Char('r')
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::SHIFT) =>
            {
                let idx = self.selected_page;
                let current_title = self.site.pages[idx].head.title.clone();
                self.modal = Some(Modal::RenamePagePrompt {
                    title: current_title,
                    page_idx: idx,
                });
                self.push_toast(ToastLevel::Info, "Rename page. Edit and press Enter.");
                true
            }
            _ => false,
        }
    }

    pub(super) fn handle_event(&mut self, evt: Event) -> anyhow::Result<()> {
        // Help/theme overlays sit above modals so F1 works from FormEdit.
        if !(self.show_help || self.show_theme) {
            if let Some(modal_result) = self.handle_modal_event(evt.clone()) {
                match modal_result {
                    ModalResult::Continue => return Ok(()),
                    ModalResult::CloseSuccess => return Ok(()),
                    ModalResult::CloseCancel => return Ok(()),
                }
            }
        }

        if self.show_help {
            match evt {
                Event::Key(k) => match k.code {
                    KeyCode::F(1) | KeyCode::Esc => {
                        self.show_help = false;
                        self.help_scroll = 0;
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.help_scroll = self
                            .help_scroll
                            .saturating_add(1)
                            .min(self.help_scroll_max);
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.help_scroll = self.help_scroll.saturating_sub(1);
                    }
                    KeyCode::PageDown => {
                        self.help_scroll = self
                            .help_scroll
                            .saturating_add(10)
                            .min(self.help_scroll_max);
                    }
                    KeyCode::PageUp => {
                        self.help_scroll = self.help_scroll.saturating_sub(10);
                    }
                    KeyCode::Home | KeyCode::Char('g') => {
                        self.help_scroll = 0;
                    }
                    KeyCode::End | KeyCode::Char('G') => {
                        self.help_scroll = self.help_scroll_max;
                    }
                    _ => {}
                },
                Event::Mouse(m) => match m.kind {
                    MouseEventKind::ScrollUp => {
                        self.help_scroll = self.help_scroll.saturating_sub(3);
                    }
                    MouseEventKind::ScrollDown => {
                        self.help_scroll = self
                            .help_scroll
                            .saturating_add(3)
                            .min(self.help_scroll_max);
                    }
                    _ => {}
                },
                _ => {}
            }
            return Ok(());
        }

        if self.show_theme {
            match evt {
                Event::Key(k) => match k.code {
                    KeyCode::F(2) | KeyCode::Esc => {
                        self.show_theme = false;
                        self.theme_scroll = 0;
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.theme_scroll = self
                            .theme_scroll
                            .saturating_add(1)
                            .min(self.theme_scroll_max);
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.theme_scroll = self.theme_scroll.saturating_sub(1);
                    }
                    KeyCode::PageDown => {
                        self.theme_scroll = self
                            .theme_scroll
                            .saturating_add(10)
                            .min(self.theme_scroll_max);
                    }
                    KeyCode::PageUp => {
                        self.theme_scroll = self.theme_scroll.saturating_sub(10);
                    }
                    KeyCode::Home | KeyCode::Char('g') => {
                        self.theme_scroll = 0;
                    }
                    KeyCode::End | KeyCode::Char('G') => {
                        self.theme_scroll = self.theme_scroll_max;
                    }
                    _ => {}
                },
                Event::Mouse(m) => match m.kind {
                    MouseEventKind::ScrollUp => {
                        self.theme_scroll = self.theme_scroll.saturating_sub(3);
                    }
                    MouseEventKind::ScrollDown => {
                        self.theme_scroll = self
                            .theme_scroll
                            .saturating_add(3)
                            .min(self.theme_scroll_max);
                    }
                    _ => {}
                },
                _ => {}
            }
            return Ok(());
        }

        match evt {
            Event::Key(k) => {
                if self.selected_sidebar_section == SidebarSection::Pages
                    && self.try_handle_pages_panel_key(&k)
                {
                    self.sync_tree_row_with_selection();
                    return Ok(());
                }
                match k.code {
                KeyCode::F(1) => self.show_help = true,
                KeyCode::F(2) => self.show_theme = true,
                KeyCode::F(3) => self.open_validation_modal(),
                KeyCode::Char('E') if k.modifiers.contains(KeyModifiers::SHIFT) => {
                    self.begin_export_flow();
                }
                KeyCode::Char('p') if !k.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.begin_preview_flow();
                }
                KeyCode::Char('q') if k.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.request_quit();
                }
                KeyCode::Up => self.handle_up(),
                KeyCode::Down => self.handle_down(),
                KeyCode::Char('k') => self.handle_up(),
                KeyCode::Char('j') => self.handle_down(),
                KeyCode::Char('h') => self.vim_collapse_selected_row(),
                KeyCode::Char('l') => self.vim_expand_selected_row(),
                KeyCode::Char('g') => self.vim_jump_to_first_row(),
                KeyCode::Char('G') => self.vim_jump_to_last_row(),
                KeyCode::PageUp => self.scroll_details_by(-5),
                KeyCode::PageDown => self.scroll_details_by(5),
                KeyCode::Char(' ') => self.toggle_selected_tree_expanded(),
                KeyCode::Enter => self.handle_enter_on_selected_row(),
                KeyCode::Tab => self.select_next_page(),
                KeyCode::BackTab => self.select_prev_page(),
                KeyCode::Char('s') => self.begin_save_prompt(),
                KeyCode::Char('/') => self.open_component_picker(),
                KeyCode::Char('d') => self.delete_selected_row(),
                KeyCode::Char('y') => self.duplicate_selected_row(),
                KeyCode::Char('u') => self.undo_last(),
                KeyCode::Char('J') => self.move_selected_row(1),
                KeyCode::Char('K') => self.move_selected_row(-1),
                KeyCode::Char('C') => self.add_column(),
                KeyCode::Char('V') => self.remove_selected_column(),
                KeyCode::Char('c') => self.select_prev_column(),
                KeyCode::Char('v') => self.select_next_column(),
                KeyCode::Char('r') => self.begin_edit_selected_column_id(),
                KeyCode::Char('f') => self.begin_edit_selected_column_width_class(),
                KeyCode::Char('A') => self.add_selected_collection_item(),
                KeyCode::Char('X') => self.remove_selected_collection_item(),
                KeyCode::Char('1') => {
                    self.selected_sidebar_section = SidebarSection::Regions;
                }
                KeyCode::Char('2') => {
                    self.selected_sidebar_section = SidebarSection::Pages;
                    self.selected_region = SelectedRegion::Page;
                    self.selected_tree_row = 0;
                    self.sync_tree_row_with_selection();
                }
                KeyCode::Char('3') => {
                    self.selected_sidebar_section = SidebarSection::Layouts;
                }
                _ => {}
                }
            },
            Event::Mouse(m) => match m.kind {
                MouseEventKind::ScrollUp => {
                    if contains(self.details_area, m.column, m.row) {
                        self.scroll_details_by(-3);
                    } else {
                        self.select_prev();
                    }
                }
                MouseEventKind::ScrollDown => {
                    if contains(self.details_area, m.column, m.row) {
                        self.scroll_details_by(3);
                    } else {
                        self.select_next();
                    }
                }
                MouseEventKind::Down(MouseButton::Left) => {
                    let col = m.column;
                    let row = m.row;
                    let now = std::time::Instant::now();
                    let is_double = if let Some((last_col, last_row, last_time)) = self.last_mouse_click {
                        last_col == col && last_row == row && now.duration_since(last_time).as_millis() < DOUBLE_CLICK_THRESHOLD_MS
                    } else {
                        false
                    };
                    self.last_mouse_click = Some((col, row, now));
                    if is_double {
                        self.handle_double_click(col, row);
                    } else {
                        self.handle_click(col, row);
                    }
                }
                _ => {}
            },
            _ => {}
        }
        self.sync_tree_row_with_selection();
        Ok(())
    }

    pub(super) fn handle_click(&mut self, x: u16, y: u16) {
        // Layout tree (list_area)
        if contains(self.list_area, x, y) {
            let tree_rows = self.build_tree_rows();
            if tree_rows.is_empty() {
                return;
            }
            let body_top = self.list_area.y.saturating_add(1);
            let body_bottom = self
                .list_area
                .y
                .saturating_add(self.list_area.height.saturating_sub(1));
            if y < body_top || y >= body_bottom {
                return;
            }
            let idx = (y - body_top) as usize + self.layout_list_state.offset();
            if idx < tree_rows.len() {
                self.selected_tree_row = idx;
                self.apply_tree_row_selection(tree_rows[idx]);
                self.selected_sidebar_section = SidebarSection::Layouts;
                self.push_toast(ToastLevel::Info, format!("Selected {}", self.tree_row_label(&tree_rows[idx])));
            }
            return;
        }

        // Pages list
        if contains(self.pages_area, x, y) {
            let body_top = self.pages_area.y.saturating_add(1);
            if y >= body_top {
                let rel = (y - body_top) as usize + self.pages_list_state.offset();
                if rel < self.site.pages.len() {
                    self.selected_page = rel;
                    self.selected_node = 0;
                    self.selected_column = 0;
                    self.selected_component = 0;
                    self.selected_nested_item = 0;
                    self.details_scroll_row = 0;
                    self.selected_tree_row = 0;
                    self.page_head_selected = false;
                    self.selected_region = SelectedRegion::Page;
                    self.selected_sidebar_section = SidebarSection::Pages;
                    self.sync_tree_row_with_selection();
                }
            }
            return;
        }

        // Regions list
        if contains(self.regions_area, x, y) {
            let body_top = self.regions_area.y.saturating_add(1);
            if y >= body_top {
                let rel = (y - body_top) as usize;
                if rel == 0 {
                    self.selected_region = SelectedRegion::Header;
                    self.selected_header_section = 0;
                    self.selected_header_column = 0;
                    self.selected_header_component = 0;
                    self.selected_sidebar_section = SidebarSection::Regions;
                    self.sync_tree_row_with_selection();
                    self.push_toast(ToastLevel::Info, "Selected Header region.");
                } else if rel == 1 {
                    self.selected_region = SelectedRegion::Footer;
                    self.selected_sidebar_section = SidebarSection::Regions;
                    self.sync_tree_row_with_selection();
                    self.push_toast(ToastLevel::Info, "Selected Footer region.");
                }
            }
            return;
        }

        // Details panel
        if contains(self.details_area, x, y) {
            let area = self.details_area;
            let content_top = area.y.saturating_add(1);
            let content_left = area.x.saturating_add(1);
            let scrollbar_x = area.x.saturating_add(area.width.saturating_sub(2));
            let content_bottom = area.y.saturating_add(area.height.saturating_sub(1));
            if y < content_top || y >= content_bottom || x < content_left || x >= scrollbar_x {
                return;
            }
            let rel = (y - content_top) as usize;
            let text_line = rel + self.details_scroll_row;
            let char_x = (x as usize).saturating_sub(content_left as usize);
            self.select_item_from_details_click(text_line, char_x);
            return;
        }
    }

    pub(super) fn handle_double_click(&mut self, x: u16, y: u16) {
        self.handle_click(x, y);
        // Special case for double-click on an item (not the title bar) in the Nodes / Pages list:
        // Switch to that page (already done in handle_click), then pin the Layout tree selection
        // to its [HEAD] row (by setting the flag + resync), and activate the Layouts sidebar section.
        // The handle_enter below will then see a PageHead row and open the unified page-head editor.
        let pages_body_top = self.pages_area.y.saturating_add(1);
        if contains(self.pages_area, x, y) && y >= pages_body_top {
            self.selected_sidebar_section = SidebarSection::Layouts;
            self.page_head_selected = true;
            self.sync_tree_row_with_selection();
        }
        self.handle_enter_on_selected_row();
    }
}
