//! Tree cursor motion, page switch, and selection summaries.
use super::super::*;

impl App {
    pub(in crate::tui) fn selection_summary(&self) -> String {
        let page = self.current_page();
        if page.nodes.is_empty() {
            return "(none)".to_string();
        }
        let ni = self.selected_node.min(page.nodes.len().saturating_sub(1));
        match &page.nodes[ni] {
            PageNode::Hero(_) => format!("node {} (dd-hero)", ni + 1),
            PageNode::Section(section) => format!(
                "node {} (dd-section:{}), column {}, component {}",
                ni + 1,
                section.id,
                self.selected_column + 1,
                self.selected_component + 1
            ),
        }
    }

    pub(in crate::tui) fn header_selection_summary(&self) -> String {
        if self.site.header.sections.is_empty() {
            return "dd-header (no sections - press '/' to add dd-section)".to_string();
        }
        let section_i = self
            .selected_header_section
            .min(self.site.header.sections.len().saturating_sub(1));
        format!(
            "dd-header:{}, section:{}, column {}, component {}",
            self.site.header.id,
            self.site.header.sections[section_i].id,
            self.selected_header_column + 1,
            self.selected_header_component + 1
        )
    }

    pub(in crate::tui) fn selected_component_owned(&self) -> Option<crate::model::SectionComponent> {
        let page = self.current_page();
        if page.nodes.is_empty() {
            return None;
        }
        let ni = self.selected_node.min(page.nodes.len().saturating_sub(1));
        let PageNode::Section(section) = &page.nodes[ni] else {
            return None;
        };
        let columns = section_columns_ref(section);
        let col_i = self.selected_column.min(columns.len().saturating_sub(1));
        let ci = component_index(columns[col_i].components.len(), self.selected_component)?;
        columns[col_i].components.get(ci).cloned()
    }

    pub(in crate::tui) fn select_prev(&mut self) {
        let rows = self.build_tree_rows();
        if rows.is_empty() {
            return;
        }
        let next = self.selected_tree_row.saturating_sub(1);
        if next != self.selected_tree_row {
            self.selected_tree_row = next;
            self.apply_tree_row_selection(rows[next]);
        }
    }

    pub(in crate::tui) fn select_next(&mut self) {
        let rows = self.build_tree_rows();
        let total = rows.len();
        if total == 0 {
            return;
        }
        let next = (self.selected_tree_row + 1).min(total - 1);
        if next != self.selected_tree_row {
            self.selected_tree_row = next;
            self.apply_tree_row_selection(rows[next]);
        }
    }

    pub(in crate::tui) fn cycle_selected_region(&mut self, delta: isize) {
        let idx = match self.selected_region {
            SelectedRegion::Site => 0,
            SelectedRegion::Header => 1,
            SelectedRegion::Footer => 2,
            // Page is not a Regions row; j starts at Site, k at Footer.
            SelectedRegion::Page => {
                if delta > 0 {
                    -1
                } else {
                    3
                }
            }
        };
        let next = (idx + delta).rem_euclid(3) as usize;
        self.selected_region = match next {
            0 => SelectedRegion::Site,
            1 => SelectedRegion::Header,
            _ => SelectedRegion::Footer,
        };
        self.selected_tree_row = 0;
    }

    pub(in crate::tui) fn warn_site_settings_unavailable(&mut self) -> bool {
        if self.selected_region == SelectedRegion::Site {
            self.push_toast(
                ToastLevel::Warning,
                "Not available on Site settings.",
            );
            true
        } else {
            false
        }
    }

    pub(in crate::tui) fn handle_up(&mut self) {
        match self.selected_sidebar_section {
            SidebarSection::Regions => {
                self.cycle_selected_region(-1);
            }
            SidebarSection::Pages => {
                if self.site.pages.is_empty() {
                    return;
                }
                if self.selected_page == 0 {
                    self.selected_page = self.site.pages.len() - 1;
                } else {
                    self.selected_page -= 1;
                }
                self.selected_node = 0;
                self.selected_tree_row = 0;
                self.selected_column = 0;
                self.selected_component = 0;
                self.selected_nested_item = 0;
                self.details_scroll_row = 0;
                self.sync_tree_row_with_selection();
            }
            SidebarSection::Layouts => {
                self.select_prev();
            }
            SidebarSection::Details => {
                self.scroll_details_by(-1);
            }
        }
    }

    pub(in crate::tui) fn handle_down(&mut self) {
        match self.selected_sidebar_section {
            SidebarSection::Regions => {
                self.cycle_selected_region(1);
            }
            SidebarSection::Pages => {
                if self.site.pages.is_empty() {
                    return;
                }
                self.selected_page = (self.selected_page + 1) % self.site.pages.len();
                self.selected_node = 0;
                self.selected_tree_row = 0;
                self.selected_column = 0;
                self.selected_component = 0;
                self.selected_nested_item = 0;
                self.details_scroll_row = 0;
                self.sync_tree_row_with_selection();
            }
            SidebarSection::Layouts => {
                self.select_next();
            }
            SidebarSection::Details => {
                self.scroll_details_by(1);
            }
        }
    }

    pub(in crate::tui) fn vim_jump_to_first_row(&mut self) {
        let rows = self.build_tree_rows();
        if rows.is_empty() {
            return;
        }
        self.selected_tree_row = 0;
        self.apply_tree_row_selection(rows[0]);
        self.details_scroll_row = 0;
    }

    pub(in crate::tui) fn vim_jump_to_last_row(&mut self) {
        let rows = self.build_tree_rows();
        if rows.is_empty() {
            return;
        }
        let last = rows.len() - 1;
        self.selected_tree_row = last;
        self.apply_tree_row_selection(rows[last]);
        self.details_scroll_row = 0;
    }

    pub(in crate::tui) fn select_next_page(&mut self) {
        if self.site.pages.is_empty() {
            return;
        }
        self.selected_page = (self.selected_page + 1) % self.site.pages.len();
        self.selected_node = 0;
        self.selected_tree_row = 0;
        self.selected_column = 0;
        self.selected_component = 0;
        self.selected_nested_item = 0;
        self.details_scroll_row = 0;
        self.sync_tree_row_with_selection();
    }

    pub(in crate::tui) fn select_prev_page(&mut self) {
        if self.site.pages.is_empty() {
            return;
        }
        if self.selected_page == 0 {
            self.selected_page = self.site.pages.len() - 1;
        } else {
            self.selected_page -= 1;
        }
        self.selected_node = 0;
        self.selected_tree_row = 0;
        self.selected_column = 0;
        self.selected_component = 0;
        self.selected_nested_item = 0;
        self.details_scroll_row = 0;
        self.sync_tree_row_with_selection();
    }

    pub(in crate::tui) fn selected_tree_row_kind(&self) -> Option<TreeRowKind> {
        let rows = self.build_tree_rows();
        if rows.is_empty() {
            return None;
        }
        Some(rows[self.selected_tree_row.min(rows.len() - 1)].kind)
    }
}
