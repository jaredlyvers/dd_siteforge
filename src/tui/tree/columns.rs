//! Column add/remove/select/reorder and column FormEdit.
use super::super::*;

impl App {
    pub(in crate::tui) fn mutate_selected_section<F>(&mut self, mutator: F, success_message: &str)
    where
        F: FnOnce(&mut crate::model::DdSection),
    {
        let prev_selected_component = self.selected_component;
        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.push_toast(ToastLevel::Warning, "No selected section.");
            return;
        }
        let idx = selected.min(page.nodes.len() - 1);
        let result = match &mut page.nodes[idx] {
            PageNode::Section(section) => {
                normalize_section_columns(section);
                mutator(section);
                let col_i = selected_column.min(section.columns.len().saturating_sub(1));
                let next_selected_component = prev_selected_component
                    .min(section.columns[col_i].components.len().saturating_sub(1));
                (Some(next_selected_component), success_message.to_string())
            }
            _ => (None, "Selected node is not a section.".to_string()),
        };
        if let Some(next_selected_component) = result.0 {
            self.selected_component = next_selected_component;
        }
        self.push_toast(ToastLevel::Info, result.1);
    }

    pub(in crate::tui) fn add_column(&mut self) {
        if self.warn_site_settings_unavailable() {
            return;
        }
        // Check if we're in Header mode
        if self.selected_region == SelectedRegion::Header {
            self.add_column_to_header_section();
            return;
        }

        self.mutate_selected_section(
            |section| {
                normalize_section_columns(section);
                let next = section.columns.len() + 1;
                section.columns.push(SectionColumn {
                    id: format!("column-{}", next),
                    width_class: "dd-u-1-1".to_string(),
                    components: Vec::new(),
                });
            },
            "Added column to section.",
        );
        if let Some(total) = self.selected_section_column_total() {
            if total > 0 {
                self.selected_column = total - 1;
            }
        }
        self.selected_component = 0;
        self.selected_nested_item = 0;
    }

    pub(in crate::tui) fn add_column_to_header_section(&mut self) {
        if self.site.header.sections.is_empty() {
            self.push_toast(ToastLevel::Warning, "No header section available. Add a section first with '/'.");
            return;
        }
        let section_idx = self
            .selected_header_section
            .min(self.site.header.sections.len().saturating_sub(1));
        let section = &mut self.site.header.sections[section_idx];
        normalize_section_columns(section);
        let next = section.columns.len() + 1;
        section.columns.push(SectionColumn {
            id: format!("column-{}", next),
            width_class: "dd-u-1-1".to_string(),
            components: Vec::new(),
        });
        self.selected_header_column = section.columns.len() - 1;
        self.selected_header_component = 0;
        let section_id = section.id.clone();
        self.push_toast(ToastLevel::Info, format!("Added column to header section '{}'.", section_id));
    }

    pub(in crate::tui) fn remove_selected_column(&mut self) {
        if self.warn_site_settings_unavailable() {
            return;
        }
        // Check if we're in Header mode
        if self.selected_region == SelectedRegion::Header {
            self.remove_column_from_header_section();
            return;
        }

        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.push_toast(ToastLevel::Warning, "No selected section.");
            return;
        }
        let ni = selected.min(page.nodes.len() - 1);
        let result = match &mut page.nodes[ni] {
            PageNode::Section(section) => {
                normalize_section_columns(section);
                if section.columns.len() <= 1 {
                    (None, "Section must keep at least one column.".to_string())
                } else {
                    let ci = selected_column.min(section.columns.len() - 1);
                    section.columns.remove(ci);
                    (
                        Some(ci.min(section.columns.len() - 1)),
                        "Removed selected column.".to_string(),
                    )
                }
            }
            _ => (None, "Selected node is not a section.".to_string()),
        };
        if let Some(next_selected_column) = result.0 {
            self.selected_column = next_selected_column;
            self.selected_component = 0;
            self.selected_nested_item = 0;
        }
        self.push_toast(ToastLevel::Info, result.1);
    }

    pub(in crate::tui) fn remove_column_from_header_section(&mut self) {
        if self.site.header.sections.is_empty() {
            self.push_toast(ToastLevel::Warning, "No header sections to modify.");
            return;
        }
        let section_idx = self
            .selected_header_section
            .min(self.site.header.sections.len().saturating_sub(1));
        let section = &mut self.site.header.sections[section_idx];
        normalize_section_columns(section);
        if section.columns.len() <= 1 {
            self.push_toast(ToastLevel::Warning, "Header section must keep at least one column.");
            return;
        }
        let ci = self.selected_header_column.min(section.columns.len() - 1);
        section.columns.remove(ci);
        self.selected_header_column = ci.min(section.columns.len() - 1);
        self.selected_header_component = 0;
        self.push_toast(ToastLevel::Info, "Removed column from header section.");
    }

    pub(in crate::tui) fn select_prev_column(&mut self) {
        if self.warn_site_settings_unavailable() {
            return;
        }
        // Check if we're in Header mode
        if self.selected_region == SelectedRegion::Header {
            let total = match self.selected_header_section_column_total() {
                Some(v) => v,
                None => {
                    self.push_toast(ToastLevel::Warning, "No header section selected.");
                    return;
                }
            };
            if total == 0 {
                self.push_toast(ToastLevel::Warning, "Selected header section has no columns.");
                return;
            }
            self.selected_header_column = self.selected_header_column.saturating_sub(1);
            self.selected_header_component = 0;
            self.push_toast(ToastLevel::Info, format!(
                "Selected header column {} of {}.",
                self.selected_header_column + 1,
                total
            ));
            return;
        }

        let total = match self.selected_section_column_total() {
            Some(v) => v,
            None => {
                self.push_toast(ToastLevel::Warning, "Selected node is not a section.");
                return;
            }
        };
        if total == 0 {
            self.push_toast(ToastLevel::Warning, "Selected section has no columns.");
            return;
        }
        self.selected_column = self.selected_column.saturating_sub(1);
        self.selected_component = 0;
        self.selected_nested_item = 0;
        self.push_toast(ToastLevel::Info, format!("Selected column {} of {}.", self.selected_column + 1, total));
    }

    pub(in crate::tui) fn select_next_column(&mut self) {
        if self.warn_site_settings_unavailable() {
            return;
        }
        // Check if we're in Header mode
        if self.selected_region == SelectedRegion::Header {
            let total = match self.selected_header_section_column_total() {
                Some(v) => v,
                None => {
                    self.push_toast(ToastLevel::Warning, "No header section selected.");
                    return;
                }
            };
            if total == 0 {
                self.push_toast(ToastLevel::Warning, "Selected header section has no columns.");
                return;
            }
            self.selected_header_column = (self.selected_header_column + 1).min(total - 1);
            self.selected_header_component = 0;
            self.push_toast(ToastLevel::Info, format!(
                "Selected header column {} of {}.",
                self.selected_header_column + 1,
                total
            ));
            return;
        }

        let total = match self.selected_section_column_total() {
            Some(v) => v,
            None => {
                self.push_toast(ToastLevel::Warning, "Selected node is not a section.");
                return;
            }
        };
        if total == 0 {
            self.push_toast(ToastLevel::Warning, "Selected section has no columns.");
            return;
        }
        self.selected_column = (self.selected_column + 1).min(total - 1);
        self.selected_component = 0;
        self.selected_nested_item = 0;
        self.push_toast(ToastLevel::Info, format!("Selected column {} of {}.", self.selected_column + 1, total));
    }

    pub(in crate::tui) fn selected_header_section_column_total(&self) -> Option<usize> {
        if self.site.header.sections.is_empty() {
            return None;
        }
        let section_idx = self
            .selected_header_section
            .min(self.site.header.sections.len().saturating_sub(1));
        Some(self.site.header.sections[section_idx].columns.len())
    }

    pub(in crate::tui) fn move_selected_column_up(&mut self) {
        if self.warn_site_settings_unavailable() {
            return;
        }
        // Check if we're in Header mode
        if self.selected_region == SelectedRegion::Header {
            if self.site.header.sections.is_empty() {
                self.push_toast(ToastLevel::Warning, "No header sections to modify.");
                return;
            }
            let section_idx = self
                .selected_header_section
                .min(self.site.header.sections.len().saturating_sub(1));
            let section = &mut self.site.header.sections[section_idx];
            normalize_section_columns(section);
            if section.columns.len() < 2 {
                self.push_toast(ToastLevel::Warning, "Need at least 2 columns.");
                return;
            }
            let ci = self.selected_header_column.min(section.columns.len() - 1);
            if ci == 0 {
                self.push_toast(ToastLevel::Info, "Column is already first.");
                return;
            }
            section.columns.swap(ci, ci - 1);
            self.selected_header_column = ci - 1;
            self.snap_tree_row_to_header_column(section_idx, ci - 1);
            self.push_toast(ToastLevel::Info, "Moved header column up.");
            return;
        }

        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.push_toast(ToastLevel::Warning, "No selected section.");
            return;
        }
        let ni = selected.min(page.nodes.len() - 1);
        let result = match &mut page.nodes[ni] {
            PageNode::Section(section) => {
                normalize_section_columns(section);
                if section.columns.len() < 2 {
                    (None, "Need at least 2 columns.".to_string())
                } else {
                    let ci = selected_column.min(section.columns.len() - 1);
                    if ci == 0 {
                        (None, "Column is already first.".to_string())
                    } else {
                        section.columns.swap(ci, ci - 1);
                        (Some(ci - 1), "Moved column up.".to_string())
                    }
                }
            }
            _ => (None, "Selected node is not a section.".to_string()),
        };
        if let Some(next_selected_column) = result.0 {
            self.selected_column = next_selected_column;
            self.selected_component = 0;
            self.selected_nested_item = 0;
            self.snap_tree_row_to_column(ni, next_selected_column);
        }
        self.push_toast(ToastLevel::Info, result.1);
    }

    pub(in crate::tui) fn move_selected_column_down(&mut self) {
        if self.warn_site_settings_unavailable() {
            return;
        }
        // Check if we're in Header mode
        if self.selected_region == SelectedRegion::Header {
            if self.site.header.sections.is_empty() {
                self.push_toast(ToastLevel::Warning, "No header sections to modify.");
                return;
            }
            let section_idx = self
                .selected_header_section
                .min(self.site.header.sections.len().saturating_sub(1));
            let section = &mut self.site.header.sections[section_idx];
            normalize_section_columns(section);
            if section.columns.len() < 2 {
                self.push_toast(ToastLevel::Warning, "Need at least 2 columns.");
                return;
            }
            let ci = self.selected_header_column.min(section.columns.len() - 1);
            if ci + 1 >= section.columns.len() {
                self.push_toast(ToastLevel::Info, "Column is already last.");
                return;
            }
            section.columns.swap(ci, ci + 1);
            self.selected_header_column = ci + 1;
            self.snap_tree_row_to_header_column(section_idx, ci + 1);
            self.push_toast(ToastLevel::Info, "Moved header column down.");
            return;
        }

        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.push_toast(ToastLevel::Warning, "No selected section.");
            return;
        }
        let ni = selected.min(page.nodes.len() - 1);
        let result = match &mut page.nodes[ni] {
            PageNode::Section(section) => {
                normalize_section_columns(section);
                if section.columns.len() < 2 {
                    (None, "Need at least 2 columns.".to_string())
                } else {
                    let ci = selected_column.min(section.columns.len() - 1);
                    if ci + 1 >= section.columns.len() {
                        (None, "Column is already last.".to_string())
                    } else {
                        section.columns.swap(ci, ci + 1);
                        (Some(ci + 1), "Moved column down.".to_string())
                    }
                }
            }
            _ => (None, "Selected node is not a section.".to_string()),
        };
        if let Some(next_selected_column) = result.0 {
            self.selected_column = next_selected_column;
            self.selected_component = 0;
            self.selected_nested_item = 0;
            self.snap_tree_row_to_column(ni, next_selected_column);
        }
        self.push_toast(ToastLevel::Info, result.1);
    }

    pub(in crate::tui) fn snap_tree_row_to_column(&mut self, node_idx: usize, column_idx: usize) {
        let rows = self.build_tree_rows();
        if let Some(idx) = rows.iter().position(|r| {
            matches!(
                r.kind,
                TreeRowKind::Column { node_idx: n, column_idx: c } if n == node_idx && c == column_idx
            )
        }) {
            self.selected_tree_row = idx;
        }
    }

    pub(in crate::tui) fn snap_tree_row_to_header_column(&mut self, section_idx: usize, column_idx: usize) {
        let rows = self.build_tree_rows();
        if let Some(idx) = rows.iter().position(|r| {
            matches!(
                r.kind,
                TreeRowKind::HeaderColumn { section_idx: s, column_idx: c } if s == section_idx && c == column_idx
            )
        }) {
            self.selected_tree_row = idx;
        }
    }

    pub(in crate::tui) fn selected_section_column_total(&mut self) -> Option<usize> {
        let page = self.current_page();
        if page.nodes.is_empty() {
            return None;
        }
        let ni = self.selected_node.min(page.nodes.len() - 1);
        match &page.nodes[ni] {
            PageNode::Hero(_) => None,
            PageNode::Section(section) => Some(section_columns_ref(section).len()),
        }
    }

    pub(in crate::tui) fn begin_edit_selected_column_id(&mut self) {
        self.open_column_form_edit("id");
    }

    pub(in crate::tui) fn begin_edit_selected_column_width_class(&mut self) {
        self.open_column_form_edit("width_class");
    }

    pub(in crate::tui) fn open_column_form_edit(&mut self, focus_id: &str) {
        if self.warn_site_settings_unavailable() {
            return;
        }
        let rows = self.build_tree_rows();
        if rows.is_empty() {
            self.push_toast(ToastLevel::Warning, "Select a column row to edit.");
            return;
        }
        let row = rows[self.selected_tree_row.min(rows.len() - 1)];
        if !self.try_open_form_edit_drilled_into_column(&row) {
            self.push_toast(ToastLevel::Warning, "Select a column row to edit.");
            return;
        }
        if let Some(Modal::FormEdit {
            state, cursor_pos, ..
        }) = self.modal.as_mut()
        {
            if let Some(idx) = state.form.fields.iter().position(|f| f.id == focus_id) {
                state.focused_field = idx;
                *cursor_pos = state.get(focus_id).len();
            }
        }
    }
}
