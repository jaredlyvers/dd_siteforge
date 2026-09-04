//! Enter / FormEdit openers and insert-picker helpers.
use super::super::*;

impl App {
    pub(in crate::tui) fn handle_enter_on_selected_row(&mut self) {
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

    pub(in crate::tui) fn open_component_picker(&mut self) {
        self.modal = Some(Modal::ComponentPicker {
            query: String::new(),
            selected: 0,
        });
    }

    pub(in crate::tui) fn try_open_form_edit(&mut self, row: &TreeRow) -> bool {
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

    pub(in crate::tui) fn try_open_form_edit_drilled_into_item(&mut self, row: &TreeRow) -> bool {
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

    pub(in crate::tui) fn try_open_form_edit_drilled_into_column(&mut self, row: &TreeRow) -> bool {
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

    pub(in crate::tui) fn try_open_hero_or_section(
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

    pub(in crate::tui) fn try_open_root(&self, row: &TreeRow) -> Option<(editform::EditFormState, cursor::Cursor, &'static str)> {
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

    pub(in crate::tui) fn insert_selected_component_kind(&mut self) {
        if matches!(self.component_kind, ComponentKind::Hero)
            && self.selected_region != SelectedRegion::Page
        {
            self.push_toast(
                ToastLevel::Warning,
                "dd-hero can only be inserted on a page.",
            );
            return;
        }
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

    pub(in crate::tui) fn add_header_section(&mut self) {
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

    pub(in crate::tui) fn add_footer_section(&mut self) {
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

    pub(in crate::tui) fn add_component_to_header_section(&mut self) {
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

    pub(in crate::tui) fn add_component_to_footer_section(&mut self) {
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

    pub(in crate::tui) fn normalize_component_picker_selection(&mut self) {
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

    pub(in crate::tui) fn filtered_component_kinds(&self, query: &str) -> Vec<ComponentKind> {
        let all = ComponentKind::all();
        // Hero is page-only; HeaderSearch/HeaderMenu are header-only. Details uses selected_region.
        let allowed: Vec<ComponentKind> = all
            .iter()
            .copied()
            .filter(|k| match k {
                ComponentKind::Hero => self.selected_region == SelectedRegion::Page,
                ComponentKind::HeaderSearch | ComponentKind::HeaderMenu => {
                    self.selected_region == SelectedRegion::Header
                }
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

    pub(in crate::tui) fn add_hero(&mut self) {
        let selected = self.selected_node;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        let hero = crate::model::DdHero {
            parent_image_url: "/assets/images/hero-new.jpg".to_string(),
            parent_class: Some(crate::model::HeroImageClass::FullFull),
            sal: Some(crate::model::SalAnimation::Fade),
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

    pub(in crate::tui) fn add_section(&mut self) {
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

    pub(in crate::tui) fn add_selected_component_to_section(&mut self) {
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
}
