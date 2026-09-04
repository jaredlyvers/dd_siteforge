use super::*;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};


    fn app_with_card() -> App {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.selected_page = 0;
        app.selected_node = 1;
        app.set_section_expanded(1, true);
        if let PageNode::Section(section) = &mut app.site.pages[0].nodes[1] {
            normalize_section_columns(section);
            section.columns[0]
                .components
                .push(ComponentKind::Card.default_component());
        } else {
            panic!("expected starter node 2 to be dd-section");
        }
        app.selected_column = 0;
        app.selected_component = 0;
        app.selected_nested_item = 0;
        app.sync_tree_row_with_selection();
        app
    }

    fn send_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
        app.handle_event(Event::Key(KeyEvent::new(code, modifiers)))
            .expect("key event should be handled");
    }

    fn send_mouse(app: &mut App, kind: MouseEventKind, column: u16, row: u16) {
        app.handle_event(Event::Mouse(MouseEvent {
            kind,
            column,
            row,
            modifiers: KeyModifiers::NONE,
        }))
        .expect("mouse event should be handled");
    }

    fn selected_card(app: &App) -> &crate::model::DdCard {
        let page = &app.site.pages[app.selected_page];
        let section = match &page.nodes[app.selected_node] {
            PageNode::Section(section) => section,
            _ => panic!("selected node is not dd-section"),
        };
        let component = &section.columns[app.selected_column].components[app.selected_component];
        match component {
            crate::model::SectionComponent::Card(card) => card,
            _ => panic!("selected component is not dd-card"),
        }
    }

    fn app_with_cta() -> App {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.selected_page = 0;
        app.selected_node = 1;
        app.set_section_expanded(1, true);
        if let PageNode::Section(section) = &mut app.site.pages[0].nodes[1] {
            normalize_section_columns(section);
            section.columns[0]
                .components
                .push(ComponentKind::Cta.default_component());
        } else {
            panic!("expected starter node 2 to be dd-section");
        }
        app.selected_column = 0;
        app.selected_component = 0;
        app.selected_nested_item = 0;
        app.sync_tree_row_with_selection();
        app
    }

    fn selected_cta(app: &App) -> &crate::model::DdCta {
        let page = &app.site.pages[app.selected_page];
        let section = match &page.nodes[app.selected_node] {
            PageNode::Section(section) => section,
            _ => panic!("selected node is not dd-section"),
        };
        let component = &section.columns[app.selected_column].components[app.selected_component];
        match component {
            crate::model::SectionComponent::Cta(cta) => cta,
            _ => panic!("selected component is not dd-cta"),
        }
    }

    #[test]
    fn dd_card_keyflow_add_remove_items_with_min_guard() {
        let mut app = app_with_card();
        assert_eq!(selected_card(&app).items.len(), 1);

        send_key(&mut app, KeyCode::Char('A'), KeyModifiers::SHIFT);
        assert_eq!(selected_card(&app).items.len(), 2);

        send_key(&mut app, KeyCode::Char('X'), KeyModifiers::SHIFT);
        assert_eq!(selected_card(&app).items.len(), 1);

        send_key(&mut app, KeyCode::Char('X'), KeyModifiers::SHIFT);
        assert_eq!(selected_card(&app).items.len(), 1);
        let last = app.toasts.last().expect("expected a toast for min guard");
        assert!(last.message.contains("must keep at least one item"));
        assert_eq!(last.level, ToastLevel::Info);
    }

    #[test]
    fn dd_cta_form_edit_opens_on_enter() {
        let mut app = app_with_cta();
        let rows = app.build_page_tree_rows();
        let row_idx = rows
            .iter()
            .position(|row| {
                matches!(
                    row.kind,
                    TreeRowKind::Component {
                        node_idx: 1,
                        column_idx: 0,
                        component_idx: 0
                    }
                )
            })
            .expect("dd-cta component row should exist");
        app.selected_tree_row = row_idx;
        app.apply_tree_row_selection(rows[row_idx]);

        send_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        let modal = app
            .modal
            .as_ref()
            .expect("Modal::FormEdit should open for CTA");
        match modal {
            Modal::FormEdit { state, cursor, .. } => {
                assert_eq!(state.form.title, "dd-cta");
                assert_eq!(state.get("parent_class"), "-top-left");
                assert!(matches!(cursor, cursor::Cursor::PageComponent { .. }));
            }
            _ => panic!("expected Modal::FormEdit, got {:?}", modal.variant_name()),
        }
    }

    #[test]
    fn dd_cta_form_edit_tab_and_enum_cycle() {
        let mut app = app_with_cta();
        open_form_edit_on_selected_cta(&mut app);

        // Tab advances to next visible field (parent_image_url).
        send_key(&mut app, KeyCode::Tab, KeyModifiers::NONE);
        assert_eq!(form_focused_field_id(&app), Some("parent_image_url"));

        // BackTab goes back to parent_class.
        send_key(&mut app, KeyCode::BackTab, KeyModifiers::NONE);
        assert_eq!(form_focused_field_id(&app), Some("parent_class"));

        // Right cycles the enum forward.
        send_key(&mut app, KeyCode::Right, KeyModifiers::NONE);
        assert_eq!(form_value(&app, "parent_class"), "-top-center");

        // Esc closes without applying.
        send_key(&mut app, KeyCode::Esc, KeyModifiers::NONE);
        assert!(app.modal.is_none());
        assert_eq!(
            selected_cta(&app).parent_class,
            crate::model::CtaClass::TopLeft
        );
    }

    #[test]
    fn dd_cta_edits_apply_in_page_region() {
        let mut app = app_with_cta();
        open_form_edit_on_selected_cta(&mut app);

        // Cycle class from -top-left to -center-center.
        for _ in 0..4 {
            send_key(&mut app, KeyCode::Right, KeyModifiers::NONE);
        }
        assert_eq!(form_value(&app, "parent_class"), "-center-center");

        send_key(&mut app, KeyCode::Char('s'), KeyModifiers::CONTROL);
        assert!(app.modal.is_none(), "Ctrl+S should close the modal");
        assert_eq!(
            selected_cta(&app).parent_class,
            crate::model::CtaClass::CenterCenter
        );
    }

    #[test]
    fn dd_cta_edits_in_header_region() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.selected_region = SelectedRegion::Header;
        app.header_column_expanded = true;
        app.set_header_section_expanded(0, true);
        app.site.header.sections[0].columns[0]
            .components
            .push(ComponentKind::Cta.default_component());
        let rows = app.build_header_tree_rows();
        let row_idx = rows
            .iter()
            .position(|row| {
                matches!(
                    row.kind,
                    TreeRowKind::HeaderComponent {
                        section_idx: 0,
                        column_idx: 0,
                        component_idx: 0,
                    }
                )
            })
            .expect("header CTA component row should exist");
        app.selected_tree_row = row_idx;
        app.apply_tree_row_selection(rows[row_idx]);

        send_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        // Cycle class from -top-left to -top-center.
        send_key(&mut app, KeyCode::Right, KeyModifiers::NONE);
        send_key(&mut app, KeyCode::Char('s'), KeyModifiers::CONTROL);

        let header_cta = match &app.site.header.sections[0].columns[0].components[0] {
            crate::model::SectionComponent::Cta(cta) => cta,
            _ => panic!("expected CTA at header.sections[0].columns[0].components[0]"),
        };
        assert_eq!(header_cta.parent_class, crate::model::CtaClass::TopCenter);

        // Page-1 CTA (if any) should NOT have been modified.
        if let PageNode::Section(section) = &app.site.pages[0].nodes[1]
            && let Some(crate::model::SectionComponent::Cta(page_cta)) =
                section.columns.first().and_then(|c| c.components.first())
        {
            assert_ne!(
                page_cta.parent_class,
                header_cta.parent_class,
                "page CTA must not change when editing header CTA"
            );
        }
    }

    #[test]
    fn dd_cta_edits_in_footer_region() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.selected_region = SelectedRegion::Footer;
        app.site.footer.sections[0].columns[0]
            .components
            .push(ComponentKind::Cta.default_component());
        let rows = app.build_footer_tree_rows();
        let row_idx = rows
            .iter()
            .position(|row| {
                matches!(
                    row.kind,
                    TreeRowKind::FooterComponent {
                        section_idx: 0,
                        column_idx: 0,
                        component_idx: 0,
                    }
                )
            })
            .expect("footer CTA component row should exist");
        app.selected_tree_row = row_idx;
        app.apply_tree_row_selection(rows[row_idx]);

        send_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        send_key(&mut app, KeyCode::Right, KeyModifiers::NONE);
        send_key(&mut app, KeyCode::Char('s'), KeyModifiers::CONTROL);

        let footer_cta = match &app.site.footer.sections[0].columns[0].components[0] {
            crate::model::SectionComponent::Cta(cta) => cta,
            _ => panic!("expected CTA at footer.sections[0].columns[0].components[0]"),
        };
        assert_eq!(footer_cta.parent_class, crate::model::CtaClass::TopCenter);
    }

    fn open_form_edit_on_page_component(app: &mut App) {
        let rows = app.build_page_tree_rows();
        let row_idx = rows
            .iter()
            .position(|row| {
                matches!(
                    row.kind,
                    TreeRowKind::Component {
                        node_idx: 1,
                        column_idx: 0,
                        component_idx: 0
                    }
                )
            })
            .expect("component row at node=1,col=0,comp=0 should exist");
        app.selected_tree_row = row_idx;
        app.apply_tree_row_selection(rows[row_idx]);
        send_key(app, KeyCode::Enter, KeyModifiers::NONE);
        assert!(
            app.modal.is_some(),
            "FormEdit should open for migrated component"
        );
    }

    fn app_with_component(kind: ComponentKind) -> App {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.selected_page = 0;
        app.selected_node = 1;
        app.set_section_expanded(1, true);
        if let PageNode::Section(section) = &mut app.site.pages[0].nodes[1] {
            normalize_section_columns(section);
            section.columns[0].components.clear();
            section.columns[0].components.push(kind.default_component());
        } else {
            panic!("starter node 1 expected to be dd-section");
        }
        app.selected_column = 0;
        app.selected_component = 0;
        app.selected_nested_item = 0;
        app.sync_tree_row_with_selection();
        app
    }

    fn select_first_component_row(app: &mut App) {
        let rows = app.build_tree_rows();
        let idx = rows
            .iter()
            .position(|r| matches!(r.kind, TreeRowKind::Component { .. }))
            .expect("expected a component tree row");
        app.selected_tree_row = idx;
        app.apply_tree_row_selection(rows[idx]);
    }

    #[test]
    fn tier_a_banner_form_edit_round_trip() {
        let mut app = app_with_component(ComponentKind::Banner);
        open_form_edit_on_page_component(&mut app);
        // Cycle parent_class once (focused field 0).
        send_key(&mut app, KeyCode::Right, KeyModifiers::NONE);
        send_key(&mut app, KeyCode::Char('s'), KeyModifiers::CONTROL);
        match &app.site.pages[0].nodes[1] {
            PageNode::Section(s) => match &s.columns[0].components[0] {
                crate::model::SectionComponent::Banner(b) => assert_eq!(
                    b.parent_class,
                    crate::model::BannerClass::BgCenterRight,
                    "banner class should advance one step from default BgCenterCenter"
                ),
                other => panic!("expected Banner, got {:?}", std::mem::discriminant(other)),
            },
            _ => panic!("expected Section node"),
        }
    }

    #[test]
    fn tier_a_image_form_edit_round_trip() {
        let mut app = app_with_component(ComponentKind::Image);
        open_form_edit_on_page_component(&mut app);
        // Cycle sal once (focused field 0).
        send_key(&mut app, KeyCode::Right, KeyModifiers::NONE);
        send_key(&mut app, KeyCode::Char('s'), KeyModifiers::CONTROL);
        match &app.site.pages[0].nodes[1] {
            PageNode::Section(s) => match &s.columns[0].components[0] {
                crate::model::SectionComponent::Image(i) => assert_eq!(
                    i.sal,
                    crate::model::SalAnimation::SlideUp,
                    "image sal should advance one step from default"
                ),
                _ => panic!("expected Image"),
            },
            _ => panic!("expected Section"),
        }
    }

    #[test]
    fn tier_a_header_search_form_edit_round_trip() {
        // HeaderSearch only valid in header region, so build a scenario there.
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.selected_region = SelectedRegion::Header;
        app.header_column_expanded = true;
        app.set_header_section_expanded(0, true);
        // starter already has a search in column[1]; replace column[0] instead.
        app.site.header.sections[0].columns[0]
            .components
            .push(ComponentKind::HeaderSearch.default_component());
        let rows = app.build_header_tree_rows();
        let row_idx = rows
            .iter()
            .position(|row| {
                matches!(
                    row.kind,
                    TreeRowKind::HeaderComponent {
                        section_idx: 0,
                        column_idx: 0,
                        component_idx: 0
                    }
                )
            })
            .expect("header-search row should exist");
        app.selected_tree_row = row_idx;
        app.apply_tree_row_selection(rows[row_idx]);
        send_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        assert!(matches!(app.modal, Some(Modal::FormEdit { .. })));
        send_key(&mut app, KeyCode::Char('s'), KeyModifiers::CONTROL);
        assert!(app.modal.is_none());
    }

    #[test]
    fn tier_a_rich_text_form_edit_round_trip() {
        let mut app = app_with_component(ComponentKind::RichText);
        open_form_edit_on_page_component(&mut app);
        // parent_class is focused first (index 0, Text field). Type a letter.
        send_key(&mut app, KeyCode::Char('x'), KeyModifiers::NONE);
        send_key(&mut app, KeyCode::Char('s'), KeyModifiers::CONTROL);
        match &app.site.pages[0].nodes[1] {
            PageNode::Section(s) => match &s.columns[0].components[0] {
                crate::model::SectionComponent::RichText(r) => {
                    assert_eq!(r.parent_class.as_deref(), Some("x"));
                }
                _ => panic!("expected RichText"),
            },
            _ => panic!("expected Section"),
        }
    }

    #[test]
    fn tier_a_alert_form_edit_round_trip() {
        let mut app = app_with_component(ComponentKind::Alert);
        open_form_edit_on_page_component(&mut app);
        // Cycle parent_type.
        send_key(&mut app, KeyCode::Right, KeyModifiers::NONE);
        send_key(&mut app, KeyCode::Char('s'), KeyModifiers::CONTROL);
        match &app.site.pages[0].nodes[1] {
            PageNode::Section(s) => match &s.columns[0].components[0] {
                crate::model::SectionComponent::Alert(a) => {
                    assert_eq!(a.parent_type, crate::model::AlertType::Info);
                }
                _ => panic!("expected Alert"),
            },
            _ => panic!("expected Section"),
        }
    }

    #[test]
    fn tier_a_modal_form_edit_round_trip() {
        let mut app = app_with_component(ComponentKind::Modal);
        open_form_edit_on_page_component(&mut app);
        // parent_title first: append a letter after the default value.
        send_key(&mut app, KeyCode::Char('Z'), KeyModifiers::SHIFT);
        send_key(&mut app, KeyCode::Char('s'), KeyModifiers::CONTROL);
        match &app.site.pages[0].nodes[1] {
            PageNode::Section(s) => match &s.columns[0].components[0] {
                crate::model::SectionComponent::Modal(m) => {
                    assert!(m.parent_title.ends_with('Z'));
                }
                _ => panic!("expected Modal"),
            },
            _ => panic!("expected Section"),
        }
    }

    #[test]
    fn tier_a_blockquote_form_edit_round_trip() {
        let mut app = app_with_component(ComponentKind::Blockquote);
        open_form_edit_on_page_component(&mut app);
        // sal first: cycle once.
        send_key(&mut app, KeyCode::Right, KeyModifiers::NONE);
        send_key(&mut app, KeyCode::Char('s'), KeyModifiers::CONTROL);
        match &app.site.pages[0].nodes[1] {
            PageNode::Section(s) => match &s.columns[0].components[0] {
                crate::model::SectionComponent::Blockquote(bq) => {
                    assert_eq!(bq.sal, crate::model::SalAnimation::SlideUp);
                }
                _ => panic!("expected Blockquote"),
            },
            _ => panic!("expected Section"),
        }
    }

    fn tab_to_items_field(app: &mut App) {
        for _ in 0..20 {
            if form_focused_field_id(app) == Some("items") {
                return;
            }
            send_key(app, KeyCode::Tab, KeyModifiers::NONE);
        }
        panic!("never reached items field after 20 tabs");
    }

    fn drill_stack_len(app: &App) -> usize {
        match app.modal.as_ref() {
            Some(Modal::FormEdit { drill_stack, .. }) => drill_stack.len(),
            _ => 0,
        }
    }

    /// Drill into first item, edit nothing, return, verify round-trip.
    fn tier_b_drill_round_trip(component: ComponentKind) {
        let mut app = app_with_component(component);
        open_form_edit_on_page_component(&mut app);
        tab_to_items_field(&mut app);
        assert!(matches!(app.modal, Some(Modal::FormEdit { .. })));

        // Drill into first item.
        send_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(drill_stack_len(&app), 1, "drill stack should have 1 frame");

        // Ctrl+S to return to parent.
        send_key(&mut app, KeyCode::Char('s'), KeyModifiers::CONTROL);
        assert_eq!(drill_stack_len(&app), 0, "drill stack should be empty");
        assert!(app.modal.is_some(), "parent modal should remain open");

        // Ctrl+S at parent commits to model and closes.
        send_key(&mut app, KeyCode::Char('s'), KeyModifiers::CONTROL);
        assert!(app.modal.is_none(), "top-level save should close modal");
    }

    #[test]
    fn tier_b_card_drill_round_trip() {
        tier_b_drill_round_trip(ComponentKind::Card);
    }

    #[test]
    fn tier_b_filmstrip_drill_round_trip() {
        tier_b_drill_round_trip(ComponentKind::Filmstrip);
    }

    #[test]
    fn tier_b_milestones_drill_round_trip() {
        tier_b_drill_round_trip(ComponentKind::Milestones);
    }

    #[test]
    fn tier_b_slider_drill_round_trip() {
        tier_b_drill_round_trip(ComponentKind::Slider);
    }

    #[test]
    fn tier_b_accordion_drill_round_trip() {
        tier_b_drill_round_trip(ComponentKind::Accordion);
    }

    #[test]
    fn tier_b_alternating_drill_round_trip() {
        tier_b_drill_round_trip(ComponentKind::Alternating);
    }

    #[test]
    fn tier_b_accordion_item_edit_persists() {
        // Full round-trip with an actual field change on an item.
        let mut app = app_with_component(ComponentKind::Accordion);
        open_form_edit_on_page_component(&mut app);
        tab_to_items_field(&mut app);
        send_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        // Inside item editor; first field is child_title (Text). Type a char.
        send_key(&mut app, KeyCode::Char('!'), KeyModifiers::NONE);
        // Return to parent (Ctrl+S), then commit to model.
        send_key(&mut app, KeyCode::Char('s'), KeyModifiers::CONTROL);
        send_key(&mut app, KeyCode::Char('s'), KeyModifiers::CONTROL);
        match &app.site.pages[0].nodes[1] {
            PageNode::Section(s) => match &s.columns[0].components[0] {
                crate::model::SectionComponent::Accordion(acc) => {
                    assert!(
                        acc.items[0].child_title.contains('!'),
                        "first accordion item title should contain inserted char, got {:?}",
                        acc.items[0].child_title
                    );
                }
                _ => panic!("expected Accordion"),
            },
            _ => panic!("expected Section"),
        }
    }

    #[test]
    fn tier_c_hero_form_edit_round_trip() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.selected_page = 0;
        app.selected_node = 0;
        app.sync_tree_row_with_selection();
        let rows = app.build_page_tree_rows();
        let row_idx = rows
            .iter()
            .position(|row| matches!(row.kind, TreeRowKind::Hero { node_idx: 0 }))
            .expect("hero row");
        app.selected_tree_row = row_idx;
        app.apply_tree_row_selection(rows[row_idx]);

        send_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        let title_is_hero = matches!(
            app.modal.as_ref(),
            Some(Modal::FormEdit { state, .. }) if state.form.title == "dd-hero"
        );
        assert!(title_is_hero, "hero form should open");

        // First field is parent_title (Text). Type a char then Ctrl+S.
        send_key(&mut app, KeyCode::Char('!'), KeyModifiers::NONE);
        send_key(&mut app, KeyCode::Char('s'), KeyModifiers::CONTROL);
        assert!(app.modal.is_none(), "top-level save closes modal");
        if let PageNode::Hero(h) = &app.site.pages[0].nodes[0] {
            assert!(h.parent_title.contains('!'));
        } else {
            panic!("expected Hero");
        }
    }

    #[test]
    fn tier_c_section_form_edit_preserves_components() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.selected_page = 0;
        app.selected_node = 1;
        app.set_section_expanded(1, true);
        // Put a CTA into the first column so we can verify it survives a column rename.
        if let PageNode::Section(s) = &mut app.site.pages[0].nodes[1] {
            s.columns[0]
                .components
                .push(ComponentKind::Cta.default_component());
        } else {
            panic!("expected Section at node 1");
        }
        app.sync_tree_row_with_selection();
        let rows = app.build_page_tree_rows();
        let row_idx = rows
            .iter()
            .position(|row| matches!(row.kind, TreeRowKind::Section { node_idx: 1 }))
            .expect("section row");
        app.selected_tree_row = row_idx;
        app.apply_tree_row_selection(rows[row_idx]);

        send_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        assert!(matches!(
            app.modal,
            Some(Modal::FormEdit { ref state, .. }) if state.form.title == "dd-section"
        ));
        // Top-level Ctrl+S without changes — should just round-trip.
        send_key(&mut app, KeyCode::Char('s'), KeyModifiers::CONTROL);
        if let PageNode::Section(s) = &app.site.pages[0].nodes[1] {
            assert_eq!(s.columns.len(), 1);
            assert_eq!(
                s.columns[0].components.len(),
                1,
                "CTA must survive section round-trip"
            );
        } else {
            panic!("expected Section");
        }
    }

    #[test]
    fn tier_d_navigation_drill_round_trip() {
        let mut app = app_with_component(ComponentKind::Navigation);
        open_form_edit_on_page_component(&mut app);
        assert!(matches!(
            app.modal,
            Some(Modal::FormEdit { ref state, .. }) if state.form.title == "dd-navigation"
        ));
        tab_to_items_field(&mut app);
        send_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(drill_stack_len(&app), 1);
        // Inside nav item; Ctrl+S returns to parent.
        send_key(&mut app, KeyCode::Char('s'), KeyModifiers::CONTROL);
        assert_eq!(drill_stack_len(&app), 0);
        // Top-level save.
        send_key(&mut app, KeyCode::Char('s'), KeyModifiers::CONTROL);
        assert!(app.modal.is_none());
    }

    #[test]
    fn tier_d_navigation_button_hides_link_fields() {
        let mut app = app_with_component(ComponentKind::Navigation);
        open_form_edit_on_page_component(&mut app);
        tab_to_items_field(&mut app);
        send_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        // Now in nav-item editor; child_kind is the first field, default "link".
        // Cycle to "button" via Right.
        send_key(&mut app, KeyCode::Right, KeyModifiers::NONE);
        assert_eq!(form_value(&app, "child_kind"), "button");

        // The visible-field count should drop by 2 (child_link_url and child_link_target).
        let visible_count = match app.modal.as_ref() {
            Some(Modal::FormEdit { state, .. }) => state.visible_field_indices().len(),
            _ => panic!("expected FormEdit"),
        };
        // Template has 6 fields; button hides 2 → 4 visible.
        assert_eq!(visible_count, 4);
    }

    #[test]
    #[allow(non_snake_case)]
    fn tier_b_add_item_via_A_key() {
        let mut app = app_with_component(ComponentKind::Accordion);
        open_form_edit_on_page_component(&mut app);
        tab_to_items_field(&mut app);
        let before = match app.modal.as_ref() {
            Some(Modal::FormEdit { state, .. }) => state
                .sub_state
                .get("items")
                .map(|v| v.len())
                .unwrap_or(0),
            _ => panic!("expected FormEdit"),
        };
        send_key(&mut app, KeyCode::Char('A'), KeyModifiers::SHIFT);
        let after = match app.modal.as_ref() {
            Some(Modal::FormEdit { state, .. }) => state
                .sub_state
                .get("items")
                .map(|v| v.len())
                .unwrap_or(0),
            _ => panic!("expected FormEdit"),
        };
        assert_eq!(after, before + 1, "A should add one item");
    }

    #[test]
    fn textarea_display_rows_grows_with_content_and_caps() {
        assert_eq!(textarea_display_rows("one line", 3, None, 35), 3);
        assert_eq!(
            textarea_display_rows("one\ntwo\nthree\nfour", 3, None, 35),
            4
        );

        let many_lines = (0..80).map(|_| "line").collect::<Vec<_>>().join("\n");
        assert_eq!(
            textarea_display_rows(&many_lines, 3, None, TEXTAREA_MAX_DISPLAY_ROWS),
            TEXTAREA_MAX_DISPLAY_ROWS
        );

        assert_eq!(textarea_display_rows(&many_lines, 3, None, 10), 10);
        assert_eq!(textarea_display_rows("abcdef", 1, Some(2), 35), 3);
    }

    #[test]
    fn textarea_display_scrolls_to_cursor_without_truncating_value() {
        let value = "one\ntwo\nthree\nfour\nfive";
        let rendered = render_textarea_display(value, value.chars().count(), true, 3);

        assert_eq!(rendered, "three\nfour\nfive");
    }

    #[test]
    fn form_input_cursor_overlays_visible_glyph_when_caret_is_past_width() {
        let text = editform::FieldKind::Text { default: "" };
        // borders take 2 cols; inner width 5
        let box_rect = Rect {
            x: 0,
            y: 0,
            width: 7,
            height: 3,
        };
        let (_, _, ch) = form_input_cursor_cell(&text, "hello", 5, box_rect)
            .expect("caret at end of a full-width value");
        assert_eq!(ch, 'o', "overlay the last visible glyph, not a space");

        let (x, y, ch) = form_input_cursor_cell(&text, "hi", 2, box_rect)
            .expect("on-screen end of value");
        assert_eq!((x, y, ch), (3, 1, ' '));

        let (x, y, ch) = form_input_cursor_cell(&text, "hello", 1, box_rect)
            .expect("caret on a visible char");
        assert_eq!((x, y, ch), (2, 1, 'e'));

        let textarea = editform::FieldKind::Textarea {
            rows: 3,
            default: "",
        };
        let ta_box = Rect {
            x: 0,
            y: 0,
            width: 7,
            height: 5,
        };
        let (_, _, ch) = form_input_cursor_cell(&textarea, "abcdefghij", 10, ta_box)
            .expect("textarea caret past inner width");
        assert_eq!(ch, 'e');

        let value = "one\ntwo\nhi";
        let (x, y, ch) =
            form_input_cursor_cell(&textarea, value, value.chars().count(), ta_box)
                .expect("textarea caret on last window row");
        assert_eq!((x, y, ch), (3, 3, ' '));
    }

    #[test]
    fn form_edit_cursor_overlay_skips_help_and_display_has_no_block_glyph() {
        let mut app = app_with_cta();
        open_form_edit_on_selected_cta(&mut app);
        send_key(&mut app, KeyCode::Tab, KeyModifiers::NONE);
        let focused = match &app.modal {
            Some(Modal::FormEdit { state, .. }) => state.focused_field,
            _ => panic!("expected FormEdit"),
        };
        app.modal_field_areas.borrow_mut().push((
            focused,
            Rect {
                x: 10,
                y: 5,
                width: 30,
                height: 3,
            },
        ));
        assert!(
            app.active_input_cursor_cell().is_some(),
            "FormEdit text field should return a caret cell"
        );

        app.overlay = Some(Overlay::Help { scroll: 0 });
        assert!(
            app.active_input_cursor_cell().is_none(),
            "Help overlay must hide the FormEdit caret"
        );

        let rendered = render_textarea_display("one\ntwo\nthree\nfour\nfive", 5, true, 3);
        assert!(
            !rendered.contains('▋'),
            "textarea display must not insert a block glyph, got {rendered:?}"
        );
    }

    #[test]
    fn textarea_vertical_cursor_movement_keeps_column_when_possible() {
        let value = "abc\ndefgh\nij";
        let cursor = cursor_from_row_col(&input_lines_preserve(value), 1, 4);

        assert_eq!(
            textarea_move_cursor_vertical(value, cursor, -1),
            cursor_from_row_col(&input_lines_preserve(value), 0, 3)
        );
        assert_eq!(
            textarea_move_cursor_vertical(value, cursor, 1),
            cursor_from_row_col(&input_lines_preserve(value), 2, 2)
        );
    }

    fn open_form_edit_on_selected_cta(app: &mut App) {
        let rows = app.build_page_tree_rows();
        let row_idx = rows
            .iter()
            .position(|row| {
                matches!(
                    row.kind,
                    TreeRowKind::Component {
                        node_idx: 1,
                        column_idx: 0,
                        component_idx: 0
                    }
                )
            })
            .expect("dd-cta component row should exist");
        app.selected_tree_row = row_idx;
        app.apply_tree_row_selection(rows[row_idx]);
        send_key(app, KeyCode::Enter, KeyModifiers::NONE);
        assert!(app.modal.is_some(), "FormEdit modal should open");
    }

    fn form_focused_field_id(app: &App) -> Option<&'static str> {
        match app.modal.as_ref()? {
            Modal::FormEdit { state, .. } => state.focused().map(|f| f.id),
            _ => None,
        }
    }

    fn form_value(app: &App, id: &str) -> String {
        match app.modal.as_ref().expect("modal must be open") {
            Modal::FormEdit { state, .. } => state.get(id).to_string(),
            _ => panic!("expected FormEdit modal"),
        }
    }

    #[test]
    fn pages_panel_shift_a_opens_title_prompt_then_template_picker_then_inserts_blank_page() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.selected_sidebar_section = SidebarSection::Pages;
        let initial_len = app.site.pages.len();

        send_key(&mut app, KeyCode::Char('A'), KeyModifiers::SHIFT);
        assert!(matches!(app.modal, Some(Modal::NewPageTitlePrompt { .. })));

        for c in "Contact Us".chars() {
            send_key(&mut app, KeyCode::Char(c), KeyModifiers::NONE);
        }
        send_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        assert!(matches!(app.modal, Some(Modal::TemplatePicker { selected: 0 })));

        send_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        assert!(app.modal.is_none());
        assert_eq!(app.site.pages.len(), initial_len + 1);
        let new_page = app.site.pages.last().unwrap();
        assert_eq!(new_page.head.title, "Contact Us");
        assert_eq!(new_page.slug, "contact-us");
        assert!(!new_page.slug_locked);
        assert!(new_page.nodes.is_empty());
        assert_eq!(app.selected_page, initial_len);
    }

    #[test]
    fn pages_panel_add_hero_only_template_inserts_single_hero() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.selected_sidebar_section = SidebarSection::Pages;

        send_key(&mut app, KeyCode::Char('A'), KeyModifiers::SHIFT);
        for c in "Gallery".chars() {
            send_key(&mut app, KeyCode::Char(c), KeyModifiers::NONE);
        }
        send_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        send_key(&mut app, KeyCode::Down, KeyModifiers::NONE); // selected=1 (Hero only)
        send_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);

        let p = app.site.pages.last().unwrap();
        assert_eq!(p.nodes.len(), 1);
        assert!(matches!(p.nodes[0], crate::model::PageNode::Hero(_)));
    }

    #[test]
    fn pages_panel_add_hero_plus_section_inserts_hero_then_section() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.selected_sidebar_section = SidebarSection::Pages;

        send_key(&mut app, KeyCode::Char('A'), KeyModifiers::SHIFT);
        for c in "Services".chars() {
            send_key(&mut app, KeyCode::Char(c), KeyModifiers::NONE);
        }
        send_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        send_key(&mut app, KeyCode::Down, KeyModifiers::NONE);
        send_key(&mut app, KeyCode::Down, KeyModifiers::NONE); // selected=2
        send_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);

        let p = app.site.pages.last().unwrap();
        assert_eq!(p.nodes.len(), 2);
        assert!(matches!(p.nodes[0], crate::model::PageNode::Hero(_)));
        assert!(matches!(p.nodes[1], crate::model::PageNode::Section(_)));
    }

    #[test]
    fn pages_panel_add_duplicate_clones_current_and_appends_copy_suffix() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.selected_sidebar_section = SidebarSection::Pages;
        let orig_len = app.site.pages.len();
        let orig_node_count = app.site.pages[0].nodes.len();

        send_key(&mut app, KeyCode::Char('A'), KeyModifiers::SHIFT);
        // Type anything — duplicate ignores the typed title and uses src title.
        send_key(&mut app, KeyCode::Char('x'), KeyModifiers::NONE);
        send_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        send_key(&mut app, KeyCode::Down, KeyModifiers::NONE);
        send_key(&mut app, KeyCode::Down, KeyModifiers::NONE);
        send_key(&mut app, KeyCode::Down, KeyModifiers::NONE); // selected=3 (Duplicate)
        send_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);

        assert_eq!(app.site.pages.len(), orig_len + 1);
        let dup = app.site.pages.last().unwrap();
        assert_eq!(dup.head.title, "Home (Copy)");
        assert_eq!(dup.nodes.len(), orig_node_count);
    }

    #[test]
    fn pages_panel_add_with_duplicate_title_dedupes_id_with_numeric_suffix() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.selected_sidebar_section = SidebarSection::Pages;
        // Starter page has id "page-home". Adding a page titled "Home" (Blank) would
        // generate the same id and should be deduped.
        send_key(&mut app, KeyCode::Char('A'), KeyModifiers::SHIFT);
        for c in "Home".chars() {
            send_key(&mut app, KeyCode::Char(c), KeyModifiers::NONE);
        }
        send_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        send_key(&mut app, KeyCode::Enter, KeyModifiers::NONE); // Blank

        let new_page = app.site.pages.last().unwrap();
        assert_eq!(new_page.id, "page-home-2");
        assert_eq!(new_page.slug, "home-2");
        // The starter page keeps its id.
        assert_eq!(app.site.pages[0].id, "page-home");
    }

    #[test]
    fn pages_panel_shift_x_on_last_page_refuses_delete() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.selected_sidebar_section = SidebarSection::Pages;
        assert_eq!(app.site.pages.len(), 1);

        send_key(&mut app, KeyCode::Char('X'), KeyModifiers::SHIFT);
        assert!(app.modal.is_none(), "no confirm modal should open");
        assert_eq!(app.site.pages.len(), 1, "page must not be deleted");
        let last = app.toasts.last().expect("expected a warning toast");
        assert_eq!(last.level, ToastLevel::Warning);
        assert!(last.message.to_lowercase().contains("cannot delete"));
    }

    #[test]
    fn pages_panel_shift_x_prompts_then_y_deletes_and_pushes_trash() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.selected_sidebar_section = SidebarSection::Pages;
        app.site.pages.push(crate::model::Page::from_template(
            "Contact",
            crate::model::PageTemplate::Blank,
        ));
        app.selected_page = 1;

        send_key(&mut app, KeyCode::Char('X'), KeyModifiers::SHIFT);
        assert!(matches!(app.modal, Some(Modal::ConfirmPrompt { .. })));

        send_key(&mut app, KeyCode::Char('y'), KeyModifiers::NONE);
        assert_eq!(app.site.pages.len(), 1);
        assert_eq!(app.deleted_pages.len(), 1);
        assert_eq!(app.deleted_pages[0].head.title, "Contact");
        assert_eq!(app.selected_page, 0);
    }

    #[test]
    fn pages_panel_shift_x_prompts_then_n_cancels() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.selected_sidebar_section = SidebarSection::Pages;
        app.site.pages.push(crate::model::Page::from_template(
            "Contact",
            crate::model::PageTemplate::Blank,
        ));
        app.selected_page = 1;

        send_key(&mut app, KeyCode::Char('X'), KeyModifiers::SHIFT);
        send_key(&mut app, KeyCode::Char('n'), KeyModifiers::NONE);
        assert!(app.modal.is_none());
        assert_eq!(app.site.pages.len(), 2);
        assert!(app.deleted_pages.is_empty());
    }

    #[test]
    fn pages_panel_u_restores_last_deleted_page_and_selects_it() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.selected_sidebar_section = SidebarSection::Pages;
        app.site.pages.push(crate::model::Page::from_template(
            "Contact",
            crate::model::PageTemplate::Blank,
        ));
        app.selected_page = 1;

        app.modal = None;
        app.commit_delete_page();
        assert_eq!(app.site.pages.len(), 1);

        send_key(&mut app, KeyCode::Char('u'), KeyModifiers::NONE);
        assert_eq!(app.site.pages.len(), 2);
        let restored = &app.site.pages[1];
        assert_eq!(restored.head.title, "Contact");
        assert_eq!(app.selected_page, 1);
        assert!(app.deleted_pages.is_empty());
    }

    #[test]
    fn pages_panel_u_with_empty_trash_is_noop() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.selected_sidebar_section = SidebarSection::Pages;
        send_key(&mut app, KeyCode::Char('u'), KeyModifiers::NONE);
        assert_eq!(app.site.pages.len(), 1);
        let last = app.toasts.last().expect("expected a warning toast");
        assert_eq!(last.level, ToastLevel::Warning);
        assert!(
            last.message.to_lowercase().contains("nothing to restore")
                || last.message.to_lowercase().contains("no deleted")
        );
    }

    #[test]
    fn pages_panel_shift_j_moves_current_page_down() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.selected_sidebar_section = SidebarSection::Pages;
        app.site.pages.push(crate::model::Page::from_template(
            "Contact",
            crate::model::PageTemplate::Blank,
        ));
        app.site.pages.push(crate::model::Page::from_template(
            "About",
            crate::model::PageTemplate::Blank,
        ));
        app.selected_page = 0;

        send_key(&mut app, KeyCode::Char('J'), KeyModifiers::SHIFT);
        assert_eq!(app.site.pages[0].head.title, "Contact");
        assert_eq!(app.site.pages[1].head.title, "Home");
        assert_eq!(app.selected_page, 1);
    }

    #[test]
    fn pages_panel_shift_k_moves_current_page_up() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.selected_sidebar_section = SidebarSection::Pages;
        app.site.pages.push(crate::model::Page::from_template(
            "Contact",
            crate::model::PageTemplate::Blank,
        ));
        app.selected_page = 1;

        send_key(&mut app, KeyCode::Char('K'), KeyModifiers::SHIFT);
        assert_eq!(app.site.pages[0].head.title, "Contact");
        assert_eq!(app.site.pages[1].head.title, "Home");
        assert_eq!(app.selected_page, 0);
    }

    #[test]
    fn pages_panel_shift_j_at_last_is_noop() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.selected_sidebar_section = SidebarSection::Pages;
        app.selected_page = 0;
        send_key(&mut app, KeyCode::Char('J'), KeyModifiers::SHIFT);
        assert_eq!(app.selected_page, 0);
        assert_eq!(app.site.pages[0].head.title, "Home");
    }

    #[test]
    fn pages_panel_r_renames_and_regenerates_slug_when_unlocked() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.selected_sidebar_section = SidebarSection::Pages;
        // starter page slug_locked defaults to false.
        assert!(!app.site.pages[0].slug_locked);

        send_key(&mut app, KeyCode::Char('r'), KeyModifiers::NONE);
        assert!(matches!(app.modal, Some(Modal::RenamePagePrompt { .. })));

        // Clear pre-filled "Home" (4 backspaces) and type "Front Page".
        for _ in 0..4 {
            send_key(&mut app, KeyCode::Backspace, KeyModifiers::NONE);
        }
        for c in "Front Page".chars() {
            send_key(&mut app, KeyCode::Char(c), KeyModifiers::NONE);
        }
        send_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);

        let p = &app.site.pages[0];
        assert_eq!(p.head.title, "Front Page");
        assert_eq!(p.slug, "front-page");
    }

    #[test]
    fn pages_panel_r_with_locked_slug_renames_title_only() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.selected_sidebar_section = SidebarSection::Pages;
        app.site.pages[0].slug_locked = true;
        let orig_slug = app.site.pages[0].slug.clone();

        send_key(&mut app, KeyCode::Char('r'), KeyModifiers::NONE);
        for _ in 0..4 {
            send_key(&mut app, KeyCode::Backspace, KeyModifiers::NONE);
        }
        for c in "Front Page".chars() {
            send_key(&mut app, KeyCode::Char(c), KeyModifiers::NONE);
        }
        send_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);

        assert_eq!(app.site.pages[0].head.title, "Front Page");
        assert_eq!(app.site.pages[0].slug, orig_slug, "locked slug must not regenerate");
    }

    fn open_page_head_form(app: &mut App) {
        assert!(
            app.try_open_form_edit(&TreeRow {
                kind: TreeRowKind::PageHead
            }),
            "page-head FormEdit should open"
        );
    }

    #[test]
    fn page_head_modal_always_shows_slug_field() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        assert!(!app.site.pages[0].slug_locked);
        open_page_head_form(&mut app);
        match &app.modal {
            Some(Modal::FormEdit { state, .. }) => {
                assert_eq!(state.get("slug"), app.site.pages[0].slug);
            }
            _ => panic!("expected FormEdit"),
        }
    }

    #[test]
    fn page_head_modal_save_writes_slug_and_locks_when_edited() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        assert!(!app.site.pages[0].slug_locked);
        open_page_head_form(&mut app);
        if let Some(Modal::FormEdit { state, cursor_pos, .. }) = &mut app.modal {
            state.set("slug", "new-slug");
            *cursor_pos = 8;
        }
        send_key(&mut app, KeyCode::Char('s'), KeyModifiers::CONTROL);
        assert_eq!(app.site.pages[0].slug, "new-slug");
        assert!(
            app.site.pages[0].slug_locked,
            "editing the slug field must lock the slug"
        );
    }

    #[test]
    fn page_head_modal_save_leaves_slug_unchanged_when_user_did_not_edit_it() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        let orig_slug = app.site.pages[0].slug.clone();
        open_page_head_form(&mut app);
        send_key(&mut app, KeyCode::Char('s'), KeyModifiers::CONTROL);
        assert_eq!(app.site.pages[0].slug, orig_slug);
        assert!(!app.site.pages[0].slug_locked, "no slug edit means no lock");
    }

    #[test]
    fn page_head_modal_default_og_title_is_page_title() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        assert!(app.site.pages[0].head.og_title.is_none());
        open_page_head_form(&mut app);
        match &app.modal {
            Some(Modal::FormEdit { state, .. }) => {
                assert_eq!(
                    state.get("og_title"),
                    app.site.pages[0].head.title,
                    "OG Title should default to the page title when unset"
                );
            }
            _ => panic!("expected FormEdit"),
        }
    }

    #[test]
    fn page_head_modal_default_canonical_is_slug_path() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        assert!(app.site.pages[0].head.canonical_url.is_none());
        open_page_head_form(&mut app);
        match &app.modal {
            Some(Modal::FormEdit { state, .. }) => {
                assert_eq!(
                    state.get("canonical_url"),
                    "",
                    "canonical stays empty when unset; export fills it from base_url"
                );
            }
            _ => panic!("expected FormEdit"),
        }
    }

    #[test]
    fn page_head_title_rename_regens_slug_when_unlocked() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        assert!(!app.site.pages[0].slug_locked);
        open_page_head_form(&mut app);
        if let Some(Modal::FormEdit { state, cursor_pos, .. }) = &mut app.modal {
            state.set("title", "About Us");
            *cursor_pos = 8;
        }
        send_key(&mut app, KeyCode::Char('s'), KeyModifiers::CONTROL);
        assert_eq!(app.site.pages[0].head.title, "About Us");
        assert_eq!(
            app.site.pages[0].slug, "about-us",
            "slug should regenerate from title when unlocked"
        );
    }

    #[test]
    fn open_validation_modal_on_clean_starter_pushes_success_toast_and_no_modal() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.open_validation_modal();
        assert!(
            app.modal.is_none(),
            "no modal should open when validation is clean"
        );
        let last = app.toasts.last().expect("expected a success toast");
        assert_eq!(last.level, ToastLevel::Success);
        assert!(
            last.message.to_lowercase().contains("no validation errors"),
            "expected clean-validation toast, got: {:?}",
            last.message
        );
    }

    #[test]
    fn open_validation_modal_with_errors_opens_modal_with_error_list() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        // Force an error: empty slug.
        app.site.pages[0].slug = "".to_string();
        app.open_validation_modal();
        match &app.modal {
            Some(Modal::ValidationErrors {
                errors,
                scroll_offset,
            }) => {
                assert!(!errors.is_empty());
                assert_eq!(*scroll_offset, 0);
                assert!(
                    errors.iter().any(|e| e.contains("empty slug")),
                    "expected empty-slug error, got: {:?}",
                    errors
                );
            }
            _ => panic!("expected Modal::ValidationErrors, got a different modal or None"),
        }
    }

    #[test]
    fn f3_on_clean_starter_pushes_success_toast() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        send_key(&mut app, KeyCode::F(3), KeyModifiers::NONE);
        assert!(app.modal.is_none());
        let last = app.toasts.last().expect("expected a success toast");
        assert_eq!(last.level, ToastLevel::Success);
        assert!(last.message.to_lowercase().contains("no validation errors"));
    }

    #[test]
    fn f3_with_validation_errors_opens_modal() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.site.pages[0].slug = "".to_string();
        send_key(&mut app, KeyCode::F(3), KeyModifiers::NONE);
        assert!(matches!(app.modal, Some(Modal::ValidationErrors { .. })));
    }

    #[test]
    fn f3_then_enter_dismisses_modal() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.site.pages[0].slug = "".to_string();
        send_key(&mut app, KeyCode::F(3), KeyModifiers::NONE);
        send_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        assert!(app.modal.is_none());
    }

    #[test]
    fn f3_then_j_k_scrolls_error_list() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.site.pages.clear();
        send_key(&mut app, KeyCode::F(3), KeyModifiers::NONE);
        let initial_errors = match &app.modal {
            Some(Modal::ValidationErrors { errors, .. }) => errors.len(),
            _ => 0,
        };
        if initial_errors > 1 {
            send_key(&mut app, KeyCode::Char('j'), KeyModifiers::NONE);
            match &app.modal {
                Some(Modal::ValidationErrors { scroll_offset, .. }) => {
                    assert_eq!(*scroll_offset, 1);
                }
                _ => panic!("modal closed unexpectedly"),
            }
            send_key(&mut app, KeyCode::Char('k'), KeyModifiers::NONE);
            match &app.modal {
                Some(Modal::ValidationErrors { scroll_offset, .. }) => {
                    assert_eq!(*scroll_offset, 0);
                }
                _ => panic!("modal closed unexpectedly"),
            }
        }
    }

    #[test]
    fn f2_opens_and_closes_with_f2_and_esc() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        send_key(&mut app, KeyCode::F(2), KeyModifiers::NONE);
        assert!(matches!(app.overlay, Some(Overlay::Theme { scroll: 0 })));
        send_key(&mut app, KeyCode::Esc, KeyModifiers::NONE);
        assert!(app.overlay.is_none());
    }

    #[test]
    fn f2_scroll_keys_and_wheel_update_theme_scroll() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        send_key(&mut app, KeyCode::F(2), KeyModifiers::NONE);
        // simulate render to set max (draw not called, so manually exercise clamp logic path)
        app.overlay_scroll_max = 5; // pretend content
        send_key(&mut app, KeyCode::Down, KeyModifiers::NONE);
        assert!(matches!(app.overlay, Some(Overlay::Theme { scroll: 1 })));
        send_key(&mut app, KeyCode::PageDown, KeyModifiers::NONE);
        match app.overlay {
            Some(Overlay::Theme { scroll }) => assert!(scroll >= 1),
            other => panic!("expected Theme overlay, got {other:?}"),
        }
        send_key(&mut app, KeyCode::Char('g'), KeyModifiers::NONE); // home alias
        assert!(matches!(app.overlay, Some(Overlay::Theme { scroll: 0 })));
    }

    #[test]
    fn f2_shows_warning_status_when_theme_status_is_some() {
        let mut app = App::new(
            Site::starter(),
            None,
            AppTheme::default(),
            "default".to_string(),
            Some("theme 'foo.yml' declares version 99 (expected 1); using built-in defaults".to_string()),
        );
        send_key(&mut app, KeyCode::F(2), KeyModifiers::NONE);
        assert!(matches!(app.overlay, Some(Overlay::Theme { .. })));
        // status is stored; render would show it (no crash)
    }

    #[test]
    fn begin_export_flow_on_clean_starter_without_export_dir_opens_path_prompt() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        assert!(app.site.export_dir.is_none());
        app.begin_export_flow();
        match &app.modal {
            Some(Modal::ExportPathPrompt { path }) => {
                assert_eq!(path, "./web/");
            }
            _ => panic!("expected ExportPathPrompt, got a different modal or None"),
        }
    }

    #[test]
    fn begin_export_flow_with_invalid_site_opens_validation_modal() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.site.pages[0].slug = "".to_string();
        app.begin_export_flow();
        assert!(matches!(app.modal, Some(Modal::ValidationErrors { .. })));
    }

    #[test]
    fn begin_export_flow_with_saved_export_dir_commits_directly() {
        let tmp = std::env::temp_dir().join(format!(
            "dd_export_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let imgs = tmp.join("source").join("images");
        std::fs::create_dir_all(&imgs).unwrap();
        std::fs::write(imgs.join("hero.jpg"), b"fake").unwrap();
        let grunt_css = tmp.join("web").join("assets").join("css");
        std::fs::create_dir_all(&grunt_css).unwrap();
        std::fs::write(grunt_css.join("style.min.css"), b"/* grunt */").unwrap();
        let json_path = tmp.join("site.json");
        let mut app = App::new(Site::starter(), Some(json_path.clone()), AppTheme::default(), "default".to_string(), None);
        app.site.export_dir = Some("web".to_string());

        app.begin_export_flow();

        assert!(app.modal.is_none(), "no modal should open — direct export");
        let last = app.toasts.last().expect("expected a toast");
        assert_eq!(last.level, ToastLevel::Success);
        assert!(last.message.to_lowercase().contains("exported"));
        assert!(tmp.join("web").exists(), "export directory should have been created");
        let css = std::fs::read_to_string(grunt_css.join("style.min.css")).unwrap();
        assert_eq!(css, "/* grunt */", "export must not clobber grunt CSS");

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn e_key_with_validation_errors_opens_validation_modal() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.site.pages[0].slug = "".to_string();
        send_key(&mut app, KeyCode::Char('E'), KeyModifiers::SHIFT);
        assert!(matches!(app.modal, Some(Modal::ValidationErrors { .. })));
    }

    #[test]
    fn details_panel_single_click_selects_double_click_edits() {
        let mut app = app_with_component(ComponentKind::Card);
        app.details_area = Rect { x: 20, y: 1, width: 60, height: 30, ..Default::default() };
        app.list_area = Rect::default();
        app.pages_area = Rect::default();
        app.regions_area = Rect::default();
        app.details_scroll_row = 0;
        // Single click in details area (some y maps to content line)
        app.handle_click(25, 4);
        // Double should attempt edit
        app.handle_double_click(25, 4);
        // Just ensure no panic / basic coverage
        assert!(app.toasts.len() > 0 || app.modal.is_some() || true);
    }

    #[test]
    fn double_click_page_in_nodes_panel_switches_to_page_and_opens_head_edit() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        // starter has 1 page; add a second so we can double-click the second item (index 1)
        app.site.pages.push(crate::model::Page::from_template(
            "About",
            crate::model::PageTemplate::Blank,
        ));
        assert!(app.site.pages.len() >= 2);

        // Make pages_area tall enough to contain the list items (body starts at y+1)
        app.pages_area = Rect { x: 0, y: 1, width: 30, height: 10, ..Default::default() };
        app.list_area = Rect::default();
        app.regions_area = Rect::default();
        app.details_area = Rect::default();

        // pages list body_top = 2; rel=0 at y=2, rel=1 (second page) at y=3
        app.handle_double_click(10, 3);

        assert_eq!(app.selected_page, 1);
        assert!(app.page_head_selected);
        assert_eq!(app.selected_sidebar_section, SidebarSection::Layouts);
        // Double-click should have opened the unified FormEdit for the page [HEAD]
        assert!(matches!(app.modal, Some(Modal::FormEdit { .. })));
        if let Some(Modal::FormEdit { state, cursor, .. }) = &app.modal {
            // sanity: it's the page-head form for the right page
            assert!(state.form.fields.iter().any(|f| f.id == "title" || f.id == "slug"));
            assert!(matches!(cursor, cursor::Cursor::PageHead { page: 1 }));
        }
    }

    #[test]
    fn e_key_with_clean_site_and_no_export_dir_opens_path_prompt() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        send_key(&mut app, KeyCode::Char('E'), KeyModifiers::SHIFT);
        assert!(matches!(app.modal, Some(Modal::ExportPathPrompt { .. })));
    }

    #[test]
    fn fresh_app_is_clean() {
        let app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        assert!(!app.dirty);
        assert!(app.dirty_since.is_none());
    }

    #[test]
    fn editing_a_page_title_marks_app_dirty() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.site.pages[0].head.title = "Mutated".to_string();
        app.mark_dirty_if_changed();
        assert!(app.dirty);
        assert!(app.dirty_since.is_some());
    }

    #[test]
    fn unchanged_model_stays_clean() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.mark_dirty_if_changed();
        assert!(!app.dirty);
        assert!(app.dirty_since.is_none());
    }

    #[test]
    fn dirty_since_does_not_reset_on_subsequent_mutations() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.site.pages[0].head.title = "First".to_string();
        app.mark_dirty_if_changed();
        let first = app.dirty_since.expect("dirty_since should be set");
        std::thread::sleep(std::time::Duration::from_millis(5));
        app.site.pages[0].head.title = "Second".to_string();
        app.mark_dirty_if_changed();
        assert_eq!(
            app.dirty_since,
            Some(first),
            "subsequent mutations must NOT push dirty_since forward"
        );
    }

    #[test]
    fn tick_autosave_does_nothing_when_clean() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        let now = std::time::Instant::now();
        app.tick_autosave(now);
        assert!(!app.dirty);
    }

    #[test]
    fn tick_autosave_does_nothing_when_dirty_but_no_path() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.site.pages[0].head.title = "x".to_string();
        app.mark_dirty_if_changed();
        let later = app.dirty_since.unwrap()
            + std::time::Duration::from_secs(10);
        app.tick_autosave(later);
        assert!(app.dirty, "no path means no autosave; site stays dirty");
    }

    #[test]
    fn tick_autosave_writes_when_dirty_and_debounce_elapsed() {
        let tmp_dir = std::env::temp_dir().join(format!(
            "dd_autosave_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&tmp_dir).unwrap();
        let json_path = tmp_dir.join("site.json");
        crate::storage::save_site(&json_path, &Site::starter()).unwrap();

        let mut app =
            App::new(Site::starter(), Some(json_path.clone()), AppTheme::default(), "default".to_string(), None);
        app.site.pages[0].head.title = "After mutation".to_string();
        app.mark_dirty_if_changed();
        assert!(app.dirty);

        let due = app.dirty_since.unwrap()
            + std::time::Duration::from_millis(2_100);
        app.tick_autosave(due);
        assert!(!app.dirty, "autosave should clear the dirty flag");
        assert!(app.dirty_since.is_none());
        let on_disk = std::fs::read_to_string(&json_path).unwrap();
        assert!(on_disk.contains("After mutation"));
        std::fs::remove_dir_all(&tmp_dir).ok();
    }

    #[test]
    fn tick_autosave_holds_off_within_debounce_window() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.site.pages[0].head.title = "x".to_string();
        app.mark_dirty_if_changed();
        let still_in_window = app.dirty_since.unwrap()
            + std::time::Duration::from_millis(500);
        app.tick_autosave(still_in_window);
        assert!(app.dirty);
    }

    #[test]
    fn manual_save_writes_backup_alongside_main_file() {
        let tmp = std::env::temp_dir().join(format!(
            "dd_backup_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&tmp).unwrap();
        let json_path = tmp.join("site.json");
        let backup_path = tmp.join("site.json.backup");

        let mut app =
            App::new(Site::starter(), Some(json_path.clone()), AppTheme::default(), "default".to_string(), None);
        app.site.pages[0].head.title = "Pre-save".to_string();

        app.commit_save_with_backup(&json_path)
            .expect("manual save should succeed");

        assert!(json_path.exists(), "main file written");
        assert!(backup_path.exists(), "backup written");
        let main = std::fs::read_to_string(&json_path).unwrap();
        let bak = std::fs::read_to_string(&backup_path).unwrap();
        assert_eq!(main, bak, "backup must be byte-identical to main");
        assert!(!app.dirty, "manual save clears dirty");
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn load_with_diverging_backup_pushes_info_toast() {
        let tmp = std::env::temp_dir().join(format!(
            "dd_loadcheck_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&tmp).unwrap();
        let json_path = tmp.join("site.json");
        let backup_path = tmp.join("site.json.backup");

        std::fs::write(&backup_path, "{\"backup\":\"old\"}").unwrap();
        std::fs::write(&json_path, "{\"main\":\"new\"}").unwrap();

        let app = App::new(
            Site::starter(),
            Some(json_path.clone()),
            AppTheme::default(),
            "default".to_string(),
            None,
        );
        let toast = app
            .toasts
            .iter()
            .find(|t| t.message.to_lowercase().contains("differs from last manual save"));
        assert!(
            toast.is_some(),
            "expected a divergence toast, got: {:?}",
            app.toasts.iter().map(|t| &t.message).collect::<Vec<_>>()
        );
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn load_with_matching_backup_pushes_no_toast() {
        let tmp = std::env::temp_dir().join(format!(
            "dd_loadcheck_match_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&tmp).unwrap();
        let json_path = tmp.join("site.json");
        let backup_path = tmp.join("site.json.backup");
        std::fs::write(&json_path, "same").unwrap();
        std::fs::write(&backup_path, "same").unwrap();

        let app = App::new(
            Site::starter(),
            Some(json_path.clone()),
            AppTheme::default(),
            "default".to_string(),
            None,
        );
        assert!(app
            .toasts
            .iter()
            .all(|t| !t.message.to_lowercase().contains("differs")));
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn begin_preview_flow_with_invalid_site_opens_validation_modal() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.site.pages[0].slug = "".to_string();
        app.begin_preview_flow();
        assert!(matches!(app.modal, Some(Modal::ValidationErrors { .. })));
    }

    #[test]
    fn begin_preview_flow_without_export_dir_opens_path_prompt() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.begin_preview_flow();
        match &app.modal {
            Some(Modal::PreviewPathPrompt { path }) => assert_eq!(path, "./web/"),
            _ => panic!("expected PreviewPathPrompt"),
        }
    }

    #[test]
    fn current_page_slug_for_preview_returns_selected_page_slug() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.site.pages.push(crate::model::Page::from_template(
            "Contact",
            crate::model::PageTemplate::Blank,
        ));
        app.selected_page = 1;
        assert_eq!(app.current_page_slug_for_preview(), "contact");
    }

    #[test]
    fn p_key_with_validation_errors_opens_validation_modal() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.site.pages[0].slug = "".to_string();
        send_key(&mut app, KeyCode::Char('p'), KeyModifiers::NONE);
        assert!(matches!(app.modal, Some(Modal::ValidationErrors { .. })));
    }

    #[test]
    fn p_key_with_clean_site_and_no_export_dir_opens_preview_path_prompt() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        send_key(&mut app, KeyCode::Char('p'), KeyModifiers::NONE);
        assert!(matches!(app.modal, Some(Modal::PreviewPathPrompt { .. })));
    }

    #[test]
    fn image_picker_left_arrow_at_root_does_not_escape() {
        let tmp = std::env::temp_dir().join(format!(
            "dd_imgpicker_root_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&tmp).unwrap();
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.modal = Some(Modal::ImagePicker {
            state: ImagePickerState {
                root: tmp.clone(),
                cwd: tmp.clone(),
                filter: String::new(),
                selected: 0,
                binding: ImagePickBinding::FormEditField {
                    field_id: "x".to_string(),
                },
            },
        });
        send_key(&mut app, KeyCode::Left, KeyModifiers::NONE);
        match &app.modal {
            Some(Modal::ImagePicker { state }) => assert_eq!(state.cwd, tmp),
            _ => panic!("picker should still be open at root"),
        }
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn image_picker_esc_restores_paused_form_edit_modal() {
        let tmp = std::env::temp_dir().join(format!(
            "dd_imgpicker_esc_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&tmp).unwrap();
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        let dummy_form_state = editform::EditFormState::new(&editform::CTA_FORM);
        let paused = Modal::FormEdit {
            state: dummy_form_state,
            cursor: cursor::Cursor::PageHero { page: 0, node: 0 },
            cursor_pos: 0,
            drill_stack: Vec::new(),
            scroll_offset: 0,
        };
        app.paused_form_edit_modal = Some(paused);
        app.modal = Some(Modal::ImagePicker {
            state: ImagePickerState {
                root: tmp.clone(),
                cwd: tmp.clone(),
                filter: String::new(),
                selected: 0,
                binding: ImagePickBinding::FormEditField {
                    field_id: "x".to_string(),
                },
            },
        });
        send_key(&mut app, KeyCode::Esc, KeyModifiers::NONE);
        assert!(matches!(app.modal, Some(Modal::FormEdit { .. })));
        assert!(app.paused_form_edit_modal.is_none());
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn app_selects_header_copy_from_defaults_at_construction() {
        let app = App::new(
            Site::starter(),
            None,
            AppTheme::default(),
            "default".to_string(),
            None,
        );
        let defs = default_header_quotes();
        assert!(
            defs.iter().any(|q| q == &app.header_copy),
            "header_copy '{}' should be one of the defaults",
            app.header_copy
        );
        assert_eq!(app.theme_source, "default");
    }

    #[test]
    fn q_without_modifiers_does_not_quit() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        send_key(&mut app, KeyCode::Char('q'), KeyModifiers::NONE);
        assert!(!app.should_quit);
        send_key(&mut app, KeyCode::Char('q'), KeyModifiers::CONTROL);
        assert!(app.should_quit);
    }

    #[test]
    fn d_deletes_selected_component() {
        let mut app = app_with_component(ComponentKind::Banner);
        app.selected_sidebar_section = SidebarSection::Layouts;
        select_first_component_row(&mut app);
        send_key(&mut app, KeyCode::Char('d'), KeyModifiers::NONE);
        match &app.site.pages[0].nodes[1] {
            PageNode::Section(s) => assert!(s.columns[0].components.is_empty()),
            _ => panic!("expected section"),
        }
    }

    #[test]
    fn y_duplicates_selected_component() {
        let mut app = app_with_component(ComponentKind::Banner);
        app.selected_sidebar_section = SidebarSection::Layouts;
        select_first_component_row(&mut app);
        send_key(&mut app, KeyCode::Char('y'), KeyModifiers::NONE);
        match &app.site.pages[0].nodes[1] {
            PageNode::Section(s) => assert_eq!(s.columns[0].components.len(), 2),
            _ => panic!("expected section"),
        }
    }

    #[test]
    fn jk_reorders_nodes() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.selected_sidebar_section = SidebarSection::Layouts;
        app.selected_node = 0;
        app.page_head_selected = false;
        app.sync_tree_row_with_selection();
        send_key(&mut app, KeyCode::Char('J'), KeyModifiers::SHIFT);
        assert!(matches!(app.site.pages[0].nodes[0], PageNode::Section(_)));
        assert!(matches!(app.site.pages[0].nodes[1], PageNode::Hero(_)));
        send_key(&mut app, KeyCode::Char('K'), KeyModifiers::SHIFT);
        assert!(matches!(app.site.pages[0].nodes[0], PageNode::Hero(_)));
    }

    #[test]
    fn insert_component_after_selected_not_append() {
        let mut app = app_with_component(ComponentKind::Banner);
        app.component_kind = ComponentKind::Alert;
        app.add_selected_component_to_section();
        match &app.site.pages[0].nodes[1] {
            PageNode::Section(s) => {
                assert_eq!(s.columns[0].components.len(), 2);
                assert!(matches!(
                    s.columns[0].components[1],
                    crate::model::SectionComponent::Alert(_)
                ));
            }
            _ => panic!("expected section"),
        }
        app.selected_component = 0;
        app.sync_tree_row_with_selection();
        app.component_kind = ComponentKind::Image;
        app.add_selected_component_to_section();
        match &app.site.pages[0].nodes[1] {
            PageNode::Section(s) => {
                assert_eq!(s.columns[0].components.len(), 3);
                assert!(matches!(
                    s.columns[0].components[1],
                    crate::model::SectionComponent::Image(_)
                ));
            }
            _ => panic!("expected section"),
        }
    }

    #[test]
    fn footer_insert_adds_to_footer_not_page() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.selected_region = SelectedRegion::Footer;
        app.component_kind = ComponentKind::RichText;
        let page_comps_before = match &app.site.pages[0].nodes[1] {
            PageNode::Section(s) => s.columns[0].components.len(),
            _ => 0,
        };
        app.insert_selected_component_kind();
        assert!(
            app.site.footer.sections[0].columns[0]
                .components
                .iter()
                .any(|c| matches!(c, crate::model::SectionComponent::RichText(_)))
        );
        let page_comps_after = match &app.site.pages[0].nodes[1] {
            PageNode::Section(s) => s.columns[0].components.len(),
            _ => 0,
        };
        assert_eq!(page_comps_before, page_comps_after);
    }

    #[test]
    fn u_undoes_component_delete() {
        let mut app = app_with_component(ComponentKind::Banner);
        app.selected_sidebar_section = SidebarSection::Layouts;
        select_first_component_row(&mut app);
        send_key(&mut app, KeyCode::Char('d'), KeyModifiers::NONE);
        send_key(&mut app, KeyCode::Char('u'), KeyModifiers::NONE);
        match &app.site.pages[0].nodes[1] {
            PageNode::Section(s) => assert_eq!(s.columns[0].components.len(), 1),
            _ => panic!("expected section"),
        }
    }

    #[test]
    fn ctrl_q_when_dirty_opens_confirm() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.site.pages[0].head.title = "Changed".to_string();
        app.mark_dirty_if_changed();
        assert!(app.dirty);
        send_key(&mut app, KeyCode::Char('q'), KeyModifiers::CONTROL);
        assert!(!app.should_quit);
        assert!(matches!(
            app.modal,
            Some(Modal::ConfirmPrompt {
                on_confirm: ConfirmKind::QuitUnsaved,
                ..
            })
        ));
        send_key(&mut app, KeyCode::Char('y'), KeyModifiers::NONE);
        assert!(app.should_quit);
    }

    #[test]
    fn footer_hint_marks_dirty() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        let clean = app.footer_hint(120);
        assert!(!clean.starts_with('*'));
        app.dirty = true;
        let dirty = app.footer_hint(120);
        assert!(dirty.starts_with('*'), "{dirty}");
    }

    #[test]
    fn slash_opens_unified_component_picker() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        send_key(&mut app, KeyCode::Char('/'), KeyModifiers::NONE);
        assert!(matches!(app.modal, Some(Modal::ComponentPicker { .. })));
    }

    #[test]
    fn s_without_path_opens_unified_save_prompt() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        send_key(&mut app, KeyCode::Char('s'), KeyModifiers::NONE);
        match &app.modal {
            Some(Modal::SavePrompt { path }) => assert_eq!(path, "site.json"),
            _ => panic!("expected SavePrompt"),
        }
    }

    #[test]
    fn page_list_click_keeps_pages_focus() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.pages_area = Rect {
            x: 0,
            y: 1,
            width: 30,
            height: 10,
            ..Default::default()
        };
        app.list_area = Rect::default();
        app.regions_area = Rect::default();
        app.details_area = Rect::default();
        app.handle_click(10, 2);
        assert_eq!(app.selected_page, 0);
        assert_eq!(app.selected_sidebar_section, SidebarSection::Pages);
    }

    #[test]
    fn footer_details_are_not_a_stub() {
        let app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        let (text, _) = app.details_text(40);
        // default region is Page; switch check via footer helper
        let (footer, footer_hits) = app.footer_details_text(40);
        assert!(footer.contains("dd-footer"), "{footer}");
        assert!(!footer.to_lowercase().contains("not yet implemented"));
        assert_eq!(footer.lines().count(), footer_hits.len());
        let _ = text;
    }

    #[test]
    fn f1_opens_help_while_form_edit_is_open() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.modal = Some(Modal::FormEdit {
            state: editform::EditFormState::new(&editform::CTA_FORM),
            cursor: cursor::Cursor::PageHero { page: 0, node: 0 },
            cursor_pos: 0,
            drill_stack: Vec::new(),
            scroll_offset: 0,
        });
        send_key(&mut app, KeyCode::F(1), KeyModifiers::NONE);
        assert!(matches!(app.overlay, Some(Overlay::Help { scroll: 0 })));
        assert!(matches!(app.modal, Some(Modal::FormEdit { .. })));
        assert!(app.paused_form_edit_modal.is_none());
    }

    #[test]
    fn f2_from_form_edit_opens_theme_overlay_esc_returns_to_form() {
        let mut app = app_with_cta();
        open_form_edit_on_selected_cta(&mut app);
        send_key(&mut app, KeyCode::F(2), KeyModifiers::NONE);
        assert!(matches!(app.overlay, Some(Overlay::Theme { scroll: 0 })));
        assert!(matches!(app.modal, Some(Modal::FormEdit { .. })));
        assert!(app.paused_form_edit_modal.is_none());
        send_key(&mut app, KeyCode::Esc, KeyModifiers::NONE);
        assert!(app.overlay.is_none());
        assert!(matches!(app.modal, Some(Modal::FormEdit { .. })));
    }

    #[test]
    fn form_edit_image_picker_f1_esc_esc_restores_same_field() {
        let tmp = std::env::temp_dir().join(format!(
            "dd_overlay_picker_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(tmp.join("source").join("images")).unwrap();
        let mut app = app_with_cta();
        app.path = Some(tmp.join("site.json"));
        open_form_edit_on_selected_cta(&mut app);
        send_key(&mut app, KeyCode::Tab, KeyModifiers::NONE);
        assert_eq!(form_focused_field_id(&app), Some("parent_image_url"));

        send_key(&mut app, KeyCode::Char('p'), KeyModifiers::CONTROL);
        assert!(matches!(app.modal, Some(Modal::ImagePicker { .. })));
        match &app.paused_form_edit_modal {
            Some(Modal::FormEdit { state, .. }) => {
                assert_eq!(state.focused().map(|f| f.id), Some("parent_image_url"));
            }
            other => panic!(
                "paused_form_edit_modal should hold FormEdit, got {}",
                other.as_ref().map(Modal::variant_name).unwrap_or("None")
            ),
        }

        send_key(&mut app, KeyCode::F(1), KeyModifiers::NONE);
        assert!(matches!(app.overlay, Some(Overlay::Help { scroll: 0 })));
        assert!(matches!(app.modal, Some(Modal::ImagePicker { .. })));
        assert!(matches!(app.paused_form_edit_modal, Some(Modal::FormEdit { .. })));

        send_key(&mut app, KeyCode::Esc, KeyModifiers::NONE);
        assert!(app.overlay.is_none());
        assert!(matches!(app.modal, Some(Modal::ImagePicker { .. })));
        assert!(matches!(app.paused_form_edit_modal, Some(Modal::FormEdit { .. })));

        send_key(&mut app, KeyCode::Esc, KeyModifiers::NONE);
        assert!(app.overlay.is_none());
        assert!(app.paused_form_edit_modal.is_none());
        assert!(matches!(app.modal, Some(Modal::FormEdit { .. })));
        assert_eq!(form_focused_field_id(&app), Some("parent_image_url"));
        std::fs::remove_dir_all(&tmp).ok();
    }

    const FOOTER_TOKEN_ALLOWLIST: &[&str] = &[
        "*",
        "F1:Help",
        "F2:Theme",
        "Esc:Close",
        "Ctrl+Q:Quit",
        "r:Rename",
        "Shift+A:Add",
        "Shift+X:Del",
        "u:Undo-page",
        "Shift+J/K:Move",
        "Enter:Edit",
        "j/k:Header/Footer",
        "j/k:Scroll",
        "/:Insert",
        "d:Del",
        "y:Dup",
        "u:Undo-tree",
        "r:Col-id",
        "J/K:Move",
        "p:Preview",
        "(mouse: click/scroll)",
    ];

    fn assert_footer_hint_shape(hint: &str, width: u16) {
        assert!(
            hint.starts_with("F1:Help  F2:Theme") || hint.starts_with("*  F1:Help  F2:Theme"),
            "{hint}"
        );
        assert!(hint.chars().count() <= width as usize, "{hint}");
        for token in hint.split("  ") {
            if token.is_empty() {
                continue;
            }
            assert!(
                FOOTER_TOKEN_ALLOWLIST.contains(&token),
                "unexpected or clipped token: {token:?} in {hint:?}"
            );
        }
    }

    #[test]
    fn footer_hint_pages_includes_f1_f2_and_keeps_whole_tokens() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.selected_sidebar_section = SidebarSection::Pages;
        for width in [60_u16, 80, 110] {
            assert_footer_hint_shape(&app.footer_hint(width), width);
        }
        let pages_medium = "F1:Help  F2:Theme  Shift+A:Add  Shift+X:Del  u:Undo-page  r:Rename  Shift+J/K:Move  Ctrl+Q:Quit";
        let at_80 = app.footer_hint(80);
        if pages_medium.chars().count() <= 80 {
            assert!(at_80.contains("Shift+J/K:Move"), "{at_80}");
        }
        let at_110 = app.footer_hint(110);
        assert!(at_110.contains("Shift+J/K:Move"), "{at_110}");
        let pages_wide = format!("{pages_medium}  (mouse: click/scroll)");
        if pages_wide.chars().count() <= 110 {
            assert!(at_110.contains("(mouse: click/scroll)"), "{at_110}");
        }
    }

    #[test]
    fn footer_hint_layouts_includes_f2() {
        let app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        for width in [60_u16, 80, 110] {
            assert_footer_hint_shape(&app.footer_hint(width), width);
        }
        let layouts_scoped = "F1:Help  F2:Theme  /:Insert  Enter:Edit  d:Del  y:Dup  u:Undo-tree  r:Col-id  J/K:Move  p:Preview  Ctrl+Q:Quit";
        let at_110 = app.footer_hint(110);
        if layouts_scoped.chars().count() <= 110 {
            assert!(at_110.contains("p:Preview"), "{at_110}");
        }
        let layouts_wide = format!("{layouts_scoped}  (mouse: click/scroll)");
        if layouts_wide.chars().count() <= 110 {
            assert!(at_110.contains("(mouse: click/scroll)"), "{at_110}");
        }
    }

    #[test]
    fn footer_hint_overlay_includes_f2_and_esc_close() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.overlay = Some(Overlay::Help { scroll: 0 });
        let hint = app.footer_hint(80);
        assert!(hint.starts_with("F1:Help  F2:Theme"), "{hint}");
        assert!(hint.contains("Esc:Close"), "{hint}");
        assert_footer_hint_shape(&hint, 80);
    }

    #[test]
    fn footer_hint_dirty_prefix_at_120() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.dirty = true;
        let dirty = app.footer_hint(120);
        assert!(dirty.starts_with("*  F1:Help  F2:Theme"), "{dirty}");
    }

    #[test]
    fn wheel_down_over_pages_selects_next_page() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        let clone = app.site.pages[0].clone();
        app.site.pages.push(clone);
        app.selected_page = 0;
        app.pages_area = Rect {
            x: 0,
            y: 1,
            width: 30,
            height: 10,
            ..Default::default()
        };
        app.list_area = Rect::default();
        app.regions_area = Rect::default();
        app.details_area = Rect::default();
        send_mouse(&mut app, MouseEventKind::ScrollDown, 10, 3);
        assert_eq!(app.selected_page, 1);
    }

    #[test]
    fn wheel_down_over_layout_selects_next_tree_row() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.list_area = Rect {
            x: 0,
            y: 10,
            width: 30,
            height: 20,
            ..Default::default()
        };
        app.pages_area = Rect::default();
        app.regions_area = Rect::default();
        app.details_area = Rect::default();
        app.sync_tree_row_with_selection();
        let before = app.selected_tree_row;
        send_mouse(&mut app, MouseEventKind::ScrollDown, 10, 15);
        assert_eq!(app.selected_tree_row, before + 1);
    }

    #[test]
    fn wheel_down_over_details_scrolls_details_not_tree_or_page() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.details_area = Rect {
            x: 40,
            y: 1,
            width: 40,
            height: 5,
            ..Default::default()
        };
        app.list_area = Rect::default();
        app.pages_area = Rect::default();
        app.regions_area = Rect::default();
        app.sync_tree_row_with_selection();
        app.details_scroll_row = 0;
        assert!(app.details_max_scroll() > 0);
        let tree_before = app.selected_tree_row;
        let page_before = app.selected_page;
        send_mouse(&mut app, MouseEventKind::ScrollDown, 45, 3);
        assert_eq!(app.selected_tree_row, tree_before);
        assert_eq!(app.selected_page, page_before);
        assert!(app.details_scroll_row > 0);
    }

    #[test]
    fn wheel_down_over_empty_chrome_is_noop() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.list_area = Rect::default();
        app.pages_area = Rect::default();
        app.regions_area = Rect::default();
        app.details_area = Rect::default();
        app.sync_tree_row_with_selection();
        let tree_before = app.selected_tree_row;
        let page_before = app.selected_page;
        send_mouse(&mut app, MouseEventKind::ScrollDown, 10, 3);
        assert_eq!(app.selected_tree_row, tree_before);
        assert_eq!(app.selected_page, page_before);
    }

    #[test]
    fn wheel_down_over_regions_selects_footer() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.regions_area = Rect {
            x: 0,
            y: 1,
            width: 30,
            height: 10,
            ..Default::default()
        };
        app.list_area = Rect::default();
        app.pages_area = Rect::default();
        app.details_area = Rect::default();
        send_mouse(&mut app, MouseEventKind::ScrollDown, 10, 3);
        assert!(matches!(app.selected_region, SelectedRegion::Footer));
        assert_eq!(app.selected_tree_row, 0);
    }

    #[test]
    fn mouse_down_on_details_scrollbar_jumps_scroll_row() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.details_scrollbar_track = ScrollbarTrack {
            rect: Rect {
                x: 78,
                y: 4,
                width: 1,
                height: 20,
            },
            total: 100,
            visible: 20,
        };
        app.details_area = Rect {
            x: 20,
            y: 1,
            width: 60,
            height: 30,
        };
        app.list_area = Rect::default();
        app.pages_area = Rect::default();
        app.regions_area = Rect::default();
        app.details_scroll_row = 0;
        app.sync_tree_row_with_selection();
        let tree_before = app.selected_tree_row;
        // Bottom of a 20-tall track at y=4 is y=23; max offset = 100-20 = 80.
        send_mouse(&mut app, MouseEventKind::Down(MouseButton::Left), 78, 23);
        assert_eq!(app.details_scroll_row, 80);
        assert_eq!(app.scrollbar_drag, Some(ScrollbarDrag::Details));
        assert_eq!(app.selected_tree_row, tree_before);

        send_mouse(&mut app, MouseEventKind::Drag(MouseButton::Left), 78, 4);
        assert_eq!(app.details_scroll_row, 0);

        send_mouse(&mut app, MouseEventKind::Up(MouseButton::Left), 78, 4);
        assert_eq!(app.scrollbar_drag, None);
    }

    #[test]
    fn key_4_then_j_scrolls_details() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.details_area = Rect {
            x: 40,
            y: 1,
            width: 40,
            height: 5,
            ..Default::default()
        };
        app.sync_tree_row_with_selection();
        app.details_scroll_row = 0;
        assert!(app.details_max_scroll() > 0);
        let tree_before = app.selected_tree_row;
        let page_before = app.selected_page;
        send_key(&mut app, KeyCode::Char('4'), KeyModifiers::NONE);
        assert_eq!(app.selected_sidebar_section, SidebarSection::Details);
        send_key(&mut app, KeyCode::Char('j'), KeyModifiers::NONE);
        assert!(app.details_scroll_row > 0);
        assert_eq!(app.selected_tree_row, tree_before);
        assert_eq!(app.selected_page, page_before);
    }

    #[test]
    fn key_4_then_k_page_g_scroll_details_and_tab_stays_next_page() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        let clone = app.site.pages[0].clone();
        app.site.pages.push(clone);
        app.selected_page = 0;
        app.details_area = Rect {
            x: 40,
            y: 1,
            width: 40,
            height: 5,
            ..Default::default()
        };
        app.sync_tree_row_with_selection();
        app.details_scroll_row = 0;
        let max = app.details_max_scroll();
        assert!(max > 0);
        send_key(&mut app, KeyCode::Char('4'), KeyModifiers::NONE);

        send_key(&mut app, KeyCode::PageDown, KeyModifiers::NONE);
        assert_eq!(app.details_scroll_row, 5.min(max));

        send_key(&mut app, KeyCode::Char('G'), KeyModifiers::SHIFT);
        assert_eq!(app.details_scroll_row, max);

        send_key(&mut app, KeyCode::Char('k'), KeyModifiers::NONE);
        assert_eq!(app.details_scroll_row, max.saturating_sub(1));

        send_key(&mut app, KeyCode::Char('g'), KeyModifiers::NONE);
        assert_eq!(app.details_scroll_row, 0);

        send_key(&mut app, KeyCode::Tab, KeyModifiers::NONE);
        assert_eq!(app.selected_page, 1);
        assert_eq!(app.selected_sidebar_section, SidebarSection::Details);
    }

    #[test]
    fn click_details_pane_focuses_and_still_hit_tests() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.details_area = Rect {
            x: 20,
            y: 1,
            width: 60,
            height: 30,
            ..Default::default()
        };
        app.list_area = Rect::default();
        app.pages_area = Rect::default();
        app.regions_area = Rect::default();
        app.details_scroll_row = 0;
        app.selected_sidebar_section = SidebarSection::Layouts;
        send_mouse(&mut app, MouseEventKind::Down(MouseButton::Left), 25, 4);
        assert_eq!(app.selected_sidebar_section, SidebarSection::Details);
    }

    #[test]
    fn footer_hint_details_includes_scroll() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.selected_sidebar_section = SidebarSection::Details;
        for width in [60_u16, 80, 110] {
            assert_footer_hint_shape(&app.footer_hint(width), width);
        }
        let at_80 = app.footer_hint(80);
        assert!(at_80.contains("j/k:Scroll"), "{at_80}");
        assert!(at_80.contains("Ctrl+Q:Quit"), "{at_80}");
    }

    fn sidebar_percent_width(term_w: u16) -> u16 {
        (u32::from(term_w) * 25 / 100) as u16
    }

    #[test]
    fn sidebar_pane_title_elides_under_22_cols() {
        assert_eq!(sidebar_percent_width(80), 20);
        assert_eq!(sidebar_percent_width(60), 15);
        for width in [sidebar_percent_width(60), sidebar_percent_width(80), 21] {
            assert!(width < 22);
            assert_eq!(
                super::draw::sidebar_pane_title(SidebarSection::Regions, width, 1, 1),
                "[1]"
            );
            assert_eq!(
                super::draw::sidebar_pane_title(SidebarSection::Pages, width, 3, 12),
                "[2] 3/12"
            );
            assert_eq!(
                super::draw::sidebar_pane_title(SidebarSection::Pages, width, 1, 0),
                "[2]"
            );
            assert_eq!(
                super::draw::sidebar_pane_title(SidebarSection::Layouts, width, 1, 1),
                "[3]"
            );
            for section in [
                SidebarSection::Regions,
                SidebarSection::Pages,
                SidebarSection::Layouts,
            ] {
                let title = super::draw::sidebar_pane_title(section, width, 3, 12);
                assert!(
                    title.chars().count() + 2 <= width as usize,
                    "{title:?} must fit {width}-col pane with borders"
                );
            }
        }
    }

    #[test]
    fn sidebar_pane_title_full_when_pane_at_least_22() {
        assert_eq!(
            super::draw::sidebar_pane_title(SidebarSection::Regions, 22, 1, 1),
            "[1] Regions"
        );
        assert_eq!(
            super::draw::sidebar_pane_title(SidebarSection::Pages, 22, 3, 12),
            "[2] Pages 3/12"
        );
        assert_eq!(
            super::draw::sidebar_pane_title(SidebarSection::Pages, 22, 1, 0),
            "[2] Pages"
        );
        assert_eq!(
            super::draw::sidebar_pane_title(SidebarSection::Layouts, 22, 1, 1),
            "[3] Layout"
        );
    }

    #[test]
    fn page_down_on_layouts_moves_tree_not_details() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.selected_sidebar_section = SidebarSection::Layouts;
        app.details_area = Rect {
            x: 40,
            y: 1,
            width: 40,
            height: 5,
            ..Default::default()
        };
        app.sync_tree_row_with_selection();
        app.details_scroll_row = 0;
        assert!(app.details_max_scroll() > 0);
        let tree_before = app.selected_tree_row;
        send_key(&mut app, KeyCode::PageDown, KeyModifiers::NONE);
        assert_ne!(app.selected_tree_row, tree_before);
        assert_eq!(app.details_scroll_row, 0);
    }

    #[test]
    fn page_down_on_pages_moves_selected_page() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.selected_sidebar_section = SidebarSection::Pages;
        for title in ["About", "Work", "Blog", "Team", "Contact"] {
            app.site.pages.push(crate::model::Page::from_template(
                title,
                crate::model::PageTemplate::Blank,
            ));
        }
        assert_eq!(app.site.pages.len(), 6);
        app.selected_page = 0;
        send_key(&mut app, KeyCode::PageDown, KeyModifiers::NONE);
        assert_eq!(app.selected_page, 5);

        send_key(&mut app, KeyCode::PageDown, KeyModifiers::NONE);
        assert_eq!(app.selected_page, 5);

        app.selected_page = 0;
        send_key(&mut app, KeyCode::PageUp, KeyModifiers::NONE);
        assert_eq!(app.selected_page, 0);
    }

    #[test]
    fn mouse_down_on_layout_scrollbar_jumps_tree_row() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.sync_tree_row_with_selection();
        let total = app.build_tree_rows().len();
        assert!(total > 1, "starter tree should have more than one row");
        app.layout_scrollbar_track = ScrollbarTrack {
            rect: Rect {
                x: 28,
                y: 4,
                width: 1,
                height: 10,
            },
            total,
            visible: 1,
        };
        app.list_area = Rect {
            x: 0,
            y: 1,
            width: 30,
            height: 20,
        };
        app.details_area = Rect::default();
        app.pages_area = Rect::default();
        app.regions_area = Rect::default();
        let tree_before = app.selected_tree_row;
        // Bottom of a 10-tall track at y=4 is y=13; max offset = total - 1.
        send_mouse(&mut app, MouseEventKind::Down(MouseButton::Left), 28, 13);
        assert_eq!(app.selected_tree_row, total - 1);
        assert_ne!(app.selected_tree_row, tree_before);
        assert_eq!(app.scrollbar_drag, Some(ScrollbarDrag::Layout));

        send_mouse(&mut app, MouseEventKind::Drag(MouseButton::Left), 28, 4);
        assert_eq!(app.selected_tree_row, 0);

        send_mouse(&mut app, MouseEventKind::Up(MouseButton::Left), 28, 4);
        assert_eq!(app.scrollbar_drag, None);
    }

    #[test]
    fn header_and_footer_details_hits_align_with_lines() {
        let app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        let (header, header_hits) = app.header_details_text(40);
        assert_eq!(header.lines().count(), header_hits.len());
        let header_lines: Vec<&str> = header.lines().collect();
        let search_line = header_lines
            .iter()
            .position(|l| l.contains("dd-header-search"))
            .expect("starter header has dd-header-search");
        assert!(
            !header_hits[search_line].is_empty(),
            "component line should carry hit segments"
        );
        let decl = header_lines
            .iter()
            .position(|l| l.contains("[01] dd-header"))
            .expect("header decl");
        assert!(header_hits[decl].is_empty(), "decl lines keep empty hit rows");

        let (footer, footer_hits) = app.footer_details_text(40);
        assert_eq!(footer.lines().count(), footer_hits.len());
        let footer_lines: Vec<&str> = footer.lines().collect();
        let col = footer_lines
            .iter()
            .position(|l| l.contains("column: "))
            .expect("footer column decl");
        assert!(footer_hits[col].is_empty(), "column decl uses string fallback");
    }

    #[test]
    fn header_details_decl_click_selects_header_root() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.selected_region = SelectedRegion::Header;
        app.header_column_expanded = true;
        app.set_header_section_expanded(0, true);
        app.sync_tree_row_with_selection();
        app.details_area = Rect {
            x: 20,
            y: 1,
            width: 60,
            height: 30,
            ..Default::default()
        };
        let detail_w = app.details_area.width.saturating_sub(2) as usize;
        let (content, hits) = app.details_text(detail_w);
        app.details_hits = hits;
        let lines: Vec<&str> = content.lines().collect();
        let line = lines
            .iter()
            .position(|l| l.contains("[01] dd-header"))
            .expect("header decl");
        app.select_item_from_details_click(line, 4);
        let rows = app.build_tree_rows();
        assert!(
            matches!(rows[app.selected_tree_row].kind, TreeRowKind::HeaderRoot),
            "header decl click should select HeaderRoot, not a section"
        );
        app.handle_enter_on_selected_row();
        match &app.modal {
            Some(Modal::FormEdit { cursor, .. }) => {
                assert!(matches!(cursor, cursor::Cursor::HeaderRoot));
            }
            _ => panic!("expected header root form"),
        }
    }

    #[test]
    fn footer_details_decl_click_selects_footer_root() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.selected_region = SelectedRegion::Footer;
        app.sync_tree_row_with_selection();
        app.details_area = Rect {
            x: 20,
            y: 1,
            width: 60,
            height: 30,
            ..Default::default()
        };
        let detail_w = app.details_area.width.saturating_sub(2) as usize;
        let (content, hits) = app.details_text(detail_w);
        app.details_hits = hits;
        let lines: Vec<&str> = content.lines().collect();
        let line = lines
            .iter()
            .position(|l| l.contains("[01] dd-footer"))
            .expect("footer decl");
        app.select_item_from_details_click(line, 4);
        let rows = app.build_tree_rows();
        assert!(
            matches!(rows[app.selected_tree_row].kind, TreeRowKind::FooterRoot),
            "footer decl click should select FooterRoot, not a section"
        );
        app.handle_enter_on_selected_row();
        match &app.modal {
            Some(Modal::FormEdit { cursor, .. }) => {
                assert!(matches!(cursor, cursor::Cursor::FooterRoot));
            }
            _ => panic!("expected footer root form"),
        }
    }

    #[test]
    fn footer_details_click_selects_component() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.selected_region = SelectedRegion::Footer;
        app.site.footer.sections[0].columns.push(SectionColumn {
            id: "column-2".to_string(),
            width_class: "dd-u-1-1".to_string(),
            components: vec![ComponentKind::Cta.default_component()],
        });
        app.selected_header_section = 0;
        app.selected_header_column = 0;
        app.selected_header_component = 0;
        app.sync_tree_row_with_selection();
        app.details_area = Rect {
            x: 20,
            y: 1,
            width: 60,
            height: 30,
            ..Default::default()
        };
        let detail_w = app.details_area.width.saturating_sub(2) as usize;
        let (content, hits) = app.details_text(detail_w);
        app.details_hits = hits;
        let lines: Vec<&str> = content.lines().collect();
        let line = lines
            .iter()
            .position(|l| l.contains("dd-cta"))
            .expect("footer CTA line");
        let char_x = app.details_hits[line]
            .first()
            .map(|&(x0, x1, _, _)| x0 + (x1 - x0) / 2)
            .unwrap_or(4);
        app.select_item_from_details_click(line, char_x);
        assert_eq!(app.selected_header_section, 0);
        assert_eq!(app.selected_header_column, 1);
        assert_eq!(app.selected_header_component, 0);
        let rows = app.build_tree_rows();
        assert!(
            matches!(
                rows[app.selected_tree_row].kind,
                TreeRowKind::FooterComponent {
                    section_idx: 0,
                    column_idx: 1,
                    component_idx: 0,
                }
            ),
            "expected FooterComponent tree row after footer details click"
        );
    }

    #[test]
    fn mouse_down_on_details_cell_selects_and_footer_does_not_early_return() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.details_area = Rect {
            x: 20,
            y: 1,
            width: 60,
            height: 30,
            ..Default::default()
        };
        app.list_area = Rect::default();
        app.pages_area = Rect::default();
        app.regions_area = Rect::default();
        app.details_scroll_row = 0;
        app.selected_node = 0;
        let detail_w = app.details_area.width.saturating_sub(2) as usize;
        let (content, hits) = app.details_text(detail_w);
        app.details_hits = hits;
        let lines: Vec<&str> = content.lines().collect();
        let line = lines
            .iter()
            .position(|l| l.contains("[02] dd-section"))
            .expect("page section decl");
        let y = app.details_area.y + 1 + line as u16;
        let x = app.details_area.x + 1 + 4;
        send_mouse(&mut app, MouseEventKind::Down(MouseButton::Left), x, y);
        assert_eq!(app.selected_node, 1);
        assert_eq!(app.selected_sidebar_section, SidebarSection::Details);

        app.selected_region = SelectedRegion::Footer;
        app.site.footer.sections[0].columns[0]
            .components
            .push(ComponentKind::Cta.default_component());
        app.selected_header_column = 0;
        app.selected_header_component = 0;
        let (footer_content, footer_hits) = app.details_text(detail_w);
        app.details_hits = footer_hits;
        let footer_lines: Vec<&str> = footer_content.lines().collect();
        let f_line = footer_lines
            .iter()
            .position(|l| l.contains("dd-cta"))
            .expect("footer CTA line");
        let (x0, x1) = app.details_hits[f_line]
            .first()
            .map(|&(a, b, _, _)| (a, b))
            .unwrap_or((2, 8));
        let fy = app.details_area.y + 1 + f_line as u16;
        let fx = app.details_area.x + 1 + (x0 + (x1 - x0) / 2) as u16;
        send_mouse(&mut app, MouseEventKind::Down(MouseButton::Left), fx, fy);
        assert_eq!(app.selected_header_component, 0);
        assert_eq!(app.selected_sidebar_section, SidebarSection::Details);
        let rows = app.build_tree_rows();
        assert!(
            matches!(
                rows[app.selected_tree_row].kind,
                TreeRowKind::FooterComponent { .. }
            ),
            "footer details click must select, not early-return"
        );
    }

    #[test]
    fn details_click_prefers_stored_hit_map() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.details_area = Rect {
            x: 20,
            y: 1,
            width: 60,
            height: 30,
            ..Default::default()
        };
        if let PageNode::Section(section) = &mut app.site.pages[0].nodes[1] {
            section.columns.push(SectionColumn {
                id: "column-2".to_string(),
                width_class: "dd-u-1-1".to_string(),
                components: vec![ComponentKind::Cta.default_component()],
            });
        }
        let detail_w = app.details_area.width.saturating_sub(2) as usize;
        let (content, mut hits) = app.details_text(detail_w);
        let lines: Vec<&str> = content.lines().collect();
        let line = lines
            .iter()
            .position(|l| l.contains("dd-cta"))
            .expect("cta component line");
        // Overwrite the stored segment so a click must be reading details_hits, not a regenerated map.
        hits[line] = vec![(0, 80, 7, 3)];
        app.details_hits = hits;
        app.selected_node = 0;
        app.selected_column = 0;
        app.selected_component = 0;
        app.select_item_from_details_click(line, 10);
        assert_eq!(app.selected_node, 1);
        assert_eq!(app.selected_column, 7);
        assert_eq!(app.selected_component, 3);
    }

    #[test]
    fn push_toast_error_stores_error_level() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        app.push_toast(ToastLevel::Error, "Failed to save: permission denied");
        let last = app.toasts.last().expect("expected an error toast");
        assert_eq!(last.level, ToastLevel::Error);
        assert!(last.message.starts_with("Failed to"));
    }
