# Graph Report - .  (2026-07-05)

## Corpus Check
- 55 files · ~128,024 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 916 nodes · 3275 edges · 41 communities (36 shown, 5 thin omitted)
- Extraction: 98% EXTRACTED · 2% INFERRED · 0% AMBIGUOUS · INFERRED: 51 edges (avg confidence: 0.86)
- Token cost: 350,729 input · 0 output

## Community Hubs (Navigation)
- [[_COMMUNITY_Cursor Component→Form Mapping|Cursor: Component→Form Mapping]]
- [[_COMMUNITY_ExportPreview Flow Tests|Export/Preview Flow Tests]]
- [[_COMMUNITY_Modal Event Dispatch|Modal Event Dispatch]]
- [[_COMMUNITY_ModalCursor Rendering|Modal/Cursor Rendering]]
- [[_COMMUNITY_HTML Renderer|HTML Renderer]]
- [[_COMMUNITY_Collection Item + Tree State|Collection Item + Tree State]]
- [[_COMMUNITY_Header Copy + Detail Panels|Header Copy + Detail Panels]]
- [[_COMMUNITY_Parent-ClassAOS Cyclers|Parent-Class/AOS Cyclers]]
- [[_COMMUNITY_Textarea + Blueprint Labels|Textarea + Blueprint Labels]]
- [[_COMMUNITY_Component Spec Docs|Component Spec Docs]]
- [[_COMMUNITY_App ColumnScroll State|App Column/Scroll State]]
- [[_COMMUNITY_Model Parent-AOS Defaults|Model: Parent-AOS Defaults]]
- [[_COMMUNITY_Model CardMedia Structs|Model: Card/Media Structs]]
- [[_COMMUNITY_Tree Navigation + Click Events|Tree Navigation + Click Events]]
- [[_COMMUNITY_Save Component Changes|Save Component Changes]]
- [[_COMMUNITY_PreviewSaveBackup IO|Preview/Save/Backup IO]]
- [[_COMMUNITY_Site Validation|Site Validation]]
- [[_COMMUNITY_CLI + Storage Round-trip|CLI + Storage Round-trip]]
- [[_COMMUNITY_AddOpen Node + Modal Openers|Add/Open Node + Modal Openers]]
- [[_COMMUNITY_Theme + Visual Shell Docs|Theme + Visual Shell Docs]]
- [[_COMMUNITY_Architecture + Feature Plans|Architecture + Feature Plans]]
- [[_COMMUNITY_AppTheme + Text Wrapping|AppTheme + Text Wrapping]]
- [[_COMMUNITY_Form-Edit Component Tests|Form-Edit Component Tests]]
- [[_COMMUNITY_Core Typed Model Structs|Core Typed Model Structs]]
- [[_COMMUNITY_Autosave Debounce + Toasts|Autosave Debounce + Toasts]]
- [[_COMMUNITY_TUI Editor Screenshot (png)|TUI Editor Screenshot (png)]]
- [[_COMMUNITY_TUI Screenshot (webp)|TUI Screenshot (webp)]]
- [[_COMMUNITY_Accordion Enum Model|Accordion Enum Model]]
- [[_COMMUNITY_Alert Enum Model|Alert Enum Model]]
- [[_COMMUNITY_Navigation Enum Model|Navigation Enum Model]]
- [[_COMMUNITY_Install Script|Install Script]]
- [[_COMMUNITY_Page Template + Duplicate|Page Template + Duplicate]]
- [[_COMMUNITY_HeadSEO Enum Model|Head/SEO Enum Model]]
- [[_COMMUNITY_Alternating Enum Model|Alternating Enum Model]]
- [[_COMMUNITY_Filmstrip Enum Model|Filmstrip Enum Model]]
- [[_COMMUNITY_Banner Enum Model|Banner Enum Model]]
- [[_COMMUNITY_Card Type Enum|Card Type Enum]]
- [[_COMMUNITY_Navigation Kind Enum|Navigation Kind Enum]]
- [[_COMMUNITY_TUI Overview + Keybindings|TUI Overview + Keybindings]]
- [[_COMMUNITY_Parent-Only Templates|Parent-Only Templates]]
- [[_COMMUNITY_Content Hierarchy Concept|Content Hierarchy Concept]]

## God Nodes (most connected - your core abstractions)
1. `App` - 285 edges
2. `EditFormState` - 71 edges
3. `send_key()` - 57 edges
4. `normalize_section_columns()` - 47 edges
5. `contains()` - 35 edges
6. `HeroAos` - 34 edges
7. `component_index()` - 29 edges
8. `SectionComponent` - 28 edges
9. `apply_edit_form_to_component()` - 27 edges
10. `render_section()` - 25 edges

## Surprising Connections (you probably didn't know these)
- `Session trash + undo delete (App.deleted_pages)` --semantically_similar_to--> `Storage + Autosave (2s debounce + .backup)`  [INFERRED] [semantically similar]
  docs/superpowers/plans/2026-04-22-page-crud.md → Architecture.md
- `Canonical Theme Schema (YAML)` --semantically_similar_to--> `Canonical Color Tokens`  [INFERRED] [semantically similar]
  THEME_STRUCTURE_STANDARD.md → LDNDDEV_TUI_VISUAL_STANDARD.md
- `F2:Theme Modal Visual Mock` --references--> `F2:Theme Modal Design Spec`  [INFERRED]
  .superpowers/brainstorm/326007-1782047649/content/f2-credits-modal-mock.html → docs/superpowers/specs/2026-06-21-f2-theme-modal-design.md
- `dd-cta Component` --semantically_similar_to--> `dd-hero Component`  [INFERRED] [semantically similar]
  components/dd-cta.md → components/dd-hero.md
- `In-TUI Export Implementation Plan` --references--> `Renderer (typed model → HTML via handlebars)`  [EXTRACTED]
  docs/superpowers/plans/2026-04-23-in-tui-export.md → Architecture.md

## Import Cycles
- None detected.

## Hyperedges (group relationships)
- **Header/Footer Allowed Column Components** — components_dd_header_component, components_dd_footer_component, components_dd_image_component, components_dd_rich_text_component, components_dd_navigation_component [EXTRACTED 1.00]
- **Parent/Child items[] Collection Component Family** — components_dd_accordion_component, components_dd_card_component, components_dd_slider_component, components_dd_milestones_component, components_dd_filmstrip_component, components_dd_alternating_component [INFERRED 0.85]
- **Global Site Chrome Stack (head/header/footer)** — components_dd_head_component, components_dd_header_component, components_dd_footer_component, components_dd_section_component [INFERRED 0.75]
- **Pre-1.0 CMS Punch List feature rollout** — docs_superpowers_specs_2026_04_22_pre_1_0_cms_punch_list_design_punch_list, docs_superpowers_plans_2026_04_22_page_crud_page_crud, docs_superpowers_plans_2026_04_23_validation_modal_validation_modal, docs_superpowers_plans_2026_04_23_in_tui_export_export_flow, docs_superpowers_plans_2026_04_28_autosave_backup_autosave_backup, docs_superpowers_plans_2026_04_29_image_picker_image_picker, docs_superpowers_plans_2026_04_28_preview_preview_flow [EXTRACTED 1.00]
- **F2:Theme modal design → mock → spec → plan** — _superpowers_brainstorm_326007_1782047649_content_design_approaches_f2_theme_design, _superpowers_brainstorm_326007_1782047649_content_f2_credits_modal_mock_f2_theme_visual_mock, docs_superpowers_specs_2026_06_21_f2_theme_modal_design_f2_theme_design, docs_superpowers_plans_2026_06_21_f2_theme_modal_f2_theme_modal, ldnddev_tui_visual_standard_f2_credits_modal [EXTRACTED 0.95]
- **Theme system definition across standards + config + shell** — theme_structure_standard_theme_schema, ldnddev_tui_visual_standard_canonical_color_tokens, dd_siteforge_theme_default_config, architecture_theme_visual_shell [INFERRED 0.85]

## Communities (41 total, 5 thin omitted)

### Community 0 - "Cursor: Component→Form Mapping"
Cohesion: 0.07
Nodes (72): HashMap, auto_scroll_for_focus(), accordion_to_form_state(), alert_to_form_state(), alternating_to_form_state(), apply_accordion_values(), apply_alert_values(), apply_alternating_values() (+64 more)

### Community 1 - "Export/Preview Flow Tests"
Cohesion: 0.09
Nodes (52): KeyCode, KeyModifiers, Rect, begin_export_flow_on_clean_starter_without_export_dir_opens_path_prompt(), begin_export_flow_with_invalid_site_opens_validation_modal(), begin_export_flow_with_saved_export_dir_commits_directly(), current_page_slug_for_preview_returns_selected_page_slug(), double_click_page_in_nodes_panel_switches_to_page_and_opens_head_edit() (+44 more)

### Community 2 - "Modal Event Dispatch"
Cohesion: 0.09
Nodes (24): KeyEvent, CtaTarget, HeroImageClass, SectionClass, breakpoint_for_panel_chars(), component_edit_group_for_mode(), contains(), filter_pages() (+16 more)

### Community 3 - "Modal/Cursor Rendering"
Cohesion: 0.10
Nodes (13): Frame, byte_index_for_char(), centered_rect(), component_search_haystack(), ComponentKind, cursor_from_row_col(), cursor_row_col(), input_lines_preserve() (+5 more)

### Community 4 - "HTML Renderer"
Cohesion: 0.16
Nodes (46): aos_token(), hero_to_json(), html_id_safe_from_title(), inline_markdown_to_html(), link_target_token(), looks_like_html_block(), markdown_to_html(), navigation_class_token() (+38 more)

### Community 5 - "Collection Item + Tree State"
Cohesion: 0.11
Nodes (5): child_link_target_to_str(), component_index(), cta_class_to_str(), parent_data_aos_to_str(), section_columns_ref()

### Community 6 - "Header Copy + Detail Panels"
Cohesion: 0.08
Nodes (33): Box, FnOnce, app_selects_header_copy_from_defaults_at_construction(), card_items_ascii_lines(), choose_header_copy(), chrono_like_format(), component_form(), ComponentPickerState (+25 more)

### Community 7 - "Parent-Class/AOS Cyclers"
Cohesion: 0.09
Nodes (4): F, nested_index(), next_child_link_target(), next_parent_data_aos()

### Community 8 - "Textarea + Blueprint Labels"
Cohesion: 0.12
Nodes (35): HashSet, app_with_component(), component_blueprint_label(), component_label(), details_panel_single_click_selects_double_click_edits(), drill_stack_len(), ensure_page_section_ids(), form_focused_field_id() (+27 more)

### Community 9 - "Component Spec Docs"
Cohesion: 0.11
Nodes (31): dd-accordion Component, dd-alert Component, dd-alternating Component, dd-banner Component, dd-blockquote Component, dd-card Component, dd-cta Component, dd-filmstrip Component (+23 more)

### Community 10 - "App Column/Scroll State"
Cohesion: 0.12
Nodes (4): RefCell, App, TreeRow, TreeRowKind

### Community 11 - "Model: Parent-AOS Defaults"
Cohesion: 0.13
Nodes (21): CtaClass, DdBlockquote, DdHeaderMenu, DdHeaderSearch, default_accordion_parent_data_aos(), default_alert_parent_data_aos(), default_alternating_parent_data_aos(), default_banner_parent_data_aos() (+13 more)

### Community 12 - "Model: Card/Media Structs"
Cohesion: 0.15
Nodes (25): CardItem, CardLinkTarget, DdCard, DdCta, DdImage, DdMilestones, DdModal, DdRichText (+17 more)

### Community 14 - "Save Component Changes"
Cohesion: 0.26
Nodes (5): EditField, normalize_section_columns(), parse_child_link_target(), parse_cta_class(), parse_parent_data_aos()

### Community 15 - "Preview/Save/Backup IO"
Cohesion: 0.14
Nodes (14): B, backup_path_for(), begin_preview_flow_with_invalid_site_opens_validation_modal(), begin_preview_flow_without_export_dir_opens_path_prompt(), copy_dir_recursive(), display_relative_path(), manual_save_writes_backup_alongside_main_file(), normalize_relative_path() (+6 more)

### Community 16 - "Site Validation"
Cohesion: 0.26
Nodes (21): check_local_image(), collect_component_image_refs(), collect_image_refs(), detects_duplicate_page_slug(), detects_missing_hero_required_fields(), is_valid_url(), Option, Path (+13 more)

### Community 17 - "CLI + Storage Round-trip"
Cohesion: 0.19
Nodes (16): P, Cli, Command, main(), Option, Result, String, export_dir_round_trips_through_save_and_load() (+8 more)

### Community 19 - "Theme + Visual Shell Docs"
Cohesion: 0.14
Nodes (18): F2:Theme Design Proposals (visual companion), F2:Theme Modal Visual Mock, Theme + Visual Shell, Theme Token sourcing convention (self.theme.*), dd_siteforge default theme config, header_quotes + choose_header_copy randomization, TUI Header + Shell Consistency Plan, build_theme_text + color_to_hex helpers (+10 more)

### Community 20 - "Architecture + Feature Plans"
Cohesion: 0.13
Nodes (18): Renderer (typed model → HTML via handlebars), Storage + Autosave (2s debounce + .backup), TUI Loop (App + Modal + draw/event loop), Validation (validate_site + validate_site_with_root), Four-point Modal Plumbing convention, Page CRUD Implementation Plan, Session trash + undo delete (App.deleted_pages), Page.slug_locked field + slug lock on save (+10 more)

### Community 21 - "AppTheme + Text Wrapping"
Cohesion: 0.17
Nodes (13): Color, Default, AppTheme, build_help_text(), build_theme_text(), color_to_hex(), count_wrapped_lines(), ModalConfig (+5 more)

### Community 22 - "Form-Edit Component Tests"
Cohesion: 0.21
Nodes (11): app_with_card(), app_with_cta(), dd_card_keyflow_add_remove_items_with_min_guard(), dd_cta_edits_apply_in_page_region(), dd_cta_edits_in_footer_region(), dd_cta_edits_in_header_region(), dd_cta_form_edit_opens_on_enter(), dd_cta_form_edit_tab_and_enum_cycle() (+3 more)

### Community 23 - "Core Typed Model Structs"
Cohesion: 0.31
Nodes (11): DdFooter, DdHeader, DdHero, DdSection, Page, PageNode, Vec, SectionColumn (+3 more)

### Community 24 - "Autosave Debounce + Toasts"
Cohesion: 0.20
Nodes (10): Instant, dirty_since_does_not_reset_on_subsequent_mutations(), editing_a_page_title_marks_app_dirty(), tick_autosave_does_nothing_when_clean(), tick_autosave_does_nothing_when_dirty_but_no_path(), tick_autosave_holds_off_within_debounce_window(), tick_autosave_writes_when_dirty_and_debounce_elapsed(), Toast (+2 more)

### Community 25 - "TUI Editor Screenshot (png)"
Cohesion: 0.27
Nodes (12): Collection Item Actions (A add, X remove), dd-hero Component, dd-section (section-1) Component, Details Panel, Keybindings (F1 help, q quit, s save, / insert, Enter edit, Space expand), Layout Tree Panel, Nodes Panel, Home Page Blueprint (+4 more)

### Community 26 - "TUI Screenshot (webp)"
Cohesion: 0.23
Nodes (12): Column 1 (left) dd-u-12-24, Column 2 (right) dd-u-12-24, dd-hero Component, dd-section (section-1) Component, Details Panel, Header dd | Page: Home, Layout Panel [3], Nodes Panel [2] (+4 more)

### Community 27 - "Accordion Enum Model"
Cohesion: 0.17
Nodes (12): AccordionClass, AccordionItem, AccordionType, DdAccordion, default_accordion_parent_class(), default_accordion_parent_type(), accordion_class_to_str(), accordion_type_to_str() (+4 more)

### Community 28 - "Alert Enum Model"
Cohesion: 0.18
Nodes (11): AlertClass, AlertType, DdAlert, default_alert_parent_class(), default_alert_parent_type(), alert_class_to_str(), alert_type_to_str(), next_alert_class() (+3 more)

### Community 29 - "Navigation Enum Model"
Cohesion: 0.18
Nodes (11): DdNavigation, default_navigation_parent_class(), default_navigation_parent_type(), NavigationClass, NavigationType, navigation_class_to_str(), navigation_type_to_str(), next_navigation_class() (+3 more)

### Community 30 - "Install Script"
Cohesion: 0.47
Nodes (9): cyan(), do_install(), do_uninstall(), green(), red(), require(), install.sh script, usage() (+1 more)

### Community 31 - "Page Template + Duplicate"
Cohesion: 0.24
Nodes (7): page_from_template_blank_has_no_nodes(), page_from_template_duplicate_deep_clones_and_appends_copy_suffix(), page_from_template_hero_only_has_one_hero_node(), page_from_template_hero_plus_section_has_hero_then_section(), PageTemplate, Self, slug_from_title()

### Community 32 - "Head/SEO Enum Model"
Cohesion: 0.22
Nodes (9): DdHead, default_head_robots(), default_head_schema_type(), RobotsDirective, SchemaType, next_robots_directive(), next_schema_type(), parse_robots_directive() (+1 more)

### Community 33 - "Alternating Enum Model"
Cohesion: 0.29
Nodes (7): AlternatingItem, AlternatingType, DdAlternating, default_alternating_parent_type(), alternating_type_to_str(), next_alternating_type(), parse_alternating_type()

### Community 34 - "Filmstrip Enum Model"
Cohesion: 0.29
Nodes (7): DdFilmstrip, default_filmstrip_parent_type(), FilmstripItem, FilmstripType, filmstrip_type_to_str(), next_filmstrip_type(), parse_filmstrip_type()

### Community 35 - "Banner Enum Model"
Cohesion: 0.33
Nodes (6): BannerClass, DdBanner, default_banner_parent_class(), banner_class_to_str(), next_banner_class(), parse_banner_class()

### Community 36 - "Card Type Enum"
Cohesion: 0.40
Nodes (5): CardType, default_card_parent_type(), card_type_to_str(), next_card_type(), parse_card_type()

### Community 37 - "Navigation Kind Enum"
Cohesion: 0.40
Nodes (5): default_navigation_child_kind(), NavigationKind, navigation_kind_to_str(), next_navigation_kind(), parse_navigation_kind()

## Knowledge Gaps
- **27 isolated node(s):** `dd-alert Component`, `dd-banner Component`, `dd-blockquote Component`, `dd-filmstrip Component`, `dd-modal Component` (+22 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **5 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `App` connect `App Column/Scroll State` to `Export/Preview Flow Tests`, `Modal Event Dispatch`, `Modal/Cursor Rendering`, `Collection Item + Tree State`, `Header Copy + Detail Panels`, `Parent-Class/AOS Cyclers`, `Textarea + Blueprint Labels`, `Model: Card/Media Structs`, `Tree Navigation + Click Events`, `Save Component Changes`, `Preview/Save/Backup IO`, `Add/Open Node + Modal Openers`, `AppTheme + Text Wrapping`, `Form-Edit Component Tests`, `Core Typed Model Structs`, `Autosave Debounce + Toasts`?**
  _High betweenness centrality (0.300) - this node is a cross-community bridge._
- **Why does `Site` connect `Core Typed Model Structs` to `Cursor: Component→Form Mapping`, `Export/Preview Flow Tests`, `HTML Renderer`, `App Column/Scroll State`, `Model: Parent-AOS Defaults`, `Model: Card/Media Structs`, `Preview/Save/Backup IO`, `Site Validation`, `CLI + Storage Round-trip`, `Page Template + Duplicate`?**
  _High betweenness centrality (0.089) - this node is a cross-community bridge._
- **Why does `EditFormState` connect `Cursor: Component→Form Mapping` to `Export/Preview Flow Tests`, `App Column/Scroll State`, `Modal/Cursor Rendering`, `Header Copy + Detail Panels`?**
  _High betweenness centrality (0.083) - this node is a cross-community bridge._
- **What connects `dd-alert Component`, `dd-banner Component`, `dd-blockquote Component` to the rest of the system?**
  _28 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Cursor: Component→Form Mapping` be split into smaller, more focused modules?**
  _Cohesion score 0.06942053930005737 - nodes in this community are weakly interconnected._
- **Should `Export/Preview Flow Tests` be split into smaller, more focused modules?**
  _Cohesion score 0.08591466978375219 - nodes in this community are weakly interconnected._
- **Should `Modal Event Dispatch` be split into smaller, more focused modules?**
  _Cohesion score 0.08525506638714186 - nodes in this community are weakly interconnected._