//! Layout tree navigation and structural edits.
use super::*;
#[derive(Clone, Copy)]
pub(super) struct TreeRow {
    pub(super) kind: TreeRowKind,
}

#[derive(Clone, Copy)]
pub(super) enum TreeRowKind {
    HeaderRoot,
    HeaderSection {
        section_idx: usize,
    },
    HeaderColumn {
        section_idx: usize,
        column_idx: usize,
    },
    HeaderComponent {
        section_idx: usize,
        column_idx: usize,
        component_idx: usize,
    },
    FooterRoot,
    FooterSection {
        section_idx: usize,
    },
    FooterColumn {
        section_idx: usize,
        column_idx: usize,
    },
    FooterComponent {
        section_idx: usize,
        column_idx: usize,
        component_idx: usize,
    },
    PageHead,
    Hero {
        node_idx: usize,
    },
    Section {
        node_idx: usize,
    },
    Column {
        node_idx: usize,
        column_idx: usize,
    },
    Component {
        node_idx: usize,
        column_idx: usize,
        component_idx: usize,
    },
    AccordionItem {
        node_idx: usize,
        column_idx: usize,
        component_idx: usize,
        item_idx: usize,
    },
    AlternatingItem {
        node_idx: usize,
        column_idx: usize,
        component_idx: usize,
        item_idx: usize,
    },
    CardItem {
        node_idx: usize,
        column_idx: usize,
        component_idx: usize,
        item_idx: usize,
    },
    FilmstripItem {
        node_idx: usize,
        column_idx: usize,
        component_idx: usize,
        item_idx: usize,
    },
    MilestonesItem {
        node_idx: usize,
        column_idx: usize,
        component_idx: usize,
        item_idx: usize,
    },
    SliderItem {
        node_idx: usize,
        column_idx: usize,
        component_idx: usize,
        item_idx: usize,
    },
}

impl App {
    pub(super) fn build_tree_rows(&self) -> Vec<TreeRow> {
        match self.selected_region {
            SelectedRegion::Header => self.build_header_tree_rows(),
            SelectedRegion::Footer => self.build_footer_tree_rows(),
            SelectedRegion::Page => self.build_page_tree_rows(),
        }
    }
    pub(super) fn build_footer_tree_rows(&self) -> Vec<TreeRow> {
        let mut rows = Vec::new();
        rows.push(TreeRow {
            kind: TreeRowKind::FooterRoot,
        });
        for (section_idx, section) in self.site.footer.sections.iter().enumerate() {
            rows.push(TreeRow {
                kind: TreeRowKind::FooterSection { section_idx },
            });
            for (column_idx, _) in section.columns.iter().enumerate() {
                rows.push(TreeRow {
                    kind: TreeRowKind::FooterColumn {
                        section_idx,
                        column_idx,
                    },
                });
                for (component_idx, _) in
                    section.columns[column_idx].components.iter().enumerate()
                {
                    rows.push(TreeRow {
                        kind: TreeRowKind::FooterComponent {
                            section_idx,
                            column_idx,
                            component_idx,
                        },
                    });
                }
            }
        }
        rows
    }
    pub(super) fn build_page_tree_rows(&self) -> Vec<TreeRow> {
        if self.site.pages.is_empty() {
            return Vec::new();
        }
        let page = self.current_page();
        let mut rows = Vec::new();
        rows.push(TreeRow {
            kind: TreeRowKind::PageHead,
        });
        for (node_idx, node) in page.nodes.iter().enumerate() {
            match node {
                PageNode::Hero(_) => rows.push(TreeRow {
                    kind: TreeRowKind::Hero { node_idx },
                }),
                PageNode::Section(section) => {
                    rows.push(TreeRow {
                        kind: TreeRowKind::Section { node_idx },
                    });
                    if self.is_section_expanded(node_idx) {
                        let columns = section_columns_ref(section);
                        for (column_idx, col) in columns.iter().enumerate() {
                            rows.push(TreeRow {
                                kind: TreeRowKind::Column {
                                    node_idx,
                                    column_idx,
                                },
                            });
                            for (component_idx, _) in col.components.iter().enumerate() {
                                rows.push(TreeRow {
                                    kind: TreeRowKind::Component {
                                        node_idx,
                                        column_idx,
                                        component_idx,
                                    },
                                });
                                if let Some(crate::model::SectionComponent::Accordion(acc)) =
                                    col.components.get(component_idx)
                                {
                                    if self.is_accordion_items_expanded(
                                        node_idx,
                                        column_idx,
                                        component_idx,
                                    ) {
                                        for (item_idx, _) in acc.items.iter().enumerate() {
                                            rows.push(TreeRow {
                                                kind: TreeRowKind::AccordionItem {
                                                    node_idx,
                                                    column_idx,
                                                    component_idx,
                                                    item_idx,
                                                },
                                            });
                                        }
                                    }
                                }
                                if let Some(crate::model::SectionComponent::Alternating(alt)) =
                                    col.components.get(component_idx)
                                {
                                    if self.is_alternating_items_expanded(
                                        node_idx,
                                        column_idx,
                                        component_idx,
                                    ) {
                                        for (item_idx, _) in alt.items.iter().enumerate() {
                                            rows.push(TreeRow {
                                                kind: TreeRowKind::AlternatingItem {
                                                    node_idx,
                                                    column_idx,
                                                    component_idx,
                                                    item_idx,
                                                },
                                            });
                                        }
                                    }
                                }
                                if let Some(crate::model::SectionComponent::Card(card)) =
                                    col.components.get(component_idx)
                                {
                                    if self.is_card_items_expanded(
                                        node_idx,
                                        column_idx,
                                        component_idx,
                                    ) {
                                        for (item_idx, _) in card.items.iter().enumerate() {
                                            rows.push(TreeRow {
                                                kind: TreeRowKind::CardItem {
                                                    node_idx,
                                                    column_idx,
                                                    component_idx,
                                                    item_idx,
                                                },
                                            });
                                        }
                                    }
                                }
                                if let Some(crate::model::SectionComponent::Filmstrip(filmstrip)) =
                                    col.components.get(component_idx)
                                {
                                    if self.is_filmstrip_items_expanded(
                                        node_idx,
                                        column_idx,
                                        component_idx,
                                    ) {
                                        for (item_idx, _) in filmstrip.items.iter().enumerate() {
                                            rows.push(TreeRow {
                                                kind: TreeRowKind::FilmstripItem {
                                                    node_idx,
                                                    column_idx,
                                                    component_idx,
                                                    item_idx,
                                                },
                                            });
                                        }
                                    }
                                }
                                if let Some(crate::model::SectionComponent::Milestones(
                                    milestones,
                                )) = col.components.get(component_idx)
                                {
                                    if self.is_milestones_items_expanded(
                                        node_idx,
                                        column_idx,
                                        component_idx,
                                    ) {
                                        for (item_idx, _) in milestones.items.iter().enumerate() {
                                            rows.push(TreeRow {
                                                kind: TreeRowKind::MilestonesItem {
                                                    node_idx,
                                                    column_idx,
                                                    component_idx,
                                                    item_idx,
                                                },
                                            });
                                        }
                                    }
                                }
                                if let Some(crate::model::SectionComponent::Slider(slider)) =
                                    col.components.get(component_idx)
                                {
                                    if self.is_slider_items_expanded(
                                        node_idx,
                                        column_idx,
                                        component_idx,
                                    ) {
                                        for (item_idx, _) in slider.items.iter().enumerate() {
                                            rows.push(TreeRow {
                                                kind: TreeRowKind::SliderItem {
                                                    node_idx,
                                                    column_idx,
                                                    component_idx,
                                                    item_idx,
                                                },
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        rows
    }
    pub(super) fn build_header_tree_rows(&self) -> Vec<TreeRow> {
        let mut rows = Vec::new();
        rows.push(TreeRow {
            kind: TreeRowKind::HeaderRoot,
        });
        if self.header_column_expanded {
            for (section_idx, section) in self.site.header.sections.iter().enumerate() {
                rows.push(TreeRow {
                    kind: TreeRowKind::HeaderSection { section_idx },
                });
                if self.is_header_section_expanded(section_idx) {
                    for (column_idx, _) in section.columns.iter().enumerate() {
                        rows.push(TreeRow {
                            kind: TreeRowKind::HeaderColumn {
                                section_idx,
                                column_idx,
                            },
                        });
                        for (component_idx, _) in
                            section.columns[column_idx].components.iter().enumerate()
                        {
                            rows.push(TreeRow {
                                kind: TreeRowKind::HeaderComponent {
                                    section_idx,
                                    column_idx,
                                    component_idx,
                                },
                            });
                        }
                    }
                }
            }
        }
        rows
    }
    pub(super) fn is_header_section_expanded(&self, section_idx: usize) -> bool {
        self.expanded_sections.contains(&(usize::MAX, section_idx))
    }
    pub(super) fn set_header_section_expanded(&mut self, section_idx: usize, expanded: bool) {
        let key = (usize::MAX, section_idx);
        if expanded {
            self.expanded_sections.insert(key);
        } else {
            self.expanded_sections.remove(&key);
        }
    }
    pub(super) fn tree_row_label(&self, row: &TreeRow) -> String {
        match &row.kind {
            TreeRowKind::HeaderRoot => {
                let marker = if self.header_column_expanded {
                    "[-]"
                } else {
                    "[+]"
                };
                format!("1. {} dd-header ({})", marker, self.site.header.id)
            }
            TreeRowKind::HeaderSection { section_idx } => {
                let section_i =
                    (*section_idx).min(self.site.header.sections.len().saturating_sub(1));
                let section = &self.site.header.sections[section_i];
                let marker = if self.is_header_section_expanded(*section_idx) {
                    "[-]"
                } else {
                    "[+]"
                };
                format!(
                    "  {} {} dd-section ({})",
                    section_i + 1,
                    marker,
                    section.id
                )
            }
            TreeRowKind::HeaderColumn {
                section_idx,
                column_idx,
            } => {
                let section_i =
                    (*section_idx).min(self.site.header.sections.len().saturating_sub(1));
                let section = &self.site.header.sections[section_i];
                let col_i = (*column_idx).min(section.columns.len().saturating_sub(1));
                let col = &section.columns[col_i];
                format!(
                    "    |- column {} ({}) [{}]",
                    col_i + 1,
                    col.id,
                    col.width_class
                )
            }
            TreeRowKind::HeaderComponent {
                section_idx,
                column_idx,
                component_idx,
            } => {
                let section_i =
                    (*section_idx).min(self.site.header.sections.len().saturating_sub(1));
                let section = &self.site.header.sections[section_i];
                let col_i = (*column_idx).min(section.columns.len().saturating_sub(1));
                let comp_i =
                    (*component_idx).min(section.columns[col_i].components.len().saturating_sub(1));
                let component = &section.columns[col_i].components[comp_i];
                let label = component_label(component);
                format!("      - {} {}", comp_i + 1, label)
            }
            TreeRowKind::FooterRoot => {
                format!("1. [FOOTER] dd-footer ({})", self.site.footer.id)
            }
            TreeRowKind::FooterSection { section_idx } => {
                let section_i =
                    (*section_idx).min(self.site.footer.sections.len().saturating_sub(1));
                let section = &self.site.footer.sections[section_i];
                format!("  {} dd-section ({})", section_i + 1, section.id)
            }
            TreeRowKind::FooterColumn {
                section_idx,
                column_idx,
            } => {
                let section_i =
                    (*section_idx).min(self.site.footer.sections.len().saturating_sub(1));
                let section = &self.site.footer.sections[section_i];
                let col_i = (*column_idx).min(section.columns.len().saturating_sub(1));
                let col = &section.columns[col_i];
                format!(
                    "    |- column {} ({}) [{}]",
                    col_i + 1,
                    col.id,
                    col.width_class
                )
            }
            TreeRowKind::FooterComponent {
                section_idx,
                column_idx,
                component_idx,
            } => {
                let section_i =
                    (*section_idx).min(self.site.footer.sections.len().saturating_sub(1));
                let section = &self.site.footer.sections[section_i];
                let col_i = (*column_idx).min(section.columns.len().saturating_sub(1));
                let comp_i =
                    (*component_idx).min(section.columns[col_i].components.len().saturating_sub(1));
                let component = &section.columns[col_i].components[comp_i];
                let label = component_label(component);
                format!("      - {} {}", comp_i + 1, label)
            }
            TreeRowKind::PageHead => {
                let page = self.current_page();
                format!("[HEAD] {}", page.head.title)
            }
            TreeRowKind::Hero { node_idx } => format!("{}. dd-hero", node_idx + 1),
            TreeRowKind::Section { node_idx } => {
                let page = self.current_page();
                let PageNode::Section(section) = &page.nodes[*node_idx] else {
                    return format!("{}. dd-section", node_idx + 1);
                };
                let marker = if self.is_section_expanded(*node_idx) {
                    "[-]"
                } else {
                    "[+]"
                };
                format!("{}. {} dd-section ({})", node_idx + 1, marker, section.id)
            }
            TreeRowKind::Column {
                node_idx,
                column_idx,
            } => {
                let page = self.current_page();
                let PageNode::Section(section) = &page.nodes[*node_idx] else {
                    return format!("  |- column {}", column_idx + 1);
                };
                let columns = section_columns_ref(section);
                let col_i = (*column_idx).min(columns.len().saturating_sub(1));
                let col = &columns[col_i];
                format!(
                    "  |- column {} ({}) [{}]",
                    col_i + 1,
                    col.id,
                    col.width_class
                )
            }
            TreeRowKind::Component {
                node_idx,
                column_idx,
                component_idx,
            } => {
                let page = self.current_page();
                let PageNode::Section(section) = &page.nodes[*node_idx] else {
                    return format!("    - component {}", component_idx + 1);
                };
                let columns = section_columns_ref(section);
                let col_i = (*column_idx).min(columns.len().saturating_sub(1));
                let comp_i =
                    (*component_idx).min(columns[col_i].components.len().saturating_sub(1));
                let component = &columns[col_i].components[comp_i];
                let label = component_label(component);
                if matches!(component, crate::model::SectionComponent::Accordion(_)) {
                    let marker = if self.is_accordion_items_expanded(*node_idx, col_i, comp_i) {
                        "[-]"
                    } else {
                        "[+]"
                    };
                    format!("    - {} {} {}", comp_i + 1, marker, label)
                } else if matches!(component, crate::model::SectionComponent::Alternating(_)) {
                    let marker = if self.is_alternating_items_expanded(*node_idx, col_i, comp_i) {
                        "[-]"
                    } else {
                        "[+]"
                    };
                    format!("    - {} {} {}", comp_i + 1, marker, label)
                } else if matches!(component, crate::model::SectionComponent::Card(_)) {
                    let marker = if self.is_card_items_expanded(*node_idx, col_i, comp_i) {
                        "[-]"
                    } else {
                        "[+]"
                    };
                    format!("    - {} {} {}", comp_i + 1, marker, label)
                } else if matches!(component, crate::model::SectionComponent::Filmstrip(_)) {
                    let marker = if self.is_filmstrip_items_expanded(*node_idx, col_i, comp_i) {
                        "[-]"
                    } else {
                        "[+]"
                    };
                    format!("    - {} {} {}", comp_i + 1, marker, label)
                } else if matches!(component, crate::model::SectionComponent::Milestones(_)) {
                    let marker = if self.is_milestones_items_expanded(*node_idx, col_i, comp_i) {
                        "[-]"
                    } else {
                        "[+]"
                    };
                    format!("    - {} {} {}", comp_i + 1, marker, label)
                } else if matches!(component, crate::model::SectionComponent::Slider(_)) {
                    let marker = if self.is_slider_items_expanded(*node_idx, col_i, comp_i) {
                        "[-]"
                    } else {
                        "[+]"
                    };
                    format!("    - {} {} {}", comp_i + 1, marker, label)
                } else {
                    format!("    - {} {}", comp_i + 1, label)
                }
            }
            TreeRowKind::AccordionItem {
                node_idx,
                column_idx,
                component_idx,
                item_idx,
            } => {
                let page = self.current_page();
                let PageNode::Section(section) = &page.nodes[*node_idx] else {
                    return format!("      - item {}", item_idx + 1);
                };
                let columns = section_columns_ref(section);
                let col_i = (*column_idx).min(columns.len().saturating_sub(1));
                let comp_i =
                    (*component_idx).min(columns[col_i].components.len().saturating_sub(1));
                let acc = match &columns[col_i].components[comp_i] {
                    crate::model::SectionComponent::Accordion(a) => a,
                    _ => return format!("      - item {}", item_idx + 1),
                };
                let item_i = (*item_idx).min(acc.items.len().saturating_sub(1));
                let item = &acc.items[item_i];
                format!(
                    "      - {}: {}",
                    item_i + 1,
                    truncate_ascii(&item.child_title, 40)
                )
            }
            TreeRowKind::AlternatingItem {
                node_idx,
                column_idx,
                component_idx,
                item_idx,
            } => {
                let page = self.current_page();
                let PageNode::Section(section) = &page.nodes[*node_idx] else {
                    return format!("      - item {}", item_idx + 1);
                };
                let columns = section_columns_ref(section);
                let col_i = (*column_idx).min(columns.len().saturating_sub(1));
                let comp_i =
                    (*component_idx).min(columns[col_i].components.len().saturating_sub(1));
                let alt = match &columns[col_i].components[comp_i] {
                    crate::model::SectionComponent::Alternating(a) => a,
                    _ => return format!("      - item {}", item_idx + 1),
                };
                let item_i = (*item_idx).min(alt.items.len().saturating_sub(1));
                let item = &alt.items[item_i];
                format!(
                    "      - {}: {}",
                    item_i + 1,
                    truncate_ascii(&item.child_title, 40)
                )
            }
            TreeRowKind::CardItem {
                node_idx,
                column_idx,
                component_idx,
                item_idx,
            } => {
                let page = self.current_page();
                let PageNode::Section(section) = &page.nodes[*node_idx] else {
                    return format!("      - item {}", item_idx + 1);
                };
                let columns = section_columns_ref(section);
                let col_i = (*column_idx).min(columns.len().saturating_sub(1));
                let comp_i =
                    (*component_idx).min(columns[col_i].components.len().saturating_sub(1));
                let card = match &columns[col_i].components[comp_i] {
                    crate::model::SectionComponent::Card(c) => c,
                    _ => return format!("      - item {}", item_idx + 1),
                };
                let item_i = (*item_idx).min(card.items.len().saturating_sub(1));
                let item = &card.items[item_i];
                format!(
                    "      - {}: {}",
                    item_i + 1,
                    truncate_ascii(&item.child_title, 40)
                )
            }
            TreeRowKind::FilmstripItem {
                node_idx,
                column_idx,
                component_idx,
                item_idx,
            } => {
                let page = self.current_page();
                let PageNode::Section(section) = &page.nodes[*node_idx] else {
                    return format!("      - item {}", item_idx + 1);
                };
                let columns = section_columns_ref(section);
                let col_i = (*column_idx).min(columns.len().saturating_sub(1));
                let comp_i =
                    (*component_idx).min(columns[col_i].components.len().saturating_sub(1));
                let filmstrip = match &columns[col_i].components[comp_i] {
                    crate::model::SectionComponent::Filmstrip(f) => f,
                    _ => return format!("      - item {}", item_idx + 1),
                };
                let item_i = (*item_idx).min(filmstrip.items.len().saturating_sub(1));
                let item = &filmstrip.items[item_i];
                format!(
                    "      - {}: {}",
                    item_i + 1,
                    truncate_ascii(&item.child_title, 40)
                )
            }
            TreeRowKind::MilestonesItem {
                node_idx,
                column_idx,
                component_idx,
                item_idx,
            } => {
                let page = self.current_page();
                let PageNode::Section(section) = &page.nodes[*node_idx] else {
                    return format!("      - item {}", item_idx + 1);
                };
                let columns = section_columns_ref(section);
                let col_i = (*column_idx).min(columns.len().saturating_sub(1));
                let comp_i =
                    (*component_idx).min(columns[col_i].components.len().saturating_sub(1));
                let milestones = match &columns[col_i].components[comp_i] {
                    crate::model::SectionComponent::Milestones(m) => m,
                    _ => return format!("      - item {}", item_idx + 1),
                };
                let item_i = (*item_idx).min(milestones.items.len().saturating_sub(1));
                let item = &milestones.items[item_i];
                format!(
                    "      - {}: {}",
                    item_i + 1,
                    truncate_ascii(&item.child_title, 40)
                )
            }
            TreeRowKind::SliderItem {
                node_idx,
                column_idx,
                component_idx,
                item_idx,
            } => {
                let page = self.current_page();
                let PageNode::Section(section) = &page.nodes[*node_idx] else {
                    return format!("      - item {}", item_idx + 1);
                };
                let columns = section_columns_ref(section);
                let col_i = (*column_idx).min(columns.len().saturating_sub(1));
                let comp_i =
                    (*component_idx).min(columns[col_i].components.len().saturating_sub(1));
                let slider = match &columns[col_i].components[comp_i] {
                    crate::model::SectionComponent::Slider(s) => s,
                    _ => return format!("      - item {}", item_idx + 1),
                };
                let item_i = (*item_idx).min(slider.items.len().saturating_sub(1));
                let item = &slider.items[item_i];
                format!(
                    "      - {}: {}",
                    item_i + 1,
                    truncate_ascii(&item.child_title, 40)
                )
            }
        }
    }
    pub(super) fn apply_tree_row_selection(&mut self, row: TreeRow) {
        self.page_head_selected = matches!(row.kind, TreeRowKind::PageHead);
        match row.kind {
            TreeRowKind::HeaderRoot { .. } => {
                self.selected_header_section = 0;
                self.selected_header_column = 0;
                self.selected_header_component = 0;
            }
            TreeRowKind::HeaderSection { section_idx } => {
                self.selected_header_section = section_idx;
                self.selected_header_column = 0;
                self.selected_header_component = 0;
            }
            TreeRowKind::HeaderColumn {
                section_idx,
                column_idx,
            } => {
                self.selected_header_section = section_idx;
                self.selected_header_column = column_idx;
                self.selected_header_component = 0;
            }
            TreeRowKind::HeaderComponent {
                section_idx,
                column_idx,
                component_idx,
            } => {
                self.selected_header_section = section_idx;
                self.selected_header_column = column_idx;
                self.selected_header_component = component_idx;
            }
            TreeRowKind::FooterRoot => {
                self.selected_header_section = 0;
                self.selected_header_column = 0;
                self.selected_header_component = 0;
            }
            TreeRowKind::FooterSection { section_idx } => {
                self.selected_header_section = section_idx;
                self.selected_header_column = 0;
                self.selected_header_component = 0;
            }
            TreeRowKind::FooterColumn {
                section_idx,
                column_idx,
            } => {
                self.selected_header_section = section_idx;
                self.selected_header_column = column_idx;
                self.selected_header_component = 0;
            }
            TreeRowKind::FooterComponent {
                section_idx,
                column_idx,
                component_idx,
            } => {
                self.selected_header_section = section_idx;
                self.selected_header_column = column_idx;
                self.selected_header_component = component_idx;
            }
            TreeRowKind::PageHead => {
                // head row; selection stays pinned but nothing specific
            }
            TreeRowKind::Hero { node_idx } => {
                self.selected_node = node_idx;
                self.selected_column = 0;
                self.selected_component = 0;
                self.selected_nested_item = 0;
            }
            TreeRowKind::Section { node_idx } => {
                self.selected_node = node_idx;
                self.selected_column = 0;
                self.selected_component = 0;
                self.selected_nested_item = 0;
            }
            TreeRowKind::Column {
                node_idx,
                column_idx,
            } => {
                self.selected_node = node_idx;
                self.selected_column = column_idx;
                self.selected_component = 0;
                self.selected_nested_item = 0;
            }
            TreeRowKind::Component {
                node_idx,
                column_idx,
                component_idx,
            } => {
                self.selected_node = node_idx;
                self.selected_column = column_idx;
                self.selected_component = component_idx;
                self.selected_nested_item = 0;
            }
            TreeRowKind::AccordionItem {
                node_idx,
                column_idx,
                component_idx,
                item_idx,
            } => {
                self.selected_node = node_idx;
                self.selected_column = column_idx;
                self.selected_component = component_idx;
                self.selected_nested_item = item_idx;
            }
            TreeRowKind::AlternatingItem {
                node_idx,
                column_idx,
                component_idx,
                item_idx,
            } => {
                self.selected_node = node_idx;
                self.selected_column = column_idx;
                self.selected_component = component_idx;
                self.selected_nested_item = item_idx;
            }
            TreeRowKind::CardItem {
                node_idx,
                column_idx,
                component_idx,
                item_idx,
            } => {
                self.selected_node = node_idx;
                self.selected_column = column_idx;
                self.selected_component = component_idx;
                self.selected_nested_item = item_idx;
            }
            TreeRowKind::FilmstripItem {
                node_idx,
                column_idx,
                component_idx,
                item_idx,
            } => {
                self.selected_node = node_idx;
                self.selected_column = column_idx;
                self.selected_component = component_idx;
                self.selected_nested_item = item_idx;
            }
            TreeRowKind::MilestonesItem {
                node_idx,
                column_idx,
                component_idx,
                item_idx,
            } => {
                self.selected_node = node_idx;
                self.selected_column = column_idx;
                self.selected_component = component_idx;
                self.selected_nested_item = item_idx;
            }
            TreeRowKind::SliderItem {
                node_idx,
                column_idx,
                component_idx,
                item_idx,
            } => {
                self.selected_node = node_idx;
                self.selected_column = column_idx;
                self.selected_component = component_idx;
                self.selected_nested_item = item_idx;
            }
        }
    }
    pub(super) fn sync_tree_row_with_selection(&mut self) {
        let rows = self.build_tree_rows();
        if rows.is_empty() {
            self.selected_tree_row = 0;
            return;
        }
        let row_matches_selection = |row: &TreeRow| match row.kind {
            TreeRowKind::HeaderRoot { .. } => true,
            TreeRowKind::HeaderSection { section_idx } => {
                section_idx == self.selected_header_section
            }
            TreeRowKind::HeaderColumn {
                section_idx,
                column_idx,
            } => {
                section_idx == self.selected_header_section
                    && column_idx == self.selected_header_column
            }
            TreeRowKind::HeaderComponent {
                section_idx,
                column_idx,
                component_idx,
            } => {
                section_idx == self.selected_header_section
                    && column_idx == self.selected_header_column
                    && component_idx == self.selected_header_component
            }
            TreeRowKind::Hero { node_idx } => {
                !self.page_head_selected && node_idx == self.selected_node
            }
            TreeRowKind::Section { node_idx } => {
                !self.page_head_selected && node_idx == self.selected_node
            }
            TreeRowKind::Column {
                node_idx,
                column_idx,
            } => node_idx == self.selected_node && column_idx == self.selected_column,
            TreeRowKind::Component {
                node_idx,
                column_idx,
                component_idx,
            } => {
                node_idx == self.selected_node
                    && column_idx == self.selected_column
                    && component_idx == self.selected_component
                    && self.selected_nested_item == 0
            }
            TreeRowKind::AccordionItem {
                node_idx,
                column_idx,
                component_idx,
                item_idx,
            } => {
                node_idx == self.selected_node
                    && column_idx == self.selected_column
                    && component_idx == self.selected_component
                    && item_idx == self.selected_nested_item
            }
            TreeRowKind::AlternatingItem {
                node_idx,
                column_idx,
                component_idx,
                item_idx,
            } => {
                node_idx == self.selected_node
                    && column_idx == self.selected_column
                    && component_idx == self.selected_component
                    && item_idx == self.selected_nested_item
            }
            TreeRowKind::CardItem {
                node_idx,
                column_idx,
                component_idx,
                item_idx,
            } => {
                node_idx == self.selected_node
                    && column_idx == self.selected_column
                    && component_idx == self.selected_component
                    && item_idx == self.selected_nested_item
            }
            TreeRowKind::FilmstripItem {
                node_idx,
                column_idx,
                component_idx,
                item_idx,
            } => {
                node_idx == self.selected_node
                    && column_idx == self.selected_column
                    && component_idx == self.selected_component
                    && item_idx == self.selected_nested_item
            }
            TreeRowKind::MilestonesItem {
                node_idx,
                column_idx,
                component_idx,
                item_idx,
            } => {
                node_idx == self.selected_node
                    && column_idx == self.selected_column
                    && component_idx == self.selected_component
                    && item_idx == self.selected_nested_item
            }
            TreeRowKind::SliderItem {
                node_idx,
                column_idx,
                component_idx,
                item_idx,
            } => {
                node_idx == self.selected_node
                    && column_idx == self.selected_column
                    && component_idx == self.selected_component
                    && item_idx == self.selected_nested_item
            }
            TreeRowKind::FooterRoot => true,
            TreeRowKind::FooterSection { section_idx } => {
                section_idx == self.selected_header_section
            }
            TreeRowKind::FooterColumn {
                section_idx,
                column_idx,
            } => {
                section_idx == self.selected_header_section
                    && column_idx == self.selected_header_column
            }
            TreeRowKind::FooterComponent {
                section_idx,
                column_idx,
                component_idx,
            } => {
                section_idx == self.selected_header_section
                    && column_idx == self.selected_header_column
                    && component_idx == self.selected_header_component
            }
            TreeRowKind::PageHead => self.page_head_selected,
        };

        if let Some(current) = rows.get(self.selected_tree_row) {
            if row_matches_selection(current) {
                return;
            }
        }

        let wanted = rows
            .iter()
            .position(row_matches_selection)
            .unwrap_or_else(|| self.selected_tree_row.min(rows.len().saturating_sub(1)));
        self.selected_tree_row = wanted;
    }
    pub(super) fn is_section_expanded(&self, node_idx: usize) -> bool {
        !self
            .expanded_sections
            .contains(&(self.selected_page, node_idx))
    }
    pub(super) fn set_section_expanded(&mut self, node_idx: usize, expanded: bool) {
        if expanded {
            self.expanded_sections
                .remove(&(self.selected_page, node_idx));
        } else {
            self.expanded_sections
                .insert((self.selected_page, node_idx));
        }
    }
    pub(super) fn is_accordion_items_expanded(
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
    pub(super) fn set_accordion_items_expanded(
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
    pub(super) fn is_alternating_items_expanded(
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
    pub(super) fn set_alternating_items_expanded(
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
    pub(super) fn is_card_items_expanded(
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
    pub(super) fn set_card_items_expanded(
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
    pub(super) fn is_filmstrip_items_expanded(
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
    pub(super) fn set_filmstrip_items_expanded(
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
    pub(super) fn is_milestones_items_expanded(
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
    pub(super) fn set_milestones_items_expanded(
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
    pub(super) fn is_slider_items_expanded(
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
    pub(super) fn set_slider_items_expanded(
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
    pub(super) fn toggle_selected_tree_expanded(&mut self) {
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
    pub(super) fn handle_enter_on_selected_row(&mut self) {
        let rows = self.build_tree_rows();
        if rows.is_empty() {
            return;
        }
        let row = rows[self.selected_tree_row.min(rows.len() - 1)];
        if self.try_open_form_edit(&row) {
            return;
        }
        if self.try_open_form_edit_drilled_into_item(&row) {
            return;
        }
        if self.try_open_form_edit_drilled_into_column(&row) {
            return;
        }
        self.push_toast(ToastLevel::Warning, "Cannot edit this row.");
    }
    pub(super) fn open_component_picker(&mut self) {
        self.modal = Some(Modal::ComponentPicker {
            query: String::new(),
            selected: 0,
        });
    }
    pub(super) fn try_open_form_edit(&mut self, row: &TreeRow) -> bool {
        // Hero and Section tree rows get the unified form too.
        if let Some((state, new_cursor, title)) = self.try_open_hero_or_section(row) {
            let cursor_pos = state.get(state.form.fields[state.focused_field].id).len();
            self.modal = Some(Modal::FormEdit {
                state,
                cursor: new_cursor,
                cursor_pos,
                drill_stack: Vec::new(),
                scroll_offset: 0,
            });
            self.push_toast(ToastLevel::Info, format!("Editing {}.", title));
            return true;
        }

        // Roots like page-head, header-root, footer use the unified form too.
        if let Some((state, new_cursor, title)) = self.try_open_root(row) {
            let cursor_pos = state.get(state.form.fields[state.focused_field].id).len();
            self.modal = Some(Modal::FormEdit {
                state,
                cursor: new_cursor,
                cursor_pos,
                drill_stack: Vec::new(),
                scroll_offset: 0,
            });
            self.push_toast(ToastLevel::Info, format!("Editing {}.", title));
            return true;
        }

        let (maybe_component, new_cursor) = match row.kind {
            TreeRowKind::HeaderComponent {
                section_idx,
                column_idx,
                component_idx,
            } => {
                let component = self
                    .site
                    .header
                    .sections
                    .get(section_idx)
                    .and_then(|s| s.columns.get(column_idx))
                    .and_then(|c| c.components.get(component_idx))
                    .cloned();
                (
                    component,
                    cursor::Cursor::HeaderComponent {
                        sec: section_idx,
                        col: column_idx,
                        comp: component_idx,
                        items: Vec::new(),
                    },
                )
            }
            TreeRowKind::FooterComponent {
                section_idx,
                column_idx,
                component_idx,
            } => {
                let component = self
                    .site
                    .footer
                    .sections
                    .get(section_idx)
                    .and_then(|s| s.columns.get(column_idx))
                    .and_then(|c| c.components.get(component_idx))
                    .cloned();
                (
                    component,
                    cursor::Cursor::FooterComponent {
                        sec: section_idx,
                        col: column_idx,
                        comp: component_idx,
                        items: Vec::new(),
                    },
                )
            }
            TreeRowKind::Component {
                node_idx,
                column_idx,
                component_idx,
            } => {
                let page_idx = self.selected_page;
                let component = self
                    .site
                    .pages
                    .get(page_idx)
                    .and_then(|p| p.nodes.get(node_idx))
                    .and_then(|n| match n {
                        PageNode::Section(s) => Some(s),
                        _ => None,
                    })
                    .and_then(|s| s.columns.get(column_idx))
                    .and_then(|c| c.components.get(component_idx))
                    .cloned();
                (
                    component,
                    cursor::Cursor::PageComponent {
                        page: page_idx,
                        node: node_idx,
                        col: column_idx,
                        comp: component_idx,
                        items: Vec::new(),
                    },
                )
            }
            _ => return false,
        };
        let Some(component) = maybe_component else {
            return false;
        };
        let Some(state) = cursor::component_to_form_state(&component) else {
            return false;
        };
        let title = state.form.title;
        let cursor_pos = state.get(state.form.fields[state.focused_field].id).len();
        self.modal = Some(Modal::FormEdit {
            state,
            cursor: new_cursor,
            cursor_pos,
            drill_stack: Vec::new(),
            scroll_offset: 0,
        });
        self.push_toast(ToastLevel::Info, format!("Editing {}.", title));
        true
    }
    pub(super) fn try_open_form_edit_drilled_into_item(&mut self, row: &TreeRow) -> bool {
        let (node_idx, column_idx, component_idx, item_idx) = match row.kind {
            TreeRowKind::AccordionItem {
                node_idx,
                column_idx,
                component_idx,
                item_idx,
            }
            | TreeRowKind::AlternatingItem {
                node_idx,
                column_idx,
                component_idx,
                item_idx,
            }
            | TreeRowKind::CardItem {
                node_idx,
                column_idx,
                component_idx,
                item_idx,
            }
            | TreeRowKind::FilmstripItem {
                node_idx,
                column_idx,
                component_idx,
                item_idx,
            }
            | TreeRowKind::MilestonesItem {
                node_idx,
                column_idx,
                component_idx,
                item_idx,
            }
            | TreeRowKind::SliderItem {
                node_idx,
                column_idx,
                component_idx,
                item_idx,
            } => (node_idx, column_idx, component_idx, item_idx),
            _ => return false,
        };
        let page_idx = self.selected_page;
        let component = self
            .site
            .pages
            .get(page_idx)
            .and_then(|p| p.nodes.get(node_idx))
            .and_then(|n| match n {
                PageNode::Section(s) => Some(s),
                _ => None,
            })
            .and_then(|s| s.columns.get(column_idx))
            .and_then(|c| c.components.get(component_idx))
            .cloned();
        let Some(component) = component else {
            return false;
        };
        let Some(mut parent_state) = cursor::component_to_form_state(&component) else {
            return false;
        };
        // Find the SubForm field (by convention named "items"). If the
        // parent doesn't have one, give up.
        let items_field_idx = parent_state.form.fields.iter().position(|f| {
            f.id == "items" && matches!(f.kind, editform::FieldKind::SubForm { .. })
        });
        let Some(items_field_idx) = items_field_idx else {
            return false;
        };
        let items_field_id = parent_state.form.fields[items_field_idx].id.to_string();
        // Clamp item_idx into the actual sub_state list.
        let len = parent_state
            .sub_state
            .get(&items_field_id)
            .map(|v| v.len())
            .unwrap_or(0);
        if len == 0 {
            return false;
        }
        let safe_item_idx = item_idx.min(len - 1);
        parent_state.focused_field = items_field_idx;
        parent_state
            .selected_sub_item
            .insert(items_field_id.clone(), safe_item_idx);

        // Drill: replace the live item state with a placeholder, push a
        // DrillFrame, install the item state as the active modal.
        let template = match &parent_state.form.fields[items_field_idx].kind {
            editform::FieldKind::SubForm { template, .. } => *template,
            _ => return false,
        };
        let placeholder = editform::EditFormState::new(template);
        let items_vec = parent_state
            .sub_state
            .get_mut(&items_field_id)
            .expect("sub_state present for SubForm field");
        let item_state = std::mem::replace(&mut items_vec[safe_item_idx], placeholder);
        let item_cursor_pos = item_state
            .get(item_state.form.fields[item_state.focused_field].id)
            .len();

        let parent_cursor_pos = parent_state
            .get(parent_state.form.fields[parent_state.focused_field].id)
            .len();
        let mut drill_stack: Vec<DrillFrame> = Vec::new();
        drill_stack.push(DrillFrame {
            parent_state,
            parent_cursor_pos,
            parent_scroll_offset: 0,
            subform_field_id: items_field_id.clone(),
            item_idx: safe_item_idx,
        });

        let title = item_state.form.title;
        self.modal = Some(Modal::FormEdit {
            state: item_state,
            cursor: cursor::Cursor::PageComponent {
                page: page_idx,
                node: node_idx,
                col: column_idx,
                comp: component_idx,
                items: vec![],
            },
            cursor_pos: item_cursor_pos,
            drill_stack,
            scroll_offset: 0,
        });
        self.push_toast(ToastLevel::Info, format!("Editing {} (item {}).", title, safe_item_idx + 1));
        true
    }
    pub(super) fn try_open_form_edit_drilled_into_column(&mut self, row: &TreeRow) -> bool {
        let (page_idx, node_idx, sec_idx, col_idx, is_header, is_footer) = match row.kind {
            TreeRowKind::Column { node_idx, column_idx } => {
                (self.selected_page, node_idx, 0, column_idx, false, false)
            }
            TreeRowKind::HeaderColumn { section_idx, column_idx } => {
                (0, 0, section_idx, column_idx, true, false)
            }
            TreeRowKind::FooterColumn { section_idx, column_idx } => {
                (0, 0, section_idx, column_idx, false, true)
            }
            _ => return false,
        };

        let (maybe_section, base_cursor, title_prefix) = if is_header {
            let section = self.site.header.sections.get(sec_idx).cloned();
            let cur = cursor::Cursor::HeaderSection { sec: sec_idx };
            (section, cur, "dd-section (header) column")
        } else if is_footer {
            let section = self.site.footer.sections.get(sec_idx).cloned();
            let cur = cursor::Cursor::FooterSection { sec: sec_idx };
            (section, cur, "dd-section (footer) column")
        } else {
            let section = self
                .site
                .pages
                .get(page_idx)
                .and_then(|p| p.nodes.get(node_idx))
                .and_then(|n| match n {
                    PageNode::Section(s) => Some(s.clone()),
                    _ => None,
                });
            let cur = cursor::Cursor::PageSection { page: page_idx, node: node_idx };
            (section, cur, "dd-section column")
        };

        let Some(section) = maybe_section else {
            return false;
        };
        let mut parent_state = cursor::section_to_form_state(&section);

        let cols_field_idx = parent_state.form.fields.iter().position(|f| {
            f.id == "columns" && matches!(f.kind, editform::FieldKind::SubForm { .. })
        });
        let Some(cols_field_idx) = cols_field_idx else {
            return false;
        };
        let cols_field_id = parent_state.form.fields[cols_field_idx].id.to_string();
        let len = parent_state
            .sub_state
            .get(&cols_field_id)
            .map(|v: &Vec<_>| v.len())
            .unwrap_or(0);
        if col_idx >= len {
            return false;
        }
        let safe_col_idx = col_idx;
        parent_state.focused_field = cols_field_idx;
        parent_state
            .selected_sub_item
            .insert(cols_field_id.clone(), safe_col_idx);

        let col_template = match &parent_state.form.fields[cols_field_idx].kind {
            editform::FieldKind::SubForm { template, .. } => *template,
            _ => return false,
        };
        let placeholder = editform::EditFormState::new(col_template);
        let cols_vec = parent_state
            .sub_state
            .get_mut(&cols_field_id)
            .expect("sub_state present for columns SubForm field");
        let col_state = std::mem::replace(&mut cols_vec[safe_col_idx], placeholder);
        let col_cursor_pos = col_state
            .get(col_state.form.fields[col_state.focused_field].id)
            .len();

        let parent_cursor_pos = parent_state
            .get(parent_state.form.fields[parent_state.focused_field].id)
            .len();

        let mut drill_stack: Vec<DrillFrame> = Vec::new();
        drill_stack.push(DrillFrame {
            parent_state,
            parent_cursor_pos,
            parent_scroll_offset: 0,
            subform_field_id: cols_field_id.clone(),
            item_idx: safe_col_idx,
        });

        let _title = col_state.form.title;
        self.modal = Some(Modal::FormEdit {
            state: col_state,
            cursor: base_cursor,
            cursor_pos: col_cursor_pos,
            drill_stack,
            scroll_offset: 0,
        });
        self.push_toast(ToastLevel::Info, format!("Editing {} (column {}).", title_prefix, safe_col_idx + 1));
        true
    }
    pub(super) fn try_open_hero_or_section(
        &self,
        row: &TreeRow,
    ) -> Option<(editform::EditFormState, cursor::Cursor, &'static str)> {
        match row.kind {
            TreeRowKind::Hero { node_idx } => {
                let page_idx = self.selected_page;
                let node = self.site.pages.get(page_idx)?.nodes.get(node_idx)?;
                if let PageNode::Hero(hero) = node {
                    let state = cursor::hero_to_form_state(hero);
                    let cur = cursor::Cursor::PageHero {
                        page: page_idx,
                        node: node_idx,
                    };
                    Some((state, cur, "dd-hero"))
                } else {
                    None
                }
            }
            TreeRowKind::Section { node_idx } => {
                let page_idx = self.selected_page;
                let node = self.site.pages.get(page_idx)?.nodes.get(node_idx)?;
                if let PageNode::Section(section) = node {
                    let state = cursor::section_to_form_state(section);
                    let cur = cursor::Cursor::PageSection {
                        page: page_idx,
                        node: node_idx,
                    };
                    Some((state, cur, "dd-section"))
                } else {
                    None
                }
            }
            TreeRowKind::HeaderSection { section_idx } => {
                let section = self.site.header.sections.get(section_idx)?;
                let state = cursor::section_to_form_state(section);
                let cur = cursor::Cursor::HeaderSection { sec: section_idx };
                Some((state, cur, "dd-section (header)"))
            }
            TreeRowKind::FooterSection { section_idx } => {
                let section = self.site.footer.sections.get(section_idx)?;
                let state = cursor::section_to_form_state(section);
                let cur = cursor::Cursor::FooterSection { sec: section_idx };
                Some((state, cur, "dd-section (footer)"))
            }
            _ => None,
        }
    }
    pub(super) fn try_open_root(&self, row: &TreeRow) -> Option<(editform::EditFormState, cursor::Cursor, &'static str)> {
        match row.kind {
            TreeRowKind::PageHead => {
                let page_idx = self.selected_page;
                let page = self.site.pages.get(page_idx)?;
                let state = cursor::page_head_to_form_state(page);
                let cur = cursor::Cursor::PageHead { page: page_idx };
                Some((state, cur, "page-head"))
            }
            TreeRowKind::HeaderRoot { .. } => {
                let state = cursor::header_root_to_form_state(&self.site.header);
                let cur = cursor::Cursor::HeaderRoot;
                Some((state, cur, "dd-header-root"))
            }
            TreeRowKind::FooterRoot => {
                let state = cursor::footer_to_form_state(&self.site.footer);
                let cur = cursor::Cursor::FooterRoot;
                Some((state, cur, "dd-footer"))
            }
            _ => None,
        }
    }
    pub(super) fn insert_selected_component_kind(&mut self) {
        self.push_undo();
        match self.component_kind {
            ComponentKind::Hero => self.add_hero(),
            ComponentKind::Section => match self.selected_region {
                SelectedRegion::Header => self.add_header_section(),
                SelectedRegion::Footer => self.add_footer_section(),
                SelectedRegion::Page => self.add_section(),
            },
            _ => match self.selected_region {
                SelectedRegion::Header => self.add_component_to_header_section(),
                SelectedRegion::Footer => self.add_component_to_footer_section(),
                SelectedRegion::Page => self.add_selected_component_to_section(),
            },
        }
    }
    pub(super) fn add_header_section(&mut self) {
        let section = crate::model::DdSection {
            id: format!("header-section-{}", self.site.header.sections.len() + 1),
            section_title: None,
            section_class: Some(crate::model::SectionClass::FullContained),
            item_box_class: Some(crate::model::SectionItemBoxClass::LBox),
            columns: vec![SectionColumn {
                id: "column-1".to_string(),
                width_class: "dd-u-1-1".to_string(),
                components: Vec::new(),
            }],
        };
        let insert_at = (self.selected_header_section + 1).min(self.site.header.sections.len());
        self.site.header.sections.insert(insert_at, section);
        self.selected_header_section = insert_at;
        self.selected_header_column = 0;
        self.selected_header_component = 0;
        self.push_toast(ToastLevel::Info, format!(
            "Added dd-section to header at position {}.",
            self.selected_header_section + 1
        ));
    }
    pub(super) fn add_footer_section(&mut self) {
        let section = crate::model::DdSection {
            id: format!("footer-section-{}", self.site.footer.sections.len() + 1),
            section_title: None,
            section_class: Some(crate::model::SectionClass::FullContained),
            item_box_class: Some(crate::model::SectionItemBoxClass::LBox),
            columns: vec![SectionColumn {
                id: "column-1".to_string(),
                width_class: "dd-u-1-1".to_string(),
                components: Vec::new(),
            }],
        };
        let insert_at = (self.selected_header_section + 1).min(self.site.footer.sections.len());
        self.site.footer.sections.insert(insert_at, section);
        self.selected_header_section = insert_at;
        self.selected_header_column = 0;
        self.selected_header_component = 0;
        self.push_toast(
            ToastLevel::Info,
            format!(
                "Added dd-section to footer at position {}.",
                self.selected_header_section + 1
            ),
        );
    }
    pub(super) fn add_component_to_header_section(&mut self) {
        if self.site.header.sections.is_empty() {
            self.push_toast(ToastLevel::Warning, "No header section available. Add a section first with '/'.");
            return;
        }
        let section_idx = self
            .selected_header_section
            .min(self.site.header.sections.len().saturating_sub(1));
        let col_idx = self.selected_header_column.min(
            self.site.header.sections[section_idx]
                .columns
                .len()
                .saturating_sub(1),
        );
        let kind = self.component_kind;
        let component = kind.default_component();
        let col = &mut self.site.header.sections[section_idx].columns[col_idx];
        let insert_at = if col.components.is_empty() {
            0
        } else {
            (self.selected_header_component + 1).min(col.components.len())
        };
        col.components.insert(insert_at, component);
        self.selected_header_component = insert_at;
        self.push_toast(ToastLevel::Info, format!(
            "Added {} to header section column '{}'.",
            kind.label(),
            self.site.header.sections[section_idx].columns[col_idx].id
        ));
    }
    pub(super) fn add_component_to_footer_section(&mut self) {
        if self.site.footer.sections.is_empty() {
            self.push_toast(
                ToastLevel::Warning,
                "No footer section available. Add a section first with '/'.",
            );
            return;
        }
        let section_idx = self
            .selected_header_section
            .min(self.site.footer.sections.len().saturating_sub(1));
        let col_idx = self.selected_header_column.min(
            self.site.footer.sections[section_idx]
                .columns
                .len()
                .saturating_sub(1),
        );
        let kind = self.component_kind;
        let component = kind.default_component();
        let col = &mut self.site.footer.sections[section_idx].columns[col_idx];
        let insert_at = if col.components.is_empty() {
            0
        } else {
            (self.selected_header_component + 1).min(col.components.len())
        };
        col.components.insert(insert_at, component);
        self.selected_header_component = insert_at;
        self.push_toast(
            ToastLevel::Info,
            format!(
                "Added {} to footer section column '{}'.",
                kind.label(),
                self.site.footer.sections[section_idx].columns[col_idx].id
            ),
        );
    }
    pub(super) fn normalize_component_picker_selection(&mut self) {
        let (query, selected) = match &self.modal {
            Some(Modal::ComponentPicker { query, selected }) => (query.clone(), *selected),
            _ => return,
        };
        let total = self.filtered_component_kinds(&query).len();
        if let Some(Modal::ComponentPicker { selected: sel, .. }) = &mut self.modal {
            *sel = if total == 0 {
                0
            } else {
                selected.min(total - 1)
            };
        }
    }
    pub(super) fn filtered_component_kinds(&self, query: &str) -> Vec<ComponentKind> {
        let all = ComponentKind::all();
        let in_header = self.selected_region == SelectedRegion::Header;
        // Gate header-only components: only show dd-header-search/dd-header-menu when in header region.
        let allowed: Vec<ComponentKind> = all
            .iter()
            .copied()
            .filter(|k| match k {
                ComponentKind::HeaderSearch | ComponentKind::HeaderMenu => in_header,
                _ => true,
            })
            .collect();
        let q = query.trim().to_ascii_lowercase();
        if q.is_empty() {
            return allowed;
        }
        let mut scored = Vec::new();
        for kind in allowed.iter().copied() {
            let hay = component_search_haystack(kind);
            if let Some(score) = fuzzy_score(&q, hay.as_str()) {
                scored.push((kind, score));
            }
        }
        scored.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.label().cmp(b.0.label())));
        scored.into_iter().map(|(kind, _)| kind).collect()
    }
    pub(super) fn selection_summary(&self) -> String {
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
    pub(super) fn header_selection_summary(&self) -> String {
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
    pub(super) fn selected_component_owned(&self) -> Option<crate::model::SectionComponent> {
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
    pub(super) fn select_prev(&mut self) {
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
    pub(super) fn select_next(&mut self) {
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
    pub(super) fn handle_up(&mut self) {
        match self.selected_sidebar_section {
            SidebarSection::Regions => {
                self.selected_region = SelectedRegion::Header;
                self.selected_tree_row = 0;
                self.push_toast(ToastLevel::Info, "Selected Header region.");
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
        }
    }
    pub(super) fn handle_down(&mut self) {
        match self.selected_sidebar_section {
            SidebarSection::Regions => {
                self.selected_region = SelectedRegion::Footer;
                self.selected_tree_row = 0;
                self.push_toast(ToastLevel::Info, "Selected Footer region.");
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
        }
    }
    pub(super) fn vim_jump_to_first_row(&mut self) {
        let rows = self.build_tree_rows();
        if rows.is_empty() {
            return;
        }
        self.selected_tree_row = 0;
        self.apply_tree_row_selection(rows[0]);
        self.details_scroll_row = 0;
    }
    pub(super) fn vim_jump_to_last_row(&mut self) {
        let rows = self.build_tree_rows();
        if rows.is_empty() {
            return;
        }
        let last = rows.len() - 1;
        self.selected_tree_row = last;
        self.apply_tree_row_selection(rows[last]);
        self.details_scroll_row = 0;
    }
    pub(super) fn vim_collapse_selected_row(&mut self) {
        let rows = self.build_tree_rows();
        if rows.is_empty() {
            return;
        }
        let row = rows[self.selected_tree_row.min(rows.len() - 1)];
        if self.tree_row_is_expanded(&row) {
            self.toggle_selected_tree_expanded();
        }
    }
    pub(super) fn vim_expand_selected_row(&mut self) {
        let rows = self.build_tree_rows();
        if rows.is_empty() {
            return;
        }
        let row = rows[self.selected_tree_row.min(rows.len() - 1)];
        if !self.tree_row_is_expanded(&row) {
            self.toggle_selected_tree_expanded();
        }
    }
    pub(super) fn tree_row_is_expanded(&self, row: &TreeRow) -> bool {
        match row.kind {
            TreeRowKind::Section { node_idx } => self.is_section_expanded(node_idx),
            TreeRowKind::HeaderSection { section_idx } => {
                self.is_header_section_expanded(section_idx)
            }
            _ => false,
        }
    }
    pub(super) fn select_next_page(&mut self) {
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
    pub(super) fn select_prev_page(&mut self) {
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
    pub(super) fn add_hero(&mut self) {
        let selected = self.selected_node;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        let hero = crate::model::DdHero {
            parent_image_url: "/assets/images/hero-new.jpg".to_string(),
            parent_class: Some(crate::model::HeroImageClass::FullFull),
            parent_data_aos: Some(crate::model::HeroAos::FadeIn),
            parent_custom_css: None,
            parent_title: "New Hero".to_string(),
            parent_subtitle: "Add subtitle".to_string(),
            parent_copy: None,
            link_1_label: None,
            link_1_url: None,
            link_1_target: Some(crate::model::CtaTarget::SelfTarget),
            link_2_label: None,
            link_2_url: None,
            link_2_target: Some(crate::model::CtaTarget::SelfTarget),
            parent_image_alt: Some("Hero image".to_string()),
            parent_image_mobile: None,
            parent_image_tablet: None,
            parent_image_desktop: None,
            parent_image_class: Some(crate::model::HeroImageClass::FullFull),
        };
        let idx = Self::selected_index_for_page(page, selected)
            .map(|v| v + 1)
            .unwrap_or(0);
        page.nodes.insert(idx, PageNode::Hero(hero));
        self.selected_node = idx;
        self.selected_column = 0;
        self.selected_component = 0;
        self.selected_nested_item = 0;
        self.push_toast(ToastLevel::Success, format!("Inserted dd-hero at position {}.", idx + 1));
    }
    pub(super) fn add_section(&mut self) {
        let selected = self.selected_node;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        let next_id = next_section_id_for_page(page);
        let section = crate::model::DdSection {
            id: next_id,
            section_title: None,
            section_class: Some(crate::model::SectionClass::FullContained),
            item_box_class: Some(crate::model::SectionItemBoxClass::LBox),
            columns: vec![SectionColumn {
                id: "column-1".to_string(),
                width_class: "dd-u-1-1".to_string(),
                components: Vec::new(),
            }],
        };
        let idx = Self::selected_index_for_page(page, selected)
            .map(|v| v + 1)
            .unwrap_or(0);
        page.nodes.insert(idx, PageNode::Section(section));
        self.selected_node = idx;
        self.selected_column = 0;
        self.selected_component = 0;
        self.selected_nested_item = 0;
        self.push_toast(ToastLevel::Success, format!("Inserted dd-section at position {}.", idx + 1));
    }
    pub(super) fn delete_selected_node(&mut self) {
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
    pub(super) fn selected_tree_row_kind(&self) -> Option<TreeRowKind> {
        let rows = self.build_tree_rows();
        if rows.is_empty() {
            return None;
        }
        Some(rows[self.selected_tree_row.min(rows.len() - 1)].kind)
    }
    pub(super) fn push_undo(&mut self) {
        self.undo_stack.push(self.site.clone());
        if self.undo_stack.len() > 20 {
            self.undo_stack.remove(0);
        }
    }
    pub(super) fn undo_last(&mut self) {
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
    pub(super) fn request_quit(&mut self) {
        if self.dirty {
            self.modal = Some(Modal::ConfirmPrompt {
                message: "Unsaved changes. Quit anyway? y/n".to_string(),
                on_confirm: ConfirmKind::QuitUnsaved,
            });
        } else {
            self.should_quit = true;
        }
    }
    pub(super) fn delete_selected_row(&mut self) {
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
    pub(super) fn delete_page_component(&mut self, node_idx: usize, column_idx: usize, component_idx: usize) {
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
    pub(super) fn delete_header_component(
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
    pub(super) fn delete_footer_component(
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
    pub(super) fn duplicate_selected_row(&mut self) {
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
    pub(super) fn duplicate_selected_collection_item(&mut self) -> bool {
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
    pub(super) fn move_selected_row(&mut self, delta: isize) {
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
    pub(super) fn move_region_component(
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
    pub(super) fn move_selected_collection_item(&mut self, delta: isize) -> bool {
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
    pub(super) fn add_selected_component_to_section(&mut self) {
        let kind = self.component_kind;
        if matches!(kind, ComponentKind::Hero | ComponentKind::Section) {
            self.push_toast(ToastLevel::Warning, "dd-hero and dd-section are top-level insert types.");
            return;
        }
        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
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
                let col_i = selected_column.min(section.columns.len().saturating_sub(1));
                let components = &mut section.columns[col_i].components;
                let inserted = kind.default_component();
                let insert_at = if components.is_empty() {
                    0
                } else {
                    (selected_component + 1).min(components.len())
                };
                components.insert(insert_at, inserted);
                (
                    Some(insert_at),
                    format!(
                        "Added {} to selected section column '{}'.",
                        kind.label(),
                        section.columns[col_i].id
                    ),
                )
            }
            _ => (None, "Selected node is not a section.".to_string()),
        };
        if let Some(new_selected_component) = result.0 {
            self.selected_component = new_selected_component;
            self.selected_nested_item = 0;
        }
        self.push_toast(ToastLevel::Info, result.1);
    }
    pub(super) fn add_selected_collection_item(&mut self) {
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
    pub(super) fn remove_selected_collection_item(&mut self) {
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
    pub(super) fn add_selected_accordion_item(&mut self) {
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
    pub(super) fn remove_selected_accordion_item(&mut self) {
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
    pub(super) fn add_selected_alternating_item(&mut self) {
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
    pub(super) fn remove_selected_alternating_item(&mut self) {
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
    pub(super) fn add_selected_card_item(&mut self) {
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
    pub(super) fn remove_selected_card_item(&mut self) {
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
    pub(super) fn add_selected_filmstrip_item(&mut self) {
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
    pub(super) fn remove_selected_filmstrip_item(&mut self) {
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
    pub(super) fn add_selected_milestones_item(&mut self) {
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
    pub(super) fn remove_selected_milestones_item(&mut self) {
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
    pub(super) fn add_selected_slider_item(&mut self) {
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
    pub(super) fn remove_selected_slider_item(&mut self) {
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
    pub(super) fn mutate_selected_section<F>(&mut self, mutator: F, success_message: &str)
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
    pub(super) fn add_column(&mut self) {
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
    pub(super) fn add_column_to_header_section(&mut self) {
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
    pub(super) fn remove_selected_column(&mut self) {
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
    pub(super) fn remove_column_from_header_section(&mut self) {
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
    pub(super) fn select_prev_column(&mut self) {
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
    pub(super) fn select_next_column(&mut self) {
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
    pub(super) fn selected_header_section_column_total(&self) -> Option<usize> {
        if self.site.header.sections.is_empty() {
            return None;
        }
        let section_idx = self
            .selected_header_section
            .min(self.site.header.sections.len().saturating_sub(1));
        Some(self.site.header.sections[section_idx].columns.len())
    }
    pub(super) fn move_selected_column_up(&mut self) {
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
    pub(super) fn move_selected_column_down(&mut self) {
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
    pub(super) fn snap_tree_row_to_column(&mut self, node_idx: usize, column_idx: usize) {
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
    pub(super) fn snap_tree_row_to_header_column(&mut self, section_idx: usize, column_idx: usize) {
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
    pub(super) fn selected_section_column_total(&mut self) -> Option<usize> {
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

    pub(super) fn begin_edit_selected_column_id(&mut self) {
        self.open_column_form_edit("id");
    }

    pub(super) fn begin_edit_selected_column_width_class(&mut self) {
        self.open_column_form_edit("width_class");
    }

    pub(super) fn open_column_form_edit(&mut self, focus_id: &str) {
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
