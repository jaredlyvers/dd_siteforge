//! Section/item expansion flags and Space/h/l toggles.
use super::super::*;

impl App {
    pub(in crate::tui) fn is_header_section_expanded(&self, section_idx: usize) -> bool {
        self.expanded_sections.contains(&(usize::MAX, section_idx))
    }

    pub(in crate::tui) fn set_header_section_expanded(&mut self, section_idx: usize, expanded: bool) {
        let key = (usize::MAX, section_idx);
        if expanded {
            self.expanded_sections.insert(key);
        } else {
            self.expanded_sections.remove(&key);
        }
    }

    pub(in crate::tui) fn is_section_expanded(&self, node_idx: usize) -> bool {
        !self
            .expanded_sections
            .contains(&(self.selected_page, node_idx))
    }

    pub(in crate::tui) fn set_section_expanded(&mut self, node_idx: usize, expanded: bool) {
        if expanded {
            self.expanded_sections
                .remove(&(self.selected_page, node_idx));
        } else {
            self.expanded_sections
                .insert((self.selected_page, node_idx));
        }
    }

    pub(in crate::tui) fn is_accordion_items_expanded(
        &self,
        node_idx: usize,
        column_idx: usize,
        component_idx: usize,
    ) -> bool {
        !self.expanded_accordion_items.contains(&(
            self.selected_page,
            node_idx,
            column_idx,
            component_idx,
        ))
    }

    pub(in crate::tui) fn set_accordion_items_expanded(
        &mut self,
        node_idx: usize,
        column_idx: usize,
        component_idx: usize,
        expanded: bool,
    ) {
        let key = (self.selected_page, node_idx, column_idx, component_idx);
        if expanded {
            self.expanded_accordion_items.remove(&key);
        } else {
            self.expanded_accordion_items.insert(key);
        }
    }

    pub(in crate::tui) fn is_alternating_items_expanded(
        &self,
        node_idx: usize,
        column_idx: usize,
        component_idx: usize,
    ) -> bool {
        !self.expanded_alternating_items.contains(&(
            self.selected_page,
            node_idx,
            column_idx,
            component_idx,
        ))
    }

    pub(in crate::tui) fn set_alternating_items_expanded(
        &mut self,
        node_idx: usize,
        column_idx: usize,
        component_idx: usize,
        expanded: bool,
    ) {
        let key = (self.selected_page, node_idx, column_idx, component_idx);
        if expanded {
            self.expanded_alternating_items.remove(&key);
        } else {
            self.expanded_alternating_items.insert(key);
        }
    }

    pub(in crate::tui) fn is_card_items_expanded(
        &self,
        node_idx: usize,
        column_idx: usize,
        component_idx: usize,
    ) -> bool {
        !self.expanded_card_items.contains(&(
            self.selected_page,
            node_idx,
            column_idx,
            component_idx,
        ))
    }

    pub(in crate::tui) fn set_card_items_expanded(
        &mut self,
        node_idx: usize,
        column_idx: usize,
        component_idx: usize,
        expanded: bool,
    ) {
        let key = (self.selected_page, node_idx, column_idx, component_idx);
        if expanded {
            self.expanded_card_items.remove(&key);
        } else {
            self.expanded_card_items.insert(key);
        }
    }

    pub(in crate::tui) fn is_filmstrip_items_expanded(
        &self,
        node_idx: usize,
        column_idx: usize,
        component_idx: usize,
    ) -> bool {
        !self.expanded_filmstrip_items.contains(&(
            self.selected_page,
            node_idx,
            column_idx,
            component_idx,
        ))
    }

    pub(in crate::tui) fn set_filmstrip_items_expanded(
        &mut self,
        node_idx: usize,
        column_idx: usize,
        component_idx: usize,
        expanded: bool,
    ) {
        let key = (self.selected_page, node_idx, column_idx, component_idx);
        if expanded {
            self.expanded_filmstrip_items.remove(&key);
        } else {
            self.expanded_filmstrip_items.insert(key);
        }
    }

    pub(in crate::tui) fn is_milestones_items_expanded(
        &self,
        node_idx: usize,
        column_idx: usize,
        component_idx: usize,
    ) -> bool {
        !self.expanded_milestones_items.contains(&(
            self.selected_page,
            node_idx,
            column_idx,
            component_idx,
        ))
    }

    pub(in crate::tui) fn set_milestones_items_expanded(
        &mut self,
        node_idx: usize,
        column_idx: usize,
        component_idx: usize,
        expanded: bool,
    ) {
        let key = (self.selected_page, node_idx, column_idx, component_idx);
        if expanded {
            self.expanded_milestones_items.remove(&key);
        } else {
            self.expanded_milestones_items.insert(key);
        }
    }

    pub(in crate::tui) fn is_slider_items_expanded(
        &self,
        node_idx: usize,
        column_idx: usize,
        component_idx: usize,
    ) -> bool {
        !self.expanded_slider_items.contains(&(
            self.selected_page,
            node_idx,
            column_idx,
            component_idx,
        ))
    }

    pub(in crate::tui) fn set_slider_items_expanded(
        &mut self,
        node_idx: usize,
        column_idx: usize,
        component_idx: usize,
        expanded: bool,
    ) {
        let key = (self.selected_page, node_idx, column_idx, component_idx);
        if expanded {
            self.expanded_slider_items.remove(&key);
        } else {
            self.expanded_slider_items.insert(key);
        }
    }

    pub(in crate::tui) fn toggle_selected_tree_expanded(&mut self) {
        let rows = self.build_tree_rows();
        if rows.is_empty() {
            return;
        }
        let row = rows[self.selected_tree_row.min(rows.len() - 1)];
        if let TreeRowKind::Component {
            node_idx,
            column_idx,
            component_idx,
        }
        | TreeRowKind::AccordionItem {
            node_idx,
            column_idx,
            component_idx,
            ..
        }
        | TreeRowKind::AlternatingItem {
            node_idx,
            column_idx,
            component_idx,
            ..
        }
        | TreeRowKind::CardItem {
            node_idx,
            column_idx,
            component_idx,
            ..
        }
        | TreeRowKind::FilmstripItem {
            node_idx,
            column_idx,
            component_idx,
            ..
        }
        | TreeRowKind::MilestonesItem {
            node_idx,
            column_idx,
            component_idx,
            ..
        }
        | TreeRowKind::SliderItem {
            node_idx,
            column_idx,
            component_idx,
            ..
        } = row.kind
        {
            let page = self.current_page();
            let Some(PageNode::Section(section)) = page.nodes.get(node_idx) else {
                self.push_toast(ToastLevel::Warning, "Selected row is not a section.");
                return;
            };
            let columns = section_columns_ref(section);
            let col_i = column_idx.min(columns.len().saturating_sub(1));
            let comp_i = component_idx.min(columns[col_i].components.len().saturating_sub(1));
            if matches!(
                columns[col_i].components.get(comp_i),
                Some(crate::model::SectionComponent::Accordion(_))
            ) {
                let expanded = self.is_accordion_items_expanded(node_idx, col_i, comp_i);
                self.set_accordion_items_expanded(node_idx, col_i, comp_i, !expanded);
                self.selected_node = node_idx;
                self.selected_column = col_i;
                self.selected_component = comp_i;
                self.selected_nested_item = 0;
                let msg = if expanded {
                    "Collapsed accordion items.".to_string()
                } else {
                    "Expanded accordion items.".to_string()
                };
                self.push_toast(ToastLevel::Info, msg);
                self.sync_tree_row_with_selection();
                return;
            }
            if matches!(
                columns[col_i].components.get(comp_i),
                Some(crate::model::SectionComponent::Alternating(_))
            ) {
                let expanded = self.is_alternating_items_expanded(node_idx, col_i, comp_i);
                self.set_alternating_items_expanded(node_idx, col_i, comp_i, !expanded);
                self.selected_node = node_idx;
                self.selected_column = col_i;
                self.selected_component = comp_i;
                self.selected_nested_item = 0;
                let msg = if expanded {
                    "Collapsed alternating items.".to_string()
                } else {
                    "Expanded alternating items.".to_string()
                };
                self.push_toast(ToastLevel::Info, msg);
                self.sync_tree_row_with_selection();
                return;
            }
            if matches!(
                columns[col_i].components.get(comp_i),
                Some(crate::model::SectionComponent::Card(_))
            ) {
                let expanded = self.is_card_items_expanded(node_idx, col_i, comp_i);
                self.set_card_items_expanded(node_idx, col_i, comp_i, !expanded);
                self.selected_node = node_idx;
                self.selected_column = col_i;
                self.selected_component = comp_i;
                self.selected_nested_item = 0;
                let msg = if expanded {
                    "Collapsed card items.".to_string()
                } else {
                    "Expanded card items.".to_string()
                };
                self.push_toast(ToastLevel::Info, msg);
                self.sync_tree_row_with_selection();
                return;
            }
            if matches!(
                columns[col_i].components.get(comp_i),
                Some(crate::model::SectionComponent::Filmstrip(_))
            ) {
                let expanded = self.is_filmstrip_items_expanded(node_idx, col_i, comp_i);
                self.set_filmstrip_items_expanded(node_idx, col_i, comp_i, !expanded);
                self.selected_node = node_idx;
                self.selected_column = col_i;
                self.selected_component = comp_i;
                self.selected_nested_item = 0;
                let msg = if expanded {
                    "Collapsed filmstrip items.".to_string()
                } else {
                    "Expanded filmstrip items.".to_string()
                };
                self.push_toast(ToastLevel::Info, msg);
                self.sync_tree_row_with_selection();
                return;
            }
            if matches!(
                columns[col_i].components.get(comp_i),
                Some(crate::model::SectionComponent::Milestones(_))
            ) {
                let expanded = self.is_milestones_items_expanded(node_idx, col_i, comp_i);
                self.set_milestones_items_expanded(node_idx, col_i, comp_i, !expanded);
                self.selected_node = node_idx;
                self.selected_column = col_i;
                self.selected_component = comp_i;
                self.selected_nested_item = 0;
                let msg = if expanded {
                    "Collapsed milestones items.".to_string()
                } else {
                    "Expanded milestones items.".to_string()
                };
                self.push_toast(ToastLevel::Info, msg);
                self.sync_tree_row_with_selection();
                return;
            }
            if matches!(
                columns[col_i].components.get(comp_i),
                Some(crate::model::SectionComponent::Slider(_))
            ) {
                let expanded = self.is_slider_items_expanded(node_idx, col_i, comp_i);
                self.set_slider_items_expanded(node_idx, col_i, comp_i, !expanded);
                self.selected_node = node_idx;
                self.selected_column = col_i;
                self.selected_component = comp_i;
                self.selected_nested_item = 0;
                let msg = if expanded {
                    "Collapsed slider items.".to_string()
                } else {
                    "Expanded slider items.".to_string()
                };
                self.push_toast(ToastLevel::Info, msg);
                self.sync_tree_row_with_selection();
                return;
            }
        }
        let node_idx = match row.kind {
            TreeRowKind::HeaderRoot { .. } => {
                self.header_column_expanded = !self.header_column_expanded;
                let msg = if self.header_column_expanded {
                    "Expanded header columns.".to_string()
                } else {
                    "Collapsed header columns.".to_string()
                };
                self.push_toast(ToastLevel::Info, msg);
                self.sync_tree_row_with_selection();
                return;
            }
            TreeRowKind::HeaderSection { section_idx } => {
                let expanded = self.is_header_section_expanded(section_idx);
                self.set_header_section_expanded(section_idx, !expanded);
                self.selected_header_section = section_idx;
                self.selected_header_column = 0;
                self.selected_header_component = 0;
                let msg = if expanded {
                    "Collapsed header section.".to_string()
                } else {
                    "Expanded header section.".to_string()
                };
                self.push_toast(ToastLevel::Info, msg);
                self.sync_tree_row_with_selection();
                return;
            }
            TreeRowKind::HeaderColumn { .. } | TreeRowKind::HeaderComponent { .. } => {
                self.push_toast(ToastLevel::Info, "Press Enter to edit.");
                return;
            }
            TreeRowKind::FooterRoot
            | TreeRowKind::FooterSection { .. }
            | TreeRowKind::FooterColumn { .. }
            | TreeRowKind::FooterComponent { .. } => {
                self.push_toast(ToastLevel::Info, "Press Enter to edit.");
                return;
            }
            TreeRowKind::SiteRoot => {
                self.push_toast(ToastLevel::Info, "Press Enter to edit site settings.");
                return;
            }
            TreeRowKind::PageHead => {
                self.push_toast(ToastLevel::Info, "Press Enter to edit page head.");
                return;
            }
            TreeRowKind::Section { node_idx } => node_idx,
            TreeRowKind::Column { node_idx, .. } => node_idx,
            TreeRowKind::Component { node_idx, .. } => node_idx,
            TreeRowKind::AccordionItem { node_idx, .. } => node_idx,
            TreeRowKind::AlternatingItem { node_idx, .. } => node_idx,
            TreeRowKind::CardItem { node_idx, .. } => node_idx,
            TreeRowKind::FilmstripItem { node_idx, .. } => node_idx,
            TreeRowKind::MilestonesItem { node_idx, .. } => node_idx,
            TreeRowKind::SliderItem { node_idx, .. } => node_idx,
            TreeRowKind::Hero { .. } => {
                self.push_toast(ToastLevel::Warning, "Selected row is not a section.");
                return;
            }
        };
        let page = self.current_page();
        let Some(PageNode::Section(_)) = page.nodes.get(node_idx) else {
            self.push_toast(ToastLevel::Warning, "Selected row is not a section.");
            return;
        };
        let expanded = self.is_section_expanded(node_idx);
        self.set_section_expanded(node_idx, !expanded);
        self.selected_node = node_idx;
        self.selected_column = 0;
        self.selected_component = 0;
        self.selected_nested_item = 0;
        let msg = if expanded {
            "Collapsed section.".to_string()
        } else {
            "Expanded section.".to_string()
        };
        self.push_toast(ToastLevel::Info, msg);
        self.sync_tree_row_with_selection();
    }

    pub(in crate::tui) fn vim_collapse_selected_row(&mut self) {
        let rows = self.build_tree_rows();
        if rows.is_empty() {
            return;
        }
        let row = rows[self.selected_tree_row.min(rows.len() - 1)];
        if self.tree_row_is_expanded(&row) {
            self.toggle_selected_tree_expanded();
        }
    }

    pub(in crate::tui) fn vim_expand_selected_row(&mut self) {
        let rows = self.build_tree_rows();
        if rows.is_empty() {
            return;
        }
        let row = rows[self.selected_tree_row.min(rows.len() - 1)];
        if !self.tree_row_is_expanded(&row) {
            self.toggle_selected_tree_expanded();
        }
    }

    pub(in crate::tui) fn tree_row_is_expanded(&self, row: &TreeRow) -> bool {
        match row.kind {
            TreeRowKind::Section { node_idx } => self.is_section_expanded(node_idx),
            TreeRowKind::HeaderSection { section_idx } => {
                self.is_header_section_expanded(section_idx)
            }
            _ => false,
        }
    }
}
