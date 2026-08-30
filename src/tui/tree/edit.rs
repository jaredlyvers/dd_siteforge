//! Undo, delete, duplicate, and reorder selected rows.
use super::super::*;

impl App {
    pub(in crate::tui) fn delete_selected_node(&mut self) {
        let selected = self.selected_node;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.push_toast(ToastLevel::Warning, "No node to delete.");
            return;
        }
        let idx = selected.min(page.nodes.len() - 1);
        page.nodes.remove(idx);
        if page.nodes.is_empty() {
            self.selected_node = 0;
            self.selected_column = 0;
            self.selected_component = 0;
            self.selected_nested_item = 0;
        } else {
            self.selected_node = idx.min(page.nodes.len() - 1);
            self.selected_column = 0;
            self.selected_component = 0;
            self.selected_nested_item = 0;
        }
        self.push_toast(ToastLevel::Info, format!("Deleted node {}.", idx + 1));
    }

    pub(in crate::tui) fn push_undo(&mut self) {
        self.undo_stack.push(self.site.clone());
        if self.undo_stack.len() > 20 {
            self.undo_stack.remove(0);
        }
    }

    pub(in crate::tui) fn undo_last(&mut self) {
        let Some(site) = self.undo_stack.pop() else {
            self.push_toast(ToastLevel::Warning, "Nothing to undo.");
            return;
        };
        self.site = site;
        if self.selected_page >= self.site.pages.len() {
            self.selected_page = self.site.pages.len().saturating_sub(1);
        }
        self.sync_tree_row_with_selection();
        self.push_toast(ToastLevel::Success, "Undid last change.");
    }

    pub(in crate::tui) fn request_quit(&mut self) {
        if self.dirty {
            self.modal = Some(Modal::ConfirmPrompt {
                message: "Unsaved changes. Quit anyway? y/n".to_string(),
                on_confirm: ConfirmKind::QuitUnsaved,
            });
        } else {
            self.should_quit = true;
        }
    }

    pub(in crate::tui) fn delete_selected_row(&mut self) {
        let Some(kind) = self.selected_tree_row_kind() else {
            self.push_toast(ToastLevel::Warning, "Nothing selected to delete.");
            return;
        };
        match kind {
            TreeRowKind::PageHead | TreeRowKind::HeaderRoot | TreeRowKind::FooterRoot => {
                self.push_toast(ToastLevel::Warning, "Cannot delete this row.");
            }
            TreeRowKind::Hero { .. } | TreeRowKind::Section { .. } => {
                self.push_undo();
                self.delete_selected_node();
                self.sync_tree_row_with_selection();
            }
            TreeRowKind::Component {
                node_idx,
                column_idx,
                component_idx,
            } => {
                self.push_undo();
                self.delete_page_component(node_idx, column_idx, component_idx);
            }
            TreeRowKind::HeaderComponent {
                section_idx,
                column_idx,
                component_idx,
            } => {
                self.push_undo();
                self.delete_header_component(section_idx, column_idx, component_idx);
            }
            TreeRowKind::FooterComponent {
                section_idx,
                column_idx,
                component_idx,
            } => {
                self.push_undo();
                self.delete_footer_component(section_idx, column_idx, component_idx);
            }
            TreeRowKind::AccordionItem { .. }
            | TreeRowKind::AlternatingItem { .. }
            | TreeRowKind::CardItem { .. }
            | TreeRowKind::FilmstripItem { .. }
            | TreeRowKind::MilestonesItem { .. }
            | TreeRowKind::SliderItem { .. } => {
                self.push_undo();
                self.remove_selected_collection_item();
                self.sync_tree_row_with_selection();
            }
            TreeRowKind::Column { .. }
            | TreeRowKind::HeaderColumn { .. }
            | TreeRowKind::FooterColumn { .. } => {
                self.push_undo();
                self.remove_selected_column();
                self.sync_tree_row_with_selection();
            }
            TreeRowKind::HeaderSection { section_idx } => {
                if self.site.header.sections.len() <= 1 {
                    self.push_toast(ToastLevel::Warning, "Cannot delete last header section.");
                    return;
                }
                self.push_undo();
                if section_idx < self.site.header.sections.len() {
                    self.site.header.sections.remove(section_idx);
                    self.selected_header_section =
                        section_idx.min(self.site.header.sections.len().saturating_sub(1));
                    self.selected_header_column = 0;
                    self.selected_header_component = 0;
                    self.push_toast(ToastLevel::Info, "Deleted header section.");
                    self.sync_tree_row_with_selection();
                }
            }
            TreeRowKind::FooterSection { section_idx } => {
                if self.site.footer.sections.len() <= 1 {
                    self.push_toast(ToastLevel::Warning, "Cannot delete last footer section.");
                    return;
                }
                self.push_undo();
                if section_idx < self.site.footer.sections.len() {
                    self.site.footer.sections.remove(section_idx);
                    self.selected_header_section =
                        section_idx.min(self.site.footer.sections.len().saturating_sub(1));
                    self.selected_header_column = 0;
                    self.selected_header_component = 0;
                    self.push_toast(ToastLevel::Info, "Deleted footer section.");
                    self.sync_tree_row_with_selection();
                }
            }
        }
    }

    pub(in crate::tui) fn delete_page_component(&mut self, node_idx: usize, column_idx: usize, component_idx: usize) {
        let new_selected = {
            let Some(page) = self.current_page_mut() else {
                return;
            };
            let Some(PageNode::Section(section)) = page.nodes.get_mut(node_idx) else {
                self.push_toast(ToastLevel::Warning, "Selected row is not a section.");
                return;
            };
            let Some(col) = section.columns.get_mut(column_idx) else {
                return;
            };
            if component_idx >= col.components.len() {
                return;
            }
            col.components.remove(component_idx);
            component_idx.min(col.components.len().saturating_sub(1))
        };
        self.selected_node = node_idx;
        self.selected_column = column_idx;
        self.selected_component = new_selected;
        self.selected_nested_item = 0;
        self.push_toast(ToastLevel::Info, "Deleted component.");
        self.sync_tree_row_with_selection();
    }

    pub(in crate::tui) fn delete_header_component(
        &mut self,
        section_idx: usize,
        column_idx: usize,
        component_idx: usize,
    ) {
        let Some(section) = self.site.header.sections.get_mut(section_idx) else {
            return;
        };
        let Some(col) = section.columns.get_mut(column_idx) else {
            return;
        };
        if component_idx >= col.components.len() {
            return;
        }
        col.components.remove(component_idx);
        self.selected_header_section = section_idx;
        self.selected_header_column = column_idx;
        self.selected_header_component = component_idx.min(col.components.len().saturating_sub(1));
        self.push_toast(ToastLevel::Info, "Deleted header component.");
        self.sync_tree_row_with_selection();
    }

    pub(in crate::tui) fn delete_footer_component(
        &mut self,
        section_idx: usize,
        column_idx: usize,
        component_idx: usize,
    ) {
        let Some(section) = self.site.footer.sections.get_mut(section_idx) else {
            return;
        };
        let Some(col) = section.columns.get_mut(column_idx) else {
            return;
        };
        if component_idx >= col.components.len() {
            return;
        }
        col.components.remove(component_idx);
        self.selected_header_section = section_idx;
        self.selected_header_column = column_idx;
        self.selected_header_component = component_idx.min(col.components.len().saturating_sub(1));
        self.push_toast(ToastLevel::Info, "Deleted footer component.");
        self.sync_tree_row_with_selection();
    }

    pub(in crate::tui) fn duplicate_selected_row(&mut self) {
        let Some(kind) = self.selected_tree_row_kind() else {
            self.push_toast(ToastLevel::Warning, "Nothing selected to duplicate.");
            return;
        };
        match kind {
            TreeRowKind::Hero { node_idx } | TreeRowKind::Section { node_idx } => {
                self.push_undo();
                let Some(page) = self.current_page_mut() else {
                    self.undo_stack.pop();
                    return;
                };
                if node_idx >= page.nodes.len() {
                    self.undo_stack.pop();
                    return;
                }
                let clone = page.nodes[node_idx].clone();
                page.nodes.insert(node_idx + 1, clone);
                self.selected_node = node_idx + 1;
                self.selected_column = 0;
                self.selected_component = 0;
                self.selected_nested_item = 0;
                self.push_toast(ToastLevel::Success, "Duplicated node.");
                self.sync_tree_row_with_selection();
            }
            TreeRowKind::Component {
                node_idx,
                column_idx,
                component_idx,
            } => {
                self.push_undo();
                let Some(page) = self.current_page_mut() else {
                    self.undo_stack.pop();
                    return;
                };
                let Some(PageNode::Section(section)) = page.nodes.get_mut(node_idx) else {
                    self.undo_stack.pop();
                    return;
                };
                let Some(col) = section.columns.get_mut(column_idx) else {
                    self.undo_stack.pop();
                    return;
                };
                if component_idx >= col.components.len() {
                    self.undo_stack.pop();
                    return;
                }
                let clone = col.components[component_idx].clone();
                col.components.insert(component_idx + 1, clone);
                self.selected_node = node_idx;
                self.selected_column = column_idx;
                self.selected_component = component_idx + 1;
                self.push_toast(ToastLevel::Success, "Duplicated component.");
                self.sync_tree_row_with_selection();
            }
            TreeRowKind::HeaderComponent {
                section_idx,
                column_idx,
                component_idx,
            } => {
                self.push_undo();
                let Some(section) = self.site.header.sections.get_mut(section_idx) else {
                    self.undo_stack.pop();
                    return;
                };
                let Some(col) = section.columns.get_mut(column_idx) else {
                    self.undo_stack.pop();
                    return;
                };
                if component_idx >= col.components.len() {
                    self.undo_stack.pop();
                    return;
                }
                let clone = col.components[component_idx].clone();
                col.components.insert(component_idx + 1, clone);
                self.selected_header_component = component_idx + 1;
                self.push_toast(ToastLevel::Success, "Duplicated header component.");
                self.sync_tree_row_with_selection();
            }
            TreeRowKind::FooterComponent {
                section_idx,
                column_idx,
                component_idx,
            } => {
                self.push_undo();
                let Some(section) = self.site.footer.sections.get_mut(section_idx) else {
                    self.undo_stack.pop();
                    return;
                };
                let Some(col) = section.columns.get_mut(column_idx) else {
                    self.undo_stack.pop();
                    return;
                };
                if component_idx >= col.components.len() {
                    self.undo_stack.pop();
                    return;
                }
                let clone = col.components[component_idx].clone();
                col.components.insert(component_idx + 1, clone);
                self.selected_header_component = component_idx + 1;
                self.push_toast(ToastLevel::Success, "Duplicated footer component.");
                self.sync_tree_row_with_selection();
            }
            TreeRowKind::AccordionItem { .. }
            | TreeRowKind::AlternatingItem { .. }
            | TreeRowKind::CardItem { .. }
            | TreeRowKind::FilmstripItem { .. }
            | TreeRowKind::MilestonesItem { .. }
            | TreeRowKind::SliderItem { .. } => {
                self.push_undo();
                if self.duplicate_selected_collection_item() {
                    self.push_toast(ToastLevel::Success, "Duplicated item.");
                    self.sync_tree_row_with_selection();
                } else {
                    self.undo_stack.pop();
                    self.push_toast(ToastLevel::Warning, "Could not duplicate item.");
                }
            }
            _ => {
                self.push_toast(ToastLevel::Warning, "Cannot duplicate this row.");
            }
        }
    }

    pub(in crate::tui) fn duplicate_selected_collection_item(&mut self) -> bool {
        let ni = self.selected_node;
        let col_i = self.selected_column;
        let selected_component = self.selected_component;
        let item_idx = self.selected_nested_item;
        let Some(page) = self.current_page_mut() else {
            return false;
        };
        let ni = ni.min(page.nodes.len().saturating_sub(1));
        let PageNode::Section(section) = &mut page.nodes[ni] else {
            return false;
        };
        let col_i = col_i.min(section.columns.len().saturating_sub(1));
        let ci = match component_index(
            section.columns[col_i].components.len(),
            selected_component,
        ) {
            Some(v) => v,
            None => return false,
        };
        let components = &mut section.columns[col_i].components[ci];
        let inserted = match components {
            crate::model::SectionComponent::Accordion(a) if item_idx < a.items.len() => {
                let clone = a.items[item_idx].clone();
                a.items.insert(item_idx + 1, clone);
                true
            }
            crate::model::SectionComponent::Alternating(a) if item_idx < a.items.len() => {
                let clone = a.items[item_idx].clone();
                a.items.insert(item_idx + 1, clone);
                true
            }
            crate::model::SectionComponent::Card(a) if item_idx < a.items.len() => {
                let clone = a.items[item_idx].clone();
                a.items.insert(item_idx + 1, clone);
                true
            }
            crate::model::SectionComponent::Filmstrip(a) if item_idx < a.items.len() => {
                let clone = a.items[item_idx].clone();
                a.items.insert(item_idx + 1, clone);
                true
            }
            crate::model::SectionComponent::Milestones(a) if item_idx < a.items.len() => {
                let clone = a.items[item_idx].clone();
                a.items.insert(item_idx + 1, clone);
                true
            }
            crate::model::SectionComponent::Slider(a) if item_idx < a.items.len() => {
                let clone = a.items[item_idx].clone();
                a.items.insert(item_idx + 1, clone);
                true
            }
            _ => false,
        };
        if inserted {
            self.selected_nested_item = item_idx + 1;
        }
        inserted
    }

    pub(in crate::tui) fn move_selected_row(&mut self, delta: isize) {
        let Some(kind) = self.selected_tree_row_kind() else {
            return;
        };
        match kind {
            TreeRowKind::Hero { node_idx } | TreeRowKind::Section { node_idx } => {
                let dest = node_idx as isize + delta;
                let len = self.current_page().nodes.len();
                if len < 2 || node_idx >= len || dest < 0 || dest as usize >= len {
                    return;
                }
                self.push_undo();
                let page = self.current_page_mut().unwrap();
                page.nodes.swap(node_idx, dest as usize);
                self.selected_node = dest as usize;
                self.push_toast(ToastLevel::Info, "Moved node.");
                self.sync_tree_row_with_selection();
            }
            TreeRowKind::Component {
                node_idx,
                column_idx,
                component_idx,
            } => {
                let dest = component_idx as isize + delta;
                let can_move = match self.current_page().nodes.get(node_idx) {
                    Some(PageNode::Section(section)) => section
                        .columns
                        .get(column_idx)
                        .map(|col| dest >= 0 && (dest as usize) < col.components.len())
                        .unwrap_or(false),
                    _ => false,
                };
                if !can_move {
                    return;
                }
                self.push_undo();
                let page = self.current_page_mut().unwrap();
                let PageNode::Section(section) = &mut page.nodes[node_idx] else {
                    self.undo_stack.pop();
                    return;
                };
                section.columns[column_idx]
                    .components
                    .swap(component_idx, dest as usize);
                self.selected_component = dest as usize;
                self.push_toast(ToastLevel::Info, "Moved component.");
                self.sync_tree_row_with_selection();
            }
            TreeRowKind::HeaderComponent {
                section_idx,
                column_idx,
                component_idx,
            } => self.move_region_component(false, section_idx, column_idx, component_idx, delta),
            TreeRowKind::FooterComponent {
                section_idx,
                column_idx,
                component_idx,
            } => self.move_region_component(true, section_idx, column_idx, component_idx, delta),
            TreeRowKind::Column { .. }
            | TreeRowKind::HeaderColumn { .. }
            | TreeRowKind::FooterColumn { .. } => {
                self.push_undo();
                if delta > 0 {
                    self.move_selected_column_down();
                } else {
                    self.move_selected_column_up();
                }
            }
            TreeRowKind::AccordionItem { .. }
            | TreeRowKind::AlternatingItem { .. }
            | TreeRowKind::CardItem { .. }
            | TreeRowKind::FilmstripItem { .. }
            | TreeRowKind::MilestonesItem { .. }
            | TreeRowKind::SliderItem { .. } => {
                self.push_undo();
                if self.move_selected_collection_item(delta) {
                    self.push_toast(ToastLevel::Info, "Moved item.");
                    self.sync_tree_row_with_selection();
                } else {
                    self.undo_stack.pop();
                }
            }
            _ => {}
        }
    }

    pub(in crate::tui) fn move_region_component(
        &mut self,
        footer: bool,
        section_idx: usize,
        column_idx: usize,
        component_idx: usize,
        delta: isize,
    ) {
        let dest = component_idx as isize + delta;
        let len = if footer {
            self.site
                .footer
                .sections
                .get(section_idx)
                .and_then(|s| s.columns.get(column_idx))
                .map(|c| c.components.len())
        } else {
            self.site
                .header
                .sections
                .get(section_idx)
                .and_then(|s| s.columns.get(column_idx))
                .map(|c| c.components.len())
        };
        let Some(len) = len else {
            return;
        };
        if dest < 0 || dest as usize >= len {
            return;
        }
        self.push_undo();
        let sections = if footer {
            &mut self.site.footer.sections
        } else {
            &mut self.site.header.sections
        };
        let col = &mut sections[section_idx].columns[column_idx];
        col.components.swap(component_idx, dest as usize);
        self.selected_header_component = dest as usize;
        self.push_toast(
            ToastLevel::Info,
            if footer {
                "Moved footer component."
            } else {
                "Moved header component."
            },
        );
        self.sync_tree_row_with_selection();
    }

    pub(in crate::tui) fn move_selected_collection_item(&mut self, delta: isize) -> bool {
        let ni = self.selected_node;
        let col_i = self.selected_column;
        let selected_component = self.selected_component;
        let item_idx = self.selected_nested_item;
        let Some(page) = self.current_page_mut() else {
            return false;
        };
        let ni = ni.min(page.nodes.len().saturating_sub(1));
        let PageNode::Section(section) = &mut page.nodes[ni] else {
            return false;
        };
        let col_i = col_i.min(section.columns.len().saturating_sub(1));
        let ci = match component_index(
            section.columns[col_i].components.len(),
            selected_component,
        ) {
            Some(v) => v,
            None => return false,
        };
        let dest = item_idx as isize + delta;
        if dest < 0 {
            return false;
        }
        let dest = dest as usize;
        let swapped = match &mut section.columns[col_i].components[ci] {
            crate::model::SectionComponent::Accordion(a) if dest < a.items.len() => {
                a.items.swap(item_idx, dest);
                true
            }
            crate::model::SectionComponent::Alternating(a) if dest < a.items.len() => {
                a.items.swap(item_idx, dest);
                true
            }
            crate::model::SectionComponent::Card(a) if dest < a.items.len() => {
                a.items.swap(item_idx, dest);
                true
            }
            crate::model::SectionComponent::Filmstrip(a) if dest < a.items.len() => {
                a.items.swap(item_idx, dest);
                true
            }
            crate::model::SectionComponent::Milestones(a) if dest < a.items.len() => {
                a.items.swap(item_idx, dest);
                true
            }
            crate::model::SectionComponent::Slider(a) if dest < a.items.len() => {
                a.items.swap(item_idx, dest);
                true
            }
            _ => false,
        };
        if swapped {
            self.selected_nested_item = dest;
        }
        swapped
    }
}
