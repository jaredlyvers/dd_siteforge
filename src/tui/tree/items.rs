//! Add/remove nested collection items on the selected component.
use super::super::*;

impl App {
    pub(in crate::tui) fn add_selected_collection_item(&mut self) {
        if self.warn_site_settings_unavailable() {
            return;
        }
        let component = self.selected_component_owned();
        match component {
            Some(crate::model::SectionComponent::Accordion(_)) => {
                self.add_selected_accordion_item()
            }
            Some(crate::model::SectionComponent::Alternating(_)) => {
                self.add_selected_alternating_item()
            }
            Some(crate::model::SectionComponent::Card(_)) => self.add_selected_card_item(),
            Some(crate::model::SectionComponent::Filmstrip(_)) => {
                self.add_selected_filmstrip_item()
            }
            Some(crate::model::SectionComponent::Milestones(_)) => {
                self.add_selected_milestones_item()
            }
            Some(crate::model::SectionComponent::Slider(_)) => self.add_selected_slider_item(),
            Some(_) => {
                self.push_toast(ToastLevel::Warning, "Selected component does not support collection items.");
            }
            None => {
                self.push_toast(ToastLevel::Warning, "No selected collection component.");
            }
        }
    }

    pub(in crate::tui) fn remove_selected_collection_item(&mut self) {
        if self.warn_site_settings_unavailable() {
            return;
        }
        let component = self.selected_component_owned();
        match component {
            Some(crate::model::SectionComponent::Accordion(_)) => {
                self.remove_selected_accordion_item()
            }
            Some(crate::model::SectionComponent::Alternating(_)) => {
                self.remove_selected_alternating_item()
            }
            Some(crate::model::SectionComponent::Card(_)) => self.remove_selected_card_item(),
            Some(crate::model::SectionComponent::Filmstrip(_)) => {
                self.remove_selected_filmstrip_item()
            }
            Some(crate::model::SectionComponent::Milestones(_)) => {
                self.remove_selected_milestones_item()
            }
            Some(crate::model::SectionComponent::Slider(_)) => self.remove_selected_slider_item(),
            Some(_) => {
                self.push_toast(ToastLevel::Warning, "Selected component does not support collection items.");
            }
            None => {
                self.push_toast(ToastLevel::Warning, "No selected collection component.");
            }
        }
    }

    pub(in crate::tui) fn add_selected_accordion_item(&mut self) {
        let rows = self.build_page_tree_rows();
        if rows.is_empty() {
            self.push_toast(ToastLevel::Warning, "No selected section.");
            return;
        }
        let row = rows[self.selected_tree_row.min(rows.len() - 1)];
        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let preferred_insert_after = match row.kind {
            TreeRowKind::AccordionItem { item_idx, .. } => Some(item_idx),
            _ => None,
        };
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
                let col_i = selected_column.min(section.columns.len().saturating_sub(1));
                let components = &mut section.columns[col_i].components;
                if let Some(ci) = component_index(components.len(), selected_component) {
                    if let crate::model::SectionComponent::Accordion(acc) = &mut components[ci] {
                        let insert_idx = preferred_insert_after
                            .map(|i| (i + 1).min(acc.items.len()))
                            .unwrap_or(acc.items.len());
                        let next_num = acc.items.len() + 1;
                        acc.items.insert(
                            insert_idx,
                            crate::model::AccordionItem {
                                child_title: format!("Accordion Item {}", next_num),
                                child_copy: "Accordion content".to_string(),
                            },
                        );
                        (
                            Some((ni, col_i, ci, insert_idx)),
                            format!("Added accordion item {}.", insert_idx + 1),
                        )
                    } else {
                        (None, "Selected component is not dd-accordion.".to_string())
                    }
                } else {
                    (None, "Section has no components.".to_string())
                }
            }
            _ => (None, "Selected node is not a section.".to_string()),
        };
        if let Some((node_idx, column_idx, component_idx, item_idx)) = result.0 {
            self.selected_node = node_idx;
            self.selected_column = column_idx;
            self.selected_component = component_idx;
            self.selected_nested_item = item_idx;
            self.set_accordion_items_expanded(node_idx, column_idx, component_idx, true);
        }
        self.push_toast(ToastLevel::Info, result.1);
    }

    pub(in crate::tui) fn remove_selected_accordion_item(&mut self) {
        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let selected_nested_item = self.selected_nested_item;
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
                let col_i = selected_column.min(section.columns.len().saturating_sub(1));
                let components = &mut section.columns[col_i].components;
                if let Some(ci) = component_index(components.len(), selected_component) {
                    if let crate::model::SectionComponent::Accordion(acc) = &mut components[ci] {
                        if acc.items.len() <= 1 {
                            (
                                None,
                                "dd-accordion must keep at least one item.".to_string(),
                            )
                        } else {
                            let remove_idx = selected_nested_item.min(acc.items.len() - 1);
                            acc.items.remove(remove_idx);
                            let next_item_idx = remove_idx.min(acc.items.len() - 1);
                            (
                                Some((ni, col_i, ci, next_item_idx)),
                                format!("Removed accordion item {}.", remove_idx + 1),
                            )
                        }
                    } else {
                        (None, "Selected component is not dd-accordion.".to_string())
                    }
                } else {
                    (None, "Section has no components.".to_string())
                }
            }
            _ => (None, "Selected node is not a section.".to_string()),
        };
        if let Some((node_idx, column_idx, component_idx, item_idx)) = result.0 {
            self.selected_node = node_idx;
            self.selected_column = column_idx;
            self.selected_component = component_idx;
            self.selected_nested_item = item_idx;
            self.set_accordion_items_expanded(node_idx, column_idx, component_idx, true);
        }
        self.push_toast(ToastLevel::Info, result.1);
    }

    pub(in crate::tui) fn add_selected_alternating_item(&mut self) {
        let rows = self.build_page_tree_rows();
        if rows.is_empty() {
            self.push_toast(ToastLevel::Warning, "No selected section.");
            return;
        }
        let row = rows[self.selected_tree_row.min(rows.len() - 1)];
        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let preferred_insert_after = match row.kind {
            TreeRowKind::AlternatingItem { item_idx, .. } => Some(item_idx),
            _ => None,
        };
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
                let col_i = selected_column.min(section.columns.len().saturating_sub(1));
                let components = &mut section.columns[col_i].components;
                if let Some(ci) = component_index(components.len(), selected_component) {
                    if let crate::model::SectionComponent::Alternating(alt) = &mut components[ci] {
                        let insert_idx = preferred_insert_after
                            .map(|i| (i + 1).min(alt.items.len()))
                            .unwrap_or(alt.items.len());
                        let next_num = alt.items.len() + 1;
                        alt.items.insert(
                            insert_idx,
                            crate::model::AlternatingItem {
                                child_image_url: "https://dummyimage.com/600x400/000/fff".to_string(),
                                child_image_alt: format!("Alternating image {}", next_num),
                                child_title: format!("Alternating Item {}", next_num),
                                child_copy: "Alternating content".to_string(),
                            },
                        );
                        (
                            Some((ni, col_i, ci, insert_idx)),
                            format!("Added alternating item {}.", insert_idx + 1),
                        )
                    } else {
                        (
                            None,
                            "Selected component is not dd-alternating.".to_string(),
                        )
                    }
                } else {
                    (None, "Section has no components.".to_string())
                }
            }
            _ => (None, "Selected node is not a section.".to_string()),
        };
        if let Some((node_idx, column_idx, component_idx, item_idx)) = result.0 {
            self.selected_node = node_idx;
            self.selected_column = column_idx;
            self.selected_component = component_idx;
            self.selected_nested_item = item_idx;
            self.set_alternating_items_expanded(node_idx, column_idx, component_idx, true);
        }
        self.push_toast(ToastLevel::Info, result.1);
    }

    pub(in crate::tui) fn remove_selected_alternating_item(&mut self) {
        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let selected_nested_item = self.selected_nested_item;
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
                let col_i = selected_column.min(section.columns.len().saturating_sub(1));
                let components = &mut section.columns[col_i].components;
                if let Some(ci) = component_index(components.len(), selected_component) {
                    if let crate::model::SectionComponent::Alternating(alt) = &mut components[ci] {
                        if alt.items.len() <= 1 {
                            (
                                None,
                                "dd-alternating must keep at least one item.".to_string(),
                            )
                        } else {
                            let remove_idx = selected_nested_item.min(alt.items.len() - 1);
                            alt.items.remove(remove_idx);
                            let next_item_idx = remove_idx.min(alt.items.len() - 1);
                            (
                                Some((ni, col_i, ci, next_item_idx)),
                                format!("Removed alternating item {}.", remove_idx + 1),
                            )
                        }
                    } else {
                        (
                            None,
                            "Selected component is not dd-alternating.".to_string(),
                        )
                    }
                } else {
                    (None, "Section has no components.".to_string())
                }
            }
            _ => (None, "Selected node is not a section.".to_string()),
        };
        if let Some((node_idx, column_idx, component_idx, item_idx)) = result.0 {
            self.selected_node = node_idx;
            self.selected_column = column_idx;
            self.selected_component = component_idx;
            self.selected_nested_item = item_idx;
            self.set_alternating_items_expanded(node_idx, column_idx, component_idx, true);
        }
        self.push_toast(ToastLevel::Info, result.1);
    }

    pub(in crate::tui) fn add_selected_card_item(&mut self) {
        let rows = self.build_page_tree_rows();
        if rows.is_empty() {
            self.push_toast(ToastLevel::Warning, "No selected section.");
            return;
        }
        let row = rows[self.selected_tree_row.min(rows.len() - 1)];
        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let preferred_insert_after = match row.kind {
            TreeRowKind::CardItem { item_idx, .. } => Some(item_idx),
            _ => None,
        };
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
                let col_i = selected_column.min(section.columns.len().saturating_sub(1));
                let components = &mut section.columns[col_i].components;
                if let Some(ci) = component_index(components.len(), selected_component) {
                    if let crate::model::SectionComponent::Card(card) = &mut components[ci] {
                        let insert_idx = preferred_insert_after
                            .map(|i| (i + 1).min(card.items.len()))
                            .unwrap_or(card.items.len());
                        let next_num = card.items.len() + 1;
                        card.items.insert(
                            insert_idx,
                            crate::model::CardItem {
                                child_image_url: "https://dummyimage.com/720x720/000/fff"
                                    .to_string(),
                                child_image_alt: "Image alt text".to_string(),
                                child_title: format!("Title {}", next_num),
                                child_subtitle: "Subtitle".to_string(),
                                child_copy: "Copy".to_string(),
                                child_link_url: Some("/front".to_string()),
                                child_link_target: Some(crate::model::CardLinkTarget::SelfTarget),
                                child_link_label: Some("Learn More".to_string()),
                            },
                        );
                        (
                            Some(insert_idx),
                            format!("Added dd-card item {}.", insert_idx + 1),
                        )
                    } else {
                        (None, "Selected component is not dd-card.".to_string())
                    }
                } else {
                    (None, "Section has no components.".to_string())
                }
            }
            _ => (None, "Selected node is not a section.".to_string()),
        };
        if let Some(item_i) = result.0 {
            self.selected_nested_item = item_i;
            self.set_card_items_expanded(ni, selected_column, selected_component, true);
            self.sync_tree_row_with_selection();
        }
        self.push_toast(ToastLevel::Info, result.1);
    }

    pub(in crate::tui) fn remove_selected_card_item(&mut self) {
        let rows = self.build_page_tree_rows();
        if rows.is_empty() {
            self.push_toast(ToastLevel::Warning, "No selected section.");
            return;
        }
        let row = rows[self.selected_tree_row.min(rows.len() - 1)];
        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let selected_nested_item = self.selected_nested_item;
        let preferred_remove = match row.kind {
            TreeRowKind::CardItem { item_idx, .. } => Some(item_idx),
            _ => None,
        };
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
                let col_i = selected_column.min(section.columns.len().saturating_sub(1));
                let components = &mut section.columns[col_i].components;
                if let Some(ci) = component_index(components.len(), selected_component) {
                    if let crate::model::SectionComponent::Card(card) = &mut components[ci] {
                        if card.items.len() <= 1 {
                            (None, "dd-card must keep at least one item.".to_string())
                        } else {
                            let remove_i = preferred_remove.unwrap_or_else(|| {
                                selected_nested_item.min(card.items.len().saturating_sub(1))
                            });
                            card.items.remove(remove_i);
                            let next_i = remove_i.min(card.items.len().saturating_sub(1));
                            (
                                Some(next_i),
                                format!("Removed dd-card item {}.", remove_i + 1),
                            )
                        }
                    } else {
                        (None, "Selected component is not dd-card.".to_string())
                    }
                } else {
                    (None, "Section has no components.".to_string())
                }
            }
            _ => (None, "Selected node is not a section.".to_string()),
        };
        if let Some(item_i) = result.0 {
            self.selected_nested_item = item_i;
            self.set_card_items_expanded(ni, selected_column, selected_component, true);
            self.sync_tree_row_with_selection();
        }
        self.push_toast(ToastLevel::Info, result.1);
    }

    pub(in crate::tui) fn add_selected_filmstrip_item(&mut self) {
        let rows = self.build_page_tree_rows();
        if rows.is_empty() {
            self.push_toast(ToastLevel::Warning, "No selected section.");
            return;
        }
        let row = rows[self.selected_tree_row.min(rows.len() - 1)];
        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let preferred_insert_after = match row.kind {
            TreeRowKind::FilmstripItem { item_idx, .. } => Some(item_idx),
            _ => None,
        };
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
                let col_i = selected_column.min(section.columns.len().saturating_sub(1));
                let components = &mut section.columns[col_i].components;
                if let Some(ci) = component_index(components.len(), selected_component) {
                    if let crate::model::SectionComponent::Filmstrip(filmstrip) =
                        &mut components[ci]
                    {
                        let insert_idx = preferred_insert_after
                            .map(|i| (i + 1).min(filmstrip.items.len()))
                            .unwrap_or(filmstrip.items.len());
                        let next_num = filmstrip.items.len() + 1;
                        filmstrip.items.insert(
                            insert_idx,
                            crate::model::FilmstripItem {
                                child_image_url: "https://dummyimage.com/256x256/000/fff".to_string(),
                                child_image_alt: "Image alt text".to_string(),
                                child_title: format!("Title {}", next_num),
                            },
                        );
                        (
                            Some(insert_idx),
                            format!("Added dd-filmstrip item {}.", insert_idx + 1),
                        )
                    } else {
                        (None, "Selected component is not dd-filmstrip.".to_string())
                    }
                } else {
                    (None, "Section has no components.".to_string())
                }
            }
            _ => (None, "Selected node is not a section.".to_string()),
        };
        if let Some(item_i) = result.0 {
            self.selected_nested_item = item_i;
            self.set_filmstrip_items_expanded(ni, selected_column, selected_component, true);
            self.sync_tree_row_with_selection();
        }
        self.push_toast(ToastLevel::Info, result.1);
    }

    pub(in crate::tui) fn remove_selected_filmstrip_item(&mut self) {
        let rows = self.build_page_tree_rows();
        if rows.is_empty() {
            self.push_toast(ToastLevel::Warning, "No selected section.");
            return;
        }
        let row = rows[self.selected_tree_row.min(rows.len() - 1)];
        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let selected_nested_item = self.selected_nested_item;
        let preferred_remove = match row.kind {
            TreeRowKind::FilmstripItem { item_idx, .. } => Some(item_idx),
            _ => None,
        };
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
                let col_i = selected_column.min(section.columns.len().saturating_sub(1));
                let components = &mut section.columns[col_i].components;
                if let Some(ci) = component_index(components.len(), selected_component) {
                    if let crate::model::SectionComponent::Filmstrip(filmstrip) =
                        &mut components[ci]
                    {
                        if filmstrip.items.len() <= 1 {
                            (
                                None,
                                "dd-filmstrip must keep at least one item.".to_string(),
                            )
                        } else {
                            let remove_i = preferred_remove.unwrap_or_else(|| {
                                selected_nested_item.min(filmstrip.items.len().saturating_sub(1))
                            });
                            filmstrip.items.remove(remove_i);
                            let next_i = remove_i.min(filmstrip.items.len().saturating_sub(1));
                            (
                                Some(next_i),
                                format!("Removed dd-filmstrip item {}.", remove_i + 1),
                            )
                        }
                    } else {
                        (None, "Selected component is not dd-filmstrip.".to_string())
                    }
                } else {
                    (None, "Section has no components.".to_string())
                }
            }
            _ => (None, "Selected node is not a section.".to_string()),
        };
        if let Some(item_i) = result.0 {
            self.selected_nested_item = item_i;
            self.set_filmstrip_items_expanded(ni, selected_column, selected_component, true);
            self.sync_tree_row_with_selection();
        }
        self.push_toast(ToastLevel::Info, result.1);
    }

    pub(in crate::tui) fn add_selected_milestones_item(&mut self) {
        let rows = self.build_page_tree_rows();
        if rows.is_empty() {
            self.push_toast(ToastLevel::Warning, "No selected section.");
            return;
        }
        let row = rows[self.selected_tree_row.min(rows.len() - 1)];
        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let preferred_insert_after = match row.kind {
            TreeRowKind::MilestonesItem { item_idx, .. } => Some(item_idx),
            _ => None,
        };
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
                let col_i = selected_column.min(section.columns.len().saturating_sub(1));
                let components = &mut section.columns[col_i].components;
                if let Some(ci) = component_index(components.len(), selected_component) {
                    if let crate::model::SectionComponent::Milestones(milestones) =
                        &mut components[ci]
                    {
                        let insert_idx = preferred_insert_after
                            .map(|i| (i + 1).min(milestones.items.len()))
                            .unwrap_or(milestones.items.len());
                        let next_num = milestones.items.len() + 1;
                        milestones.items.insert(
                            insert_idx,
                            crate::model::MilestonesItem {
                                child_percentage: "70".to_string(),
                                child_title: format!("Title {}", next_num),
                                child_subtitle: "Subtitle".to_string(),
                                child_copy: "Copy".to_string(),
                                child_link_url: None,
                                child_link_target: Some(crate::model::CardLinkTarget::SelfTarget),
                                child_link_label: None,
                            },
                        );
                        (
                            Some(insert_idx),
                            format!("Added dd-milestones item {}.", insert_idx + 1),
                        )
                    } else {
                        (None, "Selected component is not dd-milestones.".to_string())
                    }
                } else {
                    (None, "Section has no components.".to_string())
                }
            }
            _ => (None, "Selected node is not a section.".to_string()),
        };
        if let Some(item_i) = result.0 {
            self.selected_nested_item = item_i;
            self.set_milestones_items_expanded(ni, selected_column, selected_component, true);
            self.sync_tree_row_with_selection();
        }
        self.push_toast(ToastLevel::Info, result.1);
    }

    pub(in crate::tui) fn remove_selected_milestones_item(&mut self) {
        let rows = self.build_page_tree_rows();
        if rows.is_empty() {
            self.push_toast(ToastLevel::Warning, "No selected section.");
            return;
        }
        let row = rows[self.selected_tree_row.min(rows.len() - 1)];
        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let selected_nested_item = self.selected_nested_item;
        let preferred_remove = match row.kind {
            TreeRowKind::MilestonesItem { item_idx, .. } => Some(item_idx),
            _ => None,
        };
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
                let col_i = selected_column.min(section.columns.len().saturating_sub(1));
                let components = &mut section.columns[col_i].components;
                if let Some(ci) = component_index(components.len(), selected_component) {
                    if let crate::model::SectionComponent::Milestones(milestones) =
                        &mut components[ci]
                    {
                        if milestones.items.len() <= 1 {
                            (
                                None,
                                "dd-milestones must keep at least one item.".to_string(),
                            )
                        } else {
                            let remove_i = preferred_remove.unwrap_or_else(|| {
                                selected_nested_item.min(milestones.items.len().saturating_sub(1))
                            });
                            milestones.items.remove(remove_i);
                            let next_i = remove_i.min(milestones.items.len().saturating_sub(1));
                            (
                                Some(next_i),
                                format!("Removed dd-milestones item {}.", remove_i + 1),
                            )
                        }
                    } else {
                        (None, "Selected component is not dd-milestones.".to_string())
                    }
                } else {
                    (None, "Section has no components.".to_string())
                }
            }
            _ => (None, "Selected node is not a section.".to_string()),
        };
        if let Some(item_i) = result.0 {
            self.selected_nested_item = item_i;
            self.set_milestones_items_expanded(ni, selected_column, selected_component, true);
            self.sync_tree_row_with_selection();
        }
        self.push_toast(ToastLevel::Info, result.1);
    }

    pub(in crate::tui) fn add_selected_slider_item(&mut self) {
        let rows = self.build_page_tree_rows();
        if rows.is_empty() {
            self.push_toast(ToastLevel::Warning, "No selected section.");
            return;
        }
        let row = rows[self.selected_tree_row.min(rows.len() - 1)];
        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let preferred_insert_after = match row.kind {
            TreeRowKind::SliderItem { item_idx, .. } => Some(item_idx),
            _ => None,
        };
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
                let col_i = selected_column.min(section.columns.len().saturating_sub(1));
                let components = &mut section.columns[col_i].components;
                if let Some(ci) = component_index(components.len(), selected_component) {
                    if let crate::model::SectionComponent::Slider(slider) = &mut components[ci] {
                        let insert_idx = preferred_insert_after
                            .map(|i| (i + 1).min(slider.items.len()))
                            .unwrap_or(slider.items.len());
                        let next_num = slider.items.len() + 1;
                        slider.items.insert(
                            insert_idx,
                            crate::model::SliderItem {
                                child_title: format!("Title {}", next_num),
                                child_copy: "Copy".to_string(),
                                child_link_url: Some("/path".to_string()),
                                child_link_target: Some(crate::model::CardLinkTarget::SelfTarget),
                                child_link_label: Some("Learn More".to_string()),
                                child_image_url: "https://dummyimage.com/720x720/000/fff"
                                    .to_string(),
                                child_image_alt: "Image alt text".to_string(),
                            },
                        );
                        (
                            Some(insert_idx),
                            format!("Added dd-slider item {}.", insert_idx + 1),
                        )
                    } else {
                        (None, "Selected component is not dd-slider.".to_string())
                    }
                } else {
                    (None, "Section has no components.".to_string())
                }
            }
            _ => (None, "Selected node is not a section.".to_string()),
        };
        if let Some(item_i) = result.0 {
            self.selected_nested_item = item_i;
            self.set_slider_items_expanded(ni, selected_column, selected_component, true);
            self.sync_tree_row_with_selection();
        }
        self.push_toast(ToastLevel::Info, result.1);
    }

    pub(in crate::tui) fn remove_selected_slider_item(&mut self) {
        let rows = self.build_page_tree_rows();
        if rows.is_empty() {
            self.push_toast(ToastLevel::Warning, "No selected section.");
            return;
        }
        let row = rows[self.selected_tree_row.min(rows.len() - 1)];
        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let selected_nested_item = self.selected_nested_item;
        let preferred_remove = match row.kind {
            TreeRowKind::SliderItem { item_idx, .. } => Some(item_idx),
            _ => None,
        };
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
                let col_i = selected_column.min(section.columns.len().saturating_sub(1));
                let components = &mut section.columns[col_i].components;
                if let Some(ci) = component_index(components.len(), selected_component) {
                    if let crate::model::SectionComponent::Slider(slider) = &mut components[ci] {
                        if slider.items.len() <= 1 {
                            (None, "dd-slider must keep at least one item.".to_string())
                        } else {
                            let remove_i = preferred_remove.unwrap_or_else(|| {
                                selected_nested_item.min(slider.items.len().saturating_sub(1))
                            });
                            slider.items.remove(remove_i);
                            let next_i = remove_i.min(slider.items.len().saturating_sub(1));
                            (
                                Some(next_i),
                                format!("Removed dd-slider item {}.", remove_i + 1),
                            )
                        }
                    } else {
                        (None, "Selected component is not dd-slider.".to_string())
                    }
                } else {
                    (None, "Section has no components.".to_string())
                }
            }
            _ => (None, "Selected node is not a section.".to_string()),
        };
        if let Some(item_i) = result.0 {
            self.selected_nested_item = item_i;
            self.set_slider_items_expanded(ni, selected_column, selected_component, true);
            self.sync_tree_row_with_selection();
        }
        self.push_toast(ToastLevel::Info, result.1);
    }
}
