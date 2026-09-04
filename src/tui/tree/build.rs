//! Build the layout tree and keep selection in sync.
use super::super::*;

impl App {
    pub(in crate::tui) fn build_tree_rows(&self) -> Vec<TreeRow> {
        match self.selected_region {
            SelectedRegion::Site => self.build_site_tree_rows(),
            SelectedRegion::Header => self.build_header_tree_rows(),
            SelectedRegion::Footer => self.build_footer_tree_rows(),
            SelectedRegion::Page => self.build_page_tree_rows(),
        }
    }

    pub(in crate::tui) fn build_site_tree_rows(&self) -> Vec<TreeRow> {
        vec![TreeRow {
            kind: TreeRowKind::SiteRoot,
        }]
    }

    pub(in crate::tui) fn build_footer_tree_rows(&self) -> Vec<TreeRow> {
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

    pub(in crate::tui) fn build_page_tree_rows(&self) -> Vec<TreeRow> {
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

    pub(in crate::tui) fn build_header_tree_rows(&self) -> Vec<TreeRow> {
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

    pub(in crate::tui) fn tree_row_label(&self, row: &TreeRow) -> String {
        match &row.kind {
            TreeRowKind::SiteRoot => "Site settings".to_string(),
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

    pub(in crate::tui) fn apply_tree_row_selection(&mut self, row: TreeRow) {
        self.page_head_selected = matches!(row.kind, TreeRowKind::PageHead);
        match row.kind {
            TreeRowKind::SiteRoot => {}
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

    pub(in crate::tui) fn sync_tree_row_with_selection(&mut self) {
        let rows = self.build_tree_rows();
        if rows.is_empty() {
            self.selected_tree_row = 0;
            return;
        }
        let row_matches_selection = |row: &TreeRow| match row.kind {
            TreeRowKind::SiteRoot => true,
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
}
