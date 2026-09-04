//! Keyboard, mouse, and Pages-panel dispatch.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Pane {
    Regions,
    Pages,
    Layout,
    Details,
}

impl App {
    /// First matching pane. Overlap (shouldn't happen) is Regions, Pages, Layout, Details.
    fn pane_at(&self, x: u16, y: u16) -> Option<Pane> {
        if contains(self.regions_area, x, y) {
            return Some(Pane::Regions);
        }
        if contains(self.pages_area, x, y) {
            return Some(Pane::Pages);
        }
        if contains(self.list_area, x, y) {
            return Some(Pane::Layout);
        }
        if contains(self.details_area, x, y) {
            return Some(Pane::Details);
        }
        None
    }

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
        // Overlay sits above modal + paused FormEdit. Esc/F1/F2 close only
        // the overlay; they must not drop ImagePicker or the paused form.
        if self.overlay.is_some() {
            if matches!(
                &evt,
                Event::Key(k) if matches!(k.code, KeyCode::F(1) | KeyCode::F(2) | KeyCode::Esc)
            ) {
                self.overlay = None;
                return Ok(());
            }

            let (track, drag_kind) = match &self.overlay {
                Some(Overlay::Help { .. }) => (self.help_scrollbar_track, ScrollbarDrag::Help),
                Some(Overlay::Theme { .. }) => (self.theme_scrollbar_track, ScrollbarDrag::Theme),
                None => unreachable!(),
            };
            let max = self.overlay_scroll_max;
            if let Some(Overlay::Help { scroll } | Overlay::Theme { scroll }) = &mut self.overlay {
                match evt {
                    Event::Key(k) => match k.code {
                        KeyCode::Down | KeyCode::Char('j') => {
                            *scroll = scroll.saturating_add(1).min(max);
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            *scroll = scroll.saturating_sub(1);
                        }
                        KeyCode::PageDown => {
                            *scroll = scroll.saturating_add(10).min(max);
                        }
                        KeyCode::PageUp => {
                            *scroll = scroll.saturating_sub(10);
                        }
                        KeyCode::Home | KeyCode::Char('g') => {
                            *scroll = 0;
                        }
                        KeyCode::End | KeyCode::Char('G') => {
                            *scroll = max;
                        }
                        _ => {}
                    },
                    Event::Mouse(m) => match m.kind {
                        MouseEventKind::ScrollUp => {
                            *scroll = scroll.saturating_sub(3);
                        }
                        MouseEventKind::ScrollDown => {
                            *scroll = scroll.saturating_add(3).min(max);
                        }
                        MouseEventKind::Down(MouseButton::Left) => {
                            if contains(track.rect, m.column, m.row) {
                                *scroll = (track.offset_at(m.row) as u16).min(max);
                                self.scrollbar_drag = Some(drag_kind);
                            }
                        }
                        MouseEventKind::Drag(MouseButton::Left) => {
                            if self.scrollbar_drag == Some(drag_kind) {
                                *scroll = (track.offset_at(m.row) as u16).min(max);
                            }
                        }
                        MouseEventKind::Up(_) => {
                            self.scrollbar_drag = None;
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
            return Ok(());
        }

        if let Some(modal_result) = self.handle_modal_event(evt.clone()) {
            match modal_result {
                ModalResult::Continue => return Ok(()),
                ModalResult::CloseSuccess => return Ok(()),
                ModalResult::CloseCancel => return Ok(()),
            }
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
                KeyCode::F(1) => self.overlay = Some(Overlay::Help { scroll: 0 }),
                KeyCode::F(2) => self.overlay = Some(Overlay::Theme { scroll: 0 }),
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
                KeyCode::PageUp => self.page_focused_pane(-5),
                KeyCode::PageDown => self.page_focused_pane(5),
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
                MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                    let up = matches!(m.kind, MouseEventKind::ScrollUp);
                    match self.pane_at(m.column, m.row) {
                        Some(Pane::Regions) => {
                            self.selected_region = if up {
                                SelectedRegion::Header
                            } else {
                                SelectedRegion::Footer
                            };
                            self.selected_tree_row = 0;
                        }
                        Some(Pane::Pages) => {
                            if up {
                                self.select_prev_page();
                            } else {
                                self.select_next_page();
                            }
                        }
                        Some(Pane::Layout) => {
                            if up {
                                self.select_prev();
                            } else {
                                self.select_next();
                            }
                        }
                        Some(Pane::Details) => {
                            self.scroll_details_by(if up { -3 } else { 3 });
                        }
                        None => {}
                    }
                }
                MouseEventKind::Down(MouseButton::Left) => {
                    let col = m.column;
                    let row = m.row;
                    // Hit-test scrollbars before pane click-to-select so a click on
                    // the bar jumps scroll instead of selecting a grain / tree row.
                    if contains(self.details_scrollbar_track.rect, col, row) {
                        self.details_scroll_row = self.details_scrollbar_track.offset_at(row);
                        self.scrollbar_drag = Some(ScrollbarDrag::Details);
                    } else if contains(self.layout_scrollbar_track.rect, col, row) {
                        self.jump_layout_to_scrollbar_y(row);
                        self.scrollbar_drag = Some(ScrollbarDrag::Layout);
                    } else {
                        self.scrollbar_drag = None;
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
                }
                MouseEventKind::Drag(MouseButton::Left) => {
                    if self.scrollbar_drag == Some(ScrollbarDrag::Details) {
                        self.details_scroll_row = self.details_scrollbar_track.offset_at(m.row);
                    } else if self.scrollbar_drag == Some(ScrollbarDrag::Layout) {
                        self.jump_layout_to_scrollbar_y(m.row);
                    }
                }
                MouseEventKind::Up(_) => {
                    self.scrollbar_drag = None;
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
            if contains(self.layout_scrollbar_track.rect, x, y) {
                return;
            }
            let idx = (y - body_top) as usize + self.layout_list_state.offset();
            if idx < tree_rows.len() {
                self.selected_tree_row = idx;
                self.apply_tree_row_selection(tree_rows[idx]);
                self.selected_sidebar_section = SidebarSection::Layouts;
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
                } else if rel == 1 {
                    self.selected_region = SelectedRegion::Footer;
                    self.selected_sidebar_section = SidebarSection::Regions;
                    self.sync_tree_row_with_selection();
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
        // Special case for double-click on an item (not the title bar) in the Pages list:
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

    /// PageUp/PageDown follow the focused sidebar pane. Layouts jump the tree
    /// selection; Pages jump the page list (clamped, no wrap); Regions (and
    /// any other pane) still scroll the Details blueprint.
    fn page_focused_pane(&mut self, delta: isize) {
        let steps = delta.unsigned_abs();
        match self.selected_sidebar_section {
            SidebarSection::Layouts => {
                for _ in 0..steps {
                    if delta > 0 {
                        self.select_next();
                    } else {
                        self.select_prev();
                    }
                }
            }
            SidebarSection::Pages => self.jump_selected_page_by(delta),
            SidebarSection::Regions => self.scroll_details_by(delta),
        }
    }

    /// Move `selected_page` by `delta`, clamped to `[0, n-1]`. No-op at the
    /// ends (unlike Tab / wheel, which wrap via `select_next_page`).
    fn jump_selected_page_by(&mut self, delta: isize) {
        if self.site.pages.is_empty() {
            return;
        }
        let last = (self.site.pages.len() - 1) as isize;
        let next = (self.selected_page as isize + delta).clamp(0, last) as usize;
        if next == self.selected_page {
            return;
        }
        self.selected_page = next;
        self.selected_node = 0;
        self.selected_tree_row = 0;
        self.selected_column = 0;
        self.selected_component = 0;
        self.selected_nested_item = 0;
        self.details_scroll_row = 0;
        self.sync_tree_row_with_selection();
    }

    /// Jump `selected_tree_row` to the first row of the window under `y` on
    /// the Layout scrollbar (proportional, same mapping as Details).
    fn jump_layout_to_scrollbar_y(&mut self, y: u16) {
        let offset = self.layout_scrollbar_track.offset_at(y);
        let rows = self.build_tree_rows();
        if rows.is_empty() {
            return;
        }
        let idx = offset.min(rows.len() - 1);
        self.selected_tree_row = idx;
        self.apply_tree_row_selection(rows[idx]);
        *self.layout_list_state.offset_mut() = offset.min(rows.len().saturating_sub(1));
        self.layout_list_state.select(Some(idx));
        self.selected_sidebar_section = SidebarSection::Layouts;
    }
}
