use std::collections::HashSet;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;

use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers, MouseButton,
    MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Terminal;
use serde::Deserialize;

use crate::model::{PageNode, SectionColumn, Site};
use crate::storage::save_site;

const AUTOSAVE_DEBOUNCE: std::time::Duration = std::time::Duration::from_secs(2);

pub mod cursor;
pub mod editform;

pub fn run_tui(site: Site, path: Option<PathBuf>) -> anyhow::Result<()> {
    let theme = match AppTheme::load() {
        Ok(theme) => theme,
        Err(err) => {
            eprintln!("failed to load theme config, using defaults: {err}");
            AppTheme::default()
        }
    };

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(site, path, theme);
    let run_res = app.run(&mut terminal);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    run_res
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SidebarSection {
    Regions,
    Pages,
    Layouts,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SelectedRegion {
    Page,
    Header,
    Footer,
}

struct App {
    site: Site,
    theme: AppTheme,
    selected_page: usize,
    selected_node: usize,
    selected_tree_row: usize,
    selected_column: usize,
    selected_component: usize,
    selected_nested_item: usize,
    selected_sidebar_section: SidebarSection,
    selected_region: SelectedRegion,
    selected_header_section: usize,
    selected_header_column: usize,
    selected_header_component: usize,
    /// True when the `[HEAD]` row is the active tree selection. Needed
    /// because page-head has no `selected_*` index of its own; without this
    /// flag, `sync_tree_row_with_selection` would always fall back to the
    /// first Hero/Section row and make `[HEAD]` unreachable via j/k.
    page_head_selected: bool,
    /// Session trash — deleted pages pushed here for `u` undo.
    /// Not persisted. Capped at 20 entries (oldest drops off).
    deleted_pages: Vec<crate::model::Page>,
    /// Title captured while the TemplatePicker is open after the title prompt.
    /// None outside of the add-page flow.
    pending_new_page_title: Option<String>,
    /// Ephemeral bottom-right notifications; expire ~5s after `shown_at`.
    toasts: Vec<Toast>,
    /// True when in-memory site differs from `last_saved_json`.
    dirty: bool,
    /// Instant of the first mutation since `last_saved_json` was synced.
    /// `None` while clean.
    dirty_since: Option<std::time::Instant>,
    /// JSON snapshot of the site at the most recent successful disk write.
    /// Used both for dirty detection and for skipping no-op autosaves.
    last_saved_json: String,
    list_area: Rect,
    details_area: Rect,
    details_scroll_row: usize,
    status: String,
    path: Option<PathBuf>,
    should_quit: bool,
    save_prompt_open: bool,
    save_input: String,
    input_mode: Option<InputMode>,
    input_buffer: String,
    input_cursor: usize,
    multiline_value_area: Option<Rect>,
    multiline_scroll_row: usize,
    component_picker: Option<ComponentPickerState>,
    edit_modal: Option<EditModalState>,
    /// New unified modal system - will replace the above during migration
    modal: Option<Modal>,
    component_kind: ComponentKind,
    show_help: bool,
    /// Vertical scroll offset (in rows) for the F1 help modal.
    help_scroll: u16,
    /// Maximum legal `help_scroll` value, recomputed every render based on
    /// the current modal area + wrapped row count. Read by event handlers
    /// to clamp scroll keystrokes without needing the frame.
    help_scroll_max: u16,
    /// Per-frame cache of (field_idx, input_area_rect) for whichever
    /// multi-field modal is currently rendered. Click-to-focus lookups
    /// search this cache; render writes it. Empty when no eligible modal
    /// is open.
    modal_field_areas: std::cell::RefCell<Vec<(usize, Rect)>>,
    /// FormEdit modal that was paused when the image picker opened on top
    /// of it. Restored when the picker closes (Esc or after a commit).
    paused_form_edit_modal: Option<Modal>,
    expanded_sections: HashSet<(usize, usize)>,
    expanded_accordion_items: HashSet<(usize, usize, usize, usize)>,
    expanded_alternating_items: HashSet<(usize, usize, usize, usize)>,
    expanded_card_items: HashSet<(usize, usize, usize, usize)>,
    expanded_filmstrip_items: HashSet<(usize, usize, usize, usize)>,
    expanded_milestones_items: HashSet<(usize, usize, usize, usize)>,
    expanded_slider_items: HashSet<(usize, usize, usize, usize)>,
    header_column_expanded: bool,
    header_components_expanded: HashSet<usize>,
}

// ============================================================================
// UNIFIED MODAL SYSTEM
// ============================================================================

/// All modal types in the application
enum Modal {
    /// Multi-field edit modal (Hero, Section, etc.)
    Edit {
        title: String,
        fields: Vec<EditField>,
        selected_field: usize,
        scroll_offset: usize,
        visible_fields: usize,
        on_save: Box<dyn FnOnce(&mut App, &[EditField])>,
    },
    /// Component picker for inserting components
    ComponentPicker { query: String, selected: usize },
    /// Save file dialog
    SavePrompt { path: String },
    /// Single field edit (legacy, will be migrated to Edit)
    SingleField {
        mode: InputMode,
        buffer: String,
        cursor: usize,
        multiline: bool,
    },
    /// Template picker for adding a new page.
    TemplatePicker {
        /// Index within the template option list that is currently highlighted.
        selected: usize,
    },
    /// Title entry prompt shown before the TemplatePicker when adding a new page.
    NewPageTitlePrompt {
        title: String,
    },
    /// Path entry prompt shown when exporting the site to a local directory.
    ExportPathPrompt {
        path: String,
    },
    /// Path entry prompt shown when previewing the site in a browser.
    PreviewPathPrompt {
        path: String,
    },
    /// Title-edit prompt shown when renaming an existing page.
    RenamePagePrompt {
        title: String,
        page_idx: usize,
    },
    /// Generic yes/no confirmation prompt.
    ConfirmPrompt {
        message: String,
        on_confirm: ConfirmKind,
    },
    /// Scrollable list of validation errors.
    ValidationErrors {
        errors: Vec<String>,
        scroll_offset: usize,
    },
    /// File picker rooted at `./source/images/`.
    ImagePicker {
        state: ImagePickerState,
    },
    /// Page picker — lists site pages and writes `/<slug>` to a URL field.
    PagePicker {
        state: PagePickerState,
    },
    /// Unified form editor: all fields of a component rendered together,
    /// Tab moves between fields, Left/Right cycles enums, Ctrl+S saves via
    /// `cursor::apply_edit_form_to_component`.
    ///
    /// When `drill_stack` is non-empty, the editor is currently inside a
    /// nested SubForm item; Ctrl+S/Esc return to the outer parent rather
    /// than committing to the model.
    FormEdit {
        state: editform::EditFormState,
        cursor: cursor::Cursor,
        cursor_pos: usize, // text cursor within focused field's string
        drill_stack: Vec<DrillFrame>,
        scroll_offset: u16, // vertical row scroll within the form content
    },
}

/// One frame of drill-down context: parent form state plus the (subform id,
/// item idx) we entered from. When we return, we copy the current state into
/// `parent_state.sub_state[subform_field_id][item_idx]` and make the parent
/// the active state again.
struct DrillFrame {
    parent_state: editform::EditFormState,
    parent_cursor_pos: usize,
    parent_scroll_offset: u16,
    subform_field_id: String,
    item_idx: usize,
}

/// Common modal result returned from event handling
enum ModalResult {
    /// Stay open, continue handling events
    Continue,
    /// Close modal with success
    CloseSuccess,
    /// Close modal with cancel
    CloseCancel,
}

/// The action to execute when a ConfirmPrompt is confirmed.
#[derive(Debug, Clone)]
enum ConfirmKind {
    DeletePage,
}

/// Live state of an open image picker. `root` and `cwd` are absolute
/// paths; `cwd` is always equal to or a descendant of `root`.
#[derive(Debug, Clone)]
struct ImagePickerState {
    root: std::path::PathBuf,
    cwd: std::path::PathBuf,
    filter: String,
    selected: usize,
    binding: ImagePickBinding,
}

#[derive(Debug, Clone)]
enum ImagePickBinding {
    /// Write back into the FormEdit modal's currently-focused URL field.
    FormEditField { field_id: String },
}

/// Live state of an open page picker. Lists site pages by title; on Enter
/// writes `/<slug>` into the bound URL field.
#[derive(Debug, Clone)]
struct PagePickerState {
    /// Snapshot of (slug, title) pairs at modal-open time. The picker
    /// doesn't track site mutations while open — it operates on a frozen
    /// list and the underlying site is back-burnered while paused.
    pages: Vec<(String, String)>,
    filter: String,
    selected: usize,
    binding: PagePickBinding,
}

#[derive(Debug, Clone)]
enum PagePickBinding {
    /// Write back into the FormEdit modal's currently-focused URL field.
    FormEditField { field_id: String },
}

/// Visual/semantic class of a toast notification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ToastLevel {
    Success,
    Info,
    Warning,
}

/// A transient bottom-right notification. Expires ~5s after `shown_at`.
#[derive(Debug, Clone)]
struct Toast {
    level: ToastLevel,
    message: String,
    shown_at: std::time::Instant,
}

/// Unified modal configuration
struct ModalConfig {
    width_percent: u16,
    height_percent: u16,
    show_scrollbar: bool,
    footer_text: String,
}

impl Default for ModalConfig {
    fn default() -> Self {
        Self {
            width_percent: 80,
            height_percent: 80,
            show_scrollbar: true,
            footer_text: "Tab/Up/Down: navigate | Ctrl+S: save | Esc: cancel".to_string(),
        }
    }
}

// Legacy structs kept for backward compatibility during migration
struct ComponentPickerState {
    query: String,
    selected: usize,
}

#[derive(Clone)]
struct EditField {
    label: String,
    value: String,
    buffer: String,
    cursor: usize,
    is_multiline: bool,
    rows: u16, // Height of the input field (1 for single-line, 3/5/etc for textarea)
}

// Deprecated: will be removed once fully migrated to Modal enum
struct EditModalState {
    title: String,
    fields: Vec<EditField>,
    selected_field: usize,
    scroll_offset: usize,
    visible_fields: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum InputMode {
    EditHeroImage,
    EditHeroClass,
    EditHeroAos,
    EditHeroCustomCss,
    EditHeroTitle,
    EditHeroSubtitle,
    EditHeroCopy,
    EditHeroCtaText,
    EditHeroCtaLink,
    EditHeroCtaTarget,
    EditHeroCtaText2,
    EditHeroCtaLink2,
    EditHeroCtaTarget2,
    EditSectionId,
    EditSectionTitle,
    EditSectionClass,
    EditColumnId,
    EditColumnWidthClass,
    EditBannerClass,
    EditBannerDataAos,
    EditBannerImageUrl,
    EditBannerImageAlt,
    EditCtaClass,
    EditCtaImageUrl,
    EditCtaImageAlt,
    EditCtaDataAos,
    EditCtaTitle,
    EditCtaSubtitle,
    EditCtaCopy,
    EditCtaLinkUrl,
    EditCtaLinkTarget,
    EditCtaLinkLabel,
    EditBlockquoteDataAos,
    EditBlockquoteImageUrl,
    EditBlockquoteImageAlt,
    EditBlockquotePersonsName,
    EditBlockquotePersonsTitle,
    EditBlockquoteCopy,
    EditAccordionType,
    EditAccordionClass,
    EditAccordionAos,
    EditAccordionGroupName,
    EditAccordionFirstTitle,
    EditAccordionFirstContent,
    EditAlternatingType,
    EditAlternatingClass,
    EditAlternatingDataAos,
    EditAlternatingItemImage,
    EditAlternatingItemImageAlt,
    EditAlternatingItemTitle,
    EditAlternatingItemCopy,
    EditAlertType,
    EditAlertClass,
    EditAlertDataAos,
    EditAlertTitle,
    EditAlertCopy,
    EditCardType,
    EditCardDataAos,
    EditCardWidth,
    EditCardItemImageUrl,
    EditCardItemImageAlt,
    EditCardItemTitle,
    EditCardItemSubtitle,
    EditCardItemCopy,
    EditCardItemLinkUrl,
    EditCardItemLinkTarget,
    EditCardItemLinkLabel,
    EditFilmstripType,
    EditFilmstripDataAos,
    EditFilmstripItemImageUrl,
    EditFilmstripItemImageAlt,
    EditFilmstripItemTitle,
    EditMilestonesDataAos,
    EditMilestonesWidth,
    EditMilestonesItemPercentage,
    EditMilestonesItemTitle,
    EditMilestonesItemSubtitle,
    EditMilestonesItemCopy,
    EditMilestonesItemLinkUrl,
    EditMilestonesItemLinkTarget,
    EditMilestonesItemLinkLabel,
    EditModalTitle,
    EditModalCopy,
    EditSliderTitle,
    EditSliderItemTitle,
    EditSliderItemCopy,
    EditSliderItemLinkUrl,
    EditSliderItemLinkTarget,
    EditSliderItemLinkLabel,
    EditSliderItemImageUrl,
    EditSliderItemImageAlt,
    EditHeaderId,
    EditHeaderClass,
    EditHeaderCustomCss,
    EditHeaderColumnId,
    EditHeaderColumnWidthClass,
    EditHeaderPlaceholderContent,
    // New section components (dd-image, dd-rich_text, dd-navigation, dd-header-search, dd-header-menu)
    EditImageUrl,
    EditImageAlt,
    EditImageLinkUrl,
    EditImageLinkTarget,
    EditImageDataAos,
    EditRichTextClass,
    EditRichTextCopy,
    EditRichTextDataAos,
    EditNavigationType,
    EditNavigationClass,
    EditNavigationWidth,
    EditNavigationDataAos,
    EditNavigationItemKind,
    EditNavigationItemLabel,
    EditNavigationItemUrl,
    EditNavigationItemTarget,
    EditNavigationItemCss,
    EditHeaderSearchWidth,
    EditHeaderSearchDataAos,
    EditHeaderMenuWidth,
    EditHeaderMenuDataAos,
    // Global chrome (footer + page head)
    EditFooterId,
    EditFooterCustomCss,
    EditHeadTitle,
    EditHeadMetaDescription,
    EditHeadCanonicalUrl,
    EditHeadRobots,
    EditHeadSchemaType,
    EditHeadOgTitle,
    EditHeadOgDescription,
    EditHeadOgImage,
}

#[derive(Clone, Copy)]
enum ComponentKind {
    Hero,
    Section,
    Banner,
    Cta,
    Blockquote,
    Accordion,
    Alternating,
    Card,
    Filmstrip,
    Milestones,
    Modal,
    Slider,
    Alert,
    Image,
    RichText,
    Navigation,
    HeaderSearch,
    HeaderMenu,
}

#[derive(Clone, Copy)]
struct AppTheme {
    // Core UI backgrounds
    background: Color,
    panel_background: Color,
    popup_background: Color,
    // Text colors
    foreground: Color,
    muted: Color,
    disabled: Color,
    text_inverse: Color,
    text_labels: Color,
    text_active_focus: Color,
    modal_labels: Color,
    modal_text: Color,
    // Accent colors
    title: Color,
    active: Color,
    // Border colors
    border: Color,
    border_active: Color,
    // Input field colors (split border vs text, default vs focus)
    input_border_default: Color,
    input_border_focus: Color,
    input_text_default: Color,
    input_text_focus: Color,
    cursor: Color,
    // Scrollbar colors
    scrollbar: Color,
    scrollbar_hover: Color,
    // Selection colors
    selected_background: Color,
    selected_foreground: Color,
    // Semantic colors
    success: Color,
    warning: Color,
    error: Color,
    info: Color,
    // File-role colors (THEME_STRUCTURE_STANDARD.md section 8)
    folders: Color,
    files: Color,
    links: Color,
    // Backwards-compat aliases (used by older code paths that haven't been
    // migrated to the split border/text inputs yet).
    input_default: Color,
    input_focus: Color,
}

#[derive(Debug, Deserialize)]
struct ThemeFile {
    colors: PaletteFile,
}

#[derive(Debug, Deserialize)]
struct PaletteFile {
    // Core backgrounds
    base_background: String,
    body_background: Option<String>,
    modal_background: Option<String>,
    // Text colors — new names match THEME_STRUCTURE_STANDARD.md, old names
    // kept as aliases for in-tree theme files.
    #[serde(alias = "text")]
    text_primary: String,
    #[serde(alias = "subtext0")]
    text_secondary: Option<String>,
    text_disabled: Option<String>,
    text_inverse: Option<String>,
    text_labels: Option<String>,
    text_active_focus: Option<String>,
    modal_labels: Option<String>,
    modal_text: Option<String>,
    // Selection
    selected_background: String,
    // Borders
    border_default: String,
    border_active: Option<String>,
    // Scrollbar
    scrollbar: Option<String>,
    scrollbar_hover: Option<String>,
    // Input field colors — split for border vs text, default vs focus
    input_border_default: Option<String>,
    input_border_focus: Option<String>,
    input_text_default: Option<String>,
    input_text_focus: Option<String>,
    cursor: Option<String>,
    // Backwards-compat: plain input_default/input_focus still accepted
    input_default: Option<String>,
    input_focus: Option<String>,
    // Accent
    active: Option<String>,
    // Semantic
    success: Option<String>,
    warning: Option<String>,
    error: Option<String>,
    info: Option<String>,
    // File roles (currently unused in the TUI, kept for schema completeness)
    #[serde(default)]
    folders: Option<String>,
    #[serde(default)]
    files: Option<String>,
    #[serde(default)]
    links: Option<String>,
}

#[derive(Clone, Copy)]
struct TreeRow {
    kind: TreeRowKind,
}

#[derive(Clone, Copy)]
enum TreeRowKind {
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

// ============================================================================
// UNIFIED MODAL RENDERING AND EVENT HANDLING
// ============================================================================

impl App {
    /// Check if any modal is currently open
    fn is_modal_open(&self) -> bool {
        self.modal.is_some()
            || self.edit_modal.is_some()
            || self.component_picker.is_some()
            || self.input_mode.is_some()
            || self.save_prompt_open
    }

    /// Main modal rendering entry point
    fn render_modal(&self, frame: &mut ratatui::Frame) {
        // During migration, check legacy modals first
        if let Some(modal) = &self.modal {
            self.render_unified_modal(frame, modal);
        } else if let Some(edit_modal) = &self.edit_modal {
            // Legacy edit modal - will be migrated
            self.render_edit_modal_legacy(frame, edit_modal);
        } else if let Some(picker) = &self.component_picker {
            self.render_component_picker_legacy(frame, picker);
        } else if self.save_prompt_open {
            self.render_save_prompt_legacy(frame);
        } else if self.input_mode.is_some() {
            self.render_input_mode_legacy(frame);
        }
    }

    /// Render the new unified modal
    fn render_unified_modal(&self, frame: &mut ratatui::Frame, modal: &Modal) {
        match modal {
            Modal::Edit {
                title,
                fields,
                selected_field,
                scroll_offset,
                visible_fields,
                ..
            } => {
                self.render_edit_modal_unified(
                    frame,
                    title,
                    fields,
                    *selected_field,
                    *scroll_offset,
                    *visible_fields,
                );
            }
            Modal::ComponentPicker { query, selected } => {
                self.render_component_picker_unified(frame, query, *selected);
            }
            Modal::SavePrompt { path } => {
                self.render_save_prompt_unified(frame, path);
            }
            Modal::SingleField {
                mode,
                buffer,
                cursor,
                multiline,
            } => {
                self.render_single_field_unified(frame, *mode, buffer, *cursor, *multiline);
            }
            Modal::FormEdit {
                state,
                cursor_pos,
                scroll_offset,
                ..
            } => {
                self.render_form_edit_modal(frame, state, *cursor_pos, *scroll_offset);
            }
            Modal::TemplatePicker { selected } => {
                self.render_template_picker_modal(frame, *selected);
            }
            Modal::NewPageTitlePrompt { title } => {
                self.render_new_page_title_prompt(frame, title);
            }
            Modal::ExportPathPrompt { path } => {
                self.render_export_path_prompt(frame, path);
            }
            Modal::PreviewPathPrompt { path } => {
                self.render_preview_path_prompt(frame, path);
            }
            Modal::RenamePagePrompt { title, page_idx } => {
                self.render_rename_page_prompt(frame, title, *page_idx);
            }
            Modal::ConfirmPrompt { message, .. } => {
                self.render_confirm_prompt(frame, message);
            }
            Modal::ValidationErrors { errors, scroll_offset } => {
                self.render_validation_errors_modal(frame, errors, *scroll_offset);
            }
            Modal::ImagePicker { state } => self.render_image_picker_modal(frame, state),
            Modal::PagePicker { state } => self.render_page_picker_modal(frame, state),
        }
    }

    /// Render the unified component-editor modal per the team mockup:
    /// solid popup background, title on the top border, help text at the
    /// top of the content area, each field rendered as "Label:" + a
    /// 1px-bordered input box. Content scrolls vertically; a peach
    /// scrollbar on the right indicates position when scrollable.
    fn render_form_edit_modal(
        &self,
        frame: &mut ratatui::Frame,
        state: &editform::EditFormState,
        cursor_pos: usize,
        scroll_offset: u16,
    ) {
        let area = centered_rect(70, 80, frame.area());
        frame.render_widget(Clear, area);

        // Outer border + title with solid modal background.
        let outer = Block::default()
            .title(format!(" Edit Item -- {} ", state.form.title))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.modal_labels))
            .title_style(
                Style::default()
                    .fg(self.theme.modal_labels)
                    .add_modifier(Modifier::BOLD),
            )
            .style(Style::default().bg(self.theme.popup_background));
        let inner = outer.inner(area);
        frame.render_widget(outer, area);
        if inner.height < 3 || inner.width < 6 {
            return;
        }

        // Help row at the very top of the content area.
        let help_rect = Rect::new(inner.x, inner.y, inner.width, 1);
        let help_text = "Tab/Up/Down: navigate | Ctrl+S: save | Esc: cancel";
        frame.render_widget(
            Paragraph::new(help_text).style(
                Style::default()
                    .fg(self.theme.modal_labels)
                    .bg(self.theme.popup_background)
                    .add_modifier(Modifier::BOLD),
            ),
            help_rect,
        );

        // Content area begins 2 rows below (help + spacer). Reserve 1 col for scrollbar.
        if inner.height < 4 {
            return;
        }
        let content_top = inner.y.saturating_add(2);
        let content_height = inner.height.saturating_sub(2);
        let scrollbar_col = inner
            .x
            .saturating_add(inner.width.saturating_sub(1));
        let content_rect = Rect::new(inner.x, content_top, inner.width.saturating_sub(1), content_height);

        // Build virtual field layout: each entry holds (field_idx, label_y, box_y, box_height).
        #[derive(Clone, Copy)]
        struct Slot {
            idx: usize,
            label_y: u16,
            box_y: u16,
            box_height: u16,
        }
        let mut slots: Vec<Slot> = Vec::new();
        let mut virt_y: u16 = 0;
        for (idx, field) in state.form.fields.iter().enumerate() {
            if !state.field_visible(field) {
                continue;
            }
            let content_rows: u16 = match &field.kind {
                editform::FieldKind::Textarea { rows, .. } => (*rows).max(1),
                editform::FieldKind::SubForm { .. } => {
                    let items_len = state
                        .sub_state
                        .get(field.id)
                        .map(|v| v.len())
                        .unwrap_or(0);
                    // header line + one row per item (at least 1 placeholder row)
                    (1 + items_len.max(1)) as u16
                }
                _ => 1,
            };
            let box_height = content_rows.saturating_add(2); // +2 for borders
            let label_y = virt_y;
            let box_y = virt_y.saturating_add(1);
            slots.push(Slot {
                idx,
                label_y,
                box_y,
                box_height,
            });
            virt_y = virt_y.saturating_add(1 + box_height + 1); // label + box + blank separator
        }
        let total_height = virt_y;
        let max_scroll = total_height.saturating_sub(content_height);
        let scroll = scroll_offset.min(max_scroll);

        // Refresh the per-frame click-to-focus cache for this modal.
        self.modal_field_areas.borrow_mut().clear();

        for slot in &slots {
            let field = &state.form.fields[slot.idx];
            let focused = slot.idx == state.focused_field;
            let label_screen = slot.label_y as i32 - scroll as i32;
            let box_top_screen = slot.box_y as i32 - scroll as i32;
            let box_bottom_screen = box_top_screen + slot.box_height as i32;
            // Skip entries entirely outside the content window.
            if box_bottom_screen <= 0 || label_screen >= content_height as i32 {
                continue;
            }

            // Label row.
            if label_screen >= 0 && label_screen < content_height as i32 {
                let label_rect = Rect::new(
                    content_rect.x,
                    content_rect.y + label_screen as u16,
                    content_rect.width,
                    1,
                );
                let label_color = if focused {
                    self.theme.text_active_focus
                } else {
                    self.theme.text_labels
                };
                let label_mod = if focused {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                };
                frame.render_widget(
                    Paragraph::new(format!("{}:", field.label)).style(
                        Style::default()
                            .fg(label_color)
                            .bg(self.theme.popup_background)
                            .add_modifier(label_mod),
                    ),
                    label_rect,
                );
            }

            // Input box — only drawn when fully visible so partial borders don't flash.
            if box_top_screen >= 0 && box_bottom_screen <= content_height as i32 {
                let border_color = if focused {
                    self.theme.input_border_focus
                } else {
                    self.theme.input_border_default
                };
                let box_rect = Rect::new(
                    content_rect.x,
                    content_rect.y + box_top_screen as u16,
                    content_rect.width,
                    slot.box_height,
                );
                let field_block = Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color).bg(self.theme.popup_background))
                    .style(Style::default().bg(self.theme.popup_background));
                let inner_rect = field_block.inner(box_rect);
                frame.render_widget(field_block, box_rect);
                self.modal_field_areas
                    .borrow_mut()
                    .push((slot.idx, box_rect));
                self.render_form_field_value(
                    frame,
                    field,
                    state,
                    cursor_pos,
                    focused,
                    inner_rect,
                );
            }
        }

        // Scrollbar on the right column when content exceeds window.
        if total_height > content_height {
            let track_bg = Block::default().style(Style::default().bg(self.theme.popup_background));
            frame.render_widget(
                track_bg,
                Rect::new(scrollbar_col, content_top, 1, content_height),
            );
            let thumb_height = ((content_height as u32 * content_height as u32
                / total_height.max(1) as u32) as u16)
                .max(1);
            let travel = content_height.saturating_sub(thumb_height);
            let thumb_y = if max_scroll == 0 {
                0
            } else {
                ((scroll as u32 * travel as u32) / max_scroll.max(1) as u32) as u16
            };
            let thumb = Paragraph::new(vec!["█".to_string(); thumb_height as usize].join("\n"))
                .style(Style::default().fg(self.theme.scrollbar).bg(self.theme.popup_background));
            frame.render_widget(
                thumb,
                Rect::new(scrollbar_col, content_top + thumb_y, 1, thumb_height),
            );
        }
    }

    /// Render the value portion of a form field inside the given inner rect.
    fn render_form_field_value(
        &self,
        frame: &mut ratatui::Frame,
        field: &editform::FormField,
        state: &editform::EditFormState,
        cursor_pos: usize,
        focused: bool,
        rect: Rect,
    ) {
        let text_color = if focused {
            self.theme.input_text_focus
        } else {
            self.theme.input_text_default
        };
        let value_style = Style::default()
            .fg(text_color)
            .bg(self.theme.popup_background);

        match &field.kind {
            editform::FieldKind::Text { .. } | editform::FieldKind::Url { .. } => {
                let value = state.get(field.id);
                let display = if focused {
                    render_cursor_line(value, cursor_pos)
                } else {
                    value.to_string()
                };
                frame.render_widget(Paragraph::new(display).style(value_style), rect);
            }
            editform::FieldKind::Textarea { .. } => {
                let value = state.get(field.id);
                frame.render_widget(
                    Paragraph::new(value.to_string())
                        .style(value_style)
                        .wrap(Wrap { trim: false }),
                    rect,
                );
            }
            editform::FieldKind::Enum { options, .. } => {
                let value = state.get(field.id);
                let display = format!("< {} >", value);
                let mut style = value_style;
                if !options.iter().any(|o| *o == value) {
                    style = Style::default()
                        .fg(self.theme.error)
                        .bg(self.theme.popup_background);
                }
                frame.render_widget(Paragraph::new(display).style(style), rect);
            }
            editform::FieldKind::OptionalLinkTriple { .. } => {
                // Reserved — hero migration uses 3 flat fields instead.
            }
            editform::FieldKind::SubForm {
                summary_field_id, ..
            } => {
                let items = state.sub_state.get(field.id).cloned().unwrap_or_default();
                let selected = state
                    .selected_sub_item
                    .get(field.id)
                    .copied()
                    .unwrap_or(0);
                let mut lines: Vec<String> = Vec::new();
                lines.push(format!(
                    "{} item(s) — A add · X remove · Enter edit",
                    items.len()
                ));
                if items.is_empty() {
                    lines.push("  (no items; press A to add)".to_string());
                } else {
                    for (i, item) in items.iter().enumerate() {
                        let summary = item
                            .values
                            .get(*summary_field_id)
                            .cloned()
                            .unwrap_or_default();
                        let summary = if summary.trim().is_empty() {
                            "(untitled)".to_string()
                        } else {
                            summary
                        };
                        let marker = if focused && i == selected { ">" } else { " " };
                        lines.push(format!("  {} {}. {}", marker, i + 1, summary));
                    }
                }
                frame.render_widget(Paragraph::new(lines.join("\n")).style(value_style), rect);
            }
        }
    }
}

/// Returns the (top_y, bottom_y) virtual rows of the focused field within
/// the form's layout. Used for auto-scrolling the form editor to keep the
/// focused field visible.
fn focused_field_virtual_rows(state: &editform::EditFormState) -> (u16, u16) {
    let mut y: u16 = 0;
    for (idx, field) in state.form.fields.iter().enumerate() {
        if !state.field_visible(field) {
            continue;
        }
        let content_rows: u16 = match &field.kind {
            editform::FieldKind::Textarea { rows, .. } => (*rows).max(1),
            editform::FieldKind::SubForm { .. } => {
                let items_len = state
                    .sub_state
                    .get(field.id)
                    .map(|v| v.len())
                    .unwrap_or(0);
                (1 + items_len.max(1)) as u16
            }
            _ => 1,
        };
        let box_height = content_rows.saturating_add(2);
        let entry_height = 1u16.saturating_add(box_height).saturating_add(1);
        if idx == state.focused_field {
            return (y, y.saturating_add(1).saturating_add(box_height));
        }
        y = y.saturating_add(entry_height);
    }
    (0, 0)
}

/// Compute a new scroll offset that keeps the focused field in view given
/// a conservative estimate of the content window height. 16 rows covers the
/// common case of an 80% / 80% modal on a standard terminal.
fn auto_scroll_for_focus(state: &editform::EditFormState, current_scroll: u16) -> u16 {
    const ESTIMATED_VISIBLE: u16 = 16;
    let (top, bottom) = focused_field_virtual_rows(state);
    if top < current_scroll {
        top
    } else if bottom > current_scroll.saturating_add(ESTIMATED_VISIBLE) {
        bottom.saturating_sub(ESTIMATED_VISIBLE)
    } else {
        current_scroll
    }
}

/// Insert a block cursor `▋` at `cursor_pos` in `value`. Used by the form
/// editor to show where typing will land in a single-line text field.
fn render_cursor_line(value: &str, cursor_pos: usize) -> String {
    let chars: Vec<char> = value.chars().collect();
    let pos = cursor_pos.min(chars.len());
    let mut out = String::with_capacity(value.len() + 3);
    for (i, ch) in chars.iter().enumerate() {
        if i == pos {
            out.push('▋');
        }
        out.push(*ch);
    }
    if pos >= chars.len() {
        out.push('▋');
    }
    out
}

impl App {
    fn __marker_after_form_editor_helpers(&self) {}

    /// Unified edit modal renderer with variable-height textarea support
    fn render_edit_modal_unified(
        &self,
        frame: &mut ratatui::Frame,
        title: &str,
        fields: &[EditField],
        selected_field: usize,
        scroll_offset: usize,
        _visible_fields: usize,
    ) {
        let area = centered_rect(95, 90, frame.area());
        frame.render_widget(Clear, area);

        // Draw modal frame
        let modal_block = Block::default()
            .title(format!("Edit - {}", title))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.border))
            .title_style(
                Style::default()
                    .fg(self.theme.title)
                    .add_modifier(Modifier::BOLD),
            );

        frame.render_widget(modal_block.clone(), area);
        let inner = modal_block.inner(area);

        // Calculate layout
        let header_height = 1u16;
        let footer_height = 1u16;
        let scrollbar_width = 2u16;

        // Header
        let header =
            Paragraph::new("Edit fields below:").style(Style::default().fg(self.theme.muted));
        frame.render_widget(
            header,
            Rect {
                x: inner.x,
                y: inner.y,
                width: inner.width,
                height: header_height,
            },
        );

        // Calculate content area
        let content_height = inner.height.saturating_sub(header_height + footer_height);
        let content_width = inner.width.saturating_sub(scrollbar_width + 2);

        // Calculate field heights and determine visible range based on pixel rows
        // Field height = label (1) + input rows + padding (1)
        let field_heights: Vec<u16> = fields
            .iter()
            .map(|f| 1 + f.rows + 1) // label + rows + bottom padding
            .collect();

        let total_content_height: u16 = field_heights.iter().sum();
        let show_scrollbar = total_content_height > content_height;

        // Find which fields to render based on scroll offset (in rows)
        let mut current_y = 0u16;
        let mut visible_start = 0usize;
        let mut visible_end = fields.len();
        let mut y_offsets = Vec::new();

        // Convert scroll_offset (field index) to pixel offset
        let scroll_y: u16 = field_heights.iter().take(scroll_offset).sum();

        for (idx, height) in field_heights.iter().enumerate() {
            let field_bottom = current_y + height;
            let is_visible =
                field_bottom > scroll_y && current_y < scroll_y.saturating_add(content_height);

            if is_visible {
                if y_offsets.is_empty() {
                    visible_start = idx;
                }
                y_offsets.push((idx, current_y.saturating_sub(scroll_y)));
                visible_end = idx + 1;
            }
            current_y += height;
        }

        // Render visible fields
        let mut cursor_pos: Option<(u16, u16)> = None;
        // Refresh the per-frame click-to-focus cache for this modal.
        self.modal_field_areas.borrow_mut().clear();

        for (idx, rel_y) in y_offsets {
            let field = &fields[idx];
            let is_selected = idx == selected_field;
            let y_offset = header_height + rel_y;

            // Label — text_labels for default, text_active_focus for active.
            let label_color = if is_selected {
                self.theme.text_active_focus
            } else {
                self.theme.text_labels
            };
            let label = Paragraph::new(format!("{}:", field.label))
                .style(Style::default().fg(label_color).bg(self.theme.popup_background));
            frame.render_widget(
                label,
                Rect {
                    x: inner.x + 1,
                    y: inner.y + y_offset,
                    width: content_width,
                    height: 1,
                },
            );

            // Input box border — input_border_default / input_border_focus.
            let border_color = if is_selected {
                self.theme.input_border_focus
            } else {
                self.theme.input_border_default
            };

            let input_height = field.rows + 2; // rows inside + top/bottom border

            // For multiline textareas, show scrolling content
            let lines: Vec<&str> = field.buffer.lines().collect();
            let visible_lines: Vec<String> =
                if field.is_multiline && lines.len() > field.rows as usize {
                    // Show last N lines that fit, or calculate scroll based on cursor position
                    let cursor_line = field.buffer[..field.buffer.len().min(field.cursor)]
                        .lines()
                        .count()
                        .saturating_sub(1);
                    let start_line = cursor_line.saturating_sub(field.rows as usize - 1);
                    lines
                        .iter()
                        .skip(start_line)
                        .take(field.rows as usize)
                        .map(|s| s.to_string())
                        .collect()
                } else {
                    lines.iter().map(|s| s.to_string()).collect()
                };

            let display_text = if visible_lines.is_empty() {
                " ".to_string()
            } else {
                visible_lines.join("\n")
            };

            // Input text — input_text_default / input_text_focus.
            let text_color = if is_selected {
                self.theme.input_text_focus
            } else {
                self.theme.input_text_default
            };
            let input_box = Paragraph::new(display_text)
                .style(
                    Style::default()
                        .fg(text_color)
                        .bg(self.theme.popup_background),
                )
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .style(Style::default().bg(self.theme.popup_background))
                        .border_style(Style::default().fg(border_color)),
                )
                .wrap(ratatui::widgets::Wrap { trim: false });

            let input_area = Rect {
                x: inner.x + 1,
                y: inner.y + y_offset + 1,
                width: content_width,
                height: input_height,
            };
            frame.render_widget(input_box, input_area);
            self.modal_field_areas
                .borrow_mut()
                .push((idx, input_area));

            // Position cursor for selected field
            if is_selected {
                // Calculate cursor position within the visible text
                let text_before_cursor = &field.buffer[..field.buffer.len().min(field.cursor)];
                let cursor_line = text_before_cursor.lines().count().saturating_sub(1);
                let line_start_pos = text_before_cursor
                    .rfind('\n')
                    .map(|pos| pos + 1)
                    .unwrap_or(0);
                let col_in_line = field.cursor.saturating_sub(line_start_pos);

                // Adjust for scrolling
                let visible_start_line = if field.is_multiline && lines.len() > field.rows as usize
                {
                    cursor_line.saturating_sub(field.rows as usize - 1)
                } else {
                    0
                };
                let visible_line = cursor_line.saturating_sub(visible_start_line);

                let cursor_x =
                    input_area.x + 1 + (col_in_line as u16).min(input_area.width.saturating_sub(3));
                let cursor_y = input_area.y + 1 + visible_line as u16;

                if cursor_y < input_area.y + input_height - 1 {
                    cursor_pos = Some((cursor_x, cursor_y));
                }
            }
        }

        // Scrollbar
        if show_scrollbar {
            let scrollbar_x = inner.x + inner.width.saturating_sub(2);
            let scrollbar_top = header_height + 1;
            let scrollbar_height = inner
                .height
                .saturating_sub(header_height + footer_height + 2);

            // Track
            for y_offset in 0..scrollbar_height {
                frame.render_widget(
                    Paragraph::new("│").style(Style::default().fg(self.theme.border)),
                    Rect {
                        x: scrollbar_x,
                        y: inner.y + scrollbar_top + y_offset,
                        width: 1,
                        height: 1,
                    },
                );
            }

            // Thumb
            let scrollbar_height_usize = scrollbar_height as usize;
            let thumb_size = ((content_height as usize * scrollbar_height_usize)
                / total_content_height as usize)
                .max(1);
            let thumb_pos: u16 = if total_content_height > content_height {
                ((scroll_y as usize * (scrollbar_height_usize.saturating_sub(thumb_size)))
                    / (total_content_height as usize - content_height as usize))
                    as u16
            } else {
                0
            };

            for i in 0..(thumb_size as u16) {
                let y = scrollbar_top + thumb_pos + i;
                if y < scrollbar_top + scrollbar_height {
                    frame.render_widget(
                        Paragraph::new("█").style(Style::default().fg(self.theme.active)),
                        Rect {
                            x: scrollbar_x,
                            y: inner.y + y,
                            width: 1,
                            height: 1,
                        },
                    );
                }
            }
        }

        // Footer
        let visible_count = visible_end.saturating_sub(visible_start);
        let footer_text = format!(
            "{}-{} of {} fields | Tab/Up/Down: navigate | Ctrl+S: save | Esc: cancel",
            visible_start + 1,
            visible_end,
            fields.len()
        );
        let footer = Paragraph::new(footer_text).style(Style::default().fg(self.theme.muted));
        frame.render_widget(
            footer,
            Rect {
                x: inner.x + 1,
                y: inner.y + inner.height.saturating_sub(footer_height),
                width: inner.width.saturating_sub(2),
                height: footer_height,
            },
        );

        if let Some((x, y)) = cursor_pos {
            // Paint a single cell with the themed `cursor` background so the
            // cursor position is legible regardless of the terminal's own
            // cursor color. The real terminal cursor is still placed on top
            // so it pulses/blinks as the user expects.
            let cursor_cell = Paragraph::new(" ").style(
                Style::default()
                    .fg(self.theme.popup_background)
                    .bg(self.theme.cursor),
            );
            frame.render_widget(
                cursor_cell,
                Rect {
                    x,
                    y,
                    width: 1,
                    height: 1,
                },
            );
            frame.set_cursor_position((x, y));
        }
    }

    /// Render scrollbar
    fn render_scrollbar(
        &self,
        frame: &mut ratatui::Frame,
        inner: Rect,
        scroll_offset: usize,
        visible_count: usize,
        total_count: usize,
        header_height: u16,
        footer_height: u16,
    ) {
        let scrollbar_x = inner.x + inner.width.saturating_sub(2);
        let scrollbar_top = header_height + 1;
        let scrollbar_height = inner
            .height
            .saturating_sub(header_height + footer_height + 2);

        // Track
        for y_offset in 0..scrollbar_height {
            frame.render_widget(
                Paragraph::new("│").style(Style::default().fg(self.theme.border)),
                Rect {
                    x: scrollbar_x,
                    y: inner.y + scrollbar_top + y_offset,
                    width: 1,
                    height: 1,
                },
            );
        }

        // Thumb
        let thumb_size = ((visible_count * scrollbar_height as usize) / total_count).max(1);
        let thumb_pos = if total_count > visible_count {
            ((scroll_offset * (scrollbar_height as usize - thumb_size))
                / (total_count - visible_count)) as u16
        } else {
            0
        };

        for i in 0..(thumb_size as u16) {
            let y = scrollbar_top + thumb_pos + i;
            if y < scrollbar_top + scrollbar_height {
                frame.render_widget(
                    Paragraph::new("█").style(Style::default().fg(self.theme.active)),
                    Rect {
                        x: scrollbar_x,
                        y: inner.y + y,
                        width: 1,
                        height: 1,
                    },
                );
            }
        }
    }

    /// Unified component picker renderer
    fn render_component_picker_unified(
        &self,
        frame: &mut ratatui::Frame,
        query: &str,
        selected: usize,
    ) {
        let config = ModalConfig {
            width_percent: 70,
            height_percent: 70,
            show_scrollbar: false,
            footer_text: "Type to filter | Up/Down: select | Enter: insert | Esc: cancel"
                .to_string(),
        };

        let area = centered_rect(config.width_percent, config.height_percent, frame.area());
        frame.render_widget(Clear, area);

        let modal_block = Block::default()
            .title("Insert Component")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.border))
            .title_style(
                Style::default()
                    .fg(self.theme.title)
                    .add_modifier(Modifier::BOLD),
            );

        frame.render_widget(modal_block.clone(), area);
        let inner = modal_block.inner(area);

        // Search box
        let search_text = format!("Search: {}", query);
        let search = Paragraph::new(search_text).style(Style::default().fg(self.theme.foreground));
        frame.render_widget(
            search,
            Rect {
                x: inner.x,
                y: inner.y,
                width: inner.width,
                height: 1,
            },
        );

        // Filtered list
        let filtered = self.filtered_component_kinds(query);
        let items: Vec<ListItem> = filtered
            .iter()
            .enumerate()
            .map(|(idx, kind)| {
                let style = if idx == selected {
                    Style::default()
                        .fg(self.theme.selected_foreground)
                        .bg(self.theme.selected_background)
                } else {
                    Style::default().fg(self.theme.foreground)
                };
                ListItem::new(kind.label()).style(style)
            })
            .collect();

        let list = List::new(items)
            .block(Block::default())
            .highlight_symbol("> ");

        frame.render_widget(
            list,
            Rect {
                x: inner.x,
                y: inner.y + 2,
                width: inner.width,
                height: inner.height.saturating_sub(3),
            },
        );

        // Footer
        let footer =
            Paragraph::new(&config.footer_text[..]).style(Style::default().fg(self.theme.muted));
        frame.render_widget(
            footer,
            Rect {
                x: inner.x,
                y: inner.y + inner.height.saturating_sub(1),
                width: inner.width,
                height: 1,
            },
        );
    }

    /// Unified save prompt renderer
    fn render_save_prompt_unified(&self, frame: &mut ratatui::Frame, path: &str) {
        let config = ModalConfig {
            width_percent: 70,
            height_percent: 35,
            show_scrollbar: false,
            footer_text: "Enter: save | Esc: cancel".to_string(),
        };

        let area = centered_rect(config.width_percent, config.height_percent, frame.area());
        frame.render_widget(Clear, area);

        let modal_block = Block::default()
            .title("Save Page")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.border))
            .title_style(
                Style::default()
                    .fg(self.theme.title)
                    .add_modifier(Modifier::BOLD),
            );

        frame.render_widget(modal_block.clone(), area);
        let inner = modal_block.inner(area);

        let content = format!("Save file path:\n{}\n\n{}", path, config.footer_text);
        let prompt = Paragraph::new(content).style(
            Style::default()
                .fg(self.theme.foreground)
                .bg(self.theme.popup_background),
        );

        frame.render_widget(prompt, inner);
    }

    fn render_template_picker_modal(&self, frame: &mut ratatui::Frame, selected: usize) {
        use ratatui::widgets::{List, ListItem, ListState};
        let area = centered_rect(60, 30, frame.area());
        frame.render_widget(Clear, area);
        let options = ["Blank", "Hero only", "Hero + Section", "Duplicate current"];
        let items: Vec<ListItem> = options.iter().map(|s| ListItem::new(*s)).collect();
        let mut state = ListState::default();
        state.select(Some(selected.min(options.len() - 1)));
        let list = List::new(items)
            .block(
                Block::default()
                    .title(" New page — choose template ")
                    .borders(Borders::ALL)
                    .style(
                        Style::default()
                            .fg(self.theme.modal_text)
                            .bg(self.theme.popup_background),
                    )
                    .border_style(Style::default().fg(self.theme.border_active))
                    .title_style(Style::default().fg(self.theme.modal_labels)),
            )
            .highlight_style(
                Style::default()
                    .fg(self.theme.selected_foreground)
                    .bg(self.theme.selected_background)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");
        frame.render_stateful_widget(list, area, &mut state);
    }

    fn render_new_page_title_prompt(&self, frame: &mut ratatui::Frame, title: &str) {
        self.render_single_input_modal(
            frame,
            " New page — title ",
            "Title",
            title,
            "Enter or Ctrl+S: continue  |  Esc: cancel",
        );
    }

    fn render_export_path_prompt(&self, frame: &mut ratatui::Frame, path: &str) {
        self.render_single_input_modal(
            frame,
            " Export — output directory ",
            "Path (relative to site JSON)",
            path,
            "Enter or Ctrl+S: export  |  Esc: cancel",
        );
    }

    fn render_preview_path_prompt(&self, frame: &mut ratatui::Frame, path: &str) {
        self.render_single_input_modal(
            frame,
            " Preview — output directory ",
            "Path (relative to site JSON)",
            path,
            "Enter or Ctrl+S: preview  |  Esc: cancel",
        );
    }

    fn render_rename_page_prompt(&self, frame: &mut ratatui::Frame, title: &str, _page_idx: usize) {
        self.render_single_input_modal(
            frame,
            " Rename page ",
            "Title",
            title,
            "Enter or Ctrl+S: save  |  Esc: cancel",
        );
    }

    /// Shared single-text-field modal matching the unified edit-modal look:
    /// outer bordered block with solid popup background, a label row, a
    /// 1px-bordered input box holding `value`, cursor inside the input box,
    /// and a footer hint line at the bottom.
    fn render_single_input_modal(
        &self,
        frame: &mut ratatui::Frame,
        outer_title: &str,
        label: &str,
        value: &str,
        footer_text: &str,
    ) {
        let area = centered_rect(60, 30, frame.area());
        frame.render_widget(Clear, area);

        let modal_block = Block::default()
            .title(outer_title.to_string())
            .borders(Borders::ALL)
            .style(Style::default().bg(self.theme.popup_background))
            .border_style(Style::default().fg(self.theme.border_active))
            .title_style(
                Style::default()
                    .fg(self.theme.title)
                    .add_modifier(Modifier::BOLD),
            );
        frame.render_widget(modal_block.clone(), area);
        let inner = modal_block.inner(area);

        if inner.width < 6 || inner.height < 6 {
            return;
        }

        let padding_x: u16 = 2;
        let content_x = inner.x + padding_x;
        let content_w = inner.width.saturating_sub(padding_x * 2);

        // Label row
        let label_area = Rect {
            x: content_x,
            y: inner.y + 1,
            width: content_w,
            height: 1,
        };
        // Single-input modal has exactly one field always focused, so label
        // uses the text_active_focus token.
        let label_para = Paragraph::new(format!("{}:", label)).style(
            Style::default()
                .fg(self.theme.text_active_focus)
                .bg(self.theme.popup_background),
        );
        frame.render_widget(label_para, label_area);

        // Bordered input box (3 rows tall: border + content + border)
        let input_area = Rect {
            x: content_x,
            y: inner.y + 2,
            width: content_w,
            height: 3,
        };
        let input = Paragraph::new(value)
            .style(
                Style::default()
                    .fg(self.theme.input_text_focus)
                    .bg(self.theme.popup_background),
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .style(Style::default().bg(self.theme.popup_background))
                    .border_style(Style::default().fg(self.theme.input_border_focus)),
            );
        frame.render_widget(input, input_area);

        // Cursor inside the input box (one row below top border, after last char).
        let max_x = input_area.x + input_area.width.saturating_sub(2);
        let cursor_x = (input_area.x + 1 + value.chars().count() as u16).min(max_x);
        let cursor_y = input_area.y + 1;
        // Paint a themed cursor cell so the cursor color follows the theme.
        let cursor_cell = Paragraph::new(" ").style(
            Style::default()
                .fg(self.theme.popup_background)
                .bg(self.theme.cursor),
        );
        frame.render_widget(
            cursor_cell,
            Rect {
                x: cursor_x,
                y: cursor_y,
                width: 1,
                height: 1,
            },
        );
        frame.set_cursor_position((cursor_x, cursor_y));

        // Footer hint row at the bottom of inner
        let footer_y = inner.y + inner.height.saturating_sub(2);
        let footer_area = Rect {
            x: content_x,
            y: footer_y,
            width: content_w,
            height: 1,
        };
        let footer = Paragraph::new(footer_text).style(
            Style::default()
                .fg(self.theme.muted)
                .bg(self.theme.popup_background),
        );
        frame.render_widget(footer, footer_area);
    }

    fn render_confirm_prompt(&self, frame: &mut ratatui::Frame, message: &str) {
        let area = centered_rect(70, 35, frame.area());
        frame.render_widget(Clear, area);

        let modal_block = Block::default()
            .title(" Confirm ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.border_active))
            .title_style(
                Style::default()
                    .fg(self.theme.title)
                    .add_modifier(Modifier::BOLD),
            );

        frame.render_widget(modal_block.clone(), area);
        let inner = modal_block.inner(area);

        let content = format!("{}\n\ny = confirm, n / Esc = cancel", message);
        let prompt = Paragraph::new(content).style(
            Style::default()
                .fg(self.theme.foreground)
                .bg(self.theme.popup_background),
        );

        frame.render_widget(prompt, inner);
    }

    fn render_validation_errors_modal(
        &self,
        frame: &mut ratatui::Frame,
        errors: &[String],
        scroll_offset: usize,
    ) {
        let area = centered_rect(70, 60, frame.area());
        frame.render_widget(Clear, area);

        let outer_title = format!(" Validation — {} error(s) ", errors.len());
        let modal_block = Block::default()
            .title(outer_title)
            .borders(Borders::ALL)
            .style(Style::default().bg(self.theme.popup_background))
            .border_style(Style::default().fg(self.theme.border_active))
            .title_style(
                Style::default()
                    .fg(self.theme.title)
                    .add_modifier(Modifier::BOLD),
            );
        frame.render_widget(modal_block.clone(), area);
        let inner = modal_block.inner(area);

        if inner.width < 4 || inner.height < 3 {
            return;
        }

        let padding_x: u16 = 2;
        let content_x = inner.x + padding_x;
        let content_w = inner.width.saturating_sub(padding_x * 2);
        let footer_height: u16 = 1;
        let list_height = inner.height.saturating_sub(footer_height);

        let wrapped_lines = self.wrap_validation_lines(errors, content_w as usize);
        let visible: Vec<String> = wrapped_lines
            .iter()
            .skip(scroll_offset)
            .take(list_height as usize)
            .cloned()
            .collect();

        let body = Paragraph::new(visible.join("\n")).style(
            Style::default()
                .fg(self.theme.foreground)
                .bg(self.theme.popup_background),
        );
        frame.render_widget(
            body,
            Rect {
                x: content_x,
                y: inner.y,
                width: content_w,
                height: list_height,
            },
        );

        let footer_y = inner.y + inner.height.saturating_sub(footer_height);
        let footer_area = Rect {
            x: content_x,
            y: footer_y,
            width: content_w,
            height: 1,
        };
        let footer_text = if wrapped_lines.len() > list_height as usize {
            "j / k or \u{2191} / \u{2193} to scroll  |  Enter or Esc to dismiss"
        } else {
            "Enter or Esc to dismiss"
        };
        let footer = Paragraph::new(footer_text).style(
            Style::default()
                .fg(self.theme.muted)
                .bg(self.theme.popup_background),
        );
        frame.render_widget(footer, footer_area);
    }

    /// Push a transient toast notification. Caps at 4 entries; oldest dropped.
    fn push_toast(&mut self, level: ToastLevel, message: impl Into<String>) {
        self.toasts.push(Toast {
            level,
            message: message.into(),
            shown_at: std::time::Instant::now(),
        });
        if self.toasts.len() > 4 {
            self.toasts.remove(0);
        }
    }

    /// Drop toasts older than 5 seconds. Called every render tick.
    fn prune_toasts(&mut self) {
        let now = std::time::Instant::now();
        self.toasts
            .retain(|t| now.duration_since(t.shown_at) < std::time::Duration::from_secs(5));
    }

    /// Render the toast stack at the bottom-right of `area`. Each toast is a
    /// single-line bordered box, stacked upward.
    fn render_toasts(&self, frame: &mut ratatui::Frame, area: Rect) {
        if self.toasts.is_empty() {
            return;
        }
        let toast_w: u16 = 60;
        let gap: u16 = 1;
        let max_width = area.width.saturating_sub(2);
        let width = toast_w.min(max_width);
        if width < 10 {
            return;
        }
        let right_x = area.x + area.width.saturating_sub(width + 1);
        let toast_h: u16 = 3;
        let mut y = area.y + area.height.saturating_sub(toast_h);
        for toast in self.toasts.iter().rev() {
            if y + toast_h > area.y + area.height {
                break;
            }
            let rect = Rect {
                x: right_x,
                y,
                width,
                height: toast_h,
            };
            let (glyph, accent) = match toast.level {
                ToastLevel::Success => ("✓", self.theme.success),
                ToastLevel::Info => ("ℹ", self.theme.info),
                ToastLevel::Warning => ("⚠", self.theme.warning),
            };
            frame.render_widget(Clear, rect);
            let block = Block::default()
                .borders(Borders::ALL)
                .style(Style::default().bg(self.theme.popup_background))
                .border_style(Style::default().fg(accent));
            let inner_x = rect.x + 2;
            let inner_y = rect.y + 1;
            let inner_w = rect.width.saturating_sub(4);
            frame.render_widget(block, rect);
            let text = format!("{} {}", glyph, toast.message);
            let body = Paragraph::new(text).style(
                Style::default()
                    .fg(accent)
                    .bg(self.theme.popup_background),
            );
            frame.render_widget(
                body,
                Rect {
                    x: inner_x,
                    y: inner_y,
                    width: inner_w,
                    height: 1,
                },
            );
            if y < area.y + toast_h + gap {
                break;
            }
            y = y.saturating_sub(toast_h + gap);
        }
    }

    /// Turn raw validator messages into a numbered, pre-wrapped list. Each
    /// input entry becomes one or more output rows depending on `width`.
    fn wrap_validation_lines(&self, errors: &[String], width: usize) -> Vec<String> {
        let mut out = Vec::with_capacity(errors.len());
        for (i, err) in errors.iter().enumerate() {
            let prefix = format!("{}. ", i + 1);
            let indent = " ".repeat(prefix.len());
            let body_w = width.saturating_sub(prefix.len()).max(1);
            let mut first = true;
            let mut remaining = err.as_str();
            while !remaining.is_empty() {
                let take = remaining.chars().take(body_w).count();
                let split_byte = remaining
                    .char_indices()
                    .nth(take)
                    .map(|(i, _)| i)
                    .unwrap_or(remaining.len());
                let (chunk, rest) = remaining.split_at(split_byte);
                let line = if first {
                    format!("{}{}", prefix, chunk)
                } else {
                    format!("{}{}", indent, chunk)
                };
                out.push(line);
                remaining = rest;
                first = false;
            }
        }
        out
    }

    fn handle_validation_errors_event(&mut self, key: event::KeyEvent) -> Option<ModalResult> {
        use crossterm::event::KeyCode;
        let (errors_len, scroll) = match &self.modal {
            Some(Modal::ValidationErrors { errors, scroll_offset }) => {
                (errors.len(), *scroll_offset)
            }
            _ => return Some(ModalResult::CloseCancel),
        };
        match key.code {
            KeyCode::Enter | KeyCode::Esc => {
                self.modal = None;
                Some(ModalResult::CloseSuccess)
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(Modal::ValidationErrors { scroll_offset, .. }) = self.modal.as_mut() {
                    *scroll_offset = scroll_offset.saturating_sub(1);
                }
                Some(ModalResult::Continue)
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(Modal::ValidationErrors { scroll_offset, .. }) = self.modal.as_mut() {
                    if scroll + 1 < errors_len.max(1) {
                        *scroll_offset += 1;
                    }
                }
                Some(ModalResult::Continue)
            }
            KeyCode::PageUp => {
                if let Some(Modal::ValidationErrors { scroll_offset, .. }) = self.modal.as_mut() {
                    *scroll_offset = scroll_offset.saturating_sub(5);
                }
                Some(ModalResult::Continue)
            }
            KeyCode::PageDown => {
                if let Some(Modal::ValidationErrors { scroll_offset, .. }) = self.modal.as_mut() {
                    *scroll_offset = (scroll + 5).min(errors_len.saturating_sub(1));
                }
                Some(ModalResult::Continue)
            }
            _ => Some(ModalResult::Continue),
        }
    }

    fn render_image_picker_modal(
        &self,
        frame: &mut ratatui::Frame,
        state: &ImagePickerState,
    ) {
        let area = centered_rect(70, 70, frame.area());
        frame.render_widget(Clear, area);

        let outer = Block::default()
            .title(" Pick image ")
            .borders(Borders::ALL)
            .style(Style::default().bg(self.theme.popup_background))
            .border_style(Style::default().fg(self.theme.border_active))
            .title_style(
                Style::default()
                    .fg(self.theme.title)
                    .add_modifier(Modifier::BOLD),
            );
        let inner = outer.inner(area);
        frame.render_widget(outer, area);
        if inner.height < 5 || inner.width < 10 {
            return;
        }

        let pad: u16 = 2;
        let content_x = inner.x + pad;
        let content_w = inner.width.saturating_sub(pad * 2);

        // Row 0: cwd path (relative to root).
        let rel = state.cwd.strip_prefix(&state.root).unwrap_or(&state.cwd);
        let rel_str = rel.to_string_lossy();
        let cwd_label = if rel_str.is_empty() {
            "Folder: ./source/images/".to_string()
        } else {
            format!("Folder: ./source/images/{}", rel_str)
        };
        frame.render_widget(
            Paragraph::new(cwd_label).style(
                Style::default()
                    .fg(self.theme.muted)
                    .bg(self.theme.popup_background),
            ),
            Rect::new(content_x, inner.y, content_w, 1),
        );

        // Row 1: filter input (with a trailing underscore as a fake cursor).
        let filter_label = format!("Filter: {}_", state.filter);
        frame.render_widget(
            Paragraph::new(filter_label).style(
                Style::default()
                    .fg(self.theme.text_active_focus)
                    .bg(self.theme.popup_background),
            ),
            Rect::new(content_x, inner.y + 1, content_w, 1),
        );

        // Body: filtered entry list, with vertical scroll keeping selection in view.
        let entries = list_dir_entries(&state.cwd);
        let filtered = filter_entries(&entries, &state.filter);
        let body_y = inner.y + 3;
        let body_h = inner.height.saturating_sub(5);
        let visible = body_h as usize;
        let start = if filtered.is_empty() {
            0
        } else if state.selected >= visible {
            state.selected + 1 - visible
        } else {
            0
        };

        if filtered.is_empty() {
            frame.render_widget(
                Paragraph::new("(no matches)").style(
                    Style::default()
                        .fg(self.theme.muted)
                        .bg(self.theme.popup_background),
                ),
                Rect::new(content_x, body_y, content_w, 1),
            );
        } else {
            for (i, entry) in filtered.iter().skip(start).take(visible).enumerate() {
                let row = body_y + i as u16;
                let is_selected = (start + i) == state.selected;
                let glyph = if entry.is_dir { "/" } else { " " };
                let line = format!("{} {}", glyph, entry.name);
                let (fg, bg) = if is_selected {
                    (
                        self.theme.selected_foreground,
                        self.theme.selected_background,
                    )
                } else if entry.is_dir {
                    (self.theme.folders, self.theme.popup_background)
                } else {
                    (self.theme.files, self.theme.popup_background)
                };
                frame.render_widget(
                    Paragraph::new(line).style(Style::default().fg(fg).bg(bg)),
                    Rect::new(content_x, row, content_w, 1),
                );
            }
        }

        // Footer hint.
        let footer_y = inner.y + inner.height.saturating_sub(1);
        frame.render_widget(
            Paragraph::new(
                "↑/↓: move  |  →/Enter: descend or pick  |  ←: parent  |  type: filter  |  Esc: cancel",
            )
            .style(
                Style::default()
                    .fg(self.theme.muted)
                    .bg(self.theme.popup_background),
            ),
            Rect::new(content_x, footer_y, content_w, 1),
        );
    }

    fn handle_image_picker_event(&mut self, key: event::KeyEvent) -> Option<ModalResult> {
        use crossterm::event::{KeyCode, KeyModifiers};
        let Some(Modal::ImagePicker { state }) = self.modal.as_mut() else {
            return Some(ModalResult::CloseCancel);
        };

        match key.code {
            KeyCode::Esc => {
                self.modal = self.paused_form_edit_modal.take();
                self.push_toast(ToastLevel::Info, "Image pick cancelled.");
                Some(ModalResult::CloseCancel)
            }
            KeyCode::Up if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                state.selected = state.selected.saturating_sub(1);
                Some(ModalResult::Continue)
            }
            KeyCode::Down if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                let entries = list_dir_entries(&state.cwd);
                let filtered = filter_entries(&entries, &state.filter);
                if !filtered.is_empty() {
                    state.selected = (state.selected + 1).min(filtered.len() - 1);
                }
                Some(ModalResult::Continue)
            }
            KeyCode::Left if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                if state.cwd != state.root {
                    if let Some(parent) = state.cwd.parent() {
                        state.cwd = parent.to_path_buf();
                        state.filter.clear();
                        state.selected = 0;
                    }
                }
                Some(ModalResult::Continue)
            }
            KeyCode::Right if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.image_picker_descend_or_pick();
                Some(ModalResult::Continue)
            }
            KeyCode::Enter => {
                self.image_picker_descend_or_pick();
                Some(ModalResult::Continue)
            }
            KeyCode::Backspace => {
                state.filter.pop();
                state.selected = 0;
                Some(ModalResult::Continue)
            }
            KeyCode::Char(c)
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && (c.is_alphanumeric() || c == '-' || c == '_' || c == '.') =>
            {
                state.filter.push(c);
                state.selected = 0;
                Some(ModalResult::Continue)
            }
            _ => Some(ModalResult::Continue),
        }
    }

    /// Resolve the current selection: descend into a directory or commit a
    /// file pick. Called by both `l` and `Enter`.
    fn image_picker_descend_or_pick(&mut self) {
        let (cwd, root, selected_name, is_dir, binding) = {
            let Some(Modal::ImagePicker { state }) = self.modal.as_ref() else {
                return;
            };
            let entries = list_dir_entries(&state.cwd);
            let filtered = filter_entries(&entries, &state.filter);
            let Some(entry) = filtered.get(state.selected) else {
                return;
            };
            (
                state.cwd.clone(),
                state.root.clone(),
                entry.name.clone(),
                entry.is_dir,
                state.binding.clone(),
            )
        };

        if is_dir {
            if let Some(Modal::ImagePicker { state }) = self.modal.as_mut() {
                state.cwd = cwd.join(&selected_name);
                state.filter.clear();
                state.selected = 0;
            }
            return;
        }

        // File pick: build the output-relative path under assets/images/.
        let target_full = cwd.join(&selected_name);
        let rel_under_root = target_full
            .strip_prefix(&root)
            .unwrap_or(&target_full)
            .to_string_lossy()
            .replace('\\', "/");
        let stored = format!("assets/images/{}", rel_under_root);

        self.commit_image_pick(stored, binding);
    }

    /// Apply the picked path to the binding's target and restore the paused FormEdit modal.
    fn commit_image_pick(&mut self, value: String, binding: ImagePickBinding) {
        match binding {
            ImagePickBinding::FormEditField { field_id } => {
                self.modal = self.paused_form_edit_modal.take();
                if let Some(Modal::FormEdit { state, cursor_pos, .. }) = self.modal.as_mut() {
                    state.set(&field_id, value.clone());
                    *cursor_pos = state.get(&field_id).len();
                    self.push_toast(
                        ToastLevel::Success,
                        format!("Picked image: {}", value),
                    );
                } else {
                    self.push_toast(
                        ToastLevel::Warning,
                        "Image pick lost: parent form modal closed.",
                    );
                }
            }
        }
    }

    fn render_page_picker_modal(
        &self,
        frame: &mut ratatui::Frame,
        state: &PagePickerState,
    ) {
        let area = centered_rect(60, 60, frame.area());
        frame.render_widget(Clear, area);
        let outer = Block::default()
            .title(" Pick page ")
            .borders(Borders::ALL)
            .style(Style::default().bg(self.theme.popup_background))
            .border_style(Style::default().fg(self.theme.border_active))
            .title_style(
                Style::default()
                    .fg(self.theme.title)
                    .add_modifier(Modifier::BOLD),
            );
        let inner = outer.inner(area);
        frame.render_widget(outer, area);
        if inner.height < 5 || inner.width < 10 {
            return;
        }
        let pad: u16 = 2;
        let content_x = inner.x + pad;
        let content_w = inner.width.saturating_sub(pad * 2);

        // Filter row.
        let filter_label = format!("Filter: {}_", state.filter);
        frame.render_widget(
            Paragraph::new(filter_label).style(
                Style::default()
                    .fg(self.theme.text_active_focus)
                    .bg(self.theme.popup_background),
            ),
            Rect::new(content_x, inner.y, content_w, 1),
        );

        // Filtered list.
        let filtered = filter_pages(&state.pages, &state.filter);
        let body_y = inner.y + 2;
        let body_h = inner.height.saturating_sub(3);
        let visible = body_h as usize;
        let start = if filtered.is_empty() {
            0
        } else if state.selected >= visible {
            state.selected + 1 - visible
        } else {
            0
        };

        if filtered.is_empty() {
            frame.render_widget(
                Paragraph::new("(no matches)").style(
                    Style::default()
                        .fg(self.theme.muted)
                        .bg(self.theme.popup_background),
                ),
                Rect::new(content_x, body_y, content_w, 1),
            );
        } else {
            for (i, (slug, title)) in
                filtered.iter().skip(start).take(visible).enumerate()
            {
                let row = body_y + i as u16;
                let is_selected = (start + i) == state.selected;
                let line = format!("{}  /{}", title, slug);
                let (fg, bg) = if is_selected {
                    (
                        self.theme.selected_foreground,
                        self.theme.selected_background,
                    )
                } else {
                    (self.theme.foreground, self.theme.popup_background)
                };
                frame.render_widget(
                    Paragraph::new(line).style(Style::default().fg(fg).bg(bg)),
                    Rect::new(content_x, row, content_w, 1),
                );
            }
        }

        let footer_y = inner.y + inner.height.saturating_sub(1);
        frame.render_widget(
            Paragraph::new(
                "↑/↓: move  |  Enter: pick  |  type: filter  |  Esc: cancel",
            )
            .style(
                Style::default()
                    .fg(self.theme.muted)
                    .bg(self.theme.popup_background),
            ),
            Rect::new(content_x, footer_y, content_w, 1),
        );
    }

    fn handle_page_picker_event(
        &mut self,
        key: event::KeyEvent,
    ) -> Option<ModalResult> {
        use crossterm::event::{KeyCode, KeyModifiers};
        let Some(Modal::PagePicker { state }) = self.modal.as_mut() else {
            return Some(ModalResult::CloseCancel);
        };
        match key.code {
            KeyCode::Esc => {
                self.modal = self.paused_form_edit_modal.take();
                self.push_toast(ToastLevel::Info, "Page pick cancelled.");
                Some(ModalResult::CloseCancel)
            }
            KeyCode::Up if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                state.selected = state.selected.saturating_sub(1);
                Some(ModalResult::Continue)
            }
            KeyCode::Down if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                let filtered = filter_pages(&state.pages, &state.filter);
                if !filtered.is_empty() {
                    state.selected =
                        (state.selected + 1).min(filtered.len() - 1);
                }
                Some(ModalResult::Continue)
            }
            KeyCode::Enter => {
                self.commit_page_pick();
                Some(ModalResult::CloseSuccess)
            }
            KeyCode::Backspace => {
                state.filter.pop();
                state.selected = 0;
                Some(ModalResult::Continue)
            }
            KeyCode::Char(c)
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && (c.is_alphanumeric() || c == '-' || c == '_' || c == ' ') =>
            {
                state.filter.push(c);
                state.selected = 0;
                Some(ModalResult::Continue)
            }
            _ => Some(ModalResult::Continue),
        }
    }

    /// Resolve the highlighted page → write `/<slug>` to the bound field.
    fn commit_page_pick(&mut self) {
        let (slug, binding) = {
            let Some(Modal::PagePicker { state }) = self.modal.as_ref() else {
                return;
            };
            let filtered = filter_pages(&state.pages, &state.filter);
            let Some((slug, _)) = filtered.get(state.selected).cloned() else {
                return;
            };
            (slug, state.binding.clone())
        };
        match binding {
            PagePickBinding::FormEditField { field_id } => {
                self.modal = self.paused_form_edit_modal.take();
                if let Some(Modal::FormEdit {
                    state, cursor_pos, ..
                }) = self.modal.as_mut()
                {
                    let value = format!("/{}", slug);
                    state.set(&field_id, value.clone());
                    *cursor_pos = state.get(&field_id).len();
                    self.push_toast(
                        ToastLevel::Success,
                        format!("Picked page: {}", value),
                    );
                } else {
                    self.push_toast(
                        ToastLevel::Warning,
                        "Page pick lost: parent form modal closed.",
                    );
                }
            }
        }
    }

    /// Unified single field renderer (legacy mode)
    fn render_single_field_unified(
        &self,
        frame: &mut ratatui::Frame,
        mode: InputMode,
        buffer: &str,
        cursor: usize,
        multiline: bool,
    ) {
        // This will be simplified once we migrate all single fields to the Edit modal
        let area = centered_rect(72, 60, frame.area());
        frame.render_widget(Clear, area);

        let label = self.input_mode_label(mode);
        let content = if multiline {
            format!(
                "Editing: {}\n\n{}\n\nEnter: newline | Ctrl+S: save | Esc: cancel",
                label, buffer
            )
        } else {
            format!(
                "Editing: {}\n\n{}\n\nEnter: save | Esc: cancel",
                label, buffer
            )
        };

        let modal = Paragraph::new(content)
            .style(
                Style::default()
                    .fg(self.theme.foreground)
                    .bg(self.theme.popup_background),
            )
            .block(
                Block::default()
                    .title("Edit Item")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(self.theme.input_focus)),
            );

        frame.render_widget(modal, area);

        // Set cursor
        let inner_width = area.width.saturating_sub(2) as usize;
        let cursor_x = area
            .x
            .saturating_add(3)
            .saturating_add(cursor.min(inner_width.saturating_sub(1)) as u16);
        let cursor_y = area.y.saturating_add(4);
        frame.set_cursor_position((cursor_x, cursor_y));
    }

    fn input_mode_label(&self, mode: InputMode) -> &'static str {
        match mode {
            InputMode::EditHeroImage => "hero.image",
            InputMode::EditHeroClass => "hero.class",
            InputMode::EditSectionId => "section.id",
            // ... add more as needed
            _ => "field",
        }
    }

    /// Unified modal event handling
    fn handle_modal_event(&mut self, evt: Event) -> Option<ModalResult> {
        let _ = self.modal.as_ref()?;

        if let Event::Key(key) = &evt {
            let key = *key;
            return match self.modal.as_ref()? {
                Modal::Edit { .. } => self.handle_edit_modal_event_unified(key),
                Modal::ComponentPicker { .. } => {
                    self.handle_component_picker_event_unified(key)
                }
                Modal::SavePrompt { .. } => self.handle_save_prompt_event_unified(key),
                Modal::SingleField { .. } => self.handle_single_field_event_unified(key),
                Modal::FormEdit { .. } => self.handle_form_edit_event(key),
                Modal::TemplatePicker { .. } => self.handle_template_picker_event(key),
                Modal::NewPageTitlePrompt { .. } => self.handle_new_page_title_prompt_event(key),
                Modal::ExportPathPrompt { .. } => self.handle_export_path_prompt_event(key),
                Modal::PreviewPathPrompt { .. } => self.handle_preview_path_prompt_event(key),
                Modal::RenamePagePrompt { .. } => self.handle_rename_page_prompt_event(key),
                Modal::ConfirmPrompt { .. } => self.handle_confirm_prompt_event(key),
                Modal::ValidationErrors { .. } => self.handle_validation_errors_event(key),
                Modal::ImagePicker { .. } => self.handle_image_picker_event(key),
                Modal::PagePicker { .. } => self.handle_page_picker_event(key),
            };
        }

        if let Event::Mouse(m) = &evt {
            let kind = m.kind;
            // Click-to-focus inside multi-field modals: pick the input box
            // whose cached rect contains the click.
            if matches!(kind, MouseEventKind::Down(MouseButton::Left)) {
                let (col, row) = (m.column, m.row);
                let hit = self
                    .modal_field_areas
                    .borrow()
                    .iter()
                    .find(|(_, r)| {
                        col >= r.x
                            && col < r.x + r.width
                            && row >= r.y
                            && row < r.y + r.height
                    })
                    .map(|(idx, _)| *idx);
                if let Some(idx) = hit {
                    if let Some(modal) = self.modal.as_mut() {
                        match modal {
                            Modal::Edit { selected_field, .. } => {
                                *selected_field = idx;
                            }
                            Modal::FormEdit { state, .. } => {
                                state.focused_field = idx;
                            }
                            _ => {}
                        }
                    }
                    return Some(ModalResult::Continue);
                }
            }
            match kind {
                MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                    let delta: i32 = if matches!(kind, MouseEventKind::ScrollUp) { -3 } else { 3 };
                    if let Some(modal) = self.modal.as_mut() {
                        match modal {
                            Modal::ValidationErrors { errors, scroll_offset } => {
                                let max = errors.len().saturating_sub(1);
                                let next = (*scroll_offset as i32 + delta).max(0) as usize;
                                *scroll_offset = next.min(max);
                            }
                            Modal::FormEdit { scroll_offset, .. } => {
                                let next = (*scroll_offset as i32 + delta).max(0) as u16;
                                *scroll_offset = next;
                            }
                            Modal::Edit { selected_field, scroll_offset, visible_fields, fields, .. } => {
                                let total = fields.len();
                                let next = (*scroll_offset as i32 + delta).max(0) as usize;
                                let cap = total.saturating_sub((*visible_fields).max(1));
                                *scroll_offset = next.min(cap);
                                if *selected_field < *scroll_offset {
                                    *selected_field = *scroll_offset;
                                } else if *selected_field >= *scroll_offset + *visible_fields {
                                    *selected_field =
                                        (*scroll_offset + *visible_fields).saturating_sub(1);
                                }
                            }
                            _ => {}
                        }
                    }
                    return Some(ModalResult::Continue);
                }
                _ => {}
            }
        }

        Some(ModalResult::Continue)
    }

    /// Handle keyboard events while `Modal::FormEdit` is active.
    fn handle_form_edit_event(&mut self, key: event::KeyEvent) -> Option<ModalResult> {
        use crossterm::event::{KeyCode, KeyModifiers};

        // Ctrl+P: open image picker on image_url fields, page picker on
        // link_url fields. Heuristic on field id since both kinds are
        // FieldKind::Url today.
        if matches!(key.code, KeyCode::Char('p'))
            && key.modifiers.contains(KeyModifiers::CONTROL)
        {
            let Some(Modal::FormEdit { state, .. }) = self.modal.as_ref() else {
                return Some(ModalResult::Continue);
            };
            let field_opt = state.form.fields.get(state.focused_field);
            let field_id = match field_opt {
                Some(f) if matches!(f.kind, editform::FieldKind::Url { .. }) => f.id.to_string(),
                _ => return Some(ModalResult::Continue),
            };

            if field_id.contains("image") {
                let base = self
                    .path
                    .as_ref()
                    .and_then(|p| p.parent().map(std::path::PathBuf::from))
                    .unwrap_or_else(|| std::path::PathBuf::from("."));
                let root = base.join("source").join("images");
                if !root.exists() {
                    self.push_toast(
                        ToastLevel::Warning,
                        format!("Source folder not found: {}", root.display()),
                    );
                    return Some(ModalResult::Continue);
                }
                let paused = self.modal.take();
                self.paused_form_edit_modal = paused;
                self.modal = Some(Modal::ImagePicker {
                    state: ImagePickerState {
                        root: root.clone(),
                        cwd: root,
                        filter: String::new(),
                        selected: 0,
                        binding: ImagePickBinding::FormEditField { field_id },
                    },
                });
                return Some(ModalResult::Continue);
            }

            if field_id.contains("link") {
                let pages: Vec<(String, String)> = self
                    .site
                    .pages
                    .iter()
                    .map(|p| (p.slug.clone(), p.head.title.clone()))
                    .collect();
                if pages.is_empty() {
                    self.push_toast(
                        ToastLevel::Warning,
                        "No pages to pick from.".to_string(),
                    );
                    return Some(ModalResult::Continue);
                }
                let paused = self.modal.take();
                self.paused_form_edit_modal = paused;
                self.modal = Some(Modal::PagePicker {
                    state: PagePickerState {
                        pages,
                        filter: String::new(),
                        selected: 0,
                        binding: PagePickBinding::FormEditField { field_id },
                    },
                });
                return Some(ModalResult::Continue);
            }

            // URL field with neither "image" nor "link" in its id — no
            // picker fits. Fall through silently so Ctrl+P is a no-op.
            return Some(ModalResult::Continue);
        }

        // Ctrl+S: drilled-down form returns to its parent; top-level form
        // commits to the model.
        if matches!(key.code, KeyCode::Char('s')) && key.modifiers.contains(KeyModifiers::CONTROL) {
            let taken = self.modal.take();
            if let Some(Modal::FormEdit {
                state,
                cursor,
                cursor_pos,
                mut drill_stack,
                scroll_offset: _,
            }) = taken
            {
                if let Some(frame) = drill_stack.pop() {
                    // Returning from a drilled-in item — write current state back
                    // into the parent's sub_state and make the parent the active form.
                    let mut parent = frame.parent_state;
                    let items = parent
                        .sub_state
                        .entry(frame.subform_field_id.clone())
                        .or_default();
                    if frame.item_idx < items.len() {
                        items[frame.item_idx] = state;
                    } else {
                        items.push(state);
                    }
                    self.status = "Item saved — editing parent.".to_string();
                    self.modal = Some(Modal::FormEdit {
                        state: parent,
                        cursor,
                        cursor_pos: frame.parent_cursor_pos,
                        drill_stack,
                        scroll_offset: frame.parent_scroll_offset,
                    });
                    return Some(ModalResult::Continue);
                }
                // Top-level save: commit to the model.
                match cursor::apply_edit_form_to_component(&mut self.site, &cursor, &state) {
                    Ok(()) => {
                        let msg = format!("Saved {}.", state.form.title);
                        self.push_toast(ToastLevel::Success, msg);
                        return Some(ModalResult::CloseSuccess);
                    }
                    Err(e) => {
                        self.status = format!("Save failed: {e}");
                        self.modal = Some(Modal::FormEdit {
                            state,
                            cursor,
                            cursor_pos,
                            drill_stack,
                            scroll_offset: 0,
                        });
                        return Some(ModalResult::Continue);
                    }
                }
            }
            return Some(ModalResult::CloseCancel);
        }
        // Esc: drilled-down discards and returns; top-level closes.
        if matches!(key.code, KeyCode::Esc) {
            let taken = self.modal.take();
            if let Some(Modal::FormEdit {
                state: _,
                cursor,
                cursor_pos: _,
                mut drill_stack,
                scroll_offset: _,
            }) = taken
            {
                if let Some(frame) = drill_stack.pop() {
                    self.status = "Item edit cancelled.".to_string();
                    self.modal = Some(Modal::FormEdit {
                        state: frame.parent_state,
                        cursor,
                        cursor_pos: frame.parent_cursor_pos,
                        drill_stack,
                        scroll_offset: frame.parent_scroll_offset,
                    });
                    return Some(ModalResult::Continue);
                }
            }
            self.modal = None;
            return Some(ModalResult::CloseCancel);
        }

        let Some(Modal::FormEdit {
            state,
            cursor_pos,
            scroll_offset,
            ..
        }) = self.modal.as_mut()
        else {
            return Some(ModalResult::CloseCancel);
        };

        // Snapshot the focused field's id and kind (to satisfy borrow rules before mutation).
        let focused_idx = state.focused_field;
        let (field_id, is_enum, is_textarea, is_subform, accepts_text) = match state
            .form
            .fields
            .get(focused_idx)
        {
            Some(f) => (
                f.id,
                matches!(f.kind, editform::FieldKind::Enum { .. }),
                matches!(f.kind, editform::FieldKind::Textarea { .. }),
                matches!(f.kind, editform::FieldKind::SubForm { .. }),
                matches!(
                    f.kind,
                    editform::FieldKind::Text { .. }
                        | editform::FieldKind::Url { .. }
                        | editform::FieldKind::Textarea { .. }
                ),
            ),
            None => return Some(ModalResult::CloseCancel),
        };

        // SubForm collection handling: A/X/Enter/Up/Down operate on items list.
        if is_subform {
            match key.code {
                KeyCode::Char('A') => {
                    if let Some(new_item) = state.new_sub_item(field_id) {
                        let items = state.sub_state.entry(field_id.to_string()).or_default();
                        let selected = state
                            .selected_sub_item
                            .get(field_id)
                            .copied()
                            .unwrap_or(0);
                        let insert_at = if items.is_empty() {
                            0
                        } else {
                            (selected + 1).min(items.len())
                        };
                        items.insert(insert_at, new_item);
                        state
                            .selected_sub_item
                            .insert(field_id.to_string(), insert_at);
                        self.status = "Item added.".to_string();
                    }
                    return Some(ModalResult::Continue);
                }
                KeyCode::Char('X') => {
                    let min_items = match state.form.fields[focused_idx].kind {
                        editform::FieldKind::SubForm { min_items, .. } => min_items,
                        _ => 0,
                    };
                    let items = state.sub_state.entry(field_id.to_string()).or_default();
                    if items.len() > min_items {
                        let selected = state
                            .selected_sub_item
                            .get(field_id)
                            .copied()
                            .unwrap_or(0);
                        if selected < items.len() {
                            items.remove(selected);
                            let new_sel = selected.min(items.len().saturating_sub(1));
                            state
                                .selected_sub_item
                                .insert(field_id.to_string(), new_sel);
                            self.status = "Item removed.".to_string();
                        }
                    } else {
                        self.status = format!("Must keep at least {min_items} item(s).");
                    }
                    return Some(ModalResult::Continue);
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    let selected = state
                        .selected_sub_item
                        .get(field_id)
                        .copied()
                        .unwrap_or(0);
                    let items_len = state
                        .sub_state
                        .get(field_id)
                        .map(|v| v.len())
                        .unwrap_or(0);
                    if items_len == 0 {
                        state.focus_prev();
                    *scroll_offset = auto_scroll_for_focus(state, *scroll_offset);
                        *cursor_pos =
                            state.get(state.form.fields[state.focused_field].id).len();
                    } else if selected == 0 {
                        state.focus_prev();
                    *scroll_offset = auto_scroll_for_focus(state, *scroll_offset);
                        *cursor_pos =
                            state.get(state.form.fields[state.focused_field].id).len();
                    } else {
                        state
                            .selected_sub_item
                            .insert(field_id.to_string(), selected - 1);
                    }
                    return Some(ModalResult::Continue);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    let selected = state
                        .selected_sub_item
                        .get(field_id)
                        .copied()
                        .unwrap_or(0);
                    let items_len = state
                        .sub_state
                        .get(field_id)
                        .map(|v| v.len())
                        .unwrap_or(0);
                    if selected + 1 < items_len {
                        state
                            .selected_sub_item
                            .insert(field_id.to_string(), selected + 1);
                    } else {
                        state.focus_next();
                    *scroll_offset = auto_scroll_for_focus(state, *scroll_offset);
                        *cursor_pos =
                            state.get(state.form.fields[state.focused_field].id).len();
                    }
                    return Some(ModalResult::Continue);
                }
                KeyCode::Enter => {
                    // Drill into the selected item by taking ownership of the modal.
                    let taken = self.modal.take();
                    if let Some(Modal::FormEdit {
                        mut state,
                        cursor,
                        cursor_pos,
                        mut drill_stack,
                        scroll_offset,
                    }) = taken
                    {
                        let selected = state
                            .selected_sub_item
                            .get(field_id)
                            .copied()
                            .unwrap_or(0);
                        let items_len = state
                            .sub_state
                            .get(field_id)
                            .map(|v| v.len())
                            .unwrap_or(0);
                        if selected < items_len {
                            let template = match &state.form.fields[focused_idx].kind {
                                editform::FieldKind::SubForm { template, .. } => *template,
                                _ => unreachable!(
                                    "is_subform was true but kind is not SubForm"
                                ),
                            };
                            let placeholder = editform::EditFormState::new(template);
                            let items = state
                                .sub_state
                                .get_mut(field_id)
                                .expect("sub_state present for SubForm field");
                            let item_state = std::mem::replace(&mut items[selected], placeholder);
                            let item_cursor_pos = item_state
                                .get(item_state.form.fields[item_state.focused_field].id)
                                .len();
                            drill_stack.push(DrillFrame {
                                parent_state: state,
                                parent_cursor_pos: cursor_pos,
                                parent_scroll_offset: scroll_offset,
                                subform_field_id: field_id.to_string(),
                                item_idx: selected,
                            });
                            self.modal = Some(Modal::FormEdit {
                                state: item_state,
                                cursor,
                                cursor_pos: item_cursor_pos,
                                drill_stack,
                                scroll_offset: 0,
                            });
                            self.status = "Editing item. Ctrl+S returns to parent.".to_string();
                        } else {
                            // Nothing to drill into; restore modal unchanged.
                            self.modal = Some(Modal::FormEdit {
                                state,
                                cursor,
                                cursor_pos,
                                drill_stack,
                                scroll_offset,
                            });
                        }
                    }
                    return Some(ModalResult::Continue);
                }
                _ => {}
            }
        }

        match key.code {
            KeyCode::Tab => {
                state.focus_next();
                    *scroll_offset = auto_scroll_for_focus(state, *scroll_offset);
                *cursor_pos = state.get(state.form.fields[state.focused_field].id).len();
            }
            KeyCode::BackTab => {
                state.focus_prev();
                    *scroll_offset = auto_scroll_for_focus(state, *scroll_offset);
                *cursor_pos = state.get(state.form.fields[state.focused_field].id).len();
            }
            KeyCode::Left => {
                if is_enum {
                    state.cycle_enum(false);
                } else if *cursor_pos > 0 {
                    *cursor_pos -= 1;
                }
            }
            KeyCode::Right => {
                if is_enum {
                    state.cycle_enum(true);
                } else {
                    let len = state.get(field_id).len();
                    if *cursor_pos < len {
                        *cursor_pos += 1;
                    }
                }
            }
            KeyCode::Up => {
                state.focus_prev();
                    *scroll_offset = auto_scroll_for_focus(state, *scroll_offset);
                *cursor_pos = state.get(state.form.fields[state.focused_field].id).len();
            }
            KeyCode::Down => {
                state.focus_next();
                    *scroll_offset = auto_scroll_for_focus(state, *scroll_offset);
                *cursor_pos = state.get(state.form.fields[state.focused_field].id).len();
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                if accepts_text {
                    let current = state.get(field_id).to_string();
                    let pos = (*cursor_pos).min(current.len());
                    let mut new = String::with_capacity(current.len() + 1);
                    new.push_str(&current[..pos]);
                    new.push(c);
                    new.push_str(&current[pos..]);
                    state.set(field_id, new);
                    *cursor_pos = pos + 1;
                }
            }
            KeyCode::Backspace => {
                if accepts_text {
                    let current = state.get(field_id).to_string();
                    let pos = (*cursor_pos).min(current.len());
                    if pos > 0 {
                        let mut new = String::with_capacity(current.len() - 1);
                        new.push_str(&current[..pos - 1]);
                        new.push_str(&current[pos..]);
                        state.set(field_id, new);
                        *cursor_pos = pos - 1;
                    }
                }
            }
            KeyCode::Enter => {
                if is_textarea {
                    let current = state.get(field_id).to_string();
                    let pos = (*cursor_pos).min(current.len());
                    let mut new = String::with_capacity(current.len() + 1);
                    new.push_str(&current[..pos]);
                    new.push('\n');
                    new.push_str(&current[pos..]);
                    state.set(field_id, new);
                    *cursor_pos = pos + 1;
                } else {
                    state.focus_next();
                    *scroll_offset = auto_scroll_for_focus(state, *scroll_offset);
                    *cursor_pos = state.get(state.form.fields[state.focused_field].id).len();
                }
            }
            _ => {}
        }

        Some(ModalResult::Continue)
    }

    fn handle_edit_modal_event_unified(&mut self, key: event::KeyEvent) -> Option<ModalResult> {
        use crossterm::event::KeyCode;

        // Extract current state
        let (title, fields, selected_field, scroll_offset, visible_fields, on_save) =
            if let Some(Modal::Edit {
                title,
                fields,
                selected_field,
                scroll_offset,
                visible_fields,
                on_save,
            }) = self.modal.take()
            {
                (
                    title,
                    fields,
                    selected_field,
                    scroll_offset,
                    visible_fields,
                    on_save,
                )
            } else {
                return Some(ModalResult::CloseCancel);
            };

        let total_fields = fields.len();
        let mut new_selected = selected_field;
        let mut new_scroll = scroll_offset;
        let mut should_close = false;
        let mut result = ModalResult::Continue;

        match key.code {
            KeyCode::Esc => {
                should_close = true;
                result = ModalResult::CloseCancel;
            }
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Save and close immediately - call on_save now before it gets moved
                on_save(self, &fields);
                let msg = format!("Saved {} changes.", title);
                self.push_toast(ToastLevel::Success, msg);
                return Some(ModalResult::CloseSuccess);
            }
            KeyCode::Up => {
                new_selected = selected_field.saturating_sub(1);
                if new_selected < new_scroll {
                    new_scroll = new_selected;
                }
            }
            KeyCode::Down => {
                new_selected = (selected_field + 1).min(total_fields.saturating_sub(1));
                if new_selected >= new_scroll + visible_fields {
                    new_scroll = new_selected.saturating_sub(visible_fields - 1);
                }
            }
            KeyCode::Tab => {
                new_selected = (selected_field + 1) % total_fields;
                if new_selected < new_scroll {
                    new_scroll = new_selected;
                } else if new_selected >= new_scroll + visible_fields {
                    new_scroll = new_selected.saturating_sub(visible_fields - 1);
                }
            }
            KeyCode::BackTab => {
                new_selected = selected_field.saturating_sub(1);
                if new_selected >= total_fields {
                    new_selected = total_fields.saturating_sub(1);
                }
                if new_selected < new_scroll {
                    new_scroll = new_selected;
                }
            }
            KeyCode::Backspace => {
                let mut new_fields = fields;
                if let Some(field) = new_fields.get_mut(selected_field) {
                    if field.cursor > 0 {
                        field.cursor -= 1;
                        if field.cursor < field.buffer.chars().count() {
                            let mut chars: Vec<char> = field.buffer.chars().collect();
                            chars.remove(field.cursor);
                            field.buffer = chars.into_iter().collect();
                        }
                    }
                }
                // Restore modal with modified fields
                self.modal = Some(Modal::Edit {
                    title,
                    fields: new_fields,
                    selected_field,
                    scroll_offset,
                    visible_fields,
                    on_save,
                });
                return Some(ModalResult::Continue);
            }
            KeyCode::Left => {
                let mut new_fields = fields;
                if let Some(field) = new_fields.get_mut(selected_field) {
                    field.cursor = field.cursor.saturating_sub(1);
                }
                self.modal = Some(Modal::Edit {
                    title,
                    fields: new_fields,
                    selected_field,
                    scroll_offset,
                    visible_fields,
                    on_save,
                });
                return Some(ModalResult::Continue);
            }
            KeyCode::Right => {
                let mut new_fields = fields;
                if let Some(field) = new_fields.get_mut(selected_field) {
                    let max = field.buffer.chars().count();
                    field.cursor = (field.cursor + 1).min(max);
                }
                self.modal = Some(Modal::Edit {
                    title,
                    fields: new_fields,
                    selected_field,
                    scroll_offset,
                    visible_fields,
                    on_save,
                });
                return Some(ModalResult::Continue);
            }
            KeyCode::Char(c) => {
                let mut new_fields = fields;
                if let Some(field) = new_fields.get_mut(selected_field) {
                    let mut chars: Vec<char> = field.buffer.chars().collect();
                    if field.cursor <= chars.len() {
                        chars.insert(field.cursor, c);
                        field.buffer = chars.into_iter().collect();
                        field.cursor += 1;
                    }
                }
                self.modal = Some(Modal::Edit {
                    title,
                    fields: new_fields,
                    selected_field,
                    scroll_offset,
                    visible_fields,
                    on_save,
                });
                return Some(ModalResult::Continue);
            }
            _ => {}
        }

        if !should_close {
            // Restore modal with updated state
            self.modal = Some(Modal::Edit {
                title,
                fields,
                selected_field: new_selected,
                scroll_offset: new_scroll,
                visible_fields,
                on_save,
            });
        }

        Some(result)
    }

    fn handle_component_picker_event_unified(
        &mut self,
        key: event::KeyEvent,
    ) -> Option<ModalResult> {
        use crossterm::event::KeyCode;

        let (query, selected) =
            if let Some(Modal::ComponentPicker { query, selected }) = self.modal.take() {
                (query, selected)
            } else {
                return Some(ModalResult::CloseCancel);
            };

        match key.code {
            KeyCode::Esc => {
                self.status = "Component picker cancelled.".to_string();
                return Some(ModalResult::CloseCancel);
            }
            KeyCode::Up => {
                let new_selected = selected.saturating_sub(1);
                self.modal = Some(Modal::ComponentPicker {
                    query,
                    selected: new_selected,
                });
            }
            KeyCode::Down => {
                let filtered = self.filtered_component_kinds(&query);
                let total = filtered.len();
                let new_selected = if total == 0 {
                    0
                } else {
                    (selected + 1).min(total - 1)
                };
                self.modal = Some(Modal::ComponentPicker {
                    query,
                    selected: new_selected,
                });
            }
            KeyCode::Backspace => {
                let mut new_query = query;
                new_query.pop();
                self.modal = Some(Modal::ComponentPicker {
                    query: new_query,
                    selected,
                });
                self.normalize_component_picker_selection();
            }
            KeyCode::Enter => {
                let filtered = self.filtered_component_kinds(&query);
                if let Some(kind) = filtered.get(selected.min(filtered.len().saturating_sub(1))) {
                    self.component_kind = *kind;
                    self.insert_selected_component_kind();
                    return Some(ModalResult::CloseSuccess);
                }
                self.status = "No component selected.".to_string();
                return Some(ModalResult::CloseCancel);
            }
            KeyCode::Char(c) => {
                let mut new_query = query;
                new_query.push(c);
                self.modal = Some(Modal::ComponentPicker {
                    query: new_query,
                    selected,
                });
                self.normalize_component_picker_selection();
            }
            _ => {
                // Restore modal if we didn't handle the key
                self.modal = Some(Modal::ComponentPicker { query, selected });
            }
        }

        self.sync_tree_row_with_selection();
        Some(ModalResult::Continue)
    }

    fn handle_save_prompt_event_unified(&mut self, key: event::KeyEvent) -> Option<ModalResult> {
        use crossterm::event::KeyCode;

        let path = if let Some(Modal::SavePrompt { path }) = self.modal.take() {
            path
        } else {
            return Some(ModalResult::CloseCancel);
        };

        match key.code {
            KeyCode::Esc => {
                self.status = "Save cancelled.".to_string();
                Some(ModalResult::CloseCancel)
            }
            KeyCode::Enter => {
                let raw = path.trim();
                if raw.is_empty() {
                    self.status = "Save path cannot be empty.".to_string();
                    self.modal = Some(Modal::SavePrompt { path });
                    Some(ModalResult::Continue)
                } else {
                    let path_buf = std::path::PathBuf::from(raw);
                    if let Err(e) = self.commit_save_with_backup(&path_buf) {
                        self.status = format!("Failed to save: {}", e);
                        self.modal = Some(Modal::SavePrompt { path });
                        Some(ModalResult::Continue)
                    } else {
                        let msg = format!("Saved {}", path_buf.display());
                        self.push_toast(ToastLevel::Success, msg);
                        Some(ModalResult::CloseSuccess)
                    }
                }
            }
            KeyCode::Backspace => {
                let mut new_path = path;
                new_path.pop();
                self.modal = Some(Modal::SavePrompt { path: new_path });
                Some(ModalResult::Continue)
            }
            KeyCode::Char(c) => {
                let mut new_path = path;
                new_path.push(c);
                self.modal = Some(Modal::SavePrompt { path: new_path });
                Some(ModalResult::Continue)
            }
            _ => {
                self.modal = Some(Modal::SavePrompt { path });
                Some(ModalResult::Continue)
            }
        }
    }

    fn handle_template_picker_event(&mut self, key: event::KeyEvent) -> Option<ModalResult> {
        use crossterm::event::KeyCode;
        let Some(Modal::TemplatePicker { selected }) = self.modal.as_mut() else {
            return Some(ModalResult::CloseCancel);
        };
        match key.code {
            KeyCode::Esc => {
                self.modal = None;
                self.status = "Add page cancelled.".to_string();
                Some(ModalResult::CloseCancel)
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if *selected > 0 {
                    *selected -= 1;
                }
                Some(ModalResult::Continue)
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if *selected < 3 {
                    *selected += 1;
                }
                Some(ModalResult::Continue)
            }
            KeyCode::Enter => {
                let picked = *selected;
                let title = self.pending_new_page_title.take().unwrap_or_default();
                if title.is_empty() {
                    self.modal = None;
                    self.status = "Cancelled — no title.".to_string();
                    return Some(ModalResult::CloseCancel);
                }
                let mut new_page = match picked {
                    0 => crate::model::Page::from_template(
                        &title,
                        crate::model::PageTemplate::Blank,
                    ),
                    1 => crate::model::Page::from_template(
                        &title,
                        crate::model::PageTemplate::HeroOnly,
                    ),
                    2 => crate::model::Page::from_template(
                        &title,
                        crate::model::PageTemplate::HeroPlusSection,
                    ),
                    3 => {
                        let src_idx =
                            self.selected_page.min(self.site.pages.len().saturating_sub(1));
                        let src = &self.site.pages[src_idx];
                        crate::model::Page::duplicate_from(src)
                    }
                    _ => crate::model::Page::from_template(
                        &title,
                        crate::model::PageTemplate::Blank,
                    ),
                };
                // Dedup id/slug to avoid collisions.
                if self.site.pages.iter().any(|p| p.id == new_page.id) {
                    let base_id = new_page.id.clone();
                    let base_slug = new_page.slug.clone();
                    for n in 2.. {
                        let candidate_id = format!("{}-{}", base_id, n);
                        if !self.site.pages.iter().any(|p| p.id == candidate_id) {
                            new_page.id = candidate_id;
                            new_page.slug = format!("{}-{}", base_slug, n);
                            break;
                        }
                    }
                }
                self.site.pages.push(new_page);
                self.selected_page = self.site.pages.len() - 1;
                self.selected_node = 0;
                self.selected_column = 0;
                self.selected_component = 0;
                self.selected_nested_item = 0;
                self.modal = None;
                let msg = format!(
                    "Added page: {}",
                    self.site.pages[self.selected_page].head.title
                );
                self.push_toast(ToastLevel::Success, msg);
                Some(ModalResult::CloseSuccess)
            }
            _ => Some(ModalResult::Continue),
        }
    }

    fn handle_new_page_title_prompt_event(
        &mut self,
        key: event::KeyEvent,
    ) -> Option<ModalResult> {
        use crossterm::event::KeyCode;

        let title = if let Some(Modal::NewPageTitlePrompt { title }) = self.modal.take() {
            title
        } else {
            return Some(ModalResult::CloseCancel);
        };

        match key.code {
            KeyCode::Esc => {
                self.status = "Add page cancelled.".to_string();
                Some(ModalResult::CloseCancel)
            }
            KeyCode::Enter => {
                let trimmed = title.trim().to_string();
                if trimmed.is_empty() {
                    self.status = "Title required.".to_string();
                    self.modal = Some(Modal::NewPageTitlePrompt { title });
                    Some(ModalResult::Continue)
                } else {
                    self.pending_new_page_title = Some(trimmed);
                    self.modal = Some(Modal::TemplatePicker { selected: 0 });
                    Some(ModalResult::Continue)
                }
            }
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                let trimmed = title.trim().to_string();
                if trimmed.is_empty() {
                    self.status = "Title required.".to_string();
                    self.modal = Some(Modal::NewPageTitlePrompt { title });
                    Some(ModalResult::Continue)
                } else {
                    self.pending_new_page_title = Some(trimmed);
                    self.modal = Some(Modal::TemplatePicker { selected: 0 });
                    Some(ModalResult::Continue)
                }
            }
            KeyCode::Backspace => {
                let mut new_title = title;
                new_title.pop();
                self.modal = Some(Modal::NewPageTitlePrompt { title: new_title });
                Some(ModalResult::Continue)
            }
            KeyCode::Char(c) => {
                let mut new_title = title;
                new_title.push(c);
                self.modal = Some(Modal::NewPageTitlePrompt { title: new_title });
                Some(ModalResult::Continue)
            }
            _ => {
                self.modal = Some(Modal::NewPageTitlePrompt { title });
                Some(ModalResult::Continue)
            }
        }
    }

    fn handle_export_path_prompt_event(&mut self, key: event::KeyEvent) -> Option<ModalResult> {
        use crossterm::event::KeyCode;
        let path = if let Some(Modal::ExportPathPrompt { path }) = self.modal.take() {
            path
        } else {
            return Some(ModalResult::CloseCancel);
        };
        match key.code {
            KeyCode::Esc => {
                self.push_toast(ToastLevel::Info, "Export cancelled.");
                Some(ModalResult::CloseCancel)
            }
            KeyCode::Enter => self.commit_export_path_from_prompt(path),
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.commit_export_path_from_prompt(path)
            }
            KeyCode::Backspace => {
                let mut new_path = path;
                new_path.pop();
                self.modal = Some(Modal::ExportPathPrompt { path: new_path });
                Some(ModalResult::Continue)
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                let mut new_path = path;
                new_path.push(c);
                self.modal = Some(Modal::ExportPathPrompt { path: new_path });
                Some(ModalResult::Continue)
            }
            _ => {
                self.modal = Some(Modal::ExportPathPrompt { path });
                Some(ModalResult::Continue)
            }
        }
    }

    fn handle_preview_path_prompt_event(&mut self, key: event::KeyEvent) -> Option<ModalResult> {
        use crossterm::event::KeyCode;
        let path = if let Some(Modal::PreviewPathPrompt { path }) = self.modal.take() {
            path
        } else {
            return Some(ModalResult::CloseCancel);
        };
        match key.code {
            KeyCode::Esc => {
                self.push_toast(ToastLevel::Info, "Preview cancelled.");
                Some(ModalResult::CloseCancel)
            }
            KeyCode::Enter => self.commit_preview_path_from_prompt(path),
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.commit_preview_path_from_prompt(path)
            }
            KeyCode::Backspace => {
                let mut new_path = path;
                new_path.pop();
                self.modal = Some(Modal::PreviewPathPrompt { path: new_path });
                Some(ModalResult::Continue)
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                let mut new_path = path;
                new_path.push(c);
                self.modal = Some(Modal::PreviewPathPrompt { path: new_path });
                Some(ModalResult::Continue)
            }
            _ => {
                self.modal = Some(Modal::PreviewPathPrompt { path });
                Some(ModalResult::Continue)
            }
        }
    }

    fn commit_preview_path_from_prompt(&mut self, path: String) -> Option<ModalResult> {
        let trimmed = path.trim();
        if trimmed.is_empty() {
            self.push_toast(ToastLevel::Warning, "Preview path required.");
            self.modal = Some(Modal::PreviewPathPrompt { path });
            Some(ModalResult::Continue)
        } else {
            self.commit_preview_to(trimmed.to_string());
            Some(ModalResult::CloseSuccess)
        }
    }

    fn commit_preview_to(&mut self, rel: String) {
        use std::path::{Path, PathBuf};
        let normalized = normalize_relative_path(&rel);
        let base = self
            .path
            .as_ref()
            .and_then(|p| p.parent().map(PathBuf::from))
            .unwrap_or_else(|| PathBuf::from("."));
        let out = base.join(Path::new(&normalized));

        if let Err(e) = crate::renderer::render_site_to_dir(&self.site, &out) {
            let msg = format!("Preview failed: {}", e);
            self.push_toast(ToastLevel::Warning, msg);
            return;
        }
        self.site.export_dir = Some(normalized.clone());
        self.copy_source_images_to(&base, &out);

        let slug = self.current_page_slug_for_preview();
        let target = out.join(format!("{}.html", slug));
        let display = display_relative_path(&base, &out, &normalized);
        let count = self.site.pages.len();
        self.push_toast(
            ToastLevel::Success,
            format!("Exported {} page(s) to {}", count, display),
        );
        match open_in_browser(&target) {
            Ok(()) => {
                self.push_toast(
                    ToastLevel::Info,
                    format!("Opening {} in browser…", target.display()),
                );
            }
            Err(e) => {
                self.push_toast(
                    ToastLevel::Warning,
                    format!("Browser open failed: {}", e),
                );
            }
        }
    }

    /// Slug of the currently selected page, falling back to the first page
    /// when the selection is somehow out of bounds (or `index` if even that
    /// fails).
    fn current_page_slug_for_preview(&self) -> String {
        let idx = self.selected_page.min(self.site.pages.len().saturating_sub(1));
        self.site
            .pages
            .get(idx)
            .map(|p| p.slug.clone())
            .unwrap_or_else(|| "index".to_string())
    }

    /// Entry point for the `p` key. Mirrors `begin_export_flow` but routes
    /// success to `commit_preview_to` so the browser opens after rendering.
    fn begin_preview_flow(&mut self) {
        let root = self.path.as_ref().and_then(|p| p.parent().map(std::path::Path::to_path_buf));
        let errors = crate::validate::validate_site_with_root(&self.site, root.as_deref());
        if !errors.is_empty() {
            self.modal = Some(Modal::ValidationErrors {
                errors,
                scroll_offset: 0,
            });
            return;
        }
        match self.site.export_dir.clone() {
            Some(dir) if !dir.trim().is_empty() => {
                self.commit_preview_to(dir);
            }
            _ => {
                self.modal = Some(Modal::PreviewPathPrompt {
                    path: "./web/".to_string(),
                });
            }
        }
    }

    /// Entry point for the `E` key. Validates first; opens ValidationErrors
    /// modal on failures. Otherwise resolves the output dir (prompting on first
    /// use) and either opens the prompt or commits the export directly.
    fn begin_export_flow(&mut self) {
        let root = self.path.as_ref().and_then(|p| p.parent().map(std::path::Path::to_path_buf));
        let errors = crate::validate::validate_site_with_root(&self.site, root.as_deref());
        if !errors.is_empty() {
            self.modal = Some(Modal::ValidationErrors {
                errors,
                scroll_offset: 0,
            });
            return;
        }
        match self.site.export_dir.clone() {
            Some(dir) if !dir.trim().is_empty() => {
                self.commit_export_to(dir);
            }
            _ => {
                self.modal = Some(Modal::ExportPathPrompt {
                    path: "./web/".to_string(),
                });
            }
        }
    }

    fn commit_export_path_from_prompt(&mut self, path: String) -> Option<ModalResult> {
        let trimmed = path.trim();
        if trimmed.is_empty() {
            self.push_toast(ToastLevel::Warning, "Export path required.");
            self.modal = Some(Modal::ExportPathPrompt { path });
            Some(ModalResult::Continue)
        } else {
            self.commit_export_to(trimmed.to_string());
            Some(ModalResult::CloseSuccess)
        }
    }

    /// Resolve `rel` against the site JSON's directory (or cwd if no path),
    /// run the renderer, best-effort copy source images, and surface the
    /// outcome as toasts. Persists the normalized `rel` as `site.export_dir`
    /// on success.
    fn commit_export_to(&mut self, rel: String) {
        use std::path::{Path, PathBuf};
        let normalized = normalize_relative_path(&rel);
        let base = self
            .path
            .as_ref()
            .and_then(|p| p.parent().map(PathBuf::from))
            .unwrap_or_else(|| PathBuf::from("."));
        let out = base.join(Path::new(&normalized));

        match crate::renderer::render_site_to_dir(&self.site, &out) {
            Ok(()) => {
                self.site.export_dir = Some(normalized.clone());
                self.copy_source_images_to(&base, &out);
                let page_count = self.site.pages.len();
                let display = display_relative_path(&base, &out, &normalized);
                let msg = format!("Exported {} page(s) to {}", page_count, display);
                self.push_toast(ToastLevel::Success, msg);
            }
            Err(e) => {
                let msg = format!("Export failed: {}", e);
                self.push_toast(ToastLevel::Warning, msg);
            }
        }
    }

    /// Recursively copy `base/source/images/` → `<out>/assets/images/` when
    /// the source exists. Silently skips when absent. Copy errors surface
    /// as a warning toast but don't fail the export. The folder name
    /// matches the path convention used in `parent_image_url` defaults
    /// (`/assets/images/...`).
    fn copy_source_images_to(&mut self, base: &std::path::Path, out: &std::path::Path) {
        let src = base.join("source").join("images");
        if !src.exists() {
            return;
        }
        let dst = out.join("assets").join("images");
        if let Err(e) = copy_dir_recursive(&src, &dst) {
            let msg = format!("Images copy skipped: {}", e);
            self.push_toast(ToastLevel::Warning, msg);
        }
    }

    fn handle_rename_page_prompt_event(&mut self, key: event::KeyEvent) -> Option<ModalResult> {
        use crossterm::event::KeyCode;
        let (title, page_idx) = match &self.modal {
            Some(Modal::RenamePagePrompt { title, page_idx }) => (title.clone(), *page_idx),
            _ => return Some(ModalResult::CloseCancel),
        };
        match key.code {
            KeyCode::Esc => {
                self.modal = None;
                self.status = "Rename cancelled.".to_string();
                Some(ModalResult::CloseCancel)
            }
            KeyCode::Enter => self.commit_rename_page(title, page_idx),
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.commit_rename_page(title, page_idx)
            }
            KeyCode::Backspace => {
                let mut new_title = title;
                new_title.pop();
                self.modal = Some(Modal::RenamePagePrompt {
                    title: new_title,
                    page_idx,
                });
                Some(ModalResult::Continue)
            }
            KeyCode::Char(c) => {
                let mut new_title = title;
                new_title.push(c);
                self.modal = Some(Modal::RenamePagePrompt {
                    title: new_title,
                    page_idx,
                });
                Some(ModalResult::Continue)
            }
            _ => Some(ModalResult::Continue),
        }
    }

    /// Commit the rename modal's current title to the given page_idx.
    /// Regenerates slug from title only when the page's slug is not locked.
    /// Empty titles keep the modal open with a "Title required." status.
    fn commit_rename_page(&mut self, title: String, page_idx: usize) -> Option<ModalResult> {
        let trimmed = title.trim();
        if trimmed.is_empty() {
            self.status = "Title required.".to_string();
            self.modal = Some(Modal::RenamePagePrompt { title, page_idx });
            return Some(ModalResult::Continue);
        }
        if let Some(page) = self.site.pages.get_mut(page_idx) {
            page.head.title = trimmed.to_string();
            if !page.slug_locked {
                page.slug = crate::model::slug_from_title(trimmed);
            }
            let msg = format!("Renamed page: {}", page.head.title);
            self.push_toast(ToastLevel::Success, msg);
        } else {
            self.status = "Page no longer exists.".to_string();
        }
        self.modal = None;
        Some(ModalResult::CloseSuccess)
    }

    fn handle_confirm_prompt_event(&mut self, key: event::KeyEvent) -> Option<ModalResult> {
        use crossterm::event::KeyCode;
        let kind = match &self.modal {
            Some(Modal::ConfirmPrompt { on_confirm, .. }) => on_confirm.clone(),
            _ => return Some(ModalResult::CloseCancel),
        };
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                match kind {
                    ConfirmKind::DeletePage => self.commit_delete_page(),
                }
                self.modal = None;
                Some(ModalResult::CloseSuccess)
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.modal = None;
                self.status = "Cancelled.".to_string();
                Some(ModalResult::CloseCancel)
            }
            _ => Some(ModalResult::Continue),
        }
    }

    fn commit_delete_page(&mut self) {
        if self.site.pages.len() <= 1 {
            self.push_toast(ToastLevel::Warning, "Cannot delete last page.");
            return;
        }
        let idx = self.selected_page.min(self.site.pages.len() - 1);
        let removed = self.site.pages.remove(idx);
        let msg = format!("Deleted page: {}", removed.head.title);
        self.push_toast(ToastLevel::Success, msg);
        self.deleted_pages.push(removed);
        // Cap trash at 20 (oldest dropped).
        if self.deleted_pages.len() > 20 {
            self.deleted_pages.remove(0);
        }
        self.selected_page = idx.min(self.site.pages.len() - 1);
        self.selected_node = 0;
        self.selected_column = 0;
        self.selected_component = 0;
        self.selected_nested_item = 0;
    }

    fn handle_single_field_event_unified(&mut self, key: event::KeyEvent) -> Option<ModalResult> {
        use crossterm::event::KeyCode;

        let (mode, buffer, cursor, multiline) = if let Some(Modal::SingleField {
            mode,
            buffer,
            cursor,
            multiline,
        }) = self.modal.take()
        {
            (mode, buffer, cursor, multiline)
        } else {
            return Some(ModalResult::CloseCancel);
        };

        match key.code {
            KeyCode::Esc => {
                self.input_mode = None;
                Some(ModalResult::CloseCancel)
            }
            KeyCode::Enter => {
                if multiline {
                    // In multiline mode, Enter adds a newline
                    let mut new_buffer = buffer;
                    let mut new_cursor = cursor;
                    let mut chars: Vec<char> = new_buffer.chars().collect();
                    if new_cursor <= chars.len() {
                        chars.insert(new_cursor, '\n');
                        new_buffer = chars.into_iter().collect();
                        new_cursor += 1;
                    }
                    self.modal = Some(Modal::SingleField {
                        mode,
                        buffer: new_buffer,
                        cursor: new_cursor,
                        multiline,
                    });
                    Some(ModalResult::Continue)
                } else {
                    // Single line mode - save and close
                    self.input_buffer = buffer;
                    self.save_input_value(mode);
                    self.input_mode = None;
                    Some(ModalResult::CloseSuccess)
                }
            }
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.input_buffer = buffer;
                self.save_input_value(mode);
                self.input_mode = None;
                Some(ModalResult::CloseSuccess)
            }
            KeyCode::Backspace => {
                let mut new_buffer = buffer;
                let mut new_cursor = cursor;
                if new_cursor > 0 {
                    new_cursor -= 1;
                    if new_cursor < new_buffer.chars().count() {
                        let mut chars: Vec<char> = new_buffer.chars().collect();
                        chars.remove(new_cursor);
                        new_buffer = chars.into_iter().collect();
                    }
                }
                self.modal = Some(Modal::SingleField {
                    mode,
                    buffer: new_buffer,
                    cursor: new_cursor,
                    multiline,
                });
                Some(ModalResult::Continue)
            }
            KeyCode::Left => {
                self.modal = Some(Modal::SingleField {
                    mode,
                    buffer,
                    cursor: cursor.saturating_sub(1),
                    multiline,
                });
                Some(ModalResult::Continue)
            }
            KeyCode::Right => {
                let max = buffer.chars().count();
                self.modal = Some(Modal::SingleField {
                    mode,
                    buffer,
                    cursor: (cursor + 1).min(max),
                    multiline,
                });
                Some(ModalResult::Continue)
            }
            KeyCode::Char(c) => {
                let mut new_buffer = buffer;
                let mut new_cursor = cursor;
                let mut chars: Vec<char> = new_buffer.chars().collect();
                if new_cursor <= chars.len() {
                    chars.insert(new_cursor, c);
                    new_buffer = chars.into_iter().collect();
                    new_cursor += 1;
                }
                self.modal = Some(Modal::SingleField {
                    mode,
                    buffer: new_buffer,
                    cursor: new_cursor,
                    multiline,
                });
                Some(ModalResult::Continue)
            }
            _ => {
                self.modal = Some(Modal::SingleField {
                    mode,
                    buffer,
                    cursor,
                    multiline,
                });
                Some(ModalResult::Continue)
            }
        }
    }

    fn save_input_value(&mut self, mode: InputMode) {
        // Legacy save logic - will be migrated to unified system
        // This is a placeholder that delegates to existing save logic
        let _ = mode;
        // TODO: Implement actual save logic based on mode
    }

    /// Legacy renderers - will be removed after migration
    fn render_edit_modal_legacy(&self, frame: &mut ratatui::Frame, modal: &EditModalState) {
        self.render_edit_modal_unified(
            frame,
            &modal.title,
            &modal.fields,
            modal.selected_field,
            modal.scroll_offset,
            modal.visible_fields,
        );
    }

    fn render_component_picker_legacy(
        &self,
        frame: &mut ratatui::Frame,
        picker: &ComponentPickerState,
    ) {
        self.render_component_picker_unified(frame, &picker.query, picker.selected);
    }

    fn render_save_prompt_legacy(&self, frame: &mut ratatui::Frame) {
        self.render_save_prompt_unified(frame, &self.save_input);
    }

    fn render_input_mode_legacy(&self, frame: &mut ratatui::Frame) {
        if let Some(mode) = self.input_mode {
            self.render_single_field_unified(
                frame,
                mode,
                &self.input_buffer,
                self.input_cursor,
                self.is_multiline_input_mode(),
            );
        }
    }
}

// ============================================================================
// MAIN APP IMPLEMENTATION
// ============================================================================

impl App {
    fn new(mut site: Site, path: Option<PathBuf>, theme: AppTheme) -> Self {
        for page in &mut site.pages {
            ensure_page_section_ids(page);
        }
        let last_saved_json = serde_json::to_string(&site).unwrap_or_default();
        let mut app = Self {
            site,
            theme,
            selected_page: 0,
            selected_node: 0,
            selected_tree_row: 0,
            selected_column: 0,
            selected_component: 0,
            selected_nested_item: 0,
            selected_sidebar_section: SidebarSection::Layouts,
            selected_region: SelectedRegion::Page,
            selected_header_section: 0,
            selected_header_column: 0,
            selected_header_component: 0,
            page_head_selected: false,
            deleted_pages: Vec::new(),
            pending_new_page_title: None,
            toasts: Vec::new(),
            list_area: Rect::default(),
            details_area: Rect::default(),
            details_scroll_row: 0,
            status: "Ready.".to_string(),
            path,
            should_quit: false,
            save_prompt_open: false,
            save_input: String::new(),
            input_mode: None,
            input_buffer: String::new(),
            input_cursor: 0,
            multiline_value_area: None,
            multiline_scroll_row: 0,
            component_picker: None,
            edit_modal: None,
            modal: None,
            component_kind: ComponentKind::Banner,
            show_help: false,
            help_scroll: 0,
            help_scroll_max: 0,
            modal_field_areas: std::cell::RefCell::new(Vec::new()),
            paused_form_edit_modal: None,
            expanded_sections: HashSet::new(),
            expanded_accordion_items: HashSet::new(),
            expanded_alternating_items: HashSet::new(),
            expanded_card_items: HashSet::new(),
            expanded_filmstrip_items: HashSet::new(),
            expanded_milestones_items: HashSet::new(),
            expanded_slider_items: HashSet::new(),
            header_column_expanded: true,
            header_components_expanded: HashSet::new(),
            dirty: false,
            dirty_since: None,
            last_saved_json,
        };

        if let Some(p) = app.path.as_ref() {
            let backup = backup_path_for(p);
            if backup.exists() && p.exists() {
                if let (Ok(main), Ok(bak)) = (
                    std::fs::read_to_string(p),
                    std::fs::read_to_string(&backup),
                ) {
                    if main != bak {
                        let mtime = std::fs::metadata(&backup)
                            .and_then(|m| m.modified())
                            .ok();
                        let when = mtime
                            .and_then(chrono_like_format)
                            .unwrap_or_else(|| "unknown".into());
                        app.push_toast(
                            ToastLevel::Info,
                            format!(
                                "Loaded state differs from last manual save ({}).",
                                when
                            ),
                        );
                    }
                }
            }
        }

        app
    }

    fn run<B: ratatui::backend::Backend>(
        &mut self,
        terminal: &mut Terminal<B>,
    ) -> anyhow::Result<()> {
        while !self.should_quit {
            self.tick_autosave(std::time::Instant::now());
            terminal.draw(|f| self.draw(f))?;

            if event::poll(Duration::from_millis(100))? {
                let evt = event::read()?;
                self.handle_event(evt)?;
                self.mark_dirty_if_changed();
            }
        }

        Ok(())
    }

    fn draw(&mut self, frame: &mut ratatui::Frame) {
        self.prune_toasts();
        self.multiline_value_area = None;
        let page = self.current_page();
        let root = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(3),
            ])
            .split(frame.area());
        let main = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
            .split(root[1]);

        // Header with "dd | Page: {name}" format
        let header_text = format!("dd | Page: {}", page.head.title);
        let header = Paragraph::new(header_text).style(
            Style::default()
                .fg(self.theme.foreground)
                .bg(self.theme.background),
        );
        frame.render_widget(header, root[0]);

        // Split sidebar into three sections: Regions, Pages, Layouts
        let sidebar = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(6), // Regions (Header, Footer)
                Constraint::Length(8), // Pages (numbered list, scrollable)
                Constraint::Min(1),    // Layouts (component tree)
            ])
            .split(main[0]);

        // Determine border colors based on active section
        let regions_border = if self.selected_sidebar_section == SidebarSection::Regions {
            self.theme.active
        } else {
            self.theme.border
        };
        let pages_border = if self.selected_sidebar_section == SidebarSection::Pages {
            self.theme.active
        } else {
            self.theme.border
        };
        let layouts_border = if self.selected_sidebar_section == SidebarSection::Layouts {
            self.theme.active
        } else {
            self.theme.border
        };

        // Regions section (Header, Footer)
        let regions_items: Vec<ListItem> = vec!["  Header", "  Footer"]
            .iter()
            .enumerate()
            .map(|(idx, label)| {
                let is_selected = match self.selected_region {
                    SelectedRegion::Header => idx == 0,
                    SelectedRegion::Footer => idx == 1,
                    SelectedRegion::Page => false,
                };
                let style = if is_selected {
                    Style::default()
                        .fg(self.theme.selected_foreground)
                        .bg(self.theme.selected_background)
                } else {
                    Style::default().fg(self.theme.foreground)
                };
                ListItem::new(*label).style(style)
            })
            .collect();
        let regions_list = List::new(regions_items)
            .block(
                Block::default()
                    .title("[1] Regions")
                    .borders(Borders::ALL)
                    .style(
                        Style::default()
                            .fg(self.theme.foreground)
                            .bg(self.theme.panel_background),
                    )
                    .border_style(Style::default().fg(regions_border))
                    .title_style(
                        Style::default()
                            .fg(self.theme.title)
                            .add_modifier(Modifier::BOLD),
                    ),
            )
            .style(
                Style::default()
                    .fg(self.theme.foreground)
                    .bg(self.theme.panel_background),
            )
            .highlight_style(
                Style::default()
                    .fg(self.theme.selected_foreground)
                    .bg(self.theme.selected_background)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");
        let mut regions_state = ListState::default();
        let regions_selected = match self.selected_region {
            SelectedRegion::Header => Some(0),
            SelectedRegion::Footer => Some(1),
            SelectedRegion::Page => None,
        };
        regions_state.select(regions_selected);
        frame.render_stateful_widget(regions_list, sidebar[0], &mut regions_state);

        // Pages section (numbered list)
        let page_items: Vec<ListItem> = self
            .site
            .pages
            .iter()
            .enumerate()
            .map(|(idx, page)| {
                let num = format!("{:02}", idx + 1);
                let title = page.head.title.trim();
                let label_body = if title.is_empty() {
                    page.slug.as_str()
                } else {
                    title
                };
                let label = format!("{} {}", num, label_body);
                let style = if idx == self.selected_page {
                    Style::default()
                        .fg(self.theme.selected_foreground)
                        .bg(self.theme.selected_background)
                } else {
                    Style::default().fg(self.theme.foreground)
                };
                ListItem::new(label).style(style)
            })
            .collect();
        let pages_list = List::new(page_items)
            .block(
                Block::default()
                    .title("[2] Nodes")
                    .borders(Borders::ALL)
                    .style(
                        Style::default()
                            .fg(self.theme.foreground)
                            .bg(self.theme.panel_background),
                    )
                    .border_style(Style::default().fg(pages_border))
                    .title_style(
                        Style::default()
                            .fg(self.theme.title)
                            .add_modifier(Modifier::BOLD),
                    ),
            )
            .style(
                Style::default()
                    .fg(self.theme.foreground)
                    .bg(self.theme.panel_background),
            )
            .highlight_style(
                Style::default()
                    .fg(self.theme.selected_foreground)
                    .bg(self.theme.selected_background)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");
        let mut pages_state = ListState::default();
        if !self.site.pages.is_empty() {
            pages_state.select(Some(self.selected_page));
        }
        frame.render_widget(pages_list, sidebar[1]);

        // Layouts section (component tree)
        let tree_rows = self.build_tree_rows();
        let layout_items: Vec<ListItem> = tree_rows
            .iter()
            .enumerate()
            .map(|(idx, row)| {
                let label = self.tree_row_label(row);
                let style = if idx == self.selected_tree_row {
                    Style::default()
                        .fg(self.theme.selected_foreground)
                        .bg(self.theme.selected_background)
                } else {
                    Style::default().fg(self.theme.foreground)
                };
                ListItem::new(label).style(style)
            })
            .collect();
        let layouts_list = List::new(layout_items)
            .block(
                Block::default()
                    .title("[3] Layout")
                    .borders(Borders::ALL)
                    .style(
                        Style::default()
                            .fg(self.theme.foreground)
                            .bg(self.theme.panel_background),
                    )
                    .border_style(Style::default().fg(layouts_border))
                    .title_style(
                        Style::default()
                            .fg(self.theme.title)
                            .add_modifier(Modifier::BOLD),
                    ),
            )
            .style(
                Style::default()
                    .fg(self.theme.foreground)
                    .bg(self.theme.panel_background),
            )
            .highlight_style(
                Style::default()
                    .fg(self.theme.selected_foreground)
                    .bg(self.theme.selected_background)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");
        let mut layouts_state = ListState::default();
        if !tree_rows.is_empty() {
            layouts_state.select(Some(self.selected_tree_row.min(tree_rows.len() - 1)));
        }
        frame.render_stateful_widget(layouts_list, sidebar[2], &mut layouts_state);
        self.list_area = sidebar[2];

        self.details_area = main[1];
        let details_width = main[1].width.saturating_sub(2) as usize;
        let details_content = self.details_text(details_width);
        let details_total_rows = details_content.lines().count().max(1);
        let details_visible_rows = main[1].height.saturating_sub(2) as usize;
        let details_max_scroll = details_total_rows.saturating_sub(details_visible_rows);
        self.details_scroll_row = self.details_scroll_row.min(details_max_scroll);
        let details = Paragraph::new(details_content)
            .style(
                Style::default()
                    .fg(self.theme.foreground)
                    .bg(self.theme.panel_background),
            )
            .block(
                Block::default()
                    .title("Details")
                    .borders(Borders::ALL)
                    .style(
                        Style::default()
                            .fg(self.theme.foreground)
                            .bg(self.theme.panel_background),
                    )
                    .border_style(Style::default().fg(self.theme.border))
                    .title_style(
                        Style::default()
                            .fg(self.theme.title)
                            .add_modifier(Modifier::BOLD),
                    ),
            )
            .scroll((self.details_scroll_row.min(u16::MAX as usize) as u16, 0))
            .wrap(Wrap { trim: true });
        frame.render_widget(details, main[1]);

        // Scrollbar on the right edge of the Details panel — only painted
        // when the content exceeds the visible window. Track lives on the
        // last column inside the border; thumb height is proportional to
        // visible/total rows.
        if details_total_rows > details_visible_rows
            && main[1].width >= 3
            && main[1].height >= 4
        {
            let track_x = main[1].x + main[1].width.saturating_sub(2);
            let track_y0 = main[1].y + 1;
            let track_h = main[1].height.saturating_sub(2);
            for row in 0..track_h {
                let cell = Paragraph::new("│").style(
                    Style::default()
                        .fg(self.theme.scrollbar)
                        .bg(self.theme.panel_background),
                );
                frame.render_widget(
                    cell,
                    Rect {
                        x: track_x,
                        y: track_y0 + row,
                        width: 1,
                        height: 1,
                    },
                );
            }
            let total = details_total_rows;
            let visible = details_visible_rows.max(1);
            let track_h_usize = track_h as usize;
            let thumb_h = ((track_h_usize * visible) / total.max(1)).max(1);
            let scroll_range = total.saturating_sub(visible).max(1);
            let thumb_top = (self.details_scroll_row * track_h_usize.saturating_sub(thumb_h))
                / scroll_range;
            for i in 0..thumb_h {
                let cell = Paragraph::new("█").style(
                    Style::default()
                        .fg(self.theme.scrollbar_hover)
                        .bg(self.theme.panel_background),
                );
                frame.render_widget(
                    cell,
                    Rect {
                        x: track_x,
                        y: track_y0 + (thumb_top + i) as u16,
                        width: 1,
                        height: 1,
                    },
                );
            }
        }

        let footer_text = format!(
            "F1 help | q quit | s save | / insert | Enter edit | Space expand/collapse | A add collection item | X remove collection item | {}",
            self.status
        );
        let footer = Paragraph::new(footer_text)
            .style(
                Style::default()
                    .fg(self.theme.muted)
                    .bg(self.theme.background),
            )
            .block(
                Block::default()
                    .title("Status")
                    .borders(Borders::ALL)
                    .style(
                        Style::default()
                            .fg(self.theme.foreground)
                            .bg(self.theme.background),
                    )
                    .border_style(Style::default().fg(self.theme.border))
                    .title_style(
                        Style::default()
                            .fg(self.theme.title)
                            .add_modifier(Modifier::BOLD),
                    ),
            );
        frame.render_widget(footer, root[2]);

        if self.show_help {
            let area = centered_rect(80, 80, frame.area());
            frame.render_widget(Clear, area);
            let block = Block::default()
                .title("Keybindings (F1 / Esc to close, j/k or arrows to scroll)")
                .borders(Borders::ALL)
                .style(
                    Style::default()
                        .fg(self.theme.foreground)
                        .bg(self.theme.popup_background),
                )
                .border_style(Style::default().fg(self.theme.border_active))
                .title_style(
                    Style::default()
                        .fg(self.theme.title)
                        .add_modifier(Modifier::BOLD),
                );
            let inner = block.inner(area);
            frame.render_widget(block, area);

            // Reserve a 1-col gutter on the right for the scrollbar so the
            // body wraps before reaching the modal border.
            let scrollbar_width: u16 = 1;
            let body_w = inner.width.saturating_sub(scrollbar_width + 1);
            let body_area = Rect {
                x: inner.x,
                y: inner.y,
                width: body_w,
                height: inner.height,
            };

            // Pre-wrap the help text against `body_w` so we know the total
            // wrapped row count and can clamp + size the scrollbar thumb.
            let raw = help_text();
            let wrapped_total = wrap_help_lines(&raw, body_w as usize);
            let visible = inner.height as usize;
            let max_scroll = wrapped_total.saturating_sub(visible) as u16;
            // Publish the max so event handlers can clamp on key/wheel events.
            self.help_scroll_max = max_scroll;
            // Clamp on render too so an inflated stored value (e.g. from
            // pressing G/End before wrapping was known) snaps back into range.
            if self.help_scroll > max_scroll {
                self.help_scroll = max_scroll;
            }
            let scroll = self.help_scroll;

            let body = Paragraph::new(raw)
                .style(
                    Style::default()
                        .fg(self.theme.foreground)
                        .bg(self.theme.popup_background),
                )
                .wrap(Wrap { trim: false })
                .scroll((scroll, 0));
            frame.render_widget(body, body_area);

            // Scrollbar: track + thumb at the right edge of `inner`.
            if (wrapped_total as u16) > inner.height {
                let track_x = inner.x + inner.width.saturating_sub(1);
                for row in 0..inner.height {
                    let cell = Paragraph::new("│").style(
                        Style::default()
                            .fg(self.theme.scrollbar)
                            .bg(self.theme.popup_background),
                    );
                    frame.render_widget(
                        cell,
                        Rect {
                            x: track_x,
                            y: inner.y + row,
                            width: 1,
                            height: 1,
                        },
                    );
                }
                // Thumb size proportional to visible/total.
                let total_h = inner.height as usize;
                let thumb_h = ((total_h * total_h) / wrapped_total.max(1)).max(1);
                let scroll_range = wrapped_total.saturating_sub(total_h).max(1);
                let thumb_top = ((scroll as usize) * total_h.saturating_sub(thumb_h))
                    / scroll_range;
                for i in 0..thumb_h {
                    let cell = Paragraph::new("█").style(
                        Style::default()
                            .fg(self.theme.scrollbar_hover)
                            .bg(self.theme.popup_background),
                    );
                    frame.render_widget(
                        cell,
                        Rect {
                            x: track_x,
                            y: inner.y + (thumb_top + i) as u16,
                            width: 1,
                            height: 1,
                        },
                    );
                }
            }
        }

        // Render edit modal if open
        if let Some(modal) = &self.edit_modal {
            // Use most of the available space, with minimum dimensions
            let area = centered_rect(95, 90, frame.area());
            frame.render_widget(Clear, area);

            // Draw modal background and border
            let modal_block = Block::default()
                .title(format!("Edit Item - {}", modal.title))
                .borders(Borders::ALL)
                .style(
                    Style::default()
                        .fg(self.theme.foreground)
                        .bg(self.theme.popup_background),
                )
                .border_style(Style::default().fg(self.theme.border))
                .title_style(
                    Style::default()
                        .fg(self.theme.title)
                        .add_modifier(Modifier::BOLD),
                );
            frame.render_widget(modal_block.clone(), area);

            let inner = modal_block.inner(area);
            let field_height = 4u16; // Label (1) + input box (3)
            let header_height = 2u16; // Header text + spacing
            let scroll_indicator_height = 1u16;
            let available_height = inner
                .height
                .saturating_sub(header_height + scroll_indicator_height);
            let visible_fields = (available_height / field_height).max(1) as usize;
            let total_fields = modal.fields.len();

            // Clone all data we need from modal before any mutable borrows
            let scroll_offset = modal.scroll_offset;
            let selected_field = modal.selected_field;

            let start = scroll_offset.min(total_fields.saturating_sub(1));
            let end = (start + visible_fields).min(total_fields);

            // Clone the fields we need to display to avoid borrow issues
            let fields_to_render: Vec<EditField> =
                modal.fields[start..end].iter().cloned().collect();

            // Update visible_fields on modal for event handler to use
            if let Some(modal_mut) = &mut self.edit_modal {
                modal_mut.visible_fields = visible_fields;
            }

            // Header text
            let header_text = "Tab/Up/Down: navigate | Ctrl+S: save | Esc: cancel";
            let header = Paragraph::new(header_text).style(Style::default().fg(self.theme.muted));
            let header_area = Rect {
                x: inner.x,
                y: inner.y,
                width: inner.width,
                height: 1,
            };
            frame.render_widget(header, header_area);

            // Render each field as an input box
            let mut cursor_pos: Option<(u16, u16)> = None;
            let has_scrollbar = total_fields > visible_fields;
            let scrollbar_width = if has_scrollbar { 3 } else { 0 };

            for (rel_idx, field) in fields_to_render.iter().enumerate() {
                let abs_idx = start + rel_idx;
                let is_selected = abs_idx == selected_field;
                let y_offset = header_height + (rel_idx as u16 * field_height);

                if y_offset + field_height > inner.height.saturating_sub(scroll_indicator_height) {
                    break;
                }

                let field_area = Rect {
                    x: inner.x + 1,
                    y: inner.y + y_offset,
                    width: inner.width.saturating_sub(2 + scrollbar_width),
                    height: field_height,
                };

                // Label
                let label = Paragraph::new(format!("{}:", field.label))
                    .style(Style::default().fg(self.theme.foreground));
                let label_area = Rect {
                    x: field_area.x,
                    y: field_area.y,
                    width: field_area.width,
                    height: 1,
                };
                frame.render_widget(label, label_area);

                // Input box with border
                let input_border_color = if is_selected {
                    self.theme.input_focus
                } else {
                    self.theme.input_default
                };

                let display_value = if field.buffer.is_empty() {
                    " ".to_string() // Space to ensure border renders
                } else {
                    field.buffer.clone()
                };

                // Input box with border - need height 3 to show border + text
                let input_box = Paragraph::new(display_value)
                    .style(
                        Style::default()
                            .fg(self.theme.foreground)
                            .bg(self.theme.popup_background),
                    )
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(input_border_color)),
                    );

                let input_area = Rect {
                    x: field_area.x,
                    y: field_area.y + 1,
                    width: field_area.width,
                    height: 3,
                };
                frame.render_widget(input_box, input_area);

                // Track cursor position for focused field
                if is_selected {
                    let cursor_x = input_area.x
                        + 1
                        + (field.cursor as u16).min(input_area.width.saturating_sub(3));
                    let cursor_y = input_area.y + 1; // Inside the box (middle row)
                    cursor_pos = Some((cursor_x, cursor_y));
                }
            }

            // Scroll indicator at bottom
            if total_fields > visible_fields {
                let scroll_text = format!("{}-{} of {}", start + 1, end, total_fields);
                let scroll_indicator =
                    Paragraph::new(scroll_text).style(Style::default().fg(self.theme.muted));
                let scroll_area = Rect {
                    x: inner.x + 1,
                    y: inner.y + inner.height.saturating_sub(1),
                    width: inner.width.saturating_sub(3), // Leave room for scrollbar
                    height: 1,
                };
                frame.render_widget(scroll_indicator, scroll_area);

                // Draw scrollbar on the right side
                let scrollbar_x = inner.x + inner.width.saturating_sub(2);
                let scrollbar_top: u16 = header_height + 1;
                let scrollbar_height: u16 = inner
                    .height
                    .saturating_sub(header_height + scroll_indicator_height + 1);

                // Draw track
                for y_offset in 0..scrollbar_height {
                    let y = scrollbar_top + y_offset;
                    frame.render_widget(
                        Paragraph::new("│").style(Style::default().fg(self.theme.border)),
                        Rect {
                            x: scrollbar_x,
                            y: inner.y + y,
                            width: 1,
                            height: 1,
                        },
                    );
                }

                // Calculate thumb position
                let scrollbar_height_usize = scrollbar_height as usize;
                let thumb_size = ((visible_fields * scrollbar_height_usize) / total_fields).max(1);
                let thumb_pos: u16 = if total_fields > visible_fields {
                    ((scroll_offset * (scrollbar_height_usize - thumb_size))
                        / (total_fields - visible_fields)) as u16
                } else {
                    0
                };

                // Draw thumb
                for i in 0..(thumb_size as u16) {
                    let y = scrollbar_top + thumb_pos + i;
                    if y < scrollbar_top + scrollbar_height {
                        frame.render_widget(
                            Paragraph::new("█").style(Style::default().fg(self.theme.active)),
                            Rect {
                                x: scrollbar_x,
                                y: inner.y + y,
                                width: 1,
                                height: 1,
                            },
                        );
                    }
                }
            }

            // Set cursor position for typing
            if let Some((x, y)) = cursor_pos {
                frame.set_cursor_position((x, y));
            }

            return;
        }

        if self.input_mode.is_some() {
            let area = centered_rect(72, 60, frame.area());
            frame.render_widget(Clear, area);
            let _edit_help = self.current_modal_fields();
            let value_block = if self.is_multiline_input_mode() {
                self.ensure_multiline_cursor_visible();
                let inner_width = area.width.saturating_sub(2) as usize;
                let box_inner = inner_width.saturating_sub(2).max(10);
                let visible_rows = self.multiline_rows();
                let all_lines = input_lines_preserve(&self.input_buffer);
                let total_rows = all_lines.len().max(1);
                let has_scroll = total_rows > visible_rows;
                let content_width = if has_scroll {
                    box_inner.saturating_sub(1).max(1)
                } else {
                    box_inner
                };
                let start = self.multiline_scroll_row.min(total_rows.saturating_sub(1));
                let end = (start + visible_rows).min(total_rows);
                let mut lines = all_lines[start..end].to_vec();
                while lines.len() < visible_rows {
                    lines.push(String::new());
                }
                self.multiline_value_area = Some(Rect {
                    x: area.x.saturating_add(2),
                    y: area.y.saturating_add(5),
                    width: content_width.min(u16::MAX as usize) as u16,
                    height: visible_rows.min(u16::MAX as usize) as u16,
                });
                let thumb_row = if has_scroll {
                    Some((self.multiline_scroll_row * visible_rows) / total_rows)
                } else {
                    None
                };
                let mut row_lines = Vec::with_capacity(visible_rows);
                for (idx, line) in lines.iter().enumerate() {
                    let mut rendered = fit_ascii_cell(line, content_width);
                    if has_scroll {
                        let ch = if Some(idx) == thumb_row { '#' } else { '|' };
                        rendered.push(ch);
                    }
                    row_lines.push(format!("|{}|", rendered));
                }
                let field_label = match self.input_mode {
                    Some(InputMode::EditHeroCopy) => {
                        "Value (textarea, 3 rows; Enter newline | Ctrl+S save):"
                    }
                    Some(InputMode::EditAlternatingItemCopy) => {
                        "Value (textarea, 5 rows; Enter newline | Ctrl+S save):"
                    }
                    Some(InputMode::EditBlockquoteCopy) => {
                        "Value (textarea, 5 rows; Enter newline | Ctrl+S save):"
                    }
                    Some(InputMode::EditCardItemCopy) => {
                        "Value (textarea, 5 rows; Enter newline | Ctrl+S save):"
                    }
                    Some(InputMode::EditAccordionFirstContent) => {
                        "Value (textarea, 5 rows; Enter newline | Ctrl+S save):"
                    }
                    _ => "Value:",
                };
                format!(
                    "{}\n+{}+\n{}\n+{}+",
                    field_label,
                    "-".repeat(box_inner),
                    row_lines.join("\n"),
                    "-".repeat(box_inner)
                )
            } else {
                // Single-line input - will be rendered separately with styled border
                self.input_buffer.clone()
            };

            // Render modal content - simplified, no editable fields list
            let modal_content = if self.is_multiline_input_mode() {
                format!(
                    "Editing: {}\n\n{}\n\nEnter: save | Esc: cancel",
                    self.current_input_mode_label(),
                    value_block
                )
            } else {
                format!(
                    "Editing: {}\n\nEnter: save | Esc: cancel",
                    self.current_input_mode_label()
                )
            };

            let modal = Paragraph::new(modal_content)
                .style(
                    Style::default()
                        .fg(self.theme.foreground)
                        .bg(self.theme.popup_background),
                )
                .block(
                    Block::default()
                        .title("Edit Item")
                        .borders(Borders::ALL)
                        .style(
                            Style::default()
                                .fg(self.theme.foreground)
                                .bg(self.theme.popup_background),
                        )
                        .border_style(Style::default().fg(self.theme.input_focus))
                        .title_style(
                            Style::default()
                                .fg(self.theme.title)
                                .add_modifier(Modifier::BOLD),
                        ),
                )
                .wrap(Wrap { trim: true });
            frame.render_widget(modal, area);

            // Render single-line input box with input_default border
            if !self.is_multiline_input_mode() {
                let input_area = Rect {
                    x: area.x + 2,
                    y: area.y + 3,
                    width: area.width.saturating_sub(4),
                    height: 3,
                };
                let input_block = Paragraph::new(value_block.as_str())
                    .style(
                        Style::default()
                            .fg(self.theme.foreground)
                            .bg(self.theme.popup_background),
                    )
                    .block(
                        Block::default()
                            .title("Value")
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(self.theme.input_default)),
                    );
                frame.render_widget(input_block, input_area);
            }
        }

        if let Some(picker) = &self.component_picker {
            let area = centered_rect(70, 70, frame.area());
            frame.render_widget(Clear, area);
            let filtered = self.filtered_component_kinds(&picker.query);
            let mut lines = Vec::new();
            lines.push(format!("Search: {}", picker.query));
            lines.push(String::new());
            if filtered.is_empty() {
                lines.push("No component matches query.".to_string());
            } else {
                for (idx, kind) in filtered.iter().enumerate() {
                    let marker = if idx == picker.selected { ">" } else { " " };
                    lines.push(format!("{marker} {}", kind.label()));
                }
            }
            lines.push(String::new());
            lines.push(
                "Type to fuzzy search (e.g. hero, dd-cta, dd-milestones, dd-modal, dd-slider)."
                    .to_string(),
            );
            lines.push("Up/Down to choose, Enter to add, Esc to cancel.".to_string());
            let picker_widget = Paragraph::new(lines.join("\n"))
                .style(
                    Style::default()
                        .fg(self.theme.foreground)
                        .bg(self.theme.popup_background),
                )
                .block(
                    Block::default()
                        .title("Add Component")
                        .borders(Borders::ALL)
                        .style(
                            Style::default()
                                .fg(self.theme.foreground)
                                .bg(self.theme.popup_background),
                        )
                        .border_style(Style::default().fg(self.theme.border))
                        .title_style(
                            Style::default()
                                .fg(self.theme.title)
                                .add_modifier(Modifier::BOLD),
                        ),
                )
                .wrap(Wrap { trim: true });
            frame.render_widget(picker_widget, area);
        }

        if self.save_prompt_open {
            let area = centered_rect(70, 35, frame.area());
            frame.render_widget(Clear, area);
            let prompt = Paragraph::new(format!(
                "Save file path:\n{}\n\nEnter: save | Esc: cancel",
                self.save_input
            ))
            .style(
                Style::default()
                    .fg(self.theme.foreground)
                    .bg(self.theme.popup_background),
            )
            .block(
                Block::default()
                    .title("Save Page")
                    .borders(Borders::ALL)
                    .style(
                        Style::default()
                            .fg(self.theme.foreground)
                            .bg(self.theme.popup_background),
                    )
                    .border_style(Style::default().fg(self.theme.border))
                    .title_style(
                        Style::default()
                            .fg(self.theme.title)
                            .add_modifier(Modifier::BOLD),
                    ),
            )
            .wrap(Wrap { trim: true });
            frame.render_widget(prompt, area);
        }

        // Render unified modal if open (handles all modal types)
        self.render_modal(frame);

        // Toasts paint last so they float above everything except the
        // active-input cursor overlay.
        self.render_toasts(frame, frame.area());

        let cursor_overlay = self.set_cursor_for_active_input(frame);
        if let Some((x, y, ch)) = cursor_overlay {
            let cursor_cell = Paragraph::new(ch.to_string()).style(
                Style::default()
                    .fg(self.theme.selected_foreground)
                    .bg(self.theme.selected_background)
                    .add_modifier(Modifier::BOLD),
            );
            frame.render_widget(
                cursor_cell,
                Rect {
                    x,
                    y,
                    width: 1,
                    height: 1,
                },
            );
        }
    }

    fn set_cursor_for_active_input(&self, frame: &mut ratatui::Frame) -> Option<(u16, u16, char)> {
        // Unified modal cursor is set directly in render function, skip here
        if self.modal.is_some() || self.edit_modal.is_some() {
            return None;
        }

        if self.input_mode.is_some() {
            let area = centered_rect(72, 60, frame.area());
            let inner_width = area.width.saturating_sub(2) as usize;
            let (x, y) = if self.is_multiline_input_mode() {
                let (row_idx, col_count) = cursor_row_col(&self.input_buffer, self.input_cursor);
                let value_area = self.multiline_value_area.unwrap_or(Rect {
                    x: area.x.saturating_add(2),
                    y: area.y.saturating_add(5),
                    width: inner_width.saturating_sub(2).min(u16::MAX as usize) as u16,
                    height: self.multiline_rows().min(u16::MAX as usize) as u16,
                });
                let max_col = value_area.width.saturating_sub(1) as usize;
                let visible_row = row_idx.saturating_sub(self.multiline_scroll_row);
                let max_row = self.multiline_rows().saturating_sub(1);
                (
                    value_area.x.saturating_add(col_count.min(max_col) as u16),
                    value_area.y.saturating_add(visible_row.min(max_row) as u16),
                )
            } else {
                // Cursor is inside the single-line input box
                // Input box is at: x=area.x+2, y=area.y+3
                // Text starts inside the border at: x+3 (2 for box margin + 1 for border), y+4 (3 for box + 1 for title/border)
                (
                    area.x.saturating_add(3).saturating_add(
                        self.input_cursor
                            .min(inner_width.saturating_sub(1)) as u16,
                    ),
                    area.y.saturating_add(4),
                )
            };
            frame.set_cursor_position((x, y));
            return Some((x, y, self.current_input_cursor_glyph()));
        }

        if self.save_prompt_open {
            let area = centered_rect(70, 35, frame.area());
            let inner_width = area.width.saturating_sub(2) as usize;
            let x = area.x.saturating_add(1).saturating_add(
                self.save_input
                    .chars()
                    .count()
                    .min(inner_width.saturating_sub(1)) as u16,
            );
            let y = area.y.saturating_add(2);
            frame.set_cursor_position((x, y));
            let ch = self
                .save_input
                .chars()
                .nth(self.save_input.chars().count())
                .unwrap_or(' ');
            return Some((x, y, ch));
        }

        if let Some(picker) = &self.component_picker {
            let area = centered_rect(70, 70, frame.area());
            let prefix = "Search: ";
            let inner_width = area.width.saturating_sub(2) as usize;
            let max_query_width = inner_width.saturating_sub(prefix.chars().count() + 1);
            let x = area
                .x
                .saturating_add(1)
                .saturating_add(prefix.chars().count() as u16)
                .saturating_add(picker.query.chars().count().min(max_query_width) as u16);
            let y = area.y.saturating_add(1);
            frame.set_cursor_position((x, y));
            return Some((x, y, ' '));
        }
        None
    }

    fn current_input_cursor_glyph(&self) -> char {
        if self.input_cursor >= self.input_buffer.chars().count() {
            return ' ';
        }
        let ch = self
            .input_buffer
            .chars()
            .nth(self.input_cursor)
            .unwrap_or(' ');
        if ch == '\n' {
            ' '
        } else {
            ch
        }
    }

    /// Try to handle a key as a Pages-panel-scoped action.
    /// Returns `true` if the key was consumed — caller should short-circuit.
    /// Future tasks populate this; today it always returns false.
    fn try_handle_pages_panel_key(&mut self, key: &event::KeyEvent) -> bool {
        use crossterm::event::{KeyCode, KeyModifiers};
        match key.code {
            KeyCode::Char('A') if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.modal = Some(Modal::NewPageTitlePrompt {
                    title: String::new(),
                });
                self.status = "New page: type a title, Enter to continue.".to_string();
                true
            }
            KeyCode::Char('X') if key.modifiers.contains(KeyModifiers::SHIFT) => {
                if self.site.pages.len() <= 1 {
                    self.push_toast(ToastLevel::Warning, "Cannot delete last page.");
                } else {
                    let title = self.site.pages[self.selected_page].head.title.clone();
                    self.modal = Some(Modal::ConfirmPrompt {
                        message: format!("Delete \"{}\"? y/n", title),
                        on_confirm: ConfirmKind::DeletePage,
                    });
                }
                true
            }
            KeyCode::Char('u') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                if let Some(page) = self.deleted_pages.pop() {
                    let msg = format!("Restored page: {}", page.head.title);
                    self.push_toast(ToastLevel::Success, msg);
                    self.site.pages.push(page);
                    self.selected_page = self.site.pages.len() - 1;
                    self.selected_node = 0;
                    self.selected_column = 0;
                    self.selected_component = 0;
                    self.selected_nested_item = 0;
                } else {
                    self.push_toast(ToastLevel::Warning, "No deleted pages to restore.");
                }
                true
            }
            KeyCode::Char('J') if key.modifiers.contains(KeyModifiers::SHIFT) => {
                let idx = self.selected_page;
                if idx + 1 < self.site.pages.len() {
                    self.site.pages.swap(idx, idx + 1);
                    self.selected_page = idx + 1;
                    self.push_toast(ToastLevel::Success, "Moved page down.");
                }
                true
            }
            KeyCode::Char('K') if key.modifiers.contains(KeyModifiers::SHIFT) => {
                let idx = self.selected_page;
                if idx > 0 {
                    self.site.pages.swap(idx, idx - 1);
                    self.selected_page = idx - 1;
                    self.push_toast(ToastLevel::Success, "Moved page up.");
                }
                true
            }
            KeyCode::Char('r')
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::SHIFT) =>
            {
                let idx = self.selected_page;
                let current_title = self.site.pages[idx].head.title.clone();
                self.modal = Some(Modal::RenamePagePrompt {
                    title: current_title,
                    page_idx: idx,
                });
                self.status = "Rename page. Edit and press Enter.".to_string();
                true
            }
            _ => false,
        }
    }

    fn handle_event(&mut self, evt: Event) -> anyhow::Result<()> {
        // Unified modal handling - takes priority over legacy modals
        if let Some(modal_result) = self.handle_modal_event(evt.clone()) {
            match modal_result {
                ModalResult::Continue => return Ok(()),
                ModalResult::CloseSuccess => return Ok(()),
                ModalResult::CloseCancel => return Ok(()),
            }
        }

        if self.show_help {
            match evt {
                Event::Key(k) => match k.code {
                    KeyCode::F(1) | KeyCode::Esc => {
                        self.show_help = false;
                        self.help_scroll = 0;
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.help_scroll = self
                            .help_scroll
                            .saturating_add(1)
                            .min(self.help_scroll_max);
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.help_scroll = self.help_scroll.saturating_sub(1);
                    }
                    KeyCode::PageDown => {
                        self.help_scroll = self
                            .help_scroll
                            .saturating_add(10)
                            .min(self.help_scroll_max);
                    }
                    KeyCode::PageUp => {
                        self.help_scroll = self.help_scroll.saturating_sub(10);
                    }
                    KeyCode::Home | KeyCode::Char('g') => {
                        self.help_scroll = 0;
                    }
                    KeyCode::End | KeyCode::Char('G') => {
                        self.help_scroll = self.help_scroll_max;
                    }
                    _ => {}
                },
                Event::Mouse(m) => match m.kind {
                    MouseEventKind::ScrollUp => {
                        self.help_scroll = self.help_scroll.saturating_sub(3);
                    }
                    MouseEventKind::ScrollDown => {
                        self.help_scroll = self
                            .help_scroll
                            .saturating_add(3)
                            .min(self.help_scroll_max);
                    }
                    _ => {}
                },
                _ => {}
            }
            return Ok(());
        }

        if self.save_prompt_open {
            return self.handle_save_prompt_event(evt);
        }

        if self.component_picker.is_some() {
            return self.handle_component_picker_event(evt);
        }

        if self.edit_modal.is_some() {
            return self.handle_edit_modal_event(evt);
        }

        if self.input_mode.is_some() {
            return self.handle_input_mode(evt);
        }
        match evt {
            Event::Key(k) => {
                if self.selected_sidebar_section == SidebarSection::Pages
                    && self.try_handle_pages_panel_key(&k)
                {
                    self.sync_tree_row_with_selection();
                    return Ok(());
                }
                match k.code {
                KeyCode::F(1) => self.show_help = true,
                KeyCode::F(3) => self.open_validation_modal(),
                KeyCode::Char('E') if k.modifiers.contains(KeyModifiers::SHIFT) => {
                    self.begin_export_flow();
                }
                KeyCode::Char('p') if !k.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.begin_preview_flow();
                }
                KeyCode::Char('q') if k.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.should_quit = true
                }
                KeyCode::Up => self.handle_up(),
                KeyCode::Down => self.handle_down(),
                KeyCode::Char('k') => self.handle_up(),
                KeyCode::Char('j') => self.handle_down(),
                KeyCode::Char('h') => self.vim_collapse_selected_row(),
                KeyCode::Char('l') => self.vim_expand_selected_row(),
                KeyCode::Char('g') => self.vim_jump_to_first_row(),
                KeyCode::Char('G') => self.vim_jump_to_last_row(),
                KeyCode::PageUp => self.scroll_details_by(-5),
                KeyCode::PageDown => self.scroll_details_by(5),
                KeyCode::Char(' ') => self.toggle_selected_tree_expanded(),
                KeyCode::Enter => self.handle_enter_on_selected_row(),
                KeyCode::Tab => self.select_next_page(),
                KeyCode::BackTab => self.select_prev_page(),
                KeyCode::Char('s') => self.begin_save_prompt(),
                KeyCode::Char('/') => self.open_component_picker(),
                KeyCode::Char('d') => self.delete_selected_node(),
                KeyCode::Char('J') => self.move_selected_column_down(),
                KeyCode::Char('K') => self.move_selected_column_up(),
                KeyCode::Char('C') => self.add_column(),
                KeyCode::Char('V') => self.remove_selected_column(),
                KeyCode::Char('c') => self.select_prev_column(),
                KeyCode::Char('v') => self.select_next_column(),
                KeyCode::Char('r') => self.begin_edit_selected_column_id(),
                KeyCode::Char('f') => self.begin_edit_selected_column_width_class(),
                KeyCode::Char('A') => self.add_selected_collection_item(),
                KeyCode::Char('X') => self.remove_selected_collection_item(),
                KeyCode::Char('1') => {
                    self.selected_sidebar_section = SidebarSection::Regions;
                    self.status = "Switched to Regions section.".to_string();
                }
                KeyCode::Char('2') => {
                    self.selected_sidebar_section = SidebarSection::Pages;
                    self.selected_region = SelectedRegion::Page;
                    self.selected_tree_row = 0;
                    self.sync_tree_row_with_selection();
                    self.status = "Switched to Pages section.".to_string();
                }
                KeyCode::Char('3') => {
                    self.selected_sidebar_section = SidebarSection::Layouts;
                    self.status = "Switched to Layout section.".to_string();
                }
                KeyCode::Char('4') => {
                    self.status = "Details panel active.".to_string();
                }
                _ => {}
                }
            },
            Event::Mouse(m) => match m.kind {
                MouseEventKind::ScrollUp => {
                    if contains(self.details_area, m.column, m.row) {
                        self.scroll_details_by(-3);
                    } else {
                        self.select_prev();
                    }
                }
                MouseEventKind::ScrollDown => {
                    if contains(self.details_area, m.column, m.row) {
                        self.scroll_details_by(3);
                    } else {
                        self.select_next();
                    }
                }
                MouseEventKind::Down(MouseButton::Left) => {
                    self.handle_click(m.column, m.row);
                }
                _ => {}
            },
            _ => {}
        }
        self.sync_tree_row_with_selection();
        Ok(())
    }

    fn handle_input_mode(&mut self, evt: Event) -> anyhow::Result<()> {
        match evt {
            Event::Key(key) => match key.code {
                KeyCode::Esc => {
                    self.input_mode = None;
                    self.input_buffer.clear();
                    self.input_cursor = 0;
                    self.multiline_scroll_row = 0;
                    self.status = "Edit cancelled.".to_string();
                    self.sync_tree_row_with_selection();
                }
                KeyCode::Enter => {
                    if self.is_multiline_input_mode()
                        && !key.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        let rows = input_lines_preserve(&self.input_buffer).len().max(1);
                        let max_rows = self.multiline_max_rows();
                        if max_rows.map(|limit| rows < limit).unwrap_or(true) {
                            self.insert_char_at_cursor('\n');
                        } else {
                            self.status = match self.input_mode {
                                Some(InputMode::EditHeroCopy) => {
                                    "hero.copy supports up to 3 lines. Press Ctrl+S to save."
                                        .to_string()
                                }
                                _ => "Line limit reached. Press Ctrl+S to save.".to_string(),
                            };
                        }
                        return Ok(());
                    }
                    let _ = self.commit_input_edit();
                    self.sync_tree_row_with_selection();
                }
                KeyCode::Char('s')
                    if key.modifiers.contains(KeyModifiers::CONTROL)
                        && self.is_multiline_input_mode() =>
                {
                    let _ = self.commit_input_edit();
                    self.sync_tree_row_with_selection();
                }
                KeyCode::Tab => {
                    self.tab_next_component_field();
                    self.sync_tree_row_with_selection();
                }
                KeyCode::BackTab => {
                    self.tab_prev_component_field();
                    self.sync_tree_row_with_selection();
                }
                KeyCode::Up => {
                    if self.is_multiline_input_mode() {
                        self.move_cursor_up_line();
                    }
                }
                KeyCode::Down => {
                    if self.is_multiline_input_mode() {
                        self.move_cursor_down_line();
                    }
                }
                KeyCode::Left => match self.input_mode {
                    Some(InputMode::EditHeroClass) => {
                        self.cycle_hero_parent_class(false);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditHeroClass) {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditHeroAos) => {
                        self.cycle_hero_parent_data_aos(false);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditHeroAos) {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditHeroCtaTarget) => {
                        self.cycle_hero_link_1_target(false, false);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditHeroCtaTarget)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditHeroCtaTarget2) => {
                        self.cycle_hero_link_1_target(true, false);
                        if let Some(v) =
                            self.value_for_component_mode(InputMode::EditHeroCtaTarget2)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditSectionClass) => {
                        self.cycle_section_class(false);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditSectionClass)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditBannerClass) => {
                        self.cycle_banner_parent_class(false);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditBannerClass) {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditBannerDataAos) => {
                        self.cycle_banner_parent_data_aos(false);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditBannerDataAos)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditCtaClass) => {
                        self.cycle_cta_parent_class(false);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditCtaClass) {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditCtaDataAos) => {
                        self.cycle_cta_parent_data_aos(false);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditCtaDataAos) {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditCtaLinkTarget) => {
                        self.cycle_parent_link_target(false);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditCtaLinkTarget)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditBlockquoteDataAos) => {
                        self.cycle_blockquote_parent_data_aos(false);
                        if let Some(v) =
                            self.value_for_component_mode(InputMode::EditBlockquoteDataAos)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditCardType) => {
                        self.cycle_card_parent_type(false);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditCardType) {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditCardDataAos) => {
                        self.cycle_card_parent_data_aos(false);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditCardDataAos) {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditCardItemLinkTarget) => {
                        self.cycle_child_link_target(false);
                        if let Some(v) =
                            self.value_for_component_mode(InputMode::EditCardItemLinkTarget)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditFilmstripType) => {
                        self.cycle_filmstrip_parent_type(false);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditFilmstripType)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditFilmstripDataAos) => {
                        self.cycle_filmstrip_parent_data_aos(false);
                        if let Some(v) =
                            self.value_for_component_mode(InputMode::EditFilmstripDataAos)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditMilestonesDataAos) => {
                        self.cycle_milestones_data_aos(false);
                        if let Some(v) =
                            self.value_for_component_mode(InputMode::EditMilestonesDataAos)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditMilestonesItemLinkTarget) => {
                        self.cycle_milestones_link_target(false);
                        if let Some(v) =
                            self.value_for_component_mode(InputMode::EditMilestonesItemLinkTarget)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditSliderItemLinkTarget) => {
                        self.cycle_slider_link_target(false);
                        if let Some(v) =
                            self.value_for_component_mode(InputMode::EditSliderItemLinkTarget)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditAlternatingType) => {
                        self.cycle_alternating_parent_type(false);
                        if let Some(v) =
                            self.value_for_component_mode(InputMode::EditAlternatingType)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditAlternatingDataAos) => {
                        self.cycle_alternating_parent_data_aos(false);
                        if let Some(v) =
                            self.value_for_component_mode(InputMode::EditAlternatingDataAos)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditAccordionType) => {
                        self.cycle_accordion_parent_type(false);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditAccordionType)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditAccordionClass) => {
                        self.cycle_accordion_parent_class(false);
                        if let Some(v) =
                            self.value_for_component_mode(InputMode::EditAccordionClass)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditAccordionAos) => {
                        self.cycle_accordion_parent_data_aos(false);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditAccordionAos)
                        {
                            self.input_buffer = v;
                        }
                    }
                    _ => self.move_cursor_left(),
                },
                KeyCode::Right => match self.input_mode {
                    Some(InputMode::EditHeroClass) => {
                        self.cycle_hero_parent_class(true);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditHeroClass) {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditHeroAos) => {
                        self.cycle_hero_parent_data_aos(true);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditHeroAos) {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditHeroCtaTarget) => {
                        self.cycle_hero_link_1_target(false, true);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditHeroCtaTarget)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditHeroCtaTarget2) => {
                        self.cycle_hero_link_1_target(true, true);
                        if let Some(v) =
                            self.value_for_component_mode(InputMode::EditHeroCtaTarget2)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditSectionClass) => {
                        self.cycle_section_class(true);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditSectionClass)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditBannerClass) => {
                        self.cycle_banner_parent_class(true);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditBannerClass) {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditBannerDataAos) => {
                        self.cycle_banner_parent_data_aos(true);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditBannerDataAos)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditCtaClass) => {
                        self.cycle_cta_parent_class(true);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditCtaClass) {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditCtaDataAos) => {
                        self.cycle_cta_parent_data_aos(true);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditCtaDataAos) {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditCtaLinkTarget) => {
                        self.cycle_parent_link_target(true);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditCtaLinkTarget)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditBlockquoteDataAos) => {
                        self.cycle_blockquote_parent_data_aos(true);
                        if let Some(v) =
                            self.value_for_component_mode(InputMode::EditBlockquoteDataAos)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditCardType) => {
                        self.cycle_card_parent_type(true);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditCardType) {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditCardDataAos) => {
                        self.cycle_card_parent_data_aos(true);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditCardDataAos) {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditCardItemLinkTarget) => {
                        self.cycle_child_link_target(true);
                        if let Some(v) =
                            self.value_for_component_mode(InputMode::EditCardItemLinkTarget)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditFilmstripType) => {
                        self.cycle_filmstrip_parent_type(true);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditFilmstripType)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditFilmstripDataAos) => {
                        self.cycle_filmstrip_parent_data_aos(true);
                        if let Some(v) =
                            self.value_for_component_mode(InputMode::EditFilmstripDataAos)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditMilestonesDataAos) => {
                        self.cycle_milestones_data_aos(true);
                        if let Some(v) =
                            self.value_for_component_mode(InputMode::EditMilestonesDataAos)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditMilestonesItemLinkTarget) => {
                        self.cycle_milestones_link_target(true);
                        if let Some(v) =
                            self.value_for_component_mode(InputMode::EditMilestonesItemLinkTarget)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditSliderItemLinkTarget) => {
                        self.cycle_slider_link_target(true);
                        if let Some(v) =
                            self.value_for_component_mode(InputMode::EditSliderItemLinkTarget)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditAlternatingType) => {
                        self.cycle_alternating_parent_type(true);
                        if let Some(v) =
                            self.value_for_component_mode(InputMode::EditAlternatingType)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditAlternatingDataAos) => {
                        self.cycle_alternating_parent_data_aos(true);
                        if let Some(v) =
                            self.value_for_component_mode(InputMode::EditAlternatingDataAos)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditAccordionType) => {
                        self.cycle_accordion_parent_type(true);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditAccordionType)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditAccordionClass) => {
                        self.cycle_accordion_parent_class(true);
                        if let Some(v) =
                            self.value_for_component_mode(InputMode::EditAccordionClass)
                        {
                            self.input_buffer = v;
                        }
                    }
                    Some(InputMode::EditAccordionAos) => {
                        self.cycle_accordion_parent_data_aos(true);
                        if let Some(v) = self.value_for_component_mode(InputMode::EditAccordionAos)
                        {
                            self.input_buffer = v;
                        }
                    }
                    _ => self.move_cursor_right(),
                },
                KeyCode::Backspace => {
                    self.delete_char_before_cursor();
                }
                KeyCode::Char(c) => {
                    self.insert_char_at_cursor(c);
                }
                _ => {}
            },
            Event::Mouse(m) => match m.kind {
                MouseEventKind::Down(MouseButton::Left) if self.is_multiline_input_mode() => {
                    self.set_multiline_cursor_from_point(m.column, m.row);
                }
                MouseEventKind::ScrollUp if self.is_multiline_input_mode() => {
                    self.move_cursor_up_line();
                }
                MouseEventKind::ScrollDown if self.is_multiline_input_mode() => {
                    self.move_cursor_down_line();
                }
                _ => {}
            },
            _ => {}
        }
        Ok(())
    }

    fn is_multiline_mode(mode: InputMode) -> bool {
        matches!(
            mode,
            InputMode::EditHeroCopy
                | InputMode::EditAlternatingItemCopy
                | InputMode::EditCtaCopy
                | InputMode::EditBlockquoteCopy
                | InputMode::EditCardItemCopy
                | InputMode::EditMilestonesItemCopy
                | InputMode::EditModalCopy
                | InputMode::EditSliderItemCopy
                | InputMode::EditAccordionFirstContent
        )
    }

    fn is_multiline_input_mode(&self) -> bool {
        self.input_mode.is_some_and(Self::is_multiline_mode)
    }

    fn multiline_rows_for_mode(mode: InputMode) -> usize {
        match mode {
            InputMode::EditHeroCopy => 3,
            InputMode::EditAlternatingItemCopy
            | InputMode::EditCtaCopy
            | InputMode::EditBlockquoteCopy
            | InputMode::EditCardItemCopy
            | InputMode::EditMilestonesItemCopy
            | InputMode::EditModalCopy
            | InputMode::EditSliderItemCopy
            | InputMode::EditAccordionFirstContent => 5,
            _ => 1,
        }
    }

    fn multiline_rows(&self) -> usize {
        self.input_mode
            .map(Self::multiline_rows_for_mode)
            .unwrap_or(1)
    }

    fn multiline_max_rows_for_mode(mode: InputMode) -> Option<usize> {
        match mode {
            InputMode::EditHeroCopy => Some(3),
            InputMode::EditAlternatingItemCopy
            | InputMode::EditCtaCopy
            | InputMode::EditBlockquoteCopy
            | InputMode::EditCardItemCopy
            | InputMode::EditMilestonesItemCopy
            | InputMode::EditModalCopy
            | InputMode::EditSliderItemCopy
            | InputMode::EditAccordionFirstContent => None,
            _ => None,
        }
    }

    fn multiline_max_rows(&self) -> Option<usize> {
        self.input_mode.and_then(Self::multiline_max_rows_for_mode)
    }

    fn ensure_multiline_cursor_visible(&mut self) {
        if !self.is_multiline_input_mode() {
            self.multiline_scroll_row = 0;
            return;
        }
        let visible_rows = self.multiline_rows();
        let (row_idx, _) = cursor_row_col(&self.input_buffer, self.input_cursor);
        if row_idx < self.multiline_scroll_row {
            self.multiline_scroll_row = row_idx;
        } else if row_idx >= self.multiline_scroll_row + visible_rows {
            self.multiline_scroll_row = row_idx.saturating_sub(visible_rows.saturating_sub(1));
        }
        let total_rows = input_lines_preserve(&self.input_buffer).len().max(1);
        let max_scroll = total_rows.saturating_sub(visible_rows);
        self.multiline_scroll_row = self.multiline_scroll_row.min(max_scroll);
    }

    fn clamp_multiline_input_if_needed(&mut self) {
        let Some(max_rows) = self.multiline_max_rows() else {
            return;
        };
        let mut lines = self
            .input_buffer
            .split('\n')
            .map(|s| s.to_string())
            .collect::<Vec<_>>();
        if lines.len() > max_rows {
            lines.truncate(max_rows);
            self.input_buffer = lines.join("\n");
            self.input_cursor = self.input_cursor.min(self.input_buffer.chars().count());
        }
    }

    fn move_cursor_left(&mut self) {
        if self.input_cursor > 0 {
            self.input_cursor -= 1;
            self.ensure_multiline_cursor_visible();
        }
    }

    fn move_cursor_right(&mut self) {
        let len = self.input_buffer.chars().count();
        if self.input_cursor < len {
            self.input_cursor += 1;
            self.ensure_multiline_cursor_visible();
        }
    }

    fn move_cursor_up_line(&mut self) {
        let (row, col) = cursor_row_col(&self.input_buffer, self.input_cursor);
        if row == 0 {
            return;
        }
        let lines = input_lines_preserve(&self.input_buffer);
        self.input_cursor = cursor_from_row_col(&lines, row - 1, col);
        self.ensure_multiline_cursor_visible();
    }

    fn move_cursor_down_line(&mut self) {
        let (row, col) = cursor_row_col(&self.input_buffer, self.input_cursor);
        let lines = input_lines_preserve(&self.input_buffer);
        if row + 1 >= lines.len() {
            return;
        }
        self.input_cursor = cursor_from_row_col(&lines, row + 1, col);
        self.ensure_multiline_cursor_visible();
    }

    fn delete_char_before_cursor(&mut self) {
        if self.input_cursor == 0 {
            return;
        }
        let remove_at = self.input_cursor - 1;
        let byte_start = byte_index_for_char(&self.input_buffer, remove_at);
        let byte_end = byte_index_for_char(&self.input_buffer, self.input_cursor);
        self.input_buffer.replace_range(byte_start..byte_end, "");
        self.input_cursor = remove_at;
        self.ensure_multiline_cursor_visible();
    }

    fn insert_char_at_cursor(&mut self, c: char) {
        if self.is_multiline_input_mode() && c == '\n' {
            let mut candidate = self.input_buffer.clone();
            let at = byte_index_for_char(&candidate, self.input_cursor);
            candidate.insert(at, '\n');
            if let Some(max_rows) = self.multiline_max_rows() {
                if input_lines_preserve(&candidate).len().max(1) > max_rows {
                    self.status = match self.input_mode {
                        Some(InputMode::EditHeroCopy) => {
                            "hero.copy supports up to 3 lines.".to_string()
                        }
                        _ => "Line limit reached.".to_string(),
                    };
                    return;
                }
            }
            self.input_buffer = candidate;
            self.input_cursor += 1;
            self.ensure_multiline_cursor_visible();
            return;
        }
        let at = byte_index_for_char(&self.input_buffer, self.input_cursor);
        self.input_buffer.insert(at, c);
        self.input_cursor += 1;
        if self.is_multiline_input_mode() {
            self.ensure_multiline_cursor_visible();
        }
    }

    fn set_multiline_cursor_from_point(&mut self, x: u16, y: u16) {
        let Some(area) = self.multiline_value_area else {
            return;
        };
        if !contains(area, x, y) {
            return;
        }
        let row = y.saturating_sub(area.y) as usize + self.multiline_scroll_row;
        let col = x.saturating_sub(area.x) as usize;
        let lines = input_lines_preserve(&self.input_buffer);
        let mut cursor = 0usize;
        for i in 0..row {
            if let Some(line) = lines.get(i) {
                cursor += line.chars().count() + 1;
            } else {
                return;
            }
        }
        if let Some(line) = lines.get(row) {
            cursor += col.min(line.chars().count());
        } else if row == lines.len() {
            cursor += 0;
        } else {
            return;
        }
        self.input_cursor = cursor.min(self.input_buffer.chars().count());
        self.ensure_multiline_cursor_visible();
    }

    fn tab_next_component_field(&mut self) {
        self.tab_component_field(true);
    }

    fn tab_prev_component_field(&mut self) {
        self.tab_component_field(false);
    }

    fn tab_component_field(&mut self, forward: bool) {
        let Some(current) = self.input_mode else {
            return;
        };
        let Some(group) = self.component_edit_group_for_active_mode(current) else {
            self.status = "Tab field navigation is available while editing hero/component fields."
                .to_string();
            return;
        };
        let Some(idx) = group.iter().position(|m| *m == current) else {
            return;
        };
        let ok = self.commit_input_edit();
        if !ok {
            return;
        }
        let next_idx = if forward {
            (idx + 1) % group.len()
        } else if idx == 0 {
            group.len() - 1
        } else {
            idx - 1
        };
        let next_mode = group[next_idx];
        if !self.set_component_input_mode(next_mode) {
            self.status = "Could not switch to next component field.".to_string();
        }
    }

    fn component_edit_group_for_active_mode(&self, mode: InputMode) -> Option<Vec<InputMode>> {
        let accordion_mode = matches!(
            mode,
            InputMode::EditAccordionType
                | InputMode::EditAccordionClass
                | InputMode::EditAccordionAos
                | InputMode::EditAccordionGroupName
                | InputMode::EditAccordionFirstTitle
                | InputMode::EditAccordionFirstContent
        );
        if accordion_mode {
            let rows = self.build_page_tree_rows();
            let row_kind = rows
                .get(self.selected_tree_row.min(rows.len().saturating_sub(1)))
                .map(|r| r.kind);
            if matches!(row_kind, Some(TreeRowKind::AccordionItem { .. })) {
                return Some(vec![
                    InputMode::EditAccordionFirstTitle,
                    InputMode::EditAccordionFirstContent,
                ]);
            }
            return Some(vec![
                InputMode::EditAccordionType,
                InputMode::EditAccordionClass,
                InputMode::EditAccordionAos,
                InputMode::EditAccordionGroupName,
            ]);
        }
        let alternating_mode = matches!(
            mode,
            InputMode::EditAlternatingType
                | InputMode::EditAlternatingClass
                | InputMode::EditAlternatingDataAos
                | InputMode::EditAlternatingItemImage
                | InputMode::EditAlternatingItemImageAlt
                | InputMode::EditAlternatingItemTitle
                | InputMode::EditAlternatingItemCopy
        );
        if alternating_mode {
            let rows = self.build_page_tree_rows();
            let row_kind = rows
                .get(self.selected_tree_row.min(rows.len().saturating_sub(1)))
                .map(|r| r.kind);
            if matches!(row_kind, Some(TreeRowKind::AlternatingItem { .. })) {
                return Some(vec![
                    InputMode::EditAlternatingItemImage,
                    InputMode::EditAlternatingItemImageAlt,
                    InputMode::EditAlternatingItemTitle,
                    InputMode::EditAlternatingItemCopy,
                ]);
            }
            return Some(vec![
                InputMode::EditAlternatingType,
                InputMode::EditAlternatingClass,
                InputMode::EditAlternatingDataAos,
            ]);
        }
        let card_mode = matches!(
            mode,
            InputMode::EditCardType
                | InputMode::EditCardDataAos
                | InputMode::EditCardWidth
                | InputMode::EditCardItemImageUrl
                | InputMode::EditCardItemImageAlt
                | InputMode::EditCardItemTitle
                | InputMode::EditCardItemSubtitle
                | InputMode::EditCardItemCopy
                | InputMode::EditCardItemLinkUrl
                | InputMode::EditCardItemLinkTarget
                | InputMode::EditCardItemLinkLabel
        );
        if card_mode {
            let rows = self.build_page_tree_rows();
            let row_kind = rows
                .get(self.selected_tree_row.min(rows.len().saturating_sub(1)))
                .map(|r| r.kind);
            if matches!(row_kind, Some(TreeRowKind::CardItem { .. })) {
                return Some(vec![
                    InputMode::EditCardItemImageUrl,
                    InputMode::EditCardItemImageAlt,
                    InputMode::EditCardItemTitle,
                    InputMode::EditCardItemSubtitle,
                    InputMode::EditCardItemCopy,
                    InputMode::EditCardItemLinkUrl,
                    InputMode::EditCardItemLinkTarget,
                    InputMode::EditCardItemLinkLabel,
                ]);
            }
            return Some(vec![
                InputMode::EditCardType,
                InputMode::EditCardDataAos,
                InputMode::EditCardWidth,
            ]);
        }
        let filmstrip_mode = matches!(
            mode,
            InputMode::EditFilmstripType
                | InputMode::EditFilmstripDataAos
                | InputMode::EditFilmstripItemImageUrl
                | InputMode::EditFilmstripItemImageAlt
                | InputMode::EditFilmstripItemTitle
        );
        if filmstrip_mode {
            let rows = self.build_page_tree_rows();
            let row_kind = rows
                .get(self.selected_tree_row.min(rows.len().saturating_sub(1)))
                .map(|r| r.kind);
            if matches!(row_kind, Some(TreeRowKind::FilmstripItem { .. })) {
                return Some(vec![
                    InputMode::EditFilmstripItemImageUrl,
                    InputMode::EditFilmstripItemImageAlt,
                    InputMode::EditFilmstripItemTitle,
                ]);
            }
            return Some(vec![
                InputMode::EditFilmstripType,
                InputMode::EditFilmstripDataAos,
            ]);
        }
        let milestones_mode = matches!(
            mode,
            InputMode::EditMilestonesDataAos
                | InputMode::EditMilestonesWidth
                | InputMode::EditMilestonesItemPercentage
                | InputMode::EditMilestonesItemTitle
                | InputMode::EditMilestonesItemSubtitle
                | InputMode::EditMilestonesItemCopy
                | InputMode::EditMilestonesItemLinkUrl
                | InputMode::EditMilestonesItemLinkTarget
                | InputMode::EditMilestonesItemLinkLabel
        );
        if milestones_mode {
            let rows = self.build_page_tree_rows();
            let row_kind = rows
                .get(self.selected_tree_row.min(rows.len().saturating_sub(1)))
                .map(|r| r.kind);
            if matches!(row_kind, Some(TreeRowKind::MilestonesItem { .. })) {
                return Some(vec![
                    InputMode::EditMilestonesItemPercentage,
                    InputMode::EditMilestonesItemTitle,
                    InputMode::EditMilestonesItemSubtitle,
                    InputMode::EditMilestonesItemCopy,
                    InputMode::EditMilestonesItemLinkUrl,
                    InputMode::EditMilestonesItemLinkTarget,
                    InputMode::EditMilestonesItemLinkLabel,
                ]);
            }
            return Some(vec![
                InputMode::EditMilestonesDataAos,
                InputMode::EditMilestonesWidth,
            ]);
        }
        let modal_mode = matches!(mode, InputMode::EditModalTitle | InputMode::EditModalCopy);
        if modal_mode {
            return Some(vec![InputMode::EditModalTitle, InputMode::EditModalCopy]);
        }
        let slider_mode = matches!(
            mode,
            InputMode::EditSliderTitle
                | InputMode::EditSliderItemTitle
                | InputMode::EditSliderItemCopy
                | InputMode::EditSliderItemLinkUrl
                | InputMode::EditSliderItemLinkTarget
                | InputMode::EditSliderItemLinkLabel
                | InputMode::EditSliderItemImageUrl
                | InputMode::EditSliderItemImageAlt
        );
        if slider_mode {
            let rows = self.build_page_tree_rows();
            let row_kind = rows
                .get(self.selected_tree_row.min(rows.len().saturating_sub(1)))
                .map(|r| r.kind);
            if matches!(row_kind, Some(TreeRowKind::SliderItem { .. })) {
                return Some(vec![
                    InputMode::EditSliderItemTitle,
                    InputMode::EditSliderItemCopy,
                    InputMode::EditSliderItemLinkUrl,
                    InputMode::EditSliderItemLinkTarget,
                    InputMode::EditSliderItemLinkLabel,
                    InputMode::EditSliderItemImageUrl,
                    InputMode::EditSliderItemImageAlt,
                ]);
            }
            return Some(vec![InputMode::EditSliderTitle]);
        }
        component_edit_group_for_mode(mode).map(|modes| modes.to_vec())
    }

    fn handle_component_picker_event(&mut self, evt: Event) -> anyhow::Result<()> {
        if let Event::Key(key) = evt {
            match key.code {
                KeyCode::Esc => {
                    self.component_picker = None;
                    self.status = "Component picker cancelled.".to_string();
                }
                KeyCode::Up => {
                    let selected = self
                        .component_picker
                        .as_ref()
                        .map(|p| p.selected)
                        .unwrap_or(0)
                        .saturating_sub(1);
                    if let Some(picker) = &mut self.component_picker {
                        picker.selected = selected;
                    }
                }
                KeyCode::Down => {
                    let (query, selected) = if let Some(picker) = &self.component_picker {
                        (picker.query.clone(), picker.selected)
                    } else {
                        (String::new(), 0)
                    };
                    let total = self.filtered_component_kinds(&query).len();
                    if let Some(picker) = &mut self.component_picker {
                        if total == 0 {
                            picker.selected = 0;
                        } else {
                            picker.selected = (selected + 1).min(total - 1);
                        }
                    }
                }
                KeyCode::Backspace => {
                    if let Some(picker) = &mut self.component_picker {
                        picker.query.pop();
                    }
                    self.normalize_component_picker_selection();
                }
                KeyCode::Enter => {
                    let (query, selected) = if let Some(picker) = &self.component_picker {
                        (picker.query.clone(), picker.selected)
                    } else {
                        (String::new(), 0)
                    };
                    let filtered = self.filtered_component_kinds(&query);
                    let Some(kind) = filtered
                        .get(selected.min(filtered.len().saturating_sub(1)))
                        .copied()
                    else {
                        self.status = "No component selected.".to_string();
                        return Ok(());
                    };
                    self.component_kind = kind;
                    self.insert_selected_component_kind();
                    self.component_picker = None;
                }
                KeyCode::Char(c) => {
                    if let Some(picker) = &mut self.component_picker {
                        picker.query.push(c);
                    }
                    self.normalize_component_picker_selection();
                }
                _ => {}
            }
        }
        self.sync_tree_row_with_selection();
        Ok(())
    }

    fn handle_edit_modal_event(&mut self, evt: Event) -> anyhow::Result<()> {
        if let Event::Mouse(m) = &evt {
            // Click-to-focus inside the legacy EditModalState fields.
            if matches!(m.kind, MouseEventKind::Down(MouseButton::Left)) {
                let (col, row) = (m.column, m.row);
                let hit = self
                    .modal_field_areas
                    .borrow()
                    .iter()
                    .find(|(_, r)| {
                        col >= r.x
                            && col < r.x + r.width
                            && row >= r.y
                            && row < r.y + r.height
                    })
                    .map(|(idx, _)| *idx);
                if let Some(idx) = hit {
                    if let Some(modal) = self.edit_modal.as_mut() {
                        modal.selected_field = idx;
                    }
                    return Ok(());
                }
            }
            if let Some(modal) = self.edit_modal.as_mut() {
                let total = modal.fields.len();
                let visible = modal.visible_fields.max(1);
                let cap = total.saturating_sub(visible);
                match m.kind {
                    MouseEventKind::ScrollUp => {
                        modal.scroll_offset = modal.scroll_offset.saturating_sub(3);
                        if modal.selected_field
                            >= modal.scroll_offset + visible
                        {
                            modal.selected_field =
                                (modal.scroll_offset + visible).saturating_sub(1);
                        }
                    }
                    MouseEventKind::ScrollDown => {
                        modal.scroll_offset =
                            (modal.scroll_offset + 3).min(cap);
                        if modal.selected_field < modal.scroll_offset {
                            modal.selected_field = modal.scroll_offset;
                        }
                    }
                    _ => {}
                }
            }
            return Ok(());
        }
        if let Event::Key(key) = evt {
            match key.code {
                KeyCode::Esc => {
                    self.edit_modal = None;
                    self.status = "Edit cancelled.".to_string();
                }
                KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.save_edit_modal_changes();
                }
                KeyCode::Up => {
                    if let Some(modal) = &mut self.edit_modal {
                        modal.selected_field = modal.selected_field.saturating_sub(1);
                        modal.scroll_offset = modal.scroll_offset.min(modal.selected_field);
                    }
                }
                KeyCode::Down => {
                    if let Some(modal) = &mut self.edit_modal {
                        let total = modal.fields.len();
                        if total > 0 {
                            modal.selected_field = (modal.selected_field + 1).min(total - 1);
                            // Adjust scroll if needed
                            let visible = modal.visible_fields.max(1);
                            if modal.selected_field >= modal.scroll_offset + visible {
                                modal.scroll_offset =
                                    modal.selected_field.saturating_sub(visible - 1);
                            }
                        }
                    }
                }
                KeyCode::Tab => {
                    if let Some(modal) = &mut self.edit_modal {
                        let total = modal.fields.len();
                        if total > 0 {
                            modal.selected_field = (modal.selected_field + 1) % total;
                            let visible = modal.visible_fields.max(1);
                            if modal.selected_field < modal.scroll_offset {
                                modal.scroll_offset = modal.selected_field;
                            } else if modal.selected_field >= modal.scroll_offset + visible {
                                modal.scroll_offset =
                                    modal.selected_field.saturating_sub(visible - 1);
                            }
                        }
                    }
                }
                KeyCode::BackTab => {
                    if let Some(modal) = &mut self.edit_modal {
                        let total = modal.fields.len();
                        if total > 0 {
                            modal.selected_field = modal.selected_field.saturating_sub(1);
                            if modal.selected_field >= total {
                                modal.selected_field = total - 1;
                            }
                            if modal.selected_field < modal.scroll_offset {
                                modal.scroll_offset = modal.selected_field;
                            }
                        }
                    }
                }
                KeyCode::Enter => {
                    self.commit_edit_modal_field();
                }
                KeyCode::Backspace => {
                    if let Some(modal) = &mut self.edit_modal {
                        let idx = modal.selected_field;
                        if let Some(field) = modal.fields.get_mut(idx) {
                            if field.cursor > 0 {
                                field.cursor -= 1;
                                if field.cursor < field.buffer.chars().count() {
                                    let mut chars: Vec<char> = field.buffer.chars().collect();
                                    chars.remove(field.cursor);
                                    field.buffer = chars.into_iter().collect();
                                }
                            }
                        }
                    }
                }
                KeyCode::Left => {
                    if let Some(modal) = &mut self.edit_modal {
                        let idx = modal.selected_field;
                        if let Some(field) = modal.fields.get_mut(idx) {
                            field.cursor = field.cursor.saturating_sub(1);
                        }
                    }
                }
                KeyCode::Right => {
                    if let Some(modal) = &mut self.edit_modal {
                        let idx = modal.selected_field;
                        if let Some(field) = modal.fields.get_mut(idx) {
                            let max = field.buffer.chars().count();
                            field.cursor = (field.cursor + 1).min(max);
                        }
                    }
                }
                KeyCode::Char(c) => {
                    if let Some(modal) = &mut self.edit_modal {
                        let idx = modal.selected_field;
                        if let Some(field) = modal.fields.get_mut(idx) {
                            let mut chars: Vec<char> = field.buffer.chars().collect();
                            if field.cursor <= chars.len() {
                                chars.insert(field.cursor, c);
                                field.buffer = chars.into_iter().collect();
                                field.cursor += 1;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn commit_edit_modal_field(&mut self) {
        // This will be overridden by specific implementations per field type
        // For now, just save the buffer back to value
        if let Some(modal) = &mut self.edit_modal {
            let idx = modal.selected_field;
            if let Some(field) = modal.fields.get_mut(idx) {
                field.value = field.buffer.clone();
            }
        }
    }

    fn save_edit_modal_changes(&mut self) {
        let Some(mut modal) = self.edit_modal.take() else {
            return;
        };
        // Commit any in-flight typing in every field so Ctrl+S saves the
        // current buffer state without requiring a prior Enter press.
        for field in &mut modal.fields {
            field.value = field.buffer.clone();
        }

        // Determine what we're editing based on the modal title
        let saved = if modal.title == "dd-hero" {
            let idx = self.selected_node;
            let Some(page) = self.current_page_mut() else {
                self.status = "Failed to save: no page.".to_string();
                return;
            };
            let idx = idx.min(page.nodes.len().saturating_sub(1));
            let hero = match &mut page.nodes[idx] {
                PageNode::Hero(h) => h,
                _ => {
                    self.status = "Failed to save: selected node is not a hero.".to_string();
                    return;
                }
            };

            // Apply changes from fields to hero
            for field in &modal.fields {
                match field.label.as_str() {
                    "Image URL" => hero.parent_image_url = field.value.clone(),
                    "Title" => hero.parent_title = field.value.clone(),
                    "Subtitle" => hero.parent_subtitle = field.value.clone(),
                    "Copy" => {
                        hero.parent_copy = if field.value.is_empty() {
                            None
                        } else {
                            Some(field.value.clone())
                        }
                    }
                    "CTA Text" => {
                        hero.link_1_label = if field.value.is_empty() {
                            None
                        } else {
                            Some(field.value.clone())
                        }
                    }
                    "CTA Link" => {
                        hero.link_1_url = if field.value.is_empty() {
                            None
                        } else {
                            Some(field.value.clone())
                        }
                    }
                    "Custom CSS" => {
                        hero.parent_custom_css = if field.value.is_empty() {
                            None
                        } else {
                            Some(field.value.clone())
                        }
                    }
                    _ => {}
                }
            }
            true
        } else if modal.title == "dd-section" {
            let idx = self.selected_node;
            let Some(page) = self.current_page_mut() else {
                self.status = "Failed to save: no page.".to_string();
                return;
            };
            let idx = idx.min(page.nodes.len().saturating_sub(1));
            let section = match &mut page.nodes[idx] {
                PageNode::Section(s) => s,
                _ => {
                    self.status = "Failed to save: selected node is not a section.".to_string();
                    return;
                }
            };

            // Apply changes from fields to section
            for field in &modal.fields {
                match field.label.as_str() {
                    "Section ID" => section.id = field.value.clone(),
                    "Section Title" => {
                        section.section_title = if field.value.is_empty() {
                            None
                        } else {
                            Some(field.value.clone())
                        }
                    }
                    _ => {}
                }
            }
            true
        } else if modal.title == "dd-banner" {
            let selected_node = self.selected_node;
            let selected_column = self.selected_column;
            let selected_component = self.selected_component;
            let Some(page) = self.current_page_mut() else {
                self.status = "Failed to save: no page.".to_string();
                return;
            };
            let ni = selected_node.min(page.nodes.len().saturating_sub(1));
            let col_i = selected_column;
            let ci = selected_component;

            if let PageNode::Section(section) = &mut page.nodes[ni] {
                normalize_section_columns(section);
                let col_i = col_i.min(section.columns.len().saturating_sub(1));
                let ci = ci.min(section.columns[col_i].components.len().saturating_sub(1));

                if let crate::model::SectionComponent::Banner(banner) =
                    &mut section.columns[col_i].components[ci]
                {
                    for field in &modal.fields {
                        match field.label.as_str() {
                            "Banner Class" => {
                                if let Some(v) = parse_banner_class(&field.value) {
                                    banner.parent_class = v;
                                }
                            }
                            "Data AOS" => {
                                if let Some(v) = parse_parent_data_aos(&field.value) {
                                    banner.parent_data_aos = v;
                                }
                            }
                            "Image URL" => banner.parent_image_url = field.value.clone(),
                            "Image Alt" => banner.parent_image_alt = field.value.clone(),
                            _ => {}
                        }
                    }
                    true
                } else {
                    false
                }
            } else {
                self.status = "Failed to save: selected node is not a section.".to_string();
                false
            }
        } else if modal.title == "dd-alert" {
            let selected_node = self.selected_node;
            let selected_column = self.selected_column;
            let selected_component = self.selected_component;
            let Some(page) = self.current_page_mut() else {
                self.status = "Failed to save: no page.".to_string();
                return;
            };
            let ni = selected_node.min(page.nodes.len().saturating_sub(1));
            let col_i = selected_column;
            let ci = selected_component;

            if let PageNode::Section(section) = &mut page.nodes[ni] {
                normalize_section_columns(section);
                let col_i = col_i.min(section.columns.len().saturating_sub(1));
                let ci = ci.min(section.columns[col_i].components.len().saturating_sub(1));

                if let crate::model::SectionComponent::Alert(alert) =
                    &mut section.columns[col_i].components[ci]
                {
                    for field in &modal.fields {
                        match field.label.as_str() {
                            "Alert Type" => {
                                if let Some(v) = parse_alert_type(&field.value) {
                                    alert.parent_type = v;
                                }
                            }
                            "Alert Class" => {
                                if let Some(v) = parse_alert_class(&field.value) {
                                    alert.parent_class = v;
                                }
                            }
                            "Data AOS" => {
                                if let Some(v) = parse_parent_data_aos(&field.value) {
                                    alert.parent_data_aos = v;
                                }
                            }
                            "Title" => {
                                alert.parent_title = if field.value.is_empty() {
                                    None
                                } else {
                                    Some(field.value.clone())
                                }
                            }
                            "Copy" => alert.parent_copy = field.value.clone(),
                            _ => {}
                        }
                    }
                    true
                } else {
                    false
                }
            } else {
                self.status = "Failed to save: selected node is not a section.".to_string();
                false
            }
        } else if modal.title == "dd-cta" {
            self.save_cta_changes(&modal.fields)
        } else if modal.title == "dd-blockquote" {
            self.save_blockquote_changes(&modal.fields)
        } else if modal.title == "dd-modal" {
            self.save_modal_changes(&modal.fields)
        } else if modal.title == "dd-filmstrip" {
            self.save_filmstrip_changes(&modal.fields)
        } else if modal.title == "dd-accordion" {
            self.save_accordion_changes(&modal.fields)
        } else if modal.title == "dd-image" {
            self.save_image_changes(&modal.fields)
        } else if modal.title == "dd-rich_text" {
            self.save_rich_text_changes(&modal.fields)
        } else if modal.title == "dd-navigation" {
            self.save_navigation_changes(&modal.fields)
        } else if modal.title == "dd-header-search" {
            self.save_header_search_changes(&modal.fields)
        } else if modal.title == "dd-header-menu" {
            self.save_header_menu_changes(&modal.fields)
        } else if modal.title == "page-head" {
            self.save_page_head_changes(&modal.fields)
        } else if modal.title == "dd-footer" {
            self.save_footer_changes(&modal.fields)
        } else if modal.title == "dd-header-root" {
            self.save_header_root_changes(&modal.fields)
        } else {
            false
        };

        if saved {
            let msg = format!("Saved {} changes.", modal.title);
            self.push_toast(ToastLevel::Success, msg);
        }
    }

    fn save_cta_changes(&mut self, fields: &[EditField]) -> bool {
        let selected_node = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let Some(page) = self.current_page_mut() else {
            self.status = "Failed to save: no page.".to_string();
            return false;
        };
        let ni = selected_node.min(page.nodes.len().saturating_sub(1));

        if let PageNode::Section(section) = &mut page.nodes[ni] {
            normalize_section_columns(section);
            let col_i = selected_column.min(section.columns.len().saturating_sub(1));
            let ci =
                selected_component.min(section.columns[col_i].components.len().saturating_sub(1));

            if let crate::model::SectionComponent::Cta(cta) =
                &mut section.columns[col_i].components[ci]
            {
                for field in fields {
                    match field.label.as_str() {
                        "CTA Class" => {
                            if let Some(v) = parse_cta_class(&field.value) {
                                cta.parent_class = v;
                            }
                        }
                        "Data AOS" => {
                            if let Some(v) = parse_parent_data_aos(&field.value) {
                                cta.parent_data_aos = v;
                            }
                        }
                        "Image URL" => cta.parent_image_url = field.value.clone(),
                        "Image Alt" => cta.parent_image_alt = field.value.clone(),
                        "Title" => cta.parent_title = field.value.clone(),
                        "Subtitle" => cta.parent_subtitle = field.value.clone(),
                        "Copy" => cta.parent_copy = field.value.clone(),
                        "Link URL" => {
                            cta.parent_link_url = if field.value.is_empty() {
                                None
                            } else {
                                Some(field.value.clone())
                            }
                        }
                        "Link Target" => cta.parent_link_target = parse_child_link_target(&field.value),
                        "Link Label" => {
                            cta.parent_link_label = if field.value.is_empty() {
                                None
                            } else {
                                Some(field.value.clone())
                            }
                        }
                        _ => {}
                    }
                }
                true
            } else {
                false
            }
        } else {
            self.status = "Failed to save: selected node is not a section.".to_string();
            false
        }
    }

    fn save_blockquote_changes(&mut self, fields: &[EditField]) -> bool {
        let selected_node = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let Some(page) = self.current_page_mut() else {
            self.status = "Failed to save: no page.".to_string();
            return false;
        };
        let ni = selected_node.min(page.nodes.len().saturating_sub(1));

        if let PageNode::Section(section) = &mut page.nodes[ni] {
            normalize_section_columns(section);
            let col_i = selected_column.min(section.columns.len().saturating_sub(1));
            let ci =
                selected_component.min(section.columns[col_i].components.len().saturating_sub(1));

            if let crate::model::SectionComponent::Blockquote(blockquote) =
                &mut section.columns[col_i].components[ci]
            {
                for field in fields {
                    match field.label.as_str() {
                        "Data AOS" => {
                            if let Some(v) = parse_parent_data_aos(&field.value) {
                                blockquote.parent_data_aos = v;
                            }
                        }
                        "Image URL" => blockquote.parent_image_url = field.value.clone(),
                        "Image Alt" => blockquote.parent_image_alt = field.value.clone(),
                        "Person Name" => blockquote.parent_name = field.value.clone(),
                        "Person Title" => blockquote.parent_role = field.value.clone(),
                        "Copy" => blockquote.parent_copy = field.value.clone(),
                        _ => {}
                    }
                }
                true
            } else {
                false
            }
        } else {
            self.status = "Failed to save: selected node is not a section.".to_string();
            false
        }
    }

    fn save_modal_changes(&mut self, fields: &[EditField]) -> bool {
        let selected_node = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let Some(page) = self.current_page_mut() else {
            self.status = "Failed to save: no page.".to_string();
            return false;
        };
        let ni = selected_node.min(page.nodes.len().saturating_sub(1));

        if let PageNode::Section(section) = &mut page.nodes[ni] {
            normalize_section_columns(section);
            let col_i = selected_column.min(section.columns.len().saturating_sub(1));
            let ci =
                selected_component.min(section.columns[col_i].components.len().saturating_sub(1));

            if let crate::model::SectionComponent::Modal(modal) =
                &mut section.columns[col_i].components[ci]
            {
                for field in fields {
                    match field.label.as_str() {
                        "Title" => modal.parent_title = field.value.clone(),
                        "Copy" => modal.parent_copy = field.value.clone(),
                        _ => {}
                    }
                }
                true
            } else {
                false
            }
        } else {
            self.status = "Failed to save: selected node is not a section.".to_string();
            false
        }
    }

    fn save_filmstrip_changes(&mut self, fields: &[EditField]) -> bool {
        let selected_node = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let Some(page) = self.current_page_mut() else {
            self.status = "Failed to save: no page.".to_string();
            return false;
        };
        let ni = selected_node.min(page.nodes.len().saturating_sub(1));

        if let PageNode::Section(section) = &mut page.nodes[ni] {
            normalize_section_columns(section);
            let col_i = selected_column.min(section.columns.len().saturating_sub(1));
            let ci =
                selected_component.min(section.columns[col_i].components.len().saturating_sub(1));

            if let crate::model::SectionComponent::Filmstrip(filmstrip) =
                &mut section.columns[col_i].components[ci]
            {
                for field in fields {
                    match field.label.as_str() {
                        "Filmstrip Type" => {
                            if let Some(v) = parse_filmstrip_type(&field.value) {
                                filmstrip.parent_type = v;
                            }
                        }
                        "Data AOS" => {
                            if let Some(v) = parse_parent_data_aos(&field.value) {
                                filmstrip.parent_data_aos = v;
                            }
                        }
                        _ => {}
                    }
                }
                true
            } else {
                false
            }
        } else {
            self.status = "Failed to save: selected node is not a section.".to_string();
            false
        }
    }

    fn save_image_changes(&mut self, fields: &[EditField]) -> bool {
        let selected_node = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let selected_region = self.selected_region;
        let selected_header_section = self.selected_header_section;
        let selected_header_column = self.selected_header_column;
        let selected_header_component = self.selected_header_component;
        let apply_image = |image: &mut crate::model::DdImage, fields: &[EditField]| {
            for field in fields {
                match field.label.as_str() {
                    "Data AOS" => {
                        if let Some(v) = parse_parent_data_aos(&field.value) {
                            image.parent_data_aos = v;
                        }
                    }
                    "Image URL" => image.parent_image_url = field.value.clone(),
                    "Image Alt" => image.parent_image_alt = field.value.clone(),
                    "Link URL" => {
                        image.parent_link_url = if field.value.is_empty() {
                            None
                        } else {
                            Some(field.value.clone())
                        };
                    }
                    "Link Target" => {
                        image.parent_link_target = parse_child_link_target(&field.value);
                    }
                    _ => {}
                }
            }
        };
        if selected_region == SelectedRegion::Header {
            if let Some(section) = self
                .site
                .header
                .sections
                .get_mut(selected_header_section)
            {
                if let Some(col) = section.columns.get_mut(selected_header_column) {
                    if let Some(comp) = col.components.get_mut(selected_header_component) {
                        if let crate::model::SectionComponent::Image(image) = comp {
                            apply_image(image, fields);
                            return true;
                        }
                    }
                }
            }
            return false;
        }
        let Some(page) = self.current_page_mut() else {
            self.status = "Failed to save: no page.".to_string();
            return false;
        };
        let ni = selected_node.min(page.nodes.len().saturating_sub(1));
        if let PageNode::Section(section) = &mut page.nodes[ni] {
            normalize_section_columns(section);
            let col_i = selected_column.min(section.columns.len().saturating_sub(1));
            let ci =
                selected_component.min(section.columns[col_i].components.len().saturating_sub(1));
            if let crate::model::SectionComponent::Image(image) =
                &mut section.columns[col_i].components[ci]
            {
                apply_image(image, fields);
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    fn save_rich_text_changes(&mut self, fields: &[EditField]) -> bool {
        let selected_node = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let selected_region = self.selected_region;
        let selected_header_section = self.selected_header_section;
        let selected_header_column = self.selected_header_column;
        let selected_header_component = self.selected_header_component;
        let apply_rt = |rt: &mut crate::model::DdRichText, fields: &[EditField]| {
            for field in fields {
                match field.label.as_str() {
                    "Parent Class" => {
                        rt.parent_class = if field.value.is_empty() {
                            None
                        } else {
                            Some(field.value.clone())
                        };
                    }
                    "Data AOS" => {
                        if let Some(v) = parse_parent_data_aos(&field.value) {
                            rt.parent_data_aos = v;
                        }
                    }
                    "Copy" => rt.parent_copy = field.value.clone(),
                    _ => {}
                }
            }
        };
        if selected_region == SelectedRegion::Header {
            if let Some(section) = self
                .site
                .header
                .sections
                .get_mut(selected_header_section)
            {
                if let Some(col) = section.columns.get_mut(selected_header_column) {
                    if let Some(comp) = col.components.get_mut(selected_header_component) {
                        if let crate::model::SectionComponent::RichText(rt) = comp {
                            apply_rt(rt, fields);
                            return true;
                        }
                    }
                }
            }
            return false;
        }
        let Some(page) = self.current_page_mut() else {
            self.status = "Failed to save: no page.".to_string();
            return false;
        };
        let ni = selected_node.min(page.nodes.len().saturating_sub(1));
        if let PageNode::Section(section) = &mut page.nodes[ni] {
            normalize_section_columns(section);
            let col_i = selected_column.min(section.columns.len().saturating_sub(1));
            let ci =
                selected_component.min(section.columns[col_i].components.len().saturating_sub(1));
            if let crate::model::SectionComponent::RichText(rt) =
                &mut section.columns[col_i].components[ci]
            {
                apply_rt(rt, fields);
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    fn save_navigation_changes(&mut self, fields: &[EditField]) -> bool {
        let selected_node = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let selected_region = self.selected_region;
        let selected_header_section = self.selected_header_section;
        let selected_header_column = self.selected_header_column;
        let selected_header_component = self.selected_header_component;
        let apply_nav = |nav: &mut crate::model::DdNavigation, fields: &[EditField]| {
            for field in fields {
                match field.label.as_str() {
                    "Nav Type" => {
                        if let Some(v) = parse_navigation_type(&field.value) {
                            nav.parent_type = v;
                        }
                    }
                    "Nav Class" => {
                        if let Some(v) = parse_navigation_class(&field.value) {
                            nav.parent_class = v;
                        }
                    }
                    "Data AOS" => {
                        if let Some(v) = parse_parent_data_aos(&field.value) {
                            nav.parent_data_aos = v;
                        }
                    }
                    "Parent Width" => nav.parent_width = field.value.clone(),
                    _ => {}
                }
            }
        };
        if selected_region == SelectedRegion::Header {
            if let Some(section) = self
                .site
                .header
                .sections
                .get_mut(selected_header_section)
            {
                if let Some(col) = section.columns.get_mut(selected_header_column) {
                    if let Some(comp) = col.components.get_mut(selected_header_component) {
                        if let crate::model::SectionComponent::Navigation(nav) = comp {
                            apply_nav(nav, fields);
                            return true;
                        }
                    }
                }
            }
            return false;
        }
        let Some(page) = self.current_page_mut() else {
            self.status = "Failed to save: no page.".to_string();
            return false;
        };
        let ni = selected_node.min(page.nodes.len().saturating_sub(1));
        if let PageNode::Section(section) = &mut page.nodes[ni] {
            normalize_section_columns(section);
            let col_i = selected_column.min(section.columns.len().saturating_sub(1));
            let ci =
                selected_component.min(section.columns[col_i].components.len().saturating_sub(1));
            if let crate::model::SectionComponent::Navigation(nav) =
                &mut section.columns[col_i].components[ci]
            {
                apply_nav(nav, fields);
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    fn save_header_search_changes(&mut self, fields: &[EditField]) -> bool {
        let selected_node = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let selected_region = self.selected_region;
        let selected_header_section = self.selected_header_section;
        let selected_header_column = self.selected_header_column;
        let selected_header_component = self.selected_header_component;
        let apply_hs = |hs: &mut crate::model::DdHeaderSearch, fields: &[EditField]| {
            for field in fields {
                match field.label.as_str() {
                    "Parent Width" => hs.parent_width = field.value.clone(),
                    "Data AOS" => {
                        if let Some(v) = parse_parent_data_aos(&field.value) {
                            hs.parent_data_aos = v;
                        }
                    }
                    _ => {}
                }
            }
        };
        if selected_region == SelectedRegion::Header {
            if let Some(section) = self
                .site
                .header
                .sections
                .get_mut(selected_header_section)
            {
                if let Some(col) = section.columns.get_mut(selected_header_column) {
                    if let Some(comp) = col.components.get_mut(selected_header_component) {
                        if let crate::model::SectionComponent::HeaderSearch(hs) = comp {
                            apply_hs(hs, fields);
                            return true;
                        }
                    }
                }
            }
            return false;
        }
        let Some(page) = self.current_page_mut() else {
            return false;
        };
        let ni = selected_node.min(page.nodes.len().saturating_sub(1));
        if let PageNode::Section(section) = &mut page.nodes[ni] {
            normalize_section_columns(section);
            let col_i = selected_column.min(section.columns.len().saturating_sub(1));
            let ci =
                selected_component.min(section.columns[col_i].components.len().saturating_sub(1));
            if let crate::model::SectionComponent::HeaderSearch(hs) =
                &mut section.columns[col_i].components[ci]
            {
                apply_hs(hs, fields);
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    fn save_header_menu_changes(&mut self, fields: &[EditField]) -> bool {
        let selected_node = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let selected_region = self.selected_region;
        let selected_header_section = self.selected_header_section;
        let selected_header_column = self.selected_header_column;
        let selected_header_component = self.selected_header_component;
        let apply_hm = |hm: &mut crate::model::DdHeaderMenu, fields: &[EditField]| {
            for field in fields {
                match field.label.as_str() {
                    "Parent Width" => hm.parent_width = field.value.clone(),
                    "Data AOS" => {
                        if let Some(v) = parse_parent_data_aos(&field.value) {
                            hm.parent_data_aos = v;
                        }
                    }
                    _ => {}
                }
            }
        };
        if selected_region == SelectedRegion::Header {
            if let Some(section) = self
                .site
                .header
                .sections
                .get_mut(selected_header_section)
            {
                if let Some(col) = section.columns.get_mut(selected_header_column) {
                    if let Some(comp) = col.components.get_mut(selected_header_component) {
                        if let crate::model::SectionComponent::HeaderMenu(hm) = comp {
                            apply_hm(hm, fields);
                            return true;
                        }
                    }
                }
            }
            return false;
        }
        let Some(page) = self.current_page_mut() else {
            return false;
        };
        let ni = selected_node.min(page.nodes.len().saturating_sub(1));
        if let PageNode::Section(section) = &mut page.nodes[ni] {
            normalize_section_columns(section);
            let col_i = selected_column.min(section.columns.len().saturating_sub(1));
            let ci =
                selected_component.min(section.columns[col_i].components.len().saturating_sub(1));
            if let crate::model::SectionComponent::HeaderMenu(hm) =
                &mut section.columns[col_i].components[ci]
            {
                apply_hm(hm, fields);
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    fn save_page_head_changes(&mut self, fields: &[EditField]) -> bool {
        let Some(page) = self.current_page_mut() else {
            return false;
        };
        let orig_title = page.head.title.clone();
        let orig_slug = page.slug.clone();
        for field in fields {
            match field.label.as_str() {
                "Title" => page.head.title = field.value.clone(),
                "Slug" => {
                    let trimmed = field.value.trim();
                    if !trimmed.is_empty() && trimmed != page.slug {
                        page.slug = trimmed.to_string();
                        // An explicit slug edit locks the slug so future
                        // title renames stop auto-regenerating it.
                        page.slug_locked = true;
                    }
                }
                "Meta Description" => {
                    page.head.meta_description = if field.value.is_empty() {
                        None
                    } else {
                        Some(field.value.clone())
                    };
                }
                "Canonical URL" => {
                    page.head.canonical_url = if field.value.is_empty() {
                        None
                    } else {
                        Some(field.value.clone())
                    };
                }
                "Robots" => {
                    if let Some(v) = parse_robots_directive(&field.value) {
                        page.head.robots = v;
                    }
                }
                "Schema Type" => {
                    if let Some(v) = parse_schema_type(&field.value) {
                        page.head.schema_type = v;
                    }
                }
                "OG Title" => {
                    page.head.og_title = if field.value.is_empty() {
                        None
                    } else {
                        Some(field.value.clone())
                    };
                }
                "OG Description" => {
                    page.head.og_description = if field.value.is_empty() {
                        None
                    } else {
                        Some(field.value.clone())
                    };
                }
                "OG Image" => {
                    page.head.og_image = if field.value.is_empty() {
                        None
                    } else {
                        Some(field.value.clone())
                    };
                }
                _ => {}
            }
        }
        // If the user changed the Title but did not manually override the
        // Slug (and it's not locked), regenerate the slug from the new title
        // so the two stay in sync — same behavior as the `r` rename flow.
        let title_changed = page.head.title != orig_title;
        let slug_user_edited = page.slug != orig_slug;
        if title_changed && !slug_user_edited && !page.slug_locked {
            let derived = crate::model::slug_from_title(&page.head.title);
            if !derived.is_empty() {
                page.slug = derived;
            }
        }
        true
    }

    fn save_footer_changes(&mut self, fields: &[EditField]) -> bool {
        for field in fields {
            match field.label.as_str() {
                "Footer ID" => self.site.footer.id = field.value.clone(),
                "Custom CSS" => {
                    self.site.footer.custom_css = if field.value.is_empty() {
                        None
                    } else {
                        Some(field.value.clone())
                    };
                }
                _ => {}
            }
        }
        true
    }

    fn save_header_root_changes(&mut self, fields: &[EditField]) -> bool {
        for field in fields {
            match field.label.as_str() {
                "Header ID" => self.site.header.id = field.value.clone(),
                "Custom CSS" => {
                    self.site.header.custom_css = if field.value.is_empty() {
                        None
                    } else {
                        Some(field.value.clone())
                    };
                }
                _ => {}
            }
        }
        true
    }

    fn save_accordion_changes(&mut self, fields: &[EditField]) -> bool {
        let selected_node = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let Some(page) = self.current_page_mut() else {
            self.status = "Failed to save: no page.".to_string();
            return false;
        };
        let ni = selected_node.min(page.nodes.len().saturating_sub(1));

        if let PageNode::Section(section) = &mut page.nodes[ni] {
            normalize_section_columns(section);
            let col_i = selected_column.min(section.columns.len().saturating_sub(1));
            let ci =
                selected_component.min(section.columns[col_i].components.len().saturating_sub(1));

            if let crate::model::SectionComponent::Accordion(accordion) =
                &mut section.columns[col_i].components[ci]
            {
                for field in fields {
                    match field.label.as_str() {
                        "Accordion Type" => {
                            if let Some(v) = parse_accordion_type(&field.value) {
                                accordion.parent_type = v;
                            }
                        }
                        "Accordion Class" => {
                            if let Some(v) = parse_accordion_class(&field.value) {
                                accordion.parent_class = v;
                            }
                        }
                        "Data AOS" => {
                            if let Some(v) = parse_parent_data_aos(&field.value) {
                                accordion.parent_data_aos = v;
                            }
                        }
                        "Group Name" => accordion.parent_group_name = field.value.clone(),
                        _ => {}
                    }
                }
                true
            } else {
                false
            }
        } else {
            self.status = "Failed to save: selected node is not a section.".to_string();
            false
        }
    }

    fn begin_save_prompt(&mut self) {
        self.component_picker = None;
        self.input_mode = None;
        self.save_prompt_open = true;
        self.save_input = self
            .path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "site.json".to_string());
        self.status = "Save prompt opened.".to_string();
    }

    fn handle_save_prompt_event(&mut self, evt: Event) -> anyhow::Result<()> {
        if let Event::Key(key) = evt {
            match key.code {
                KeyCode::Esc => {
                    self.save_prompt_open = false;
                    self.save_input.clear();
                    self.status = "Save cancelled.".to_string();
                }
                KeyCode::Enter => self.commit_save_prompt()?,
                KeyCode::Backspace => {
                    self.save_input.pop();
                }
                KeyCode::Char(c) => {
                    self.save_input.push(c);
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn commit_save_prompt(&mut self) -> anyhow::Result<()> {
        let raw = self.save_input.trim();
        if raw.is_empty() {
            self.status = "Save path cannot be empty.".to_string();
            return Ok(());
        }
        let path = PathBuf::from(raw);
        self.commit_save_with_backup(&path)?;
        self.save_prompt_open = false;
        self.save_input.clear();
        let msg = format!("Saved {}", path.display());
        self.push_toast(ToastLevel::Success, msg);
        Ok(())
    }

    fn begin_edit_selected(&mut self) {
        if self.selected_region == SelectedRegion::Header {
            self.begin_edit_header();
            return;
        }
        let page = self.current_page();
        if page.nodes.is_empty() {
            self.status = "No node selected.".to_string();
            return;
        }
        let idx = self.selected_node.min(page.nodes.len() - 1);
        match &page.nodes[idx] {
            PageNode::Hero(v) => {
                let hero = v.clone();
                self.open_edit_modal_for_hero(&hero);
            }
            PageNode::Section(v) => {
                let section = v.clone();
                self.open_edit_modal_for_section(&section);
            }
        }
    }

    fn open_edit_modal_for_hero(&mut self, hero: &crate::model::DdHero) {
        let fields = vec![
            EditField {
                label: "Image URL".to_string(),
                value: hero.parent_image_url.clone(),
                buffer: hero.parent_image_url.clone(),
                cursor: hero.parent_image_url.len(),
                is_multiline: false,
                rows: 1,
            },
            EditField {
                label: "Title".to_string(),
                value: hero.parent_title.clone(),
                buffer: hero.parent_title.clone(),
                cursor: hero.parent_title.len(),
                is_multiline: false,
                rows: 1,
            },
            EditField {
                label: "Subtitle".to_string(),
                value: hero.parent_subtitle.clone(),
                buffer: hero.parent_subtitle.clone(),
                cursor: hero.parent_subtitle.len(),
                is_multiline: false,
                rows: 1,
            },
            EditField {
                label: "Copy".to_string(),
                value: hero.parent_copy.clone().unwrap_or_default(),
                buffer: hero.parent_copy.clone().unwrap_or_default(),
                cursor: hero.parent_copy.clone().unwrap_or_default().len(),
                is_multiline: true,
                rows: 3, // Textarea: 3 rows per dd-hero.md spec
            },
            EditField {
                label: "CTA Text".to_string(),
                value: hero.link_1_label.clone().unwrap_or_default(),
                buffer: hero.link_1_label.clone().unwrap_or_default(),
                cursor: hero.link_1_label.clone().unwrap_or_default().len(),
                is_multiline: false,
                rows: 1,
            },
            EditField {
                label: "CTA Link".to_string(),
                value: hero.link_1_url.clone().unwrap_or_default(),
                buffer: hero.link_1_url.clone().unwrap_or_default(),
                cursor: hero.link_1_url.clone().unwrap_or_default().len(),
                is_multiline: false,
                rows: 1,
            },
            EditField {
                label: "Hero Class".to_string(),
                value: hero
                    .parent_class
                    .map(|c| format!("{:?}", c))
                    .unwrap_or_default(),
                buffer: hero
                    .parent_class
                    .map(|c| format!("{:?}", c))
                    .unwrap_or_default(),
                cursor: 0,
                is_multiline: false,
                rows: 1,
            },
            EditField {
                label: "Custom CSS".to_string(),
                value: hero.parent_custom_css.clone().unwrap_or_default(),
                buffer: hero.parent_custom_css.clone().unwrap_or_default(),
                cursor: hero.parent_custom_css.clone().unwrap_or_default().len(),
                is_multiline: false,
                rows: 1,
            },
        ];
        self.edit_modal = Some(EditModalState {
            title: "dd-hero".to_string(),
            fields,
            selected_field: 0,
            scroll_offset: 0,
            visible_fields: 6,
        });
        self.status =
            "Multi-field edit: Tab/Up/Down to navigate fields, type to edit, Ctrl+S to save, Esc to cancel."
                .to_string();
    }

    fn open_edit_modal_for_section(&mut self, section: &crate::model::DdSection) {
        let fields = vec![
            EditField {
                label: "Section ID".to_string(),
                value: section.id.clone(),
                buffer: section.id.clone(),
                cursor: section.id.len(),
                is_multiline: false,
                rows: 1,
            },
            EditField {
                label: "Section Title".to_string(),
                value: section.section_title.clone().unwrap_or_default(),
                buffer: section.section_title.clone().unwrap_or_default(),
                cursor: section.section_title.clone().unwrap_or_default().len(),
                is_multiline: false,
                rows: 1,
            },
            EditField {
                label: "Section Class".to_string(),
                value: section
                    .section_class
                    .map(|c| format!("{:?}", c))
                    .unwrap_or_default(),
                buffer: section
                    .section_class
                    .map(|c| format!("{:?}", c))
                    .unwrap_or_default(),
                cursor: 0,
                is_multiline: false,
                rows: 1,
            },
        ];
        self.edit_modal = Some(EditModalState {
            title: "dd-section".to_string(),
            fields,
            selected_field: 0,
            scroll_offset: 0,
            visible_fields: 6,
        });
        self.status =
            "Multi-field edit: Tab/Up/Down to navigate fields, type to edit, Ctrl+S to save, Esc to cancel."
                .to_string();
    }

    fn old_begin_edit_selected(&mut self) {
        // Deprecated - keeping for reference
        let selected = {
            let page = self.current_page();
            if page.nodes.is_empty() {
                None
            } else {
                let idx = self.selected_node.min(page.nodes.len() - 1);
                Some(match &page.nodes[idx] {
                    PageNode::Hero(v) => (InputMode::EditHeroImage, v.parent_image_url.clone()),
                    PageNode::Section(v) => (InputMode::EditSectionId, v.id.clone()),
                })
            }
        };

        let Some((mode, value)) = selected else {
            self.status = "No node selected.".to_string();
            return;
        };

        self.input_mode = Some(mode);
        self.input_buffer = value;
        self.clamp_multiline_input_if_needed();
        self.input_cursor = self.input_buffer.chars().count();
        self.ensure_multiline_cursor_visible();
        self.status = match mode {
            InputMode::EditHeroImage => {
                "Editing hero image URL. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroClass => {
                "Editing hero default class. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroAos => {
                "Editing hero data-aos option. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroCustomCss => {
                "Editing hero custom CSS classes. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroTitle => {
                "Editing hero title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroSubtitle => {
                "Editing hero subtitle. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroCopy => {
                "Editing hero copy. Enter: newline, Ctrl+S: save, esc: cancel.".to_string()
            }
            InputMode::EditHeroCtaText => {
                "Editing hero primary link text. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroCtaLink => {
                "Editing hero primary link URL. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroCtaTarget => {
                "Editing hero primary link target. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroCtaText2 => {
                "Editing hero secondary link text. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroCtaLink2 => {
                "Editing hero secondary link URL. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroCtaTarget2 => {
                "Editing hero secondary link target. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditSectionId => {
                "Editing section id. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditSectionTitle => {
                "Editing section title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditSectionClass => {
                "Editing section class. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditColumnId => {
                "Editing column id. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditColumnWidthClass => {
                "Editing column width class. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAlternatingType => {
                "Editing dd-alternating type. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAlternatingClass => {
                "Editing dd-alternating class. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAlternatingDataAos => {
                "Editing dd-alternating data-aos. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAlternatingItemImage => {
                "Editing dd-alternating item image. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAlternatingItemImageAlt => {
                "Editing dd-alternating item image alt. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAlternatingItemTitle => {
                "Editing dd-alternating item title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAlternatingItemCopy => {
                "Editing dd-alternating item copy. Enter: newline, Ctrl+S: save, esc: cancel."
                    .to_string()
            }
            InputMode::EditAlertType => {
                "Editing dd-alert type. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAlertClass => {
                "Editing dd-alert class. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAlertDataAos => {
                "Editing dd-alert data-aos. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAlertTitle => {
                "Editing dd-alert title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAlertCopy => {
                "Editing dd-alert copy. Enter: newline, Ctrl+S: save, esc: cancel.".to_string()
            }
            InputMode::EditBlockquoteDataAos => {
                "Editing dd-blockquote data-aos. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditBlockquoteImageUrl => {
                "Editing dd-blockquote image URL. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditBlockquoteImageAlt => {
                "Editing dd-blockquote image alt text. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditBlockquotePersonsName => {
                "Editing dd-blockquote person name. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditBlockquotePersonsTitle => {
                "Editing dd-blockquote person title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditBlockquoteCopy => {
                "Editing dd-blockquote copy. Enter: newline, Ctrl+S: save, esc: cancel.".to_string()
            }
            InputMode::EditBannerClass => {
                "Editing dd-banner class. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditBannerDataAos => {
                "Editing dd-banner data-aos. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditBannerImageUrl => {
                "Editing dd-banner image URL. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditBannerImageAlt => {
                "Editing dd-banner image alt text. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCtaClass => {
                "Editing dd-cta class. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCtaImageUrl => {
                "Editing dd-cta image URL. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCtaImageAlt => {
                "Editing dd-cta image alt text. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCtaDataAos => {
                "Editing dd-cta data-aos. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCtaTitle => {
                "Editing dd-cta title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCtaSubtitle => {
                "Editing dd-cta subtitle. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCtaCopy => {
                "Editing dd-cta copy. Enter: newline, Ctrl+S: save, esc: cancel.".to_string()
            }
            InputMode::EditCtaLinkUrl => {
                "Editing dd-cta link URL. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCtaLinkTarget => {
                "Editing dd-cta link target. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCtaLinkLabel => {
                "Editing dd-cta link label. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAccordionType => {
                "Editing dd-accordion type. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAccordionClass => {
                "Editing dd-accordion class. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAccordionAos => {
                "Editing dd-accordion data-aos. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAccordionGroupName => {
                "Editing dd-accordion group name. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAccordionFirstTitle => {
                "Editing dd-accordion first title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAccordionFirstContent => {
                "Editing dd-accordion item content. Enter: newline, Ctrl+S: save, esc: cancel."
                    .to_string()
            }
            InputMode::EditCardType => {
                "Editing dd-card type. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCardDataAos => {
                "Editing dd-card data-aos. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCardWidth => {
                "Editing dd-card width class. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCardItemImageUrl => {
                "Editing dd-card item image URL. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCardItemImageAlt => {
                "Editing dd-card item image alt text. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCardItemTitle => {
                "Editing dd-card item title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCardItemSubtitle => {
                "Editing dd-card item subtitle. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCardItemCopy => {
                "Editing dd-card item copy. Enter: newline, Ctrl+S: save, esc: cancel.".to_string()
            }
            InputMode::EditCardItemLinkUrl => {
                "Editing dd-card item link URL. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCardItemLinkTarget => {
                "Editing dd-card item link target. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCardItemLinkLabel => {
                "Editing dd-card item link label. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditFilmstripType => {
                "Editing dd-filmstrip type. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditFilmstripDataAos => {
                "Editing dd-filmstrip data-aos. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditFilmstripItemImageUrl => {
                "Editing dd-filmstrip item image URL. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditFilmstripItemImageAlt => {
                "Editing dd-filmstrip item image alt text. Enter to save, esc to cancel."
                    .to_string()
            }
            InputMode::EditFilmstripItemTitle => {
                "Editing dd-filmstrip item title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditMilestonesDataAos => {
                "Editing dd-milestones parent_data_aos. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditMilestonesWidth => {
                "Editing dd-milestones parent_width. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditMilestonesItemPercentage => {
                "Editing dd-milestones child_percentage. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditMilestonesItemTitle => {
                "Editing dd-milestones child_title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditMilestonesItemSubtitle => {
                "Editing dd-milestones child_subtitle. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditMilestonesItemCopy => {
                "Editing dd-milestones child_copy. Enter: newline, Ctrl+S: save, esc: cancel."
                    .to_string()
            }
            InputMode::EditMilestonesItemLinkUrl => {
                "Editing dd-milestones child_link_url. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditMilestonesItemLinkTarget => {
                "Editing dd-milestones child_link_target. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditMilestonesItemLinkLabel => {
                "Editing dd-milestones child_link_label. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditModalTitle => {
                "Editing dd-modal parent_title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditModalCopy => {
                "Editing dd-modal parent_copy. Enter: newline, Ctrl+S: save, esc: cancel."
                    .to_string()
            }
            InputMode::EditSliderTitle => {
                "Editing dd-slider parent_title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditSliderItemTitle => {
                "Editing dd-slider item child_title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditSliderItemCopy => {
                "Editing dd-slider item child_copy. Enter: newline, Ctrl+S: save, esc: cancel."
                    .to_string()
            }
            InputMode::EditSliderItemLinkUrl => {
                "Editing dd-slider item child_link_url. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditSliderItemLinkTarget => {
                "Editing dd-slider item child_link_target. Enter to save, esc to cancel."
                    .to_string()
            }
            InputMode::EditSliderItemLinkLabel => {
                "Editing dd-slider item child_link_label. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditSliderItemImageUrl => {
                "Editing dd-slider item child_image_url. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditSliderItemImageAlt => {
                "Editing dd-slider item child_image_alt. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeaderId => {
                "Editing header id. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeaderClass => {
                "Editing header class. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeaderCustomCss => {
                "Editing header custom CSS. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeaderColumnId => {
                "Editing header column id. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeaderColumnWidthClass => {
                "Editing header column width class. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeaderPlaceholderContent => {
                "Editing header placeholder content. Enter to save, esc to cancel.".to_string()
            }
            _ => "Editing field. Enter/Ctrl+S to save, esc to cancel.".to_string(),
        };
    }

    fn begin_edit_header(&mut self) {
        let rows = self.build_header_tree_rows();
        let row_kind = rows.get(self.selected_tree_row).map(|r| r.kind);
        match row_kind {
            Some(TreeRowKind::HeaderRoot) => {
                self.input_mode = Some(InputMode::EditHeaderId);
                self.input_buffer = self.site.header.id.clone();
            }
            Some(TreeRowKind::HeaderSection { section_idx }) => {
                self.input_mode = Some(InputMode::EditSectionId);
                let section_i = section_idx.min(self.site.header.sections.len().saturating_sub(1));
                self.input_buffer = self.site.header.sections[section_i].id.clone();
            }
            Some(TreeRowKind::HeaderColumn {
                section_idx,
                column_idx,
            }) => {
                self.input_mode = Some(InputMode::EditColumnId);
                let section_i = section_idx.min(self.site.header.sections.len().saturating_sub(1));
                let col_i = column_idx.min(
                    self.site.header.sections[section_i]
                        .columns
                        .len()
                        .saturating_sub(1),
                );
                self.input_buffer = self.site.header.sections[section_i].columns[col_i]
                    .id
                    .clone();
            }
            Some(TreeRowKind::HeaderComponent {
                section_idx,
                column_idx,
                component_idx,
            }) => {
                self.input_mode = Some(InputMode::EditBannerImageUrl);
                let section_i = section_idx.min(self.site.header.sections.len().saturating_sub(1));
                let col_i = column_idx.min(
                    self.site.header.sections[section_i]
                        .columns
                        .len()
                        .saturating_sub(1),
                );
                let comp_i = component_idx.min(
                    self.site.header.sections[section_i].columns[col_i]
                        .components
                        .len()
                        .saturating_sub(1),
                );
                if let crate::model::SectionComponent::Banner(banner) =
                    &self.site.header.sections[section_i].columns[col_i].components[comp_i]
                {
                    self.input_buffer = banner.parent_image_url.clone();
                } else {
                    self.input_buffer = String::new();
                }
            }
            _ => {
                self.status = "No header element selected.".to_string();
                return;
            }
        }
        self.input_cursor = self.input_buffer.chars().count();
        self.status = "Editing header. Enter to save, esc to cancel.".to_string();
    }

    fn begin_edit_selected_column_id(&mut self) {
        let selected = {
            let page = self.current_page();
            if page.nodes.is_empty() {
                None
            } else {
                let idx = self.selected_node.min(page.nodes.len() - 1);
                match &page.nodes[idx] {
                    PageNode::Hero(_) => None,
                    PageNode::Section(section) => {
                        let columns = section_columns_ref(section);
                        let col_i = self.selected_column.min(columns.len().saturating_sub(1));
                        Some((InputMode::EditColumnId, columns[col_i].id.clone()))
                    }
                }
            }
        };
        let Some((mode, value)) = selected else {
            self.status = "Selected node is not a section.".to_string();
            return;
        };
        self.input_mode = Some(mode);
        self.input_buffer = value;
        self.input_cursor = self.input_buffer.chars().count();
        self.status = "Editing selected column id. Enter to save, esc to cancel.".to_string();
    }

    fn begin_edit_selected_column_width_class(&mut self) {
        let selected = {
            let page = self.current_page();
            if page.nodes.is_empty() {
                None
            } else {
                let idx = self.selected_node.min(page.nodes.len() - 1);
                match &page.nodes[idx] {
                    PageNode::Hero(_) => None,
                    PageNode::Section(section) => {
                        let columns = section_columns_ref(section);
                        let col_i = self.selected_column.min(columns.len().saturating_sub(1));
                        Some((
                            InputMode::EditColumnWidthClass,
                            columns[col_i].width_class.clone(),
                        ))
                    }
                }
            }
        };
        let Some((mode, value)) = selected else {
            self.status = "Selected node is not a section.".to_string();
            return;
        };
        self.input_mode = Some(mode);
        self.input_buffer = value;
        self.input_cursor = self.input_buffer.chars().count();
        self.status =
            "Editing selected column width class. Enter to save, esc to cancel.".to_string();
    }

    fn commit_input_edit(&mut self) -> bool {
        let Some(mode) = self.input_mode else {
            return false;
        };
        let value = if Self::is_multiline_mode(mode) {
            self.input_buffer.clone()
        } else {
            self.input_buffer.trim().to_string()
        };
        let allow_empty = matches!(
            mode,
            InputMode::EditHeroImage
                | InputMode::EditHeroClass
                | InputMode::EditHeroCustomCss
                | InputMode::EditHeroSubtitle
                | InputMode::EditHeroCopy
                | InputMode::EditHeroCtaText
                | InputMode::EditHeroCtaLink
                | InputMode::EditHeroCtaTarget
                | InputMode::EditHeroCtaText2
                | InputMode::EditHeroCtaLink2
                | InputMode::EditHeroCtaTarget2
                | InputMode::EditSectionTitle
                | InputMode::EditAlternatingItemCopy
                | InputMode::EditCtaLinkUrl
                | InputMode::EditCtaLinkTarget
                | InputMode::EditCtaLinkLabel
                | InputMode::EditBlockquoteCopy
                | InputMode::EditCardItemLinkUrl
                | InputMode::EditCardItemLinkTarget
                | InputMode::EditCardItemLinkLabel
        );
        if value.is_empty() && !allow_empty {
            self.status = "Value cannot be empty.".to_string();
            return false;
        }
        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let selected_nested_item = self.selected_nested_item;
        let mut status = "No page available.".to_string();
        let mut clear_input = true;
        let mut applied = false;
        let Some(page) = self.current_page_mut() else {
            self.status = status;
            return false;
        };
        if page.nodes.is_empty() {
            self.status = "No node selected.".to_string();
            return false;
        }
        let idx = selected.min(page.nodes.len() - 1);
        status = match (&mut page.nodes[idx], mode) {
            (PageNode::Hero(v), InputMode::EditHeroImage) => {
                v.parent_image_url = value;
                applied = true;
                "Updated hero image.".to_string()
            }
            (PageNode::Hero(v), InputMode::EditHeroClass) => {
                let parsed = parse_hero_image_class(value.as_str());
                if let Some(parent_class) = parsed {
                    v.parent_class = Some(parent_class);
                    applied = true;
                    "Updated hero default class.".to_string()
                } else {
                    clear_input = false;
                    "Invalid hero class option.".to_string()
                }
            }
            (PageNode::Hero(v), InputMode::EditHeroAos) => {
                let parsed = parse_parent_data_aos(value.as_str());
                if let Some(aos) = parsed {
                    v.parent_data_aos = Some(aos);
                    applied = true;
                    "Updated hero data-aos option.".to_string()
                } else {
                    clear_input = false;
                    "Invalid hero data-aos option.".to_string()
                }
            }
            (PageNode::Hero(v), InputMode::EditHeroCustomCss) => {
                v.parent_custom_css = if value.is_empty() { None } else { Some(value) };
                applied = true;
                "Updated hero custom CSS classes.".to_string()
            }
            (PageNode::Hero(v), InputMode::EditHeroTitle) => {
                v.parent_title = value;
                applied = true;
                "Updated hero title.".to_string()
            }
            (PageNode::Hero(v), InputMode::EditHeroSubtitle) => {
                v.parent_subtitle = value;
                applied = true;
                "Updated hero subtitle.".to_string()
            }
            (PageNode::Hero(v), InputMode::EditHeroCopy) => {
                v.parent_copy = if value.is_empty() { None } else { Some(value) };
                applied = true;
                "Updated hero copy.".to_string()
            }
            (PageNode::Hero(v), InputMode::EditHeroCtaText) => {
                v.link_1_label = if value.is_empty() { None } else { Some(value) };
                applied = true;
                "Updated hero primary link text.".to_string()
            }
            (PageNode::Hero(v), InputMode::EditHeroCtaLink) => {
                v.link_1_url = if value.is_empty() { None } else { Some(value) };
                applied = true;
                "Updated hero primary link URL.".to_string()
            }
            (PageNode::Hero(v), InputMode::EditHeroCtaTarget) => {
                if value.is_empty() {
                    v.link_1_target = None;
                    applied = true;
                    "Updated hero primary link target.".to_string()
                } else if let Some(target) = parse_link_1_target(value.as_str()) {
                    v.link_1_target = Some(target);
                    applied = true;
                    "Updated hero primary link target.".to_string()
                } else {
                    clear_input = false;
                    "Invalid hero primary link target.".to_string()
                }
            }
            (PageNode::Hero(v), InputMode::EditHeroCtaText2) => {
                v.link_2_label = if value.is_empty() { None } else { Some(value) };
                applied = true;
                "Updated hero secondary link text.".to_string()
            }
            (PageNode::Hero(v), InputMode::EditHeroCtaLink2) => {
                v.link_2_url = if value.is_empty() { None } else { Some(value) };
                applied = true;
                "Updated hero secondary link URL.".to_string()
            }
            (PageNode::Hero(v), InputMode::EditHeroCtaTarget2) => {
                if value.is_empty() {
                    v.link_2_target = None;
                    applied = true;
                    "Updated hero secondary link target.".to_string()
                } else if let Some(target) = parse_link_1_target(value.as_str()) {
                    v.link_2_target = Some(target);
                    applied = true;
                    "Updated hero secondary link target.".to_string()
                } else {
                    clear_input = false;
                    "Invalid hero secondary link target.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditSectionId) => {
                v.id = value;
                applied = true;
                "Updated section id.".to_string()
            }
            (PageNode::Section(v), InputMode::EditSectionTitle) => {
                v.section_title = if value.is_empty() { None } else { Some(value) };
                applied = true;
                "Updated section title.".to_string()
            }
            (PageNode::Section(v), InputMode::EditSectionClass) => {
                if let Some(section_class) = parse_section_class(value.as_str()) {
                    v.section_class = Some(section_class);
                    applied = true;
                    "Updated section class.".to_string()
                } else {
                    clear_input = false;
                    "Invalid section class option.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditColumnId) => {
                let col_i = selected_column.min(v.columns.len().saturating_sub(1));
                v.columns[col_i].id = value;
                applied = true;
                "Updated column id.".to_string()
            }
            (PageNode::Section(v), InputMode::EditColumnWidthClass) => {
                let col_i = selected_column.min(v.columns.len().saturating_sub(1));
                v.columns[col_i].width_class = value;
                applied = true;
                "Updated column width class.".to_string()
            }
            (PageNode::Section(v), InputMode::EditAlternatingType) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Alternating(alt) = &mut v.columns[selected_column].components[ci]
                    {
                        if let Some(vt) = parse_alternating_type(value.as_str()) {
                            alt.parent_type = vt;
                            applied = true;
                            "Updated dd-alternating type.".to_string()
                        } else {
                            clear_input = false;
                            "Invalid dd-alternating type option.".to_string()
                        }
                    } else {
                        "Selected component is not dd-alternating.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditAlternatingClass) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Alternating(alt) = &mut v.columns[selected_column].components[ci]
                    {
                        alt.parent_class = value;
                        applied = true;
                        "Updated dd-alternating class.".to_string()
                    } else {
                        "Selected component is not dd-alternating.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditAlternatingDataAos) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Alternating(alt) = &mut v.columns[selected_column].components[ci]
                    {
                        if let Some(va) = parse_parent_data_aos(value.as_str()) {
                            alt.parent_data_aos = va;
                            applied = true;
                            "Updated dd-alternating data-aos.".to_string()
                        } else {
                            clear_input = false;
                            "Invalid dd-alternating data-aos option.".to_string()
                        }
                    } else {
                        "Selected component is not dd-alternating.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditAlternatingItemImage) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Alternating(alt) = &mut v.columns[selected_column].components[ci]
                    {
                        if let Some(ni) = nested_index(alt.items.len(), selected_nested_item) {
                            alt.items[ni].child_image_url = value;
                            applied = true;
                            format!("Updated dd-alternating item {} image.", ni + 1)
                        } else {
                            "dd-alternating has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-alternating.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditAlternatingItemImageAlt) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Alternating(alt) = &mut v.columns[selected_column].components[ci]
                    {
                        if let Some(ni) = nested_index(alt.items.len(), selected_nested_item) {
                            alt.items[ni].child_image_alt = value;
                            applied = true;
                            format!("Updated dd-alternating item {} image alt.", ni + 1)
                        } else {
                            "dd-alternating has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-alternating.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditAlternatingItemTitle) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Alternating(alt) = &mut v.columns[selected_column].components[ci]
                    {
                        if let Some(ni) = nested_index(alt.items.len(), selected_nested_item) {
                            alt.items[ni].child_title = value;
                            applied = true;
                            format!("Updated dd-alternating item {} title.", ni + 1)
                        } else {
                            "dd-alternating has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-alternating.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditAlternatingItemCopy) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Alternating(alt) = &mut v.columns[selected_column].components[ci]
                    {
                        if let Some(ni) = nested_index(alt.items.len(), selected_nested_item) {
                            alt.items[ni].child_copy = value;
                            applied = true;
                            format!("Updated dd-alternating item {} copy.", ni + 1)
                        } else {
                            "dd-alternating has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-alternating.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditBannerClass) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Banner(banner) = &mut v.columns[selected_column].components[ci] {
                        if let Some(vc) = parse_banner_class(value.as_str()) {
                            banner.parent_class = vc;
                            applied = true;
                            "Updated dd-banner class.".to_string()
                        } else {
                            clear_input = false;
                            "Invalid dd-banner class option.".to_string()
                        }
                    } else {
                        "Selected component is not dd-banner.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditBannerDataAos) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Banner(banner) = &mut v.columns[selected_column].components[ci] {
                        if let Some(va) = parse_parent_data_aos(value.as_str()) {
                            banner.parent_data_aos = va;
                            applied = true;
                            "Updated dd-banner data-aos.".to_string()
                        } else {
                            clear_input = false;
                            "Invalid dd-banner data-aos option.".to_string()
                        }
                    } else {
                        "Selected component is not dd-banner.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditBannerImageUrl) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Banner(banner) = &mut v.columns[selected_column].components[ci] {
                        banner.parent_image_url = value;
                        applied = true;
                        "Updated dd-banner image URL.".to_string()
                    } else {
                        "Selected component is not dd-banner.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditBannerImageAlt) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Banner(banner) = &mut v.columns[selected_column].components[ci] {
                        banner.parent_image_alt = value;
                        applied = true;
                        "Updated dd-banner image alt text.".to_string()
                    } else {
                        "Selected component is not dd-banner.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditCtaClass) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Cta(cta) = &mut v.columns[selected_column].components[ci] {
                        if let Some(vc) = parse_cta_class(value.as_str()) {
                            cta.parent_class = vc;
                            applied = true;
                            "Updated dd-cta class.".to_string()
                        } else {
                            clear_input = false;
                            "Invalid dd-cta class option.".to_string()
                        }
                    } else {
                        "Selected component is not dd-cta.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditCtaImageUrl) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Cta(cta) = &mut v.columns[selected_column].components[ci] {
                        cta.parent_image_url = value;
                        applied = true;
                        "Updated dd-cta image URL.".to_string()
                    } else {
                        "Selected component is not dd-cta.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditCtaImageAlt) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Cta(cta) = &mut v.columns[selected_column].components[ci] {
                        cta.parent_image_alt = value;
                        applied = true;
                        "Updated dd-cta image alt text.".to_string()
                    } else {
                        "Selected component is not dd-cta.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditCtaDataAos) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Cta(cta) = &mut v.columns[selected_column].components[ci] {
                        if let Some(va) = parse_parent_data_aos(value.as_str()) {
                            cta.parent_data_aos = va;
                            applied = true;
                            "Updated dd-cta data-aos.".to_string()
                        } else {
                            clear_input = false;
                            "Invalid dd-cta data-aos option.".to_string()
                        }
                    } else {
                        "Selected component is not dd-cta.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditCtaTitle) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Cta(cta) = &mut v.columns[selected_column].components[ci] {
                        cta.parent_title = value;
                        applied = true;
                        "Updated dd-cta title.".to_string()
                    } else {
                        "Selected component is not dd-cta.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditCtaSubtitle) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Cta(cta) = &mut v.columns[selected_column].components[ci] {
                        cta.parent_subtitle = value;
                        applied = true;
                        "Updated dd-cta subtitle.".to_string()
                    } else {
                        "Selected component is not dd-cta.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditCtaCopy) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Cta(cta) = &mut v.columns[selected_column].components[ci] {
                        cta.parent_copy = value;
                        applied = true;
                        "Updated dd-cta copy.".to_string()
                    } else {
                        "Selected component is not dd-cta.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditCtaLinkUrl) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Cta(cta) = &mut v.columns[selected_column].components[ci] {
                        cta.parent_link_url = if value.is_empty() { None } else { Some(value) };
                        applied = true;
                        "Updated dd-cta link URL.".to_string()
                    } else {
                        "Selected component is not dd-cta.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditCtaLinkTarget) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Cta(cta) = &mut v.columns[selected_column].components[ci] {
                        if value.is_empty() {
                            cta.parent_link_target = None;
                            applied = true;
                            "Updated dd-cta link target.".to_string()
                        } else if let Some(vt) = parse_child_link_target(value.as_str()) {
                            cta.parent_link_target = Some(vt);
                            applied = true;
                            "Updated dd-cta link target.".to_string()
                        } else {
                            clear_input = false;
                            "Invalid dd-cta link target option.".to_string()
                        }
                    } else {
                        "Selected component is not dd-cta.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditCtaLinkLabel) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Cta(cta) = &mut v.columns[selected_column].components[ci] {
                        cta.parent_link_label = if value.is_empty() { None } else { Some(value) };
                        applied = true;
                        "Updated dd-cta link label.".to_string()
                    } else {
                        "Selected component is not dd-cta.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditFilmstripType) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Filmstrip(filmstrip) =
                        &mut v.columns[selected_column].components[ci]
                    {
                        if let Some(vt) = parse_filmstrip_type(value.as_str()) {
                            filmstrip.parent_type = vt;
                            applied = true;
                            "Updated dd-filmstrip type.".to_string()
                        } else {
                            clear_input = false;
                            "Invalid dd-filmstrip type option.".to_string()
                        }
                    } else {
                        "Selected component is not dd-filmstrip.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditFilmstripDataAos) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Filmstrip(filmstrip) =
                        &mut v.columns[selected_column].components[ci]
                    {
                        if let Some(va) = parse_parent_data_aos(value.as_str()) {
                            filmstrip.parent_data_aos = va;
                            applied = true;
                            "Updated dd-filmstrip data-aos.".to_string()
                        } else {
                            clear_input = false;
                            "Invalid dd-filmstrip data-aos option.".to_string()
                        }
                    } else {
                        "Selected component is not dd-filmstrip.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditFilmstripItemImageUrl) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Filmstrip(filmstrip) =
                        &mut v.columns[selected_column].components[ci]
                    {
                        if let Some(ni) = nested_index(filmstrip.items.len(), selected_nested_item)
                        {
                            filmstrip.items[ni].child_image_url = value;
                            applied = true;
                            format!("Updated dd-filmstrip item {} image URL.", ni + 1)
                        } else {
                            "dd-filmstrip has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-filmstrip.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditFilmstripItemImageAlt) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Filmstrip(filmstrip) =
                        &mut v.columns[selected_column].components[ci]
                    {
                        if let Some(ni) = nested_index(filmstrip.items.len(), selected_nested_item)
                        {
                            filmstrip.items[ni].child_image_alt = value;
                            applied = true;
                            format!("Updated dd-filmstrip item {} image alt.", ni + 1)
                        } else {
                            "dd-filmstrip has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-filmstrip.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditFilmstripItemTitle) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Filmstrip(filmstrip) =
                        &mut v.columns[selected_column].components[ci]
                    {
                        if let Some(ni) = nested_index(filmstrip.items.len(), selected_nested_item)
                        {
                            filmstrip.items[ni].child_title = value;
                            applied = true;
                            format!("Updated dd-filmstrip item {} title.", ni + 1)
                        } else {
                            "dd-filmstrip has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-filmstrip.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditMilestonesDataAos) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Milestones(milestones) =
                        &mut v.columns[selected_column].components[ci]
                    {
                        if let Some(va) = parse_parent_data_aos(value.as_str()) {
                            milestones.parent_data_aos = va;
                            applied = true;
                            "Updated dd-milestones parent_data_aos.".to_string()
                        } else {
                            clear_input = false;
                            "Invalid dd-milestones parent_data_aos option.".to_string()
                        }
                    } else {
                        "Selected component is not dd-milestones.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditMilestonesWidth) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Milestones(milestones) =
                        &mut v.columns[selected_column].components[ci]
                    {
                        milestones.parent_width = value;
                        applied = true;
                        "Updated dd-milestones parent_width.".to_string()
                    } else {
                        "Selected component is not dd-milestones.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditMilestonesItemPercentage) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Milestones(milestones) =
                        &mut v.columns[selected_column].components[ci]
                    {
                        if let Some(ni) = nested_index(milestones.items.len(), selected_nested_item)
                        {
                            milestones.items[ni].child_percentage = value;
                            applied = true;
                            format!("Updated dd-milestones item {} child_percentage.", ni + 1)
                        } else {
                            "dd-milestones has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-milestones.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditMilestonesItemTitle) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Milestones(milestones) =
                        &mut v.columns[selected_column].components[ci]
                    {
                        if let Some(ni) = nested_index(milestones.items.len(), selected_nested_item)
                        {
                            milestones.items[ni].child_title = value;
                            applied = true;
                            format!("Updated dd-milestones item {} child_title.", ni + 1)
                        } else {
                            "dd-milestones has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-milestones.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditMilestonesItemSubtitle) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Milestones(milestones) =
                        &mut v.columns[selected_column].components[ci]
                    {
                        if let Some(ni) = nested_index(milestones.items.len(), selected_nested_item)
                        {
                            milestones.items[ni].child_subtitle = value;
                            applied = true;
                            format!("Updated dd-milestones item {} child_subtitle.", ni + 1)
                        } else {
                            "dd-milestones has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-milestones.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditMilestonesItemCopy) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Milestones(milestones) =
                        &mut v.columns[selected_column].components[ci]
                    {
                        if let Some(ni) = nested_index(milestones.items.len(), selected_nested_item)
                        {
                            milestones.items[ni].child_copy = value;
                            applied = true;
                            format!("Updated dd-milestones item {} child_copy.", ni + 1)
                        } else {
                            "dd-milestones has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-milestones.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditMilestonesItemLinkUrl) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Milestones(milestones) =
                        &mut v.columns[selected_column].components[ci]
                    {
                        if let Some(ni) = nested_index(milestones.items.len(), selected_nested_item)
                        {
                            milestones.items[ni].child_link_url =
                                if value.is_empty() { None } else { Some(value) };
                            applied = true;
                            format!("Updated dd-milestones item {} child_link_url.", ni + 1)
                        } else {
                            "dd-milestones has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-milestones.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditMilestonesItemLinkTarget) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Milestones(milestones) =
                        &mut v.columns[selected_column].components[ci]
                    {
                        if let Some(ni) = nested_index(milestones.items.len(), selected_nested_item)
                        {
                            if value.is_empty() {
                                milestones.items[ni].child_link_target = None;
                                applied = true;
                                format!("Updated dd-milestones item {} child_link_target.", ni + 1)
                            } else if let Some(vt) = parse_child_link_target(value.as_str()) {
                                milestones.items[ni].child_link_target = Some(vt);
                                applied = true;
                                format!("Updated dd-milestones item {} child_link_target.", ni + 1)
                            } else {
                                clear_input = false;
                                "Invalid dd-milestones child_link_target option.".to_string()
                            }
                        } else {
                            "dd-milestones has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-milestones.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditMilestonesItemLinkLabel) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Milestones(milestones) =
                        &mut v.columns[selected_column].components[ci]
                    {
                        if let Some(ni) = nested_index(milestones.items.len(), selected_nested_item)
                        {
                            milestones.items[ni].child_link_label =
                                if value.is_empty() { None } else { Some(value) };
                            applied = true;
                            format!("Updated dd-milestones item {} child_link_label.", ni + 1)
                        } else {
                            "dd-milestones has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-milestones.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditModalTitle) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Modal(modal) = &mut v.columns[selected_column].components[ci] {
                        modal.parent_title = value;
                        applied = true;
                        "Updated dd-modal parent_title.".to_string()
                    } else {
                        "Selected component is not dd-modal.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditModalCopy) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Modal(modal) = &mut v.columns[selected_column].components[ci] {
                        modal.parent_copy = value;
                        applied = true;
                        "Updated dd-modal parent_copy.".to_string()
                    } else {
                        "Selected component is not dd-modal.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditSliderTitle) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Slider(slider) = &mut v.columns[selected_column].components[ci] {
                        slider.parent_title = value;
                        applied = true;
                        "Updated dd-slider parent_title.".to_string()
                    } else {
                        "Selected component is not dd-slider.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditSliderItemTitle) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Slider(slider) = &mut v.columns[selected_column].components[ci] {
                        if let Some(ni) = nested_index(slider.items.len(), selected_nested_item) {
                            slider.items[ni].child_title = value;
                            applied = true;
                            format!("Updated dd-slider item {} child_title.", ni + 1)
                        } else {
                            "dd-slider has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-slider.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditSliderItemCopy) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Slider(slider) = &mut v.columns[selected_column].components[ci] {
                        if let Some(ni) = nested_index(slider.items.len(), selected_nested_item) {
                            slider.items[ni].child_copy = value;
                            applied = true;
                            format!("Updated dd-slider item {} child_copy.", ni + 1)
                        } else {
                            "dd-slider has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-slider.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditSliderItemLinkUrl) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Slider(slider) = &mut v.columns[selected_column].components[ci] {
                        if let Some(ni) = nested_index(slider.items.len(), selected_nested_item) {
                            slider.items[ni].child_link_url =
                                if value.is_empty() { None } else { Some(value) };
                            applied = true;
                            format!("Updated dd-slider item {} child_link_url.", ni + 1)
                        } else {
                            "dd-slider has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-slider.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditSliderItemLinkTarget) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Slider(slider) = &mut v.columns[selected_column].components[ci] {
                        if let Some(ni) = nested_index(slider.items.len(), selected_nested_item) {
                            if value.is_empty() {
                                slider.items[ni].child_link_target = None;
                                applied = true;
                                format!("Updated dd-slider item {} child_link_target.", ni + 1)
                            } else if let Some(vt) = parse_child_link_target(value.as_str()) {
                                slider.items[ni].child_link_target = Some(vt);
                                applied = true;
                                format!("Updated dd-slider item {} child_link_target.", ni + 1)
                            } else {
                                clear_input = false;
                                "Invalid dd-slider child_link_target option.".to_string()
                            }
                        } else {
                            "dd-slider has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-slider.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditSliderItemLinkLabel) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Slider(slider) = &mut v.columns[selected_column].components[ci] {
                        if let Some(ni) = nested_index(slider.items.len(), selected_nested_item) {
                            slider.items[ni].child_link_label =
                                if value.is_empty() { None } else { Some(value) };
                            applied = true;
                            format!("Updated dd-slider item {} child_link_label.", ni + 1)
                        } else {
                            "dd-slider has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-slider.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditSliderItemImageUrl) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Slider(slider) = &mut v.columns[selected_column].components[ci] {
                        if let Some(ni) = nested_index(slider.items.len(), selected_nested_item) {
                            slider.items[ni].child_image_url = value;
                            applied = true;
                            format!("Updated dd-slider item {} child_image_url.", ni + 1)
                        } else {
                            "dd-slider has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-slider.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditSliderItemImageAlt) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Slider(slider) = &mut v.columns[selected_column].components[ci] {
                        if let Some(ni) = nested_index(slider.items.len(), selected_nested_item) {
                            slider.items[ni].child_image_alt = value;
                            applied = true;
                            format!("Updated dd-slider item {} child_image_alt.", ni + 1)
                        } else {
                            "dd-slider has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-slider.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditBlockquoteDataAos) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Blockquote(blockquote) =
                        &mut v.columns[selected_column].components[ci]
                    {
                        if let Some(va) = parse_parent_data_aos(value.as_str()) {
                            blockquote.parent_data_aos = va;
                            applied = true;
                            "Updated dd-blockquote data-aos.".to_string()
                        } else {
                            clear_input = false;
                            "Invalid dd-blockquote data-aos option.".to_string()
                        }
                    } else {
                        "Selected component is not dd-blockquote.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditBlockquoteImageUrl) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Blockquote(blockquote) =
                        &mut v.columns[selected_column].components[ci]
                    {
                        blockquote.parent_image_url = value;
                        applied = true;
                        "Updated dd-blockquote image URL.".to_string()
                    } else {
                        "Selected component is not dd-blockquote.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditBlockquoteImageAlt) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Blockquote(blockquote) =
                        &mut v.columns[selected_column].components[ci]
                    {
                        blockquote.parent_image_alt = value;
                        applied = true;
                        "Updated dd-blockquote image alt text.".to_string()
                    } else {
                        "Selected component is not dd-blockquote.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditBlockquotePersonsName) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Blockquote(blockquote) =
                        &mut v.columns[selected_column].components[ci]
                    {
                        blockquote.parent_name = value;
                        applied = true;
                        "Updated dd-blockquote person name.".to_string()
                    } else {
                        "Selected component is not dd-blockquote.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditBlockquotePersonsTitle) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Blockquote(blockquote) =
                        &mut v.columns[selected_column].components[ci]
                    {
                        blockquote.parent_role = value;
                        applied = true;
                        "Updated dd-blockquote person title.".to_string()
                    } else {
                        "Selected component is not dd-blockquote.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditBlockquoteCopy) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Blockquote(blockquote) =
                        &mut v.columns[selected_column].components[ci]
                    {
                        blockquote.parent_copy = value;
                        applied = true;
                        "Updated dd-blockquote copy.".to_string()
                    } else {
                        "Selected component is not dd-blockquote.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditCardType) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Card(card) = &mut v.columns[selected_column].components[ci] {
                        if let Some(vt) = parse_card_type(value.as_str()) {
                            card.parent_type = vt;
                            applied = true;
                            "Updated dd-card type.".to_string()
                        } else {
                            clear_input = false;
                            "Invalid dd-card type option.".to_string()
                        }
                    } else {
                        "Selected component is not dd-card.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditCardDataAos) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Card(card) = &mut v.columns[selected_column].components[ci] {
                        if let Some(va) = parse_parent_data_aos(value.as_str()) {
                            card.parent_data_aos = va;
                            applied = true;
                            "Updated dd-card data-aos.".to_string()
                        } else {
                            clear_input = false;
                            "Invalid dd-card data-aos option.".to_string()
                        }
                    } else {
                        "Selected component is not dd-card.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditCardWidth) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Card(card) = &mut v.columns[selected_column].components[ci] {
                        card.parent_width = value;
                        applied = true;
                        "Updated dd-card width classes.".to_string()
                    } else {
                        "Selected component is not dd-card.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditCardItemImageUrl) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Card(card) = &mut v.columns[selected_column].components[ci] {
                        if let Some(ni) = nested_index(card.items.len(), selected_nested_item) {
                            card.items[ni].child_image_url = value;
                            applied = true;
                            format!("Updated dd-card item {} image URL.", ni + 1)
                        } else {
                            "dd-card has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-card.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditCardItemImageAlt) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Card(card) = &mut v.columns[selected_column].components[ci] {
                        if let Some(ni) = nested_index(card.items.len(), selected_nested_item) {
                            card.items[ni].child_image_alt = value;
                            applied = true;
                            format!("Updated dd-card item {} image alt.", ni + 1)
                        } else {
                            "dd-card has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-card.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditCardItemTitle) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Card(card) = &mut v.columns[selected_column].components[ci] {
                        if let Some(ni) = nested_index(card.items.len(), selected_nested_item) {
                            card.items[ni].child_title = value;
                            applied = true;
                            format!("Updated dd-card item {} title.", ni + 1)
                        } else {
                            "dd-card has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-card.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditCardItemSubtitle) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Card(card) = &mut v.columns[selected_column].components[ci] {
                        if let Some(ni) = nested_index(card.items.len(), selected_nested_item) {
                            card.items[ni].child_subtitle = value;
                            applied = true;
                            format!("Updated dd-card item {} subtitle.", ni + 1)
                        } else {
                            "dd-card has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-card.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditCardItemCopy) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Card(card) = &mut v.columns[selected_column].components[ci] {
                        if let Some(ni) = nested_index(card.items.len(), selected_nested_item) {
                            card.items[ni].child_copy = value;
                            applied = true;
                            format!("Updated dd-card item {} copy.", ni + 1)
                        } else {
                            "dd-card has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-card.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditCardItemLinkUrl) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Card(card) = &mut v.columns[selected_column].components[ci] {
                        if let Some(ni) = nested_index(card.items.len(), selected_nested_item) {
                            card.items[ni].child_link_url =
                                if value.is_empty() { None } else { Some(value) };
                            applied = true;
                            format!("Updated dd-card item {} link URL.", ni + 1)
                        } else {
                            "dd-card has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-card.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditCardItemLinkTarget) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Card(card) = &mut v.columns[selected_column].components[ci] {
                        if let Some(ni) = nested_index(card.items.len(), selected_nested_item) {
                            if value.is_empty() {
                                card.items[ni].child_link_target = None;
                                applied = true;
                                format!("Updated dd-card item {} link target.", ni + 1)
                            } else if let Some(vt) = parse_child_link_target(value.as_str()) {
                                card.items[ni].child_link_target = Some(vt);
                                applied = true;
                                format!("Updated dd-card item {} link target.", ni + 1)
                            } else {
                                clear_input = false;
                                "Invalid dd-card link target option.".to_string()
                            }
                        } else {
                            "dd-card has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-card.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditCardItemLinkLabel) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Card(card) = &mut v.columns[selected_column].components[ci] {
                        if let Some(ni) = nested_index(card.items.len(), selected_nested_item) {
                            card.items[ni].child_link_label =
                                if value.is_empty() { None } else { Some(value) };
                            applied = true;
                            format!("Updated dd-card item {} link label.", ni + 1)
                        } else {
                            "dd-card has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-card.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditAccordionType) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Accordion(acc) = &mut v.columns[selected_column].components[ci] {
                        if let Some(vt) = parse_accordion_type(value.as_str()) {
                            acc.parent_type = vt;
                            applied = true;
                            "Updated dd-accordion type.".to_string()
                        } else {
                            clear_input = false;
                            "Invalid dd-accordion type option.".to_string()
                        }
                    } else {
                        "Selected component is not dd-accordion.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditAccordionClass) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Accordion(acc) = &mut v.columns[selected_column].components[ci] {
                        if let Some(vc) = parse_accordion_class(value.as_str()) {
                            acc.parent_class = vc;
                            applied = true;
                            "Updated dd-accordion class.".to_string()
                        } else {
                            clear_input = false;
                            "Invalid dd-accordion class option.".to_string()
                        }
                    } else {
                        "Selected component is not dd-accordion.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditAccordionAos) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Accordion(acc) = &mut v.columns[selected_column].components[ci] {
                        if let Some(va) = parse_parent_data_aos(value.as_str()) {
                            acc.parent_data_aos = va;
                            applied = true;
                            "Updated dd-accordion data-aos.".to_string()
                        } else {
                            clear_input = false;
                            "Invalid dd-accordion data-aos option.".to_string()
                        }
                    } else {
                        "Selected component is not dd-accordion.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditAccordionGroupName) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Accordion(acc) = &mut v.columns[selected_column].components[ci] {
                        acc.parent_group_name = value;
                        applied = true;
                        "Updated dd-accordion group name.".to_string()
                    } else {
                        "Selected component is not dd-accordion.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditAccordionFirstTitle) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Accordion(acc) = &mut v.columns[selected_column].components[ci] {
                        if let Some(ni) = nested_index(acc.items.len(), selected_nested_item) {
                            acc.items[ni].child_title = value;
                            applied = true;
                            format!("Updated dd-accordion item {} title.", ni + 1)
                        } else {
                            "dd-accordion has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-accordion.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            (PageNode::Section(v), InputMode::EditAccordionFirstContent) => {
                if let Some(ci) = component_index(v.columns[selected_column].components.len(), selected_component) {
                    if let crate::model::SectionComponent::Accordion(acc) = &mut v.columns[selected_column].components[ci] {
                        if let Some(ni) = nested_index(acc.items.len(), selected_nested_item) {
                            acc.items[ni].child_copy = value;
                            applied = true;
                            format!("Updated dd-accordion item {} content.", ni + 1)
                        } else {
                            "dd-accordion has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-accordion.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            _ => "Edit type no longer matches selected node.".to_string(),
        };
        self.status = status;
        if clear_input {
            self.input_mode = None;
            self.input_buffer.clear();
            self.input_cursor = 0;
            self.multiline_scroll_row = 0;
        }
        applied
    }

    fn handle_click(&mut self, x: u16, y: u16) {
        if !contains(self.list_area, x, y) {
            return;
        }
        let tree_rows = self.build_tree_rows();
        if tree_rows.is_empty() {
            return;
        }
        let body_top = self.list_area.y.saturating_add(1);
        let body_bottom = self
            .list_area
            .y
            .saturating_add(self.list_area.height.saturating_sub(1));
        if y < body_top || y >= body_bottom {
            return;
        }
        let idx = (y - body_top) as usize;
        if idx < tree_rows.len() {
            self.selected_tree_row = idx;
            self.apply_tree_row_selection(tree_rows[idx]);
            self.status = format!("Selected {}", self.tree_row_label(&tree_rows[idx]));
        }
    }

    fn current_page(&self) -> &crate::model::Page {
        &self.site.pages[self.selected_page]
    }

    fn current_page_mut(&mut self) -> Option<&mut crate::model::Page> {
        self.site.pages.get_mut(self.selected_page)
    }

    fn selected_index_for_page(page: &crate::model::Page, selected_node: usize) -> Option<usize> {
        if page.nodes.is_empty() {
            None
        } else {
            Some(selected_node.min(page.nodes.len() - 1))
        }
    }

    fn build_tree_rows(&self) -> Vec<TreeRow> {
        match self.selected_region {
            SelectedRegion::Header => self.build_header_tree_rows(),
            SelectedRegion::Footer => self.build_footer_tree_rows(),
            SelectedRegion::Page => self.build_page_tree_rows(),
        }
    }

    fn build_footer_tree_rows(&self) -> Vec<TreeRow> {
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

    fn build_page_tree_rows(&self) -> Vec<TreeRow> {
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

    fn build_header_tree_rows(&self) -> Vec<TreeRow> {
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

    fn is_header_section_expanded(&self, section_idx: usize) -> bool {
        self.expanded_sections.contains(&(usize::MAX, section_idx))
    }

    fn set_header_section_expanded(&mut self, section_idx: usize, expanded: bool) {
        let key = (usize::MAX, section_idx);
        if expanded {
            self.expanded_sections.insert(key);
        } else {
            self.expanded_sections.remove(&key);
        }
    }

    fn tree_row_label(&self, row: &TreeRow) -> String {
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
                    "    {} {} dd-section ({})",
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
                    "        |- column {} ({}) [{}]",
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
                format!("            - {} {}", comp_i + 1, label)
            }
            TreeRowKind::FooterRoot => {
                format!("1. [FOOTER] dd-footer ({})", self.site.footer.id)
            }
            TreeRowKind::FooterSection { section_idx } => {
                let section_i =
                    (*section_idx).min(self.site.footer.sections.len().saturating_sub(1));
                let section = &self.site.footer.sections[section_i];
                format!("    {} dd-section ({})", section_i + 1, section.id)
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
                    "        |- column {} ({}) [{}]",
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
                format!("            - {} {}", comp_i + 1, label)
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
                    return format!("    |- column {}", column_idx + 1);
                };
                let columns = section_columns_ref(section);
                let col_i = (*column_idx).min(columns.len().saturating_sub(1));
                let col = &columns[col_i];
                format!(
                    "    |- column {} ({}) [{}]",
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
                    return format!("       - component {}", component_idx + 1);
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
                    format!("       - {} {} {}", comp_i + 1, marker, label)
                } else if matches!(component, crate::model::SectionComponent::Alternating(_)) {
                    let marker = if self.is_alternating_items_expanded(*node_idx, col_i, comp_i) {
                        "[-]"
                    } else {
                        "[+]"
                    };
                    format!("       - {} {} {}", comp_i + 1, marker, label)
                } else if matches!(component, crate::model::SectionComponent::Card(_)) {
                    let marker = if self.is_card_items_expanded(*node_idx, col_i, comp_i) {
                        "[-]"
                    } else {
                        "[+]"
                    };
                    format!("       - {} {} {}", comp_i + 1, marker, label)
                } else if matches!(component, crate::model::SectionComponent::Filmstrip(_)) {
                    let marker = if self.is_filmstrip_items_expanded(*node_idx, col_i, comp_i) {
                        "[-]"
                    } else {
                        "[+]"
                    };
                    format!("       - {} {} {}", comp_i + 1, marker, label)
                } else if matches!(component, crate::model::SectionComponent::Milestones(_)) {
                    let marker = if self.is_milestones_items_expanded(*node_idx, col_i, comp_i) {
                        "[-]"
                    } else {
                        "[+]"
                    };
                    format!("       - {} {} {}", comp_i + 1, marker, label)
                } else if matches!(component, crate::model::SectionComponent::Slider(_)) {
                    let marker = if self.is_slider_items_expanded(*node_idx, col_i, comp_i) {
                        "[-]"
                    } else {
                        "[+]"
                    };
                    format!("       - {} {} {}", comp_i + 1, marker, label)
                } else {
                    format!("       - {} {}", comp_i + 1, label)
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
                    return format!("          - item {}", item_idx + 1);
                };
                let columns = section_columns_ref(section);
                let col_i = (*column_idx).min(columns.len().saturating_sub(1));
                let comp_i =
                    (*component_idx).min(columns[col_i].components.len().saturating_sub(1));
                let acc = match &columns[col_i].components[comp_i] {
                    crate::model::SectionComponent::Accordion(a) => a,
                    _ => return format!("          - item {}", item_idx + 1),
                };
                let item_i = (*item_idx).min(acc.items.len().saturating_sub(1));
                let item = &acc.items[item_i];
                format!(
                    "          - {}: {}",
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
                    return format!("          - item {}", item_idx + 1);
                };
                let columns = section_columns_ref(section);
                let col_i = (*column_idx).min(columns.len().saturating_sub(1));
                let comp_i =
                    (*component_idx).min(columns[col_i].components.len().saturating_sub(1));
                let alt = match &columns[col_i].components[comp_i] {
                    crate::model::SectionComponent::Alternating(a) => a,
                    _ => return format!("          - item {}", item_idx + 1),
                };
                let item_i = (*item_idx).min(alt.items.len().saturating_sub(1));
                let item = &alt.items[item_i];
                format!(
                    "          - {}: {}",
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
                    return format!("          - item {}", item_idx + 1);
                };
                let columns = section_columns_ref(section);
                let col_i = (*column_idx).min(columns.len().saturating_sub(1));
                let comp_i =
                    (*component_idx).min(columns[col_i].components.len().saturating_sub(1));
                let card = match &columns[col_i].components[comp_i] {
                    crate::model::SectionComponent::Card(c) => c,
                    _ => return format!("          - item {}", item_idx + 1),
                };
                let item_i = (*item_idx).min(card.items.len().saturating_sub(1));
                let item = &card.items[item_i];
                format!(
                    "          - {}: {}",
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
                    return format!("          - item {}", item_idx + 1);
                };
                let columns = section_columns_ref(section);
                let col_i = (*column_idx).min(columns.len().saturating_sub(1));
                let comp_i =
                    (*component_idx).min(columns[col_i].components.len().saturating_sub(1));
                let filmstrip = match &columns[col_i].components[comp_i] {
                    crate::model::SectionComponent::Filmstrip(f) => f,
                    _ => return format!("          - item {}", item_idx + 1),
                };
                let item_i = (*item_idx).min(filmstrip.items.len().saturating_sub(1));
                let item = &filmstrip.items[item_i];
                format!(
                    "          - {}: {}",
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
                    return format!("          - item {}", item_idx + 1);
                };
                let columns = section_columns_ref(section);
                let col_i = (*column_idx).min(columns.len().saturating_sub(1));
                let comp_i =
                    (*component_idx).min(columns[col_i].components.len().saturating_sub(1));
                let milestones = match &columns[col_i].components[comp_i] {
                    crate::model::SectionComponent::Milestones(m) => m,
                    _ => return format!("          - item {}", item_idx + 1),
                };
                let item_i = (*item_idx).min(milestones.items.len().saturating_sub(1));
                let item = &milestones.items[item_i];
                format!(
                    "          - {}: {}",
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
                    return format!("          - item {}", item_idx + 1);
                };
                let columns = section_columns_ref(section);
                let col_i = (*column_idx).min(columns.len().saturating_sub(1));
                let comp_i =
                    (*component_idx).min(columns[col_i].components.len().saturating_sub(1));
                let slider = match &columns[col_i].components[comp_i] {
                    crate::model::SectionComponent::Slider(s) => s,
                    _ => return format!("          - item {}", item_idx + 1),
                };
                let item_i = (*item_idx).min(slider.items.len().saturating_sub(1));
                let item = &slider.items[item_i];
                format!(
                    "          - {}: {}",
                    item_i + 1,
                    truncate_ascii(&item.child_title, 40)
                )
            }
        }
    }

    fn apply_tree_row_selection(&mut self, row: TreeRow) {
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

    fn sync_tree_row_with_selection(&mut self) {
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

    fn is_section_expanded(&self, node_idx: usize) -> bool {
        !self
            .expanded_sections
            .contains(&(self.selected_page, node_idx))
    }

    fn set_section_expanded(&mut self, node_idx: usize, expanded: bool) {
        if expanded {
            self.expanded_sections
                .remove(&(self.selected_page, node_idx));
        } else {
            self.expanded_sections
                .insert((self.selected_page, node_idx));
        }
    }

    fn is_accordion_items_expanded(
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

    fn set_accordion_items_expanded(
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

    fn is_alternating_items_expanded(
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

    fn set_alternating_items_expanded(
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

    fn is_card_items_expanded(
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

    fn set_card_items_expanded(
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

    fn is_filmstrip_items_expanded(
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

    fn set_filmstrip_items_expanded(
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

    fn is_milestones_items_expanded(
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

    fn set_milestones_items_expanded(
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

    fn is_slider_items_expanded(
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

    fn set_slider_items_expanded(
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

    fn toggle_selected_tree_expanded(&mut self) {
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
                self.status = "Selected row is not a section.".to_string();
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
                self.status = if expanded {
                    "Collapsed accordion items.".to_string()
                } else {
                    "Expanded accordion items.".to_string()
                };
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
                self.status = if expanded {
                    "Collapsed alternating items.".to_string()
                } else {
                    "Expanded alternating items.".to_string()
                };
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
                self.status = if expanded {
                    "Collapsed card items.".to_string()
                } else {
                    "Expanded card items.".to_string()
                };
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
                self.status = if expanded {
                    "Collapsed filmstrip items.".to_string()
                } else {
                    "Expanded filmstrip items.".to_string()
                };
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
                self.status = if expanded {
                    "Collapsed milestones items.".to_string()
                } else {
                    "Expanded milestones items.".to_string()
                };
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
                self.status = if expanded {
                    "Collapsed slider items.".to_string()
                } else {
                    "Expanded slider items.".to_string()
                };
                self.sync_tree_row_with_selection();
                return;
            }
        }
        let node_idx = match row.kind {
            TreeRowKind::HeaderRoot { .. } => {
                self.header_column_expanded = !self.header_column_expanded;
                self.status = if self.header_column_expanded {
                    "Expanded header columns.".to_string()
                } else {
                    "Collapsed header columns.".to_string()
                };
                self.sync_tree_row_with_selection();
                return;
            }
            TreeRowKind::HeaderSection { section_idx } => {
                let expanded = self.is_header_section_expanded(section_idx);
                self.set_header_section_expanded(section_idx, !expanded);
                self.selected_header_section = section_idx;
                self.selected_header_column = 0;
                self.selected_header_component = 0;
                self.status = if expanded {
                    "Collapsed header section.".to_string()
                } else {
                    "Expanded header section.".to_string()
                };
                self.sync_tree_row_with_selection();
                return;
            }
            TreeRowKind::HeaderColumn { .. } | TreeRowKind::HeaderComponent { .. } => {
                self.status = "Press Enter to edit.".to_string();
                return;
            }
            TreeRowKind::FooterRoot
            | TreeRowKind::FooterSection { .. }
            | TreeRowKind::FooterColumn { .. }
            | TreeRowKind::FooterComponent { .. } => {
                self.status = "Press Enter to edit.".to_string();
                return;
            }
            TreeRowKind::PageHead => {
                self.status = "Press Enter to edit page head.".to_string();
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
                self.status = "Selected row is not a section.".to_string();
                return;
            }
        };
        let page = self.current_page();
        let Some(PageNode::Section(_)) = page.nodes.get(node_idx) else {
            self.status = "Selected row is not a section.".to_string();
            return;
        };
        let expanded = self.is_section_expanded(node_idx);
        self.set_section_expanded(node_idx, !expanded);
        self.selected_node = node_idx;
        self.selected_column = 0;
        self.selected_component = 0;
        self.selected_nested_item = 0;
        self.status = if expanded {
            "Collapsed section.".to_string()
        } else {
            "Expanded section.".to_string()
        };
        self.sync_tree_row_with_selection();
    }

    fn handle_enter_on_selected_row(&mut self) {
        let rows = self.build_tree_rows();
        if rows.is_empty() {
            return;
        }
        let row = rows[self.selected_tree_row.min(rows.len() - 1)];
        // New UX path: migrated components route through the unified form editor.
        if self.try_open_form_edit(&row) {
            return;
        }
        match row.kind {
            TreeRowKind::HeaderRoot { .. } => self.open_header_root_edit_modal(),
            TreeRowKind::HeaderSection { .. } => self.begin_edit_selected(),
            TreeRowKind::HeaderColumn { .. } => self.begin_edit_selected_column_width_class(),
            TreeRowKind::HeaderComponent { .. } => self.begin_edit_selected_component_primary(),
            TreeRowKind::FooterRoot => self.open_footer_edit_modal(),
            TreeRowKind::FooterSection { .. } => self.begin_edit_selected(),
            TreeRowKind::FooterColumn { .. } => self.begin_edit_selected_column_width_class(),
            TreeRowKind::FooterComponent { .. } => self.begin_edit_selected_component_primary(),
            TreeRowKind::PageHead => self.open_page_head_edit_modal(),
            TreeRowKind::Section { .. } => self.begin_edit_selected(),
            TreeRowKind::Hero { .. } => self.begin_edit_selected(),
            TreeRowKind::Column { .. } => self.begin_edit_selected_column_width_class(),
            TreeRowKind::Component { .. } => self.begin_edit_selected_component_primary(),
            TreeRowKind::AccordionItem { .. } => {
                if self.set_component_input_mode(InputMode::EditAccordionFirstTitle) {
                    return;
                }
                self.begin_edit_selected_component_primary();
            }
            TreeRowKind::AlternatingItem { .. } => {
                if self.set_component_input_mode(InputMode::EditAlternatingItemTitle) {
                    return;
                }
                self.begin_edit_selected_component_primary();
            }
            TreeRowKind::CardItem { .. } => {
                if self.set_component_input_mode(InputMode::EditCardItemImageUrl) {
                    return;
                }
                self.begin_edit_selected_component_primary();
            }
            TreeRowKind::FilmstripItem { .. } => {
                if self.set_component_input_mode(InputMode::EditFilmstripItemImageUrl) {
                    return;
                }
                self.begin_edit_selected_component_primary();
            }
            TreeRowKind::MilestonesItem { .. } => {
                if self.set_component_input_mode(InputMode::EditMilestonesItemPercentage) {
                    return;
                }
                self.begin_edit_selected_component_primary();
            }
            TreeRowKind::SliderItem { .. } => {
                if self.set_component_input_mode(InputMode::EditSliderItemTitle) {
                    return;
                }
                self.begin_edit_selected_component_primary();
            }
        }
    }

    /// Run `validate_site` on the current site. Open `Modal::ValidationErrors`
    /// if any errors; otherwise set a green status and leave no modal open.
    fn open_validation_modal(&mut self) {
        let root = self.path.as_ref().and_then(|p| p.parent().map(std::path::Path::to_path_buf));
        let errors = crate::validate::validate_site_with_root(&self.site, root.as_deref());
        if errors.is_empty() {
            self.push_toast(ToastLevel::Success, "No validation errors.");
        } else {
            self.modal = Some(Modal::ValidationErrors {
                errors,
                scroll_offset: 0,
            });
        }
    }

    fn open_component_picker(&mut self) {
        self.input_mode = None;
        self.component_picker = Some(ComponentPickerState {
            query: String::new(),
            selected: 0,
        });
        self.status = "Insert picker opened.".to_string();
    }

    /// If the selected tree row points at a migrated section component
    /// (CTA or any Tier A component), open the unified form editor for it
    /// and return true. Otherwise return false so the caller can fall back
    /// to legacy edit flows.
    fn try_open_form_edit(&mut self, row: &TreeRow) -> bool {
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
            self.status = format!("Editing {}.", title);
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
        self.status = format!("Editing {}.", title);
        true
    }

    /// Route Hero / Section tree rows (page, header, or footer scope) to the
    /// unified form editor. Returns `(state, cursor, title)` on match.
    fn try_open_hero_or_section(
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

    fn insert_selected_component_kind(&mut self) {
        match self.component_kind {
            ComponentKind::Hero => self.add_hero(),
            ComponentKind::Section => {
                if self.selected_region == SelectedRegion::Header {
                    self.add_header_section();
                } else {
                    self.add_section();
                }
            }
            _ => {
                if self.selected_region == SelectedRegion::Header {
                    self.add_component_to_header_section();
                } else {
                    self.add_selected_component_to_section();
                }
            }
        }
    }

    fn add_header_section(&mut self) {
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
        self.site.header.sections.push(section);
        self.selected_header_section = self.site.header.sections.len() - 1;
        self.selected_header_column = 0;
        self.selected_header_component = 0;
        self.status = format!(
            "Added dd-section to header at position {}.",
            self.selected_header_section + 1
        );
    }

    fn add_component_to_header_section(&mut self) {
        if self.site.header.sections.is_empty() {
            self.status = "No header section available. Add a section first with '/'.".to_string();
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
        self.site.header.sections[section_idx].columns[col_idx]
            .components
            .push(component);
        self.selected_header_component = self.site.header.sections[section_idx].columns[col_idx]
            .components
            .len()
            - 1;
        self.status = format!(
            "Added {} to header section column '{}'.",
            kind.label(),
            self.site.header.sections[section_idx].columns[col_idx].id
        );
    }

    fn normalize_component_picker_selection(&mut self) {
        let (query, selected) = if let Some(picker) = &self.component_picker {
            (picker.query.clone(), picker.selected)
        } else {
            return;
        };
        let total = self.filtered_component_kinds(&query).len();
        if let Some(picker) = &mut self.component_picker {
            picker.selected = if total == 0 {
                0
            } else {
                selected.min(total - 1)
            };
        }
    }

    fn filtered_component_kinds(&self, query: &str) -> Vec<ComponentKind> {
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

    fn selection_summary(&self) -> String {
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

    fn header_selection_summary(&self) -> String {
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

    fn current_input_mode_label(&self) -> &'static str {
        match self.input_mode {
            Some(InputMode::EditHeroImage) => "hero.image",
            Some(InputMode::EditHeroClass) => "hero.class",
            Some(InputMode::EditHeroAos) => "hero.data_aos",
            Some(InputMode::EditHeroCustomCss) => "hero.custom_css",
            Some(InputMode::EditHeroTitle) => "hero.title",
            Some(InputMode::EditHeroSubtitle) => "hero.subtitle",
            Some(InputMode::EditHeroCopy) => "hero.copy",
            Some(InputMode::EditHeroCtaText) => "hero.link_1.text",
            Some(InputMode::EditHeroCtaLink) => "hero.link_1.url",
            Some(InputMode::EditHeroCtaTarget) => "hero.link_1.target",
            Some(InputMode::EditHeroCtaText2) => "hero.link_2.text",
            Some(InputMode::EditHeroCtaLink2) => "hero.link_2.url",
            Some(InputMode::EditHeroCtaTarget2) => "hero.link_2.target",
            Some(InputMode::EditSectionId) => "section.id",
            Some(InputMode::EditSectionTitle) => "section.title",
            Some(InputMode::EditSectionClass) => "section.class",
            Some(InputMode::EditColumnId) => "section.column.id",
            Some(InputMode::EditColumnWidthClass) => "section.column.width_class",
            Some(InputMode::EditAlternatingType) => "dd-alternating.type",
            Some(InputMode::EditAlternatingClass) => "dd-alternating.class",
            Some(InputMode::EditAlternatingDataAos) => "dd-alternating.data_aos",
            Some(InputMode::EditAlternatingItemImage) => "dd-alternating.active.image",
            Some(InputMode::EditAlternatingItemImageAlt) => "dd-alternating.active.image_alt",
            Some(InputMode::EditAlternatingItemTitle) => "dd-alternating.active.title",
            Some(InputMode::EditAlternatingItemCopy) => "dd-alternating.active.copy",
            Some(InputMode::EditBannerClass) => "dd-banner.class",
            Some(InputMode::EditBannerDataAos) => "dd-banner.data_aos",
            Some(InputMode::EditBannerImageUrl) => "dd-parent_image_url",
            Some(InputMode::EditBannerImageAlt) => "dd-parent_image_alt",
            Some(InputMode::EditCtaClass) => "dd-cta.class",
            Some(InputMode::EditCtaImageUrl) => "dd-parent_image_url",
            Some(InputMode::EditCtaImageAlt) => "dd-parent_image_alt",
            Some(InputMode::EditCtaDataAos) => "dd-cta.data_aos",
            Some(InputMode::EditCtaTitle) => "dd-parent_title",
            Some(InputMode::EditCtaSubtitle) => "dd-parent_subtitle",
            Some(InputMode::EditCtaCopy) => "dd-parent_copy",
            Some(InputMode::EditCtaLinkUrl) => "dd-parent_link_url",
            Some(InputMode::EditCtaLinkTarget) => "dd-parent_link_target",
            Some(InputMode::EditCtaLinkLabel) => "dd-parent_link_label",
            Some(InputMode::EditFilmstripType) => "dd-filmstrip.type",
            Some(InputMode::EditFilmstripDataAos) => "dd-filmstrip.data_aos",
            Some(InputMode::EditFilmstripItemImageUrl) => "dd-filmstrip.active.image_url",
            Some(InputMode::EditFilmstripItemImageAlt) => "dd-filmstrip.active.image_alt",
            Some(InputMode::EditFilmstripItemTitle) => "dd-filmstrip.active.title",
            Some(InputMode::EditMilestonesDataAos) => "dd-milestones.parent_data_aos",
            Some(InputMode::EditMilestonesWidth) => "dd-milestones.parent_width",
            Some(InputMode::EditMilestonesItemPercentage) => {
                "dd-milestones.active.child_percentage"
            }
            Some(InputMode::EditMilestonesItemTitle) => "dd-milestones.active.child_title",
            Some(InputMode::EditMilestonesItemSubtitle) => "dd-milestones.active.child_subtitle",
            Some(InputMode::EditMilestonesItemCopy) => "dd-milestones.active.child_copy",
            Some(InputMode::EditMilestonesItemLinkUrl) => "dd-milestones.active.child_link_url",
            Some(InputMode::EditMilestonesItemLinkTarget) => {
                "dd-milestones.active.child_link_target"
            }
            Some(InputMode::EditMilestonesItemLinkLabel) => "dd-milestones.active.child_link_label",
            Some(InputMode::EditModalTitle) => "dd-modal.parent_title",
            Some(InputMode::EditModalCopy) => "dd-modal.parent_copy",
            Some(InputMode::EditSliderTitle) => "dd-slider.parent_title",
            Some(InputMode::EditSliderItemTitle) => "dd-slider.active.child_title",
            Some(InputMode::EditSliderItemCopy) => "dd-slider.active.child_copy",
            Some(InputMode::EditSliderItemLinkUrl) => "dd-slider.active.child_link_url",
            Some(InputMode::EditSliderItemLinkTarget) => "dd-slider.active.child_link_target",
            Some(InputMode::EditSliderItemLinkLabel) => "dd-slider.active.child_link_label",
            Some(InputMode::EditSliderItemImageUrl) => "dd-slider.active.child_image_url",
            Some(InputMode::EditSliderItemImageAlt) => "dd-slider.active.child_image_alt",
            Some(InputMode::EditBlockquoteDataAos) => "dd-blockquote.data_aos",
            Some(InputMode::EditBlockquoteImageUrl) => "parent_image_url",
            Some(InputMode::EditBlockquoteImageAlt) => "parent_image_alt",
            Some(InputMode::EditBlockquotePersonsName) => "parent_name",
            Some(InputMode::EditBlockquotePersonsTitle) => "parent_role",
            Some(InputMode::EditBlockquoteCopy) => "parent_copy",
            Some(InputMode::EditAlertType) => "dd-alert.type",
            Some(InputMode::EditAlertClass) => "dd-alert.class",
            Some(InputMode::EditAlertDataAos) => "dd-alert.data_aos",
            Some(InputMode::EditAlertTitle) => "dd-alert.parent_title",
            Some(InputMode::EditAlertCopy) => "dd-alert.parent_copy",
            Some(InputMode::EditCardType) => "parent_type",
            Some(InputMode::EditCardDataAos) => "parent_data_aos",
            Some(InputMode::EditCardWidth) => "parent_width",
            Some(InputMode::EditCardItemImageUrl) => "dd-card.active.child_image_url",
            Some(InputMode::EditCardItemImageAlt) => "dd-card.active.child_image_alt",
            Some(InputMode::EditCardItemTitle) => "dd-card.active.child_title",
            Some(InputMode::EditCardItemSubtitle) => "dd-card.active.child_subtitle",
            Some(InputMode::EditCardItemCopy) => "dd-card.active.child_copy",
            Some(InputMode::EditCardItemLinkUrl) => "dd-card.active.child_link_url",
            Some(InputMode::EditCardItemLinkTarget) => "dd-card.active.child_link_target",
            Some(InputMode::EditCardItemLinkLabel) => "dd-card.active.child_link_label",
            Some(InputMode::EditAccordionType) => "dd-accordion.type",
            Some(InputMode::EditAccordionClass) => "dd-accordion.class",
            Some(InputMode::EditAccordionAos) => "dd-accordion.data_aos",
            Some(InputMode::EditAccordionGroupName) => "dd-accordion.parent_group_name",
            Some(InputMode::EditAccordionFirstTitle) => "dd-accordion.active.title",
            Some(InputMode::EditAccordionFirstContent) => "dd-accordion.active.content",
            Some(InputMode::EditHeaderId) => "header.id",
            Some(InputMode::EditHeaderClass) => "header.class",
            Some(InputMode::EditHeaderCustomCss) => "header.custom_css",
            Some(InputMode::EditHeaderColumnId) => "header.column.id",
            Some(InputMode::EditHeaderColumnWidthClass) => "header.column.width_class",
            Some(InputMode::EditHeaderPlaceholderContent) => "header.placeholder.content",
            Some(InputMode::EditImageUrl) => "dd-image.parent_image_url",
            Some(InputMode::EditImageAlt) => "dd-image.parent_image_alt",
            Some(InputMode::EditImageLinkUrl) => "dd-image.parent_link_url",
            Some(InputMode::EditImageLinkTarget) => "dd-image.parent_link_target",
            Some(InputMode::EditImageDataAos) => "dd-image.parent_data_aos",
            Some(InputMode::EditRichTextClass) => "dd-rich_text.parent_class",
            Some(InputMode::EditRichTextCopy) => "dd-rich_text.parent_copy",
            Some(InputMode::EditRichTextDataAos) => "dd-rich_text.parent_data_aos",
            Some(InputMode::EditNavigationType) => "dd-navigation.parent_type",
            Some(InputMode::EditNavigationClass) => "dd-navigation.parent_class",
            Some(InputMode::EditNavigationWidth) => "dd-navigation.parent_width",
            Some(InputMode::EditNavigationDataAos) => "dd-navigation.parent_data_aos",
            Some(InputMode::EditNavigationItemKind) => "dd-navigation.item.child_kind",
            Some(InputMode::EditNavigationItemLabel) => "dd-navigation.item.child_link_label",
            Some(InputMode::EditNavigationItemUrl) => "dd-navigation.item.child_link_url",
            Some(InputMode::EditNavigationItemTarget) => "dd-navigation.item.child_link_target",
            Some(InputMode::EditNavigationItemCss) => "dd-navigation.item.child_link_css",
            Some(InputMode::EditHeaderSearchWidth) => "dd-header-search.parent_width",
            Some(InputMode::EditHeaderSearchDataAos) => "dd-header-search.parent_data_aos",
            Some(InputMode::EditHeaderMenuWidth) => "dd-header-menu.parent_width",
            Some(InputMode::EditHeaderMenuDataAos) => "dd-header-menu.parent_data_aos",
            Some(InputMode::EditFooterId) => "footer.id",
            Some(InputMode::EditFooterCustomCss) => "footer.custom_css",
            Some(InputMode::EditHeadTitle) => "head.title",
            Some(InputMode::EditHeadMetaDescription) => "head.meta_description",
            Some(InputMode::EditHeadCanonicalUrl) => "head.canonical_url",
            Some(InputMode::EditHeadRobots) => "head.robots",
            Some(InputMode::EditHeadSchemaType) => "head.schema_type",
            Some(InputMode::EditHeadOgTitle) => "head.og_title",
            Some(InputMode::EditHeadOgDescription) => "head.og_description",
            Some(InputMode::EditHeadOgImage) => "head.og_image",
            None => "field",
        }
    }

    fn current_modal_fields(&self) -> String {
        let page = self.current_page();
        if page.nodes.is_empty() {
            return "(none)".to_string();
        }
        let ni = self.selected_node.min(page.nodes.len().saturating_sub(1));
        match &page.nodes[ni] {
            PageNode::Hero(v) => format!(
                "- hero.image: {}\n- hero.class: {}\n- hero.data_aos: {}\n- hero.custom_css: {}\n- hero.title: {}\n- hero.subtitle: {}\n- hero.copy: {}\n- hero.link_1.text: {}\n- hero.link_1.url: {}\n- hero.link_1.target: {}\n- hero.link_2.text: {}\n- hero.link_2.url: {}\n- hero.link_2.target: {}",
                v.parent_image_url,
                hero_image_class_to_str(
                    v.parent_class
                        .unwrap_or(crate::model::HeroImageClass::FullFull)
                ),
                parent_data_aos_to_str(v.parent_data_aos.unwrap_or(crate::model::HeroAos::FadeIn)),
                v.parent_custom_css.as_deref().unwrap_or("(none)"),
                v.parent_title,
                v.parent_subtitle,
                v.parent_copy.as_deref().unwrap_or("(none)"),
                v.link_1_label.as_deref().unwrap_or("(none)"),
                v.link_1_url.as_deref().unwrap_or("(none)"),
                link_1_target_to_str(v.link_1_target.unwrap_or(crate::model::CtaTarget::SelfTarget)),
                v.link_2_label.as_deref().unwrap_or("(none)"),
                v.link_2_url.as_deref().unwrap_or("(none)"),
                link_1_target_to_str(
                    v.link_2_target
                        .unwrap_or(crate::model::CtaTarget::SelfTarget)
                )
            ),
            PageNode::Section(section) => {
                let rows = self.build_page_tree_rows();
                let row_kind = rows
                    .get(self.selected_tree_row.min(rows.len().saturating_sub(1)))
                    .map(|row| row.kind);
                match row_kind {
                    Some(TreeRowKind::Column { .. }) => {
                        let columns = section_columns_ref(section);
                        if let Some(col) =
                            columns.get(self.selected_column.min(columns.len().saturating_sub(1)))
                        {
                            vec![
                                format!("- column.id: {}", col.id),
                                format!("- column.width_class: {}", col.width_class),
                            ]
                            .join("\n")
                        } else {
                            "(none)".to_string()
                        }
                    }
                    Some(TreeRowKind::Component { .. })
                    | Some(TreeRowKind::AccordionItem { .. })
                    | Some(TreeRowKind::AlternatingItem { .. })
                    | Some(TreeRowKind::CardItem { .. })
                    | Some(TreeRowKind::FilmstripItem { .. })
                    | Some(TreeRowKind::MilestonesItem { .. })
                    | Some(TreeRowKind::SliderItem { .. }) => {
                        let columns = section_columns_ref(section);
                        if let Some(col) =
                            columns.get(self.selected_column.min(columns.len().saturating_sub(1)))
                        {
                            if let Some(component) = col.components.get(
                                self.selected_component
                                    .min(col.components.len().saturating_sub(1)),
                            ) {
                                if let crate::model::SectionComponent::Card(card) = component {
                                    match self.input_mode {
                                        Some(InputMode::EditCardType)
                                        | Some(InputMode::EditCardDataAos)
                                        | Some(InputMode::EditCardWidth) => vec![
                                            format!(
                                                "- parent_type: {}",
                                                card_type_to_str(card.parent_type)
                                            ),
                                            format!(
                                                "- parent_data_aos: {}",
                                                parent_data_aos_to_str(card.parent_data_aos)
                                            ),
                                            format!("- parent_width: {}", card.parent_width),
                                        ]
                                        .join("\n"),
                                        Some(InputMode::EditCardItemImageUrl)
                                        | Some(InputMode::EditCardItemImageAlt)
                                        | Some(InputMode::EditCardItemTitle)
                                        | Some(InputMode::EditCardItemSubtitle)
                                        | Some(InputMode::EditCardItemCopy)
                                        | Some(InputMode::EditCardItemLinkUrl)
                                        | Some(InputMode::EditCardItemLinkTarget)
                                        | Some(InputMode::EditCardItemLinkLabel) => {
                                            let item = nested_index(
                                                card.items.len(),
                                                self.selected_nested_item,
                                            )
                                            .and_then(|i| card.items.get(i));
                                            vec![
                                                format!(
                                                    "- child_image_url: {}",
                                                    item.map(|i| i.child_image_url.as_str())
                                                        .unwrap_or("(none)")
                                                ),
                                                format!(
                                                    "- child_image_alt: {}",
                                                    item.map(|i| i.child_image_alt.as_str())
                                                        .unwrap_or("(none)")
                                                ),
                                                format!(
                                                    "- child_title: {}",
                                                    item.map(|i| i.child_title.as_str())
                                                        .unwrap_or("(none)")
                                                ),
                                                format!(
                                                    "- child_subtitle: {}",
                                                    item.map(|i| i.child_subtitle.as_str())
                                                        .unwrap_or("(none)")
                                                ),
                                                format!(
                                                    "- child_copy: {}",
                                                    item.map(|i| i.child_copy.as_str())
                                                        .unwrap_or("(none)")
                                                ),
                                                format!(
                                                    "- child_link_url: {}",
                                                    item.and_then(|i| i.child_link_url.as_deref())
                                                        .unwrap_or("(none)")
                                                ),
                                                format!(
                                                    "- child_link_target: {}",
                                                    item.and_then(|i| i.child_link_target)
                                                        .map(child_link_target_to_str)
                                                        .unwrap_or("_self")
                                                ),
                                                format!(
                                                    "- child_link_label: {}",
                                                    item.and_then(|i| i.child_link_label.as_deref())
                                                        .unwrap_or("(none)")
                                                ),
                                            ]
                                            .join("\n")
                                        }
                                        _ => component_form(component, self.selected_nested_item),
                                    }
                                } else if let crate::model::SectionComponent::Filmstrip(filmstrip) =
                                    component
                                {
                                    match self.input_mode {
                                        Some(InputMode::EditFilmstripType)
                                        | Some(InputMode::EditFilmstripDataAos) => vec![
                                            format!(
                                                "- parent_type: {}",
                                                filmstrip_type_to_str(filmstrip.parent_type)
                                            ),
                                            format!(
                                                "- parent_data_aos: {}",
                                                parent_data_aos_to_str(filmstrip.parent_data_aos)
                                            ),
                                        ]
                                        .join("\n"),
                                        Some(InputMode::EditFilmstripItemImageUrl)
                                        | Some(InputMode::EditFilmstripItemImageAlt)
                                        | Some(InputMode::EditFilmstripItemTitle) => {
                                            let item = nested_index(
                                                filmstrip.items.len(),
                                                self.selected_nested_item,
                                            )
                                            .and_then(|i| filmstrip.items.get(i));
                                            vec![
                                                format!(
                                                    "- child_image_url: {}",
                                                    item.map(|i| i.child_image_url.as_str())
                                                        .unwrap_or("(none)")
                                                ),
                                                format!(
                                                    "- child_image_alt: {}",
                                                    item.map(|i| i.child_image_alt.as_str())
                                                        .unwrap_or("(none)")
                                                ),
                                                format!(
                                                    "- child_title: {}",
                                                    item.map(|i| i.child_title.as_str())
                                                        .unwrap_or("(none)")
                                                ),
                                            ]
                                            .join("\n")
                                        }
                                        _ => component_form(component, self.selected_nested_item),
                                    }
                                } else if let crate::model::SectionComponent::Milestones(
                                    milestones,
                                ) = component
                                {
                                    match self.input_mode {
                                        Some(InputMode::EditMilestonesDataAos)
                                        | Some(InputMode::EditMilestonesWidth) => vec![
                                            format!(
                                                "- parent_data_aos: {}",
                                                parent_data_aos_to_str(milestones.parent_data_aos)
                                            ),
                                            format!("- parent_width: {}", milestones.parent_width),
                                        ]
                                        .join("\n"),
                                        Some(InputMode::EditMilestonesItemPercentage)
                                        | Some(InputMode::EditMilestonesItemTitle)
                                        | Some(InputMode::EditMilestonesItemSubtitle)
                                        | Some(InputMode::EditMilestonesItemCopy)
                                        | Some(InputMode::EditMilestonesItemLinkUrl)
                                        | Some(InputMode::EditMilestonesItemLinkTarget)
                                        | Some(InputMode::EditMilestonesItemLinkLabel) => {
                                            let item = nested_index(
                                                milestones.items.len(),
                                                self.selected_nested_item,
                                            )
                                            .and_then(|i| milestones.items.get(i));
                                            vec![
                                                format!(
                                                    "- child_percentage: {}",
                                                    item.map(|i| i.child_percentage.as_str())
                                                        .unwrap_or("(none)")
                                                ),
                                                format!(
                                                    "- child_title: {}",
                                                    item.map(|i| i.child_title.as_str())
                                                        .unwrap_or("(none)")
                                                ),
                                                format!(
                                                    "- child_subtitle: {}",
                                                    item.map(|i| i.child_subtitle.as_str())
                                                        .unwrap_or("(none)")
                                                ),
                                                format!(
                                                    "- child_copy: {}",
                                                    item.map(|i| i.child_copy.as_str())
                                                        .unwrap_or("(none)")
                                                ),
                                                format!(
                                                    "- child_link_url: {}",
                                                    item.and_then(|i| i.child_link_url.as_deref())
                                                        .unwrap_or("(none)")
                                                ),
                                                format!(
                                                    "- child_link_target: {}",
                                                    item.and_then(|i| i.child_link_target)
                                                        .map(child_link_target_to_str)
                                                        .unwrap_or("_self")
                                                ),
                                                format!(
                                                    "- child_link_label: {}",
                                                    item.and_then(|i| i
                                                        .child_link_label
                                                        .as_deref())
                                                        .unwrap_or("(none)")
                                                ),
                                            ]
                                            .join("\n")
                                        }
                                        _ => component_form(component, self.selected_nested_item),
                                    }
                                } else if let crate::model::SectionComponent::Slider(slider) =
                                    component
                                {
                                    match self.input_mode {
                                        Some(InputMode::EditSliderTitle) => {
                                            vec![format!("- parent_title: {}", slider.parent_title)]
                                                .join("\n")
                                        }
                                        Some(InputMode::EditSliderItemTitle)
                                        | Some(InputMode::EditSliderItemCopy)
                                        | Some(InputMode::EditSliderItemLinkUrl)
                                        | Some(InputMode::EditSliderItemLinkTarget)
                                        | Some(InputMode::EditSliderItemLinkLabel)
                                        | Some(InputMode::EditSliderItemImageUrl)
                                        | Some(InputMode::EditSliderItemImageAlt) => {
                                            let item = nested_index(
                                                slider.items.len(),
                                                self.selected_nested_item,
                                            )
                                            .and_then(|i| slider.items.get(i));
                                            vec![
                                                format!(
                                                    "- child_title: {}",
                                                    item.map(|i| i.child_title.as_str())
                                                        .unwrap_or("(none)")
                                                ),
                                                format!(
                                                    "- child_copy: {}",
                                                    item.map(|i| i.child_copy.as_str())
                                                        .unwrap_or("(none)")
                                                ),
                                                format!(
                                                    "- child_link_url: {}",
                                                    item.and_then(|i| i.child_link_url.as_deref())
                                                        .unwrap_or("(none)")
                                                ),
                                                format!(
                                                    "- child_link_target: {}",
                                                    item.and_then(|i| i.child_link_target)
                                                        .map(child_link_target_to_str)
                                                        .unwrap_or("_self")
                                                ),
                                                format!(
                                                    "- child_link_label: {}",
                                                    item.and_then(|i| i
                                                        .child_link_label
                                                        .as_deref())
                                                        .unwrap_or("(none)")
                                                ),
                                                format!(
                                                    "- child_image_url: {}",
                                                    item.map(|i| i.child_image_url.as_str())
                                                        .unwrap_or("(none)")
                                                ),
                                                format!(
                                                    "- child_image_alt: {}",
                                                    item.map(|i| i.child_image_alt.as_str())
                                                        .unwrap_or("(none)")
                                                ),
                                            ]
                                            .join("\n")
                                        }
                                        _ => component_form(component, self.selected_nested_item),
                                    }
                                } else if let crate::model::SectionComponent::Alternating(alt) =
                                    component
                                {
                                    match self.input_mode {
                                        Some(InputMode::EditAlternatingType)
                                        | Some(InputMode::EditAlternatingClass)
                                        | Some(InputMode::EditAlternatingDataAos) => vec![
                                            format!(
                                                "- parent_type: {}",
                                                alternating_type_to_str(alt.parent_type)
                                            ),
                                            format!(
                                                "- alternating.class: {}",
                                                alt.parent_class
                                            ),
                                            format!(
                                                "- alternating.data_aos: {}",
                                                parent_data_aos_to_str(alt.parent_data_aos)
                                            ),
                                        ]
                                        .join("\n"),
                                        Some(InputMode::EditAlternatingItemImage)
                                        | Some(InputMode::EditAlternatingItemImageAlt)
                                        | Some(InputMode::EditAlternatingItemTitle)
                                        | Some(InputMode::EditAlternatingItemCopy) => {
                                            let image = nested_index(
                                                alt.items.len(),
                                                self.selected_nested_item,
                                            )
                                            .and_then(|i| alt.items.get(i))
                                            .map(|i| i.child_image_url.as_str())
                                            .unwrap_or("(none)");
                                            let image_alt = nested_index(
                                                alt.items.len(),
                                                self.selected_nested_item,
                                            )
                                            .and_then(|i| alt.items.get(i))
                                            .map(|i| i.child_image_alt.as_str())
                                            .unwrap_or("(none)");
                                            let title = nested_index(
                                                alt.items.len(),
                                                self.selected_nested_item,
                                            )
                                            .and_then(|i| alt.items.get(i))
                                            .map(|i| i.child_title.as_str())
                                            .unwrap_or("(none)");
                                            let copy = nested_index(
                                                alt.items.len(),
                                                self.selected_nested_item,
                                            )
                                            .and_then(|i| alt.items.get(i))
                                            .map(|i| i.child_copy.as_str())
                                            .unwrap_or("(none)");
                                            vec![
                                                format!("- alternating_image: {}", image),
                                                format!("- alternating_image_alt: {}", image_alt),
                                                format!("- alternating_title: {}", title),
                                                format!("- alternating_copy: {}", copy),
                                            ]
                                            .join("\n")
                                        }
                                        _ => component_form(component, self.selected_nested_item),
                                    }
                                } else if let crate::model::SectionComponent::Accordion(acc) =
                                    component
                                {
                                    match self.input_mode {
                                        Some(InputMode::EditAccordionType)
                                        | Some(InputMode::EditAccordionClass)
                                        | Some(InputMode::EditAccordionAos)
                                        | Some(InputMode::EditAccordionGroupName) => vec![
                                            format!(
                                                "- parent_type: {}",
                                                accordion_type_to_str(acc.parent_type)
                                            ),
                                            format!(
                                                "- accordion.class: {}",
                                                accordion_class_to_str(acc.parent_class)
                                            ),
                                            format!(
                                                "- accordion.data_aos: {}",
                                                parent_data_aos_to_str(acc.parent_data_aos)
                                            ),
                                            format!("- accordion.parent_group_name: {}", acc.parent_group_name),
                                        ]
                                        .join("\n"),
                                        Some(InputMode::EditAccordionFirstTitle)
                                        | Some(InputMode::EditAccordionFirstContent) => {
                                            let title = nested_index(
                                                acc.items.len(),
                                                self.selected_nested_item,
                                            )
                                            .and_then(|i| acc.items.get(i))
                                            .map(|i| i.child_title.as_str())
                                            .unwrap_or("(none)");
                                            let content = nested_index(
                                                acc.items.len(),
                                                self.selected_nested_item,
                                            )
                                            .and_then(|i| acc.items.get(i))
                                            .map(|i| i.child_copy.as_str())
                                            .unwrap_or("(none)");
                                            vec![
                                                format!("- accordion_title: {}", title),
                                                format!("- accordion_copy: {}", content),
                                            ]
                                            .join("\n")
                                        }
                                        _ => component_form(component, self.selected_nested_item),
                                    }
                                } else {
                                    component_form(component, self.selected_nested_item)
                                }
                            } else {
                                "(none)".to_string()
                            }
                        } else {
                            "(none)".to_string()
                        }
                    }
                    _ => vec![
                        format!("- section.id: {}", section.id),
                        format!(
                            "- section.title: {}",
                            section.section_title.as_deref().unwrap_or("(none)")
                        ),
                        format!(
                            "- section.class: {}",
                            section_class_to_str(
                                section
                                    .section_class
                                    .unwrap_or(crate::model::SectionClass::FullContained)
                            )
                        ),
                    ]
                    .join("\n"),
                }
            }
        }
    }

    fn set_component_input_mode(&mut self, mode: InputMode) -> bool {
        let Some(value) = self.value_for_component_mode(mode) else {
            return false;
        };
        self.input_mode = Some(mode);
        self.input_buffer = value;
        self.clamp_multiline_input_if_needed();
        self.input_cursor = self.input_buffer.chars().count();
        self.ensure_multiline_cursor_visible();
        self.status = match mode {
            InputMode::EditHeroImage => {
                "Editing hero image URL. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroClass => {
                "Editing hero default class. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroAos => {
                "Editing hero data-aos option. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroCustomCss => {
                "Editing hero custom CSS classes. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroTitle => {
                "Editing hero title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroSubtitle => {
                "Editing hero subtitle. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroCopy => {
                "Editing hero copy. Enter: newline, Ctrl+S: save, esc: cancel.".to_string()
            }
            InputMode::EditHeroCtaText => {
                "Editing hero primary link text. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroCtaLink => {
                "Editing hero primary link URL. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroCtaTarget => {
                "Editing hero primary link target. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroCtaText2 => {
                "Editing hero secondary link text. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroCtaLink2 => {
                "Editing hero secondary link URL. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditHeroCtaTarget2 => {
                "Editing hero secondary link target. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditSectionId => {
                "Editing section id. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditSectionTitle => {
                "Editing section title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditSectionClass => {
                "Editing section class. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAlternatingType => {
                "Editing dd-alternating type. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAlternatingClass => {
                "Editing dd-alternating class. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAlternatingDataAos => {
                "Editing dd-alternating data-aos. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAlternatingItemImage => {
                "Editing dd-alternating item image. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAlternatingItemImageAlt => {
                "Editing dd-alternating item image alt. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAlternatingItemTitle => {
                "Editing dd-alternating item title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAlternatingItemCopy => {
                "Editing dd-alternating item copy. Enter: newline, Ctrl+S: save, esc: cancel."
                    .to_string()
            }
            InputMode::EditAlertType => {
                "Editing dd-alert type. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAlertClass => {
                "Editing dd-alert class. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAlertDataAos => {
                "Editing dd-alert data-aos. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAlertTitle => {
                "Editing dd-alert title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAlertCopy => {
                "Editing dd-alert copy. Enter: newline, Ctrl+S: save, esc: cancel.".to_string()
            }
            InputMode::EditBannerClass => {
                "Editing dd-banner class. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditBannerDataAos => {
                "Editing dd-banner data-aos. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditBannerImageUrl => {
                "Editing dd-banner image URL. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditBannerImageAlt => {
                "Editing dd-banner image alt text. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCtaClass => {
                "Editing dd-cta class. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCtaImageUrl => {
                "Editing dd-cta image URL. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCtaImageAlt => {
                "Editing dd-cta image alt text. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCtaDataAos => {
                "Editing dd-cta data-aos. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCtaTitle => {
                "Editing dd-cta title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCtaSubtitle => {
                "Editing dd-cta subtitle. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCtaCopy => {
                "Editing dd-cta copy. Enter: newline, Ctrl+S: save, esc: cancel.".to_string()
            }
            InputMode::EditCtaLinkUrl => {
                "Editing dd-cta link URL. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCtaLinkTarget => {
                "Editing dd-cta link target. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCtaLinkLabel => {
                "Editing dd-cta link label. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditFilmstripType => {
                "Editing dd-filmstrip type. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditFilmstripDataAos => {
                "Editing dd-filmstrip data-aos. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditFilmstripItemImageUrl => {
                "Editing dd-filmstrip item image URL. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditFilmstripItemImageAlt => {
                "Editing dd-filmstrip item image alt text. Enter to save, esc to cancel."
                    .to_string()
            }
            InputMode::EditFilmstripItemTitle => {
                "Editing dd-filmstrip item title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditMilestonesDataAos => {
                "Editing dd-milestones parent_data_aos. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditMilestonesWidth => {
                "Editing dd-milestones parent_width. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditMilestonesItemPercentage => {
                "Editing dd-milestones child_percentage. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditMilestonesItemTitle => {
                "Editing dd-milestones child_title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditMilestonesItemSubtitle => {
                "Editing dd-milestones child_subtitle. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditMilestonesItemCopy => {
                "Editing dd-milestones child_copy. Enter: newline, Ctrl+S: save, esc: cancel."
                    .to_string()
            }
            InputMode::EditMilestonesItemLinkUrl => {
                "Editing dd-milestones child_link_url. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditMilestonesItemLinkTarget => {
                "Editing dd-milestones child_link_target. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditMilestonesItemLinkLabel => {
                "Editing dd-milestones child_link_label. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditModalTitle => {
                "Editing dd-modal parent_title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditModalCopy => {
                "Editing dd-modal parent_copy. Enter: newline, Ctrl+S: save, esc: cancel."
                    .to_string()
            }
            InputMode::EditSliderTitle => {
                "Editing dd-slider parent_title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditSliderItemTitle => {
                "Editing dd-slider item child_title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditSliderItemCopy => {
                "Editing dd-slider item child_copy. Enter: newline, Ctrl+S: save, esc: cancel."
                    .to_string()
            }
            InputMode::EditSliderItemLinkUrl => {
                "Editing dd-slider item child_link_url. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditSliderItemLinkTarget => {
                "Editing dd-slider item child_link_target. Enter to save, esc to cancel."
                    .to_string()
            }
            InputMode::EditSliderItemLinkLabel => {
                "Editing dd-slider item child_link_label. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditSliderItemImageUrl => {
                "Editing dd-slider item child_image_url. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditSliderItemImageAlt => {
                "Editing dd-slider item child_image_alt. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditBlockquoteDataAos => {
                "Editing dd-blockquote data-aos. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditBlockquoteImageUrl => {
                "Editing dd-blockquote image URL. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditBlockquoteImageAlt => {
                "Editing dd-blockquote image alt text. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditBlockquotePersonsName => {
                "Editing dd-blockquote person name. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditBlockquotePersonsTitle => {
                "Editing dd-blockquote person title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditBlockquoteCopy => {
                "Editing dd-blockquote copy. Enter: newline, Ctrl+S: save, esc: cancel.".to_string()
            }
            InputMode::EditCardType => {
                "Editing dd-card type. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCardDataAos => {
                "Editing dd-card data-aos. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCardWidth => {
                "Editing dd-card width classes. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCardItemImageUrl => {
                "Editing dd-card item image URL. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCardItemImageAlt => {
                "Editing dd-card item image alt. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCardItemTitle => {
                "Editing dd-card item title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCardItemSubtitle => {
                "Editing dd-card item subtitle. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCardItemCopy => {
                "Editing dd-card item copy. Enter: newline, Ctrl+S: save, esc: cancel.".to_string()
            }
            InputMode::EditCardItemLinkUrl => {
                "Editing dd-card item link URL. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCardItemLinkTarget => {
                "Editing dd-card item link target. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditCardItemLinkLabel => {
                "Editing dd-card item link label. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAccordionType => {
                "Editing dd-accordion type. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAccordionClass => {
                "Editing dd-accordion class. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAccordionAos => {
                "Editing dd-accordion data-aos. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAccordionGroupName => {
                "Editing dd-accordion group name. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAccordionFirstTitle => {
                "Editing dd-accordion item title. Enter to save, esc to cancel.".to_string()
            }
            InputMode::EditAccordionFirstContent => {
                "Editing dd-accordion item content. Enter: newline, Ctrl+S: save, esc: cancel."
                    .to_string()
            }
            _ => "Editing field. Enter to save, esc to cancel.".to_string(),
        };
        true
    }

    fn value_for_component_mode(&self, mode: InputMode) -> Option<String> {
        let page = self.current_page();
        if page.nodes.is_empty() {
            return None;
        }
        let node_idx = self.selected_node.min(page.nodes.len() - 1);
        match &page.nodes[node_idx] {
            PageNode::Hero(hero) => match mode {
                InputMode::EditHeroImage => Some(hero.parent_image_url.clone()),
                InputMode::EditHeroClass => Some(
                    hero_image_class_to_str(
                        hero.parent_class
                            .unwrap_or(crate::model::HeroImageClass::FullFull),
                    )
                    .to_string(),
                ),
                InputMode::EditHeroAos => Some(
                    parent_data_aos_to_str(hero.parent_data_aos.unwrap_or(crate::model::HeroAos::FadeIn))
                        .to_string(),
                ),
                InputMode::EditHeroCustomCss => Some(hero.parent_custom_css.clone().unwrap_or_default()),
                InputMode::EditHeroTitle => Some(hero.parent_title.clone()),
                InputMode::EditHeroSubtitle => Some(hero.parent_subtitle.clone()),
                InputMode::EditHeroCopy => Some(hero.parent_copy.clone().unwrap_or_default()),
                InputMode::EditHeroCtaText => Some(hero.link_1_label.clone().unwrap_or_default()),
                InputMode::EditHeroCtaLink => Some(hero.link_1_url.clone().unwrap_or_default()),
                InputMode::EditHeroCtaTarget => Some(
                    link_1_target_to_str(
                        hero.link_1_target
                            .unwrap_or(crate::model::CtaTarget::SelfTarget),
                    )
                    .to_string(),
                ),
                InputMode::EditHeroCtaText2 => Some(hero.link_2_label.clone().unwrap_or_default()),
                InputMode::EditHeroCtaLink2 => Some(hero.link_2_url.clone().unwrap_or_default()),
                InputMode::EditHeroCtaTarget2 => Some(
                    link_1_target_to_str(
                        hero.link_2_target
                            .unwrap_or(crate::model::CtaTarget::SelfTarget),
                    )
                    .to_string(),
                ),
                _ => None,
            },
            PageNode::Section(section) => match mode {
                InputMode::EditSectionId => Some(section.id.clone()),
                InputMode::EditSectionTitle => {
                    Some(section.section_title.clone().unwrap_or_default())
                }
                InputMode::EditSectionClass => Some(
                    section_class_to_str(
                        section
                            .section_class
                            .unwrap_or(crate::model::SectionClass::FullContained),
                    )
                    .to_string(),
                ),
                InputMode::EditColumnId | InputMode::EditColumnWidthClass => {
                    let columns = section_columns_ref(section);
                    let col_i = self.selected_column.min(columns.len().saturating_sub(1));
                    match mode {
                        InputMode::EditColumnId => Some(columns[col_i].id.clone()),
                        InputMode::EditColumnWidthClass => Some(columns[col_i].width_class.clone()),
                        _ => None,
                    }
                }
                _ => {
                    let columns = section_columns_ref(section);
                    let col_i = self.selected_column.min(columns.len().saturating_sub(1));
                    let ci =
                        component_index(columns[col_i].components.len(), self.selected_component)?;
                    let component = columns[col_i].components.get(ci)?;
                    match (mode, component) {
                        (
                            InputMode::EditAlternatingType,
                            crate::model::SectionComponent::Alternating(v),
                        ) => Some(alternating_type_to_str(v.parent_type).to_string()),
                        (
                            InputMode::EditAlternatingClass,
                            crate::model::SectionComponent::Alternating(v),
                        ) => Some(v.parent_class.clone()),
                        (
                            InputMode::EditAlternatingDataAos,
                            crate::model::SectionComponent::Alternating(v),
                        ) => Some(parent_data_aos_to_str(v.parent_data_aos).to_string()),
                        (
                            InputMode::EditAlternatingItemImage,
                            crate::model::SectionComponent::Alternating(v),
                        ) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(v.items[ni].child_image_url.clone())
                        }
                        (
                            InputMode::EditAlternatingItemImageAlt,
                            crate::model::SectionComponent::Alternating(v),
                        ) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(v.items[ni].child_image_alt.clone())
                        }
                        (
                            InputMode::EditAlternatingItemTitle,
                            crate::model::SectionComponent::Alternating(v),
                        ) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(v.items[ni].child_title.clone())
                        }
                        (
                            InputMode::EditAlternatingItemCopy,
                            crate::model::SectionComponent::Alternating(v),
                        ) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(v.items[ni].child_copy.clone())
                        }
                        (InputMode::EditBannerClass, crate::model::SectionComponent::Banner(v)) => {
                            Some(banner_class_to_str(v.parent_class).to_string())
                        }
                        (
                            InputMode::EditBannerDataAos,
                            crate::model::SectionComponent::Banner(v),
                        ) => Some(parent_data_aos_to_str(v.parent_data_aos).to_string()),
                        (
                            InputMode::EditBannerImageUrl,
                            crate::model::SectionComponent::Banner(v),
                        ) => Some(v.parent_image_url.clone()),
                        (
                            InputMode::EditBannerImageAlt,
                            crate::model::SectionComponent::Banner(v),
                        ) => Some(v.parent_image_alt.clone()),
                        (InputMode::EditCtaClass, crate::model::SectionComponent::Cta(v)) => {
                            Some(cta_class_to_str(v.parent_class).to_string())
                        }
                        (InputMode::EditCtaImageUrl, crate::model::SectionComponent::Cta(v)) => {
                            Some(v.parent_image_url.clone())
                        }
                        (InputMode::EditCtaImageAlt, crate::model::SectionComponent::Cta(v)) => {
                            Some(v.parent_image_alt.clone())
                        }
                        (InputMode::EditCtaDataAos, crate::model::SectionComponent::Cta(v)) => {
                            Some(parent_data_aos_to_str(v.parent_data_aos).to_string())
                        }
                        (InputMode::EditCtaTitle, crate::model::SectionComponent::Cta(v)) => {
                            Some(v.parent_title.clone())
                        }
                        (InputMode::EditCtaSubtitle, crate::model::SectionComponent::Cta(v)) => {
                            Some(v.parent_subtitle.clone())
                        }
                        (InputMode::EditCtaCopy, crate::model::SectionComponent::Cta(v)) => {
                            Some(v.parent_copy.clone())
                        }
                        (InputMode::EditCtaLinkUrl, crate::model::SectionComponent::Cta(v)) => {
                            Some(v.parent_link_url.clone().unwrap_or_default())
                        }
                        (InputMode::EditCtaLinkTarget, crate::model::SectionComponent::Cta(v)) => {
                            Some(
                                v.parent_link_target
                                    .map(child_link_target_to_str)
                                    .unwrap_or("_self")
                                    .to_string(),
                            )
                        }
                        (InputMode::EditCtaLinkLabel, crate::model::SectionComponent::Cta(v)) => {
                            Some(v.parent_link_label.clone().unwrap_or_default())
                        }
                        (
                            InputMode::EditFilmstripType,
                            crate::model::SectionComponent::Filmstrip(v),
                        ) => Some(filmstrip_type_to_str(v.parent_type).to_string()),
                        (
                            InputMode::EditFilmstripDataAos,
                            crate::model::SectionComponent::Filmstrip(v),
                        ) => Some(parent_data_aos_to_str(v.parent_data_aos).to_string()),
                        (
                            InputMode::EditFilmstripItemImageUrl,
                            crate::model::SectionComponent::Filmstrip(v),
                        ) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(v.items[ni].child_image_url.clone())
                        }
                        (
                            InputMode::EditFilmstripItemImageAlt,
                            crate::model::SectionComponent::Filmstrip(v),
                        ) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(v.items[ni].child_image_alt.clone())
                        }
                        (
                            InputMode::EditFilmstripItemTitle,
                            crate::model::SectionComponent::Filmstrip(v),
                        ) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(v.items[ni].child_title.clone())
                        }
                        (
                            InputMode::EditMilestonesDataAos,
                            crate::model::SectionComponent::Milestones(v),
                        ) => Some(parent_data_aos_to_str(v.parent_data_aos).to_string()),
                        (
                            InputMode::EditMilestonesWidth,
                            crate::model::SectionComponent::Milestones(v),
                        ) => Some(v.parent_width.clone()),
                        (
                            InputMode::EditMilestonesItemPercentage,
                            crate::model::SectionComponent::Milestones(v),
                        ) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(v.items[ni].child_percentage.clone())
                        }
                        (
                            InputMode::EditMilestonesItemTitle,
                            crate::model::SectionComponent::Milestones(v),
                        ) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(v.items[ni].child_title.clone())
                        }
                        (
                            InputMode::EditMilestonesItemSubtitle,
                            crate::model::SectionComponent::Milestones(v),
                        ) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(v.items[ni].child_subtitle.clone())
                        }
                        (
                            InputMode::EditMilestonesItemCopy,
                            crate::model::SectionComponent::Milestones(v),
                        ) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(v.items[ni].child_copy.clone())
                        }
                        (
                            InputMode::EditMilestonesItemLinkUrl,
                            crate::model::SectionComponent::Milestones(v),
                        ) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(v.items[ni].child_link_url.clone().unwrap_or_default())
                        }
                        (
                            InputMode::EditMilestonesItemLinkTarget,
                            crate::model::SectionComponent::Milestones(v),
                        ) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(
                                child_link_target_to_str(
                                    v.items[ni]
                                        .child_link_target
                                        .unwrap_or(crate::model::CardLinkTarget::SelfTarget),
                                )
                                .to_string(),
                            )
                        }
                        (
                            InputMode::EditMilestonesItemLinkLabel,
                            crate::model::SectionComponent::Milestones(v),
                        ) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(v.items[ni].child_link_label.clone().unwrap_or_default())
                        }
                        (InputMode::EditModalTitle, crate::model::SectionComponent::Modal(v)) => {
                            Some(v.parent_title.clone())
                        }
                        (InputMode::EditModalCopy, crate::model::SectionComponent::Modal(v)) => {
                            Some(v.parent_copy.clone())
                        }
                        (InputMode::EditSliderTitle, crate::model::SectionComponent::Slider(v)) => {
                            Some(v.parent_title.clone())
                        }
                        (
                            InputMode::EditSliderItemTitle,
                            crate::model::SectionComponent::Slider(v),
                        ) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(v.items[ni].child_title.clone())
                        }
                        (
                            InputMode::EditSliderItemCopy,
                            crate::model::SectionComponent::Slider(v),
                        ) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(v.items[ni].child_copy.clone())
                        }
                        (
                            InputMode::EditSliderItemLinkUrl,
                            crate::model::SectionComponent::Slider(v),
                        ) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(v.items[ni].child_link_url.clone().unwrap_or_default())
                        }
                        (
                            InputMode::EditSliderItemLinkTarget,
                            crate::model::SectionComponent::Slider(v),
                        ) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(
                                child_link_target_to_str(
                                    v.items[ni]
                                        .child_link_target
                                        .unwrap_or(crate::model::CardLinkTarget::SelfTarget),
                                )
                                .to_string(),
                            )
                        }
                        (
                            InputMode::EditSliderItemLinkLabel,
                            crate::model::SectionComponent::Slider(v),
                        ) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(v.items[ni].child_link_label.clone().unwrap_or_default())
                        }
                        (
                            InputMode::EditSliderItemImageUrl,
                            crate::model::SectionComponent::Slider(v),
                        ) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(v.items[ni].child_image_url.clone())
                        }
                        (
                            InputMode::EditSliderItemImageAlt,
                            crate::model::SectionComponent::Slider(v),
                        ) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(v.items[ni].child_image_alt.clone())
                        }
                        (InputMode::EditCardType, crate::model::SectionComponent::Card(v)) => {
                            Some(card_type_to_str(v.parent_type).to_string())
                        }
                        (InputMode::EditCardDataAos, crate::model::SectionComponent::Card(v)) => {
                            Some(parent_data_aos_to_str(v.parent_data_aos).to_string())
                        }
                        (InputMode::EditCardWidth, crate::model::SectionComponent::Card(v)) => {
                            Some(v.parent_width.clone())
                        }
                        (
                            InputMode::EditCardItemImageUrl,
                            crate::model::SectionComponent::Card(v),
                        ) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(v.items[ni].child_image_url.clone())
                        }
                        (
                            InputMode::EditCardItemImageAlt,
                            crate::model::SectionComponent::Card(v),
                        ) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(v.items[ni].child_image_alt.clone())
                        }
                        (InputMode::EditCardItemTitle, crate::model::SectionComponent::Card(v)) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(v.items[ni].child_title.clone())
                        }
                        (
                            InputMode::EditCardItemSubtitle,
                            crate::model::SectionComponent::Card(v),
                        ) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(v.items[ni].child_subtitle.clone())
                        }
                        (InputMode::EditCardItemCopy, crate::model::SectionComponent::Card(v)) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(v.items[ni].child_copy.clone())
                        }
                        (
                            InputMode::EditCardItemLinkUrl,
                            crate::model::SectionComponent::Card(v),
                        ) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(v.items[ni].child_link_url.clone().unwrap_or_default())
                        }
                        (
                            InputMode::EditCardItemLinkTarget,
                            crate::model::SectionComponent::Card(v),
                        ) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(
                                child_link_target_to_str(
                                    v.items[ni]
                                        .child_link_target
                                        .unwrap_or(crate::model::CardLinkTarget::SelfTarget),
                                )
                                .to_string(),
                            )
                        }
                        (
                            InputMode::EditCardItemLinkLabel,
                            crate::model::SectionComponent::Card(v),
                        ) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(v.items[ni].child_link_label.clone().unwrap_or_default())
                        }
                        (
                            InputMode::EditBlockquoteDataAos,
                            crate::model::SectionComponent::Blockquote(v),
                        ) => Some(parent_data_aos_to_str(v.parent_data_aos).to_string()),
                        (
                            InputMode::EditBlockquoteImageUrl,
                            crate::model::SectionComponent::Blockquote(v),
                        ) => Some(v.parent_image_url.clone()),
                        (
                            InputMode::EditBlockquoteImageAlt,
                            crate::model::SectionComponent::Blockquote(v),
                        ) => Some(v.parent_image_alt.clone()),
                        (
                            InputMode::EditBlockquotePersonsName,
                            crate::model::SectionComponent::Blockquote(v),
                        ) => Some(v.parent_name.clone()),
                        (
                            InputMode::EditBlockquotePersonsTitle,
                            crate::model::SectionComponent::Blockquote(v),
                        ) => Some(v.parent_role.clone()),
                        (
                            InputMode::EditBlockquoteCopy,
                            crate::model::SectionComponent::Blockquote(v),
                        ) => Some(v.parent_copy.clone()),
                        (
                            InputMode::EditAccordionType,
                            crate::model::SectionComponent::Accordion(v),
                        ) => Some(accordion_type_to_str(v.parent_type).to_string()),
                        (
                            InputMode::EditAccordionClass,
                            crate::model::SectionComponent::Accordion(v),
                        ) => Some(accordion_class_to_str(v.parent_class).to_string()),
                        (
                            InputMode::EditAccordionAos,
                            crate::model::SectionComponent::Accordion(v),
                        ) => Some(parent_data_aos_to_str(v.parent_data_aos).to_string()),
                        (
                            InputMode::EditAccordionGroupName,
                            crate::model::SectionComponent::Accordion(v),
                        ) => Some(v.parent_group_name.clone()),
                        (
                            InputMode::EditAccordionFirstTitle,
                            crate::model::SectionComponent::Accordion(v),
                        ) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(v.items[ni].child_title.clone())
                        }
                        (
                            InputMode::EditAccordionFirstContent,
                            crate::model::SectionComponent::Accordion(v),
                        ) => {
                            let ni = nested_index(v.items.len(), self.selected_nested_item)?;
                            Some(v.items[ni].child_copy.clone())
                        }
                        _ => None,
                    }
                }
            },
        }
    }

    fn selected_component_owned(&self) -> Option<crate::model::SectionComponent> {
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

    fn select_prev(&mut self) {
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

    fn select_next(&mut self) {
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

    fn handle_up(&mut self) {
        match self.selected_sidebar_section {
            SidebarSection::Regions => {
                self.selected_region = SelectedRegion::Header;
                self.selected_tree_row = 0;
                self.status = "Selected Header region.".to_string();
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

    fn handle_down(&mut self) {
        match self.selected_sidebar_section {
            SidebarSection::Regions => {
                self.selected_region = SelectedRegion::Footer;
                self.selected_tree_row = 0;
                self.status = "Selected Footer region (not yet implemented).".to_string();
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

    /// Vim `gg`/`G` analogue: jump to the first tree row.
    fn vim_jump_to_first_row(&mut self) {
        let rows = self.build_tree_rows();
        if rows.is_empty() {
            return;
        }
        self.selected_tree_row = 0;
        self.apply_tree_row_selection(rows[0]);
        self.details_scroll_row = 0;
    }

    /// Vim `G`: jump to the last tree row.
    fn vim_jump_to_last_row(&mut self) {
        let rows = self.build_tree_rows();
        if rows.is_empty() {
            return;
        }
        let last = rows.len() - 1;
        self.selected_tree_row = last;
        self.apply_tree_row_selection(rows[last]);
        self.details_scroll_row = 0;
    }

    /// Vim `h`: collapse the selected expandable row (no-op when already
    /// collapsed or the row isn't expandable).
    fn vim_collapse_selected_row(&mut self) {
        let rows = self.build_tree_rows();
        if rows.is_empty() {
            return;
        }
        let row = rows[self.selected_tree_row.min(rows.len() - 1)];
        if self.tree_row_is_expanded(&row) {
            self.toggle_selected_tree_expanded();
        }
    }

    /// Vim `l`: expand the selected expandable row.
    fn vim_expand_selected_row(&mut self) {
        let rows = self.build_tree_rows();
        if rows.is_empty() {
            return;
        }
        let row = rows[self.selected_tree_row.min(rows.len() - 1)];
        if !self.tree_row_is_expanded(&row) {
            self.toggle_selected_tree_expanded();
        }
    }

    /// Returns true when the row is expandable AND currently expanded.
    fn tree_row_is_expanded(&self, row: &TreeRow) -> bool {
        match row.kind {
            TreeRowKind::Section { node_idx } => self.is_section_expanded(node_idx),
            TreeRowKind::HeaderSection { section_idx } => {
                self.is_header_section_expanded(section_idx)
            }
            _ => false,
        }
    }

    fn select_next_page(&mut self) {
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

    fn select_prev_page(&mut self) {
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

    fn details_text(&self, detail_width: usize) -> String {
        match self.selected_region {
            SelectedRegion::Header => self.header_details_text(detail_width),
            SelectedRegion::Footer => "Footer editing not yet implemented.".to_string(),
            SelectedRegion::Page => self.page_details_text(detail_width),
        }
    }

    fn header_details_text(&self, detail_width: usize) -> String {
        let mut out = Vec::new();
        out.push("Site header".to_string());
        out.push(String::new());
        let marker = if matches!(self.selected_region, SelectedRegion::Header) {
            "*"
        } else {
            " "
        };
        out.push(format!("{}[01] dd-header {}", marker, self.site.header.id));
        out.push(header_ascii_map(
            &self.site.header,
            self.selected_header_section,
            self.selected_header_column,
            detail_width,
        ));
        out.push(String::new());
        out.push(format!(
            "Selected: {} | Insert mode: {}",
            self.header_selection_summary(),
            self.component_kind.label()
        ));
        out.join("\n")
    }

    fn page_details_text(&self, detail_width: usize) -> String {
        let page = self.current_page();
        if page.nodes.is_empty() {
            return "No nodes on this page.".to_string();
        }
        let mut out = Vec::new();
        out.push(format!("Page blueprint: {}", page.head.title));
        out.push(String::new());
        for (idx, node) in page.nodes.iter().enumerate() {
            let marker = if idx == self.selected_node { "*" } else { " " };
            match node {
                PageNode::Hero(v) => {
                    out.push(format!("{marker}[{:02}] dd-hero", idx + 1,));
                    out.push(hero_ascii_map(v, detail_width));
                }
                PageNode::Section(v) => {
                    out.push(format!("{marker}[{:02}] dd-section {}", idx + 1, v.id));
                    out.push(section_ascii_map(
                        v,
                        if idx == self.selected_node {
                            self.selected_column
                        } else {
                            0
                        },
                        detail_width,
                    ));
                }
            }
            out.push(String::new());
        }
        out.push(format!(
            "Selected: {} | Insert mode: {}",
            self.selection_summary(),
            self.component_kind.label()
        ));
        out.join("\n")
    }

    fn add_hero(&mut self) {
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
        self.status = format!("Inserted dd-hero at position {}.", idx + 1);
    }

    fn add_section(&mut self) {
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
        self.status = format!("Inserted dd-section at position {}.", idx + 1);
    }

    fn delete_selected_node(&mut self) {
        let selected = self.selected_node;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No node to delete.".to_string();
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
        self.status = format!("Deleted node {}.", idx + 1);
    }

    fn add_selected_component_to_section(&mut self) {
        let kind = self.component_kind;
        if matches!(kind, ComponentKind::Hero | ComponentKind::Section) {
            self.status = "dd-hero and dd-section are top-level insert types.".to_string();
            return;
        }
        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No selected section.".to_string();
            return;
        }
        let idx = selected.min(page.nodes.len() - 1);
        let result = match &mut page.nodes[idx] {
            PageNode::Section(section) => {
                normalize_section_columns(section);
                let col_i = selected_column.min(section.columns.len().saturating_sub(1));
                let components = &mut section.columns[col_i].components;
                let inserted = kind.default_component();
                components.push(inserted);
                (
                    Some(components.len().saturating_sub(1)),
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
        self.status = result.1;
    }

    fn cycle_hero_parent_class(&mut self, forward: bool) {
        let selected = self.selected_node;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No selected hero.".to_string();
            return;
        }
        let idx = selected.min(page.nodes.len() - 1);
        match &mut page.nodes[idx] {
            PageNode::Hero(hero) => {
                let current = hero
                    .parent_class
                    .unwrap_or(crate::model::HeroImageClass::FullFull);
                let next = next_hero_image_class(current, forward);
                hero.parent_class = Some(next);
                self.status = format!("Hero default class: {}", hero_image_class_to_str(next));
            }
            _ => {
                self.status =
                    "Left/Right hero class cycling works on a selected hero row.".to_string();
            }
        }
    }

    fn cycle_hero_parent_data_aos(&mut self, forward: bool) {
        let selected = self.selected_node;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No selected hero.".to_string();
            return;
        }
        let idx = selected.min(page.nodes.len() - 1);
        match &mut page.nodes[idx] {
            PageNode::Hero(hero) => {
                let current = hero.parent_data_aos.unwrap_or(crate::model::HeroAos::FadeIn);
                let next = next_parent_data_aos(current, forward);
                hero.parent_data_aos = Some(next);
                self.status = format!("Hero data-aos: {}", parent_data_aos_to_str(next));
            }
            _ => {
                self.status =
                    "Left/Right hero data-aos cycling works on a selected hero row.".to_string();
            }
        }
    }

    fn cycle_hero_link_1_target(&mut self, secondary: bool, forward: bool) {
        let selected = self.selected_node;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No selected hero.".to_string();
            return;
        }
        let idx = selected.min(page.nodes.len() - 1);
        match &mut page.nodes[idx] {
            PageNode::Hero(hero) => {
                let current = if secondary {
                    hero.link_2_target
                        .unwrap_or(crate::model::CtaTarget::SelfTarget)
                } else {
                    hero.link_1_target
                        .unwrap_or(crate::model::CtaTarget::SelfTarget)
                };
                let next = next_hero_link_1_target(current, forward);
                if secondary {
                    hero.link_2_target = Some(next);
                } else {
                    hero.link_1_target = Some(next);
                }
                self.status = if secondary {
                    format!("Hero link_2 target: {}", link_1_target_to_str(next))
                } else {
                    format!("Hero link_1 target: {}", link_1_target_to_str(next))
                };
            }
            _ => {
                self.status =
                    "Left/Right hero target cycling works on a selected hero row.".to_string();
            }
        }
    }

    fn cycle_section_class(&mut self, forward: bool) {
        self.mutate_selected_section(
            |s| {
                let current = s
                    .section_class
                    .unwrap_or(crate::model::SectionClass::FullContained);
                s.section_class = Some(next_section_class(current, forward));
            },
            "Cycled section class.",
        );
    }

    fn cycle_banner_parent_class(&mut self, forward: bool) {
        self.mutate_selected_banner(
            |b| {
                b.parent_class = next_banner_class(b.parent_class, forward);
            },
            "Cycled dd-banner class.",
        );
    }

    fn cycle_banner_parent_data_aos(&mut self, forward: bool) {
        self.mutate_selected_banner(
            |b| {
                b.parent_data_aos = next_parent_data_aos(b.parent_data_aos, forward);
            },
            "Cycled dd-banner data-aos.",
        );
    }

    fn cycle_blockquote_parent_data_aos(&mut self, forward: bool) {
        self.mutate_selected_blockquote(
            |b| {
                b.parent_data_aos = next_parent_data_aos(b.parent_data_aos, forward);
            },
            "Cycled dd-blockquote data-aos.",
        );
    }

    fn cycle_card_parent_type(&mut self, forward: bool) {
        self.mutate_selected_card(
            |c| {
                c.parent_type = next_card_type(c.parent_type, forward);
            },
            "Cycled dd-card type.",
        );
    }

    fn cycle_card_parent_data_aos(&mut self, forward: bool) {
        self.mutate_selected_card(
            |c| {
                c.parent_data_aos = next_parent_data_aos(c.parent_data_aos, forward);
            },
            "Cycled dd-card data-aos.",
        );
    }

    fn cycle_child_link_target(&mut self, forward: bool) {
        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let selected_nested_item = self.selected_nested_item;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No selected section.".to_string();
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
                        if let Some(item_i) = nested_index(card.items.len(), selected_nested_item) {
                            let current = card.items[item_i]
                                .child_link_target
                                .unwrap_or(crate::model::CardLinkTarget::SelfTarget);
                            let next = next_child_link_target(current, forward);
                            card.items[item_i].child_link_target = Some(next);
                            format!(
                                "dd-card item {} link target: {}",
                                item_i + 1,
                                child_link_target_to_str(next)
                            )
                        } else {
                            "dd-card has no items.".to_string()
                        }
                    } else {
                        "Selected component is not dd-card.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            _ => "Selected node is not a section.".to_string(),
        };
        self.status = result;
    }

    fn cycle_filmstrip_parent_type(&mut self, forward: bool) {
        self.mutate_selected_filmstrip(
            |f| {
                f.parent_type = next_filmstrip_type(f.parent_type, forward);
            },
            "Cycled dd-filmstrip type.",
        );
    }

    fn cycle_filmstrip_parent_data_aos(&mut self, forward: bool) {
        self.mutate_selected_filmstrip(
            |f| {
                f.parent_data_aos = next_parent_data_aos(f.parent_data_aos, forward);
            },
            "Cycled dd-filmstrip data-aos.",
        );
    }

    fn cycle_milestones_data_aos(&mut self, forward: bool) {
        self.mutate_selected_milestones(
            |m| {
                m.parent_data_aos = next_parent_data_aos(m.parent_data_aos, forward);
            },
            "Cycled dd-milestones parent_data_aos.",
        );
    }

    fn cycle_milestones_link_target(&mut self, forward: bool) {
        let selected_nested_item = self.selected_nested_item;
        self.mutate_selected_milestones(
            |m| {
                if let Some(ni) = nested_index(m.items.len(), selected_nested_item) {
                    let current = m.items[ni]
                        .child_link_target
                        .unwrap_or(crate::model::CardLinkTarget::SelfTarget);
                    m.items[ni].child_link_target = Some(next_child_link_target(current, forward));
                }
            },
            "Cycled dd-milestones child_link_target.",
        );
    }

    fn cycle_slider_link_target(&mut self, forward: bool) {
        let selected_nested_item = self.selected_nested_item;
        self.mutate_selected_slider(
            |s| {
                if let Some(ni) = nested_index(s.items.len(), selected_nested_item) {
                    let current = s.items[ni]
                        .child_link_target
                        .unwrap_or(crate::model::CardLinkTarget::SelfTarget);
                    s.items[ni].child_link_target = Some(next_child_link_target(current, forward));
                }
            },
            "Cycled dd-slider child_link_target.",
        );
    }

    fn cycle_alternating_parent_type(&mut self, forward: bool) {
        self.mutate_selected_alternating(
            |a| {
                a.parent_type = next_alternating_type(a.parent_type, forward);
            },
            "Cycled dd-alternating type.",
        );
    }

    fn cycle_alternating_parent_data_aos(&mut self, forward: bool) {
        self.mutate_selected_alternating(
            |a| {
                a.parent_data_aos = next_parent_data_aos(a.parent_data_aos, forward);
            },
            "Cycled dd-alternating data-aos.",
        );
    }

    fn mutate_selected_alternating<F>(&mut self, mutator: F, success_message: &str)
    where
        F: FnOnce(&mut crate::model::DdAlternating),
    {
        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No selected section.".to_string();
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
                        mutator(alt);
                        success_message.to_string()
                    } else {
                        "Selected component is not dd-alternating.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            _ => "Selected node is not a section.".to_string(),
        };
        self.status = result;
    }

    fn mutate_selected_banner<F>(&mut self, mutator: F, success_message: &str)
    where
        F: FnOnce(&mut crate::model::DdBanner),
    {
        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No selected section.".to_string();
            return;
        }
        let ni = selected.min(page.nodes.len() - 1);
        let result = match &mut page.nodes[ni] {
            PageNode::Section(section) => {
                normalize_section_columns(section);
                let col_i = selected_column.min(section.columns.len().saturating_sub(1));
                let components = &mut section.columns[col_i].components;
                if let Some(ci) = component_index(components.len(), selected_component) {
                    if let crate::model::SectionComponent::Banner(banner) = &mut components[ci] {
                        mutator(banner);
                        success_message.to_string()
                    } else {
                        "Selected component is not dd-banner.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            _ => "Selected node is not a section.".to_string(),
        };
        self.status = result;
    }

    fn cycle_cta_parent_class(&mut self, forward: bool) {
        self.mutate_selected_cta(
            |cta| {
                cta.parent_class = next_cta_class(cta.parent_class, forward);
            },
            "Cycled dd-cta class.",
        );
    }

    fn cycle_cta_parent_data_aos(&mut self, forward: bool) {
        self.mutate_selected_cta(
            |cta| {
                cta.parent_data_aos = next_parent_data_aos(cta.parent_data_aos, forward);
            },
            "Cycled dd-cta data-aos.",
        );
    }

    fn cycle_parent_link_target(&mut self, forward: bool) {
        self.mutate_selected_cta(
            |cta| {
                let current = cta
                    .parent_link_target
                    .unwrap_or(crate::model::CardLinkTarget::SelfTarget);
                cta.parent_link_target = Some(next_child_link_target(current, forward));
            },
            "Cycled dd-cta link target.",
        );
    }

    fn mutate_selected_cta<F>(&mut self, mutator: F, success_message: &str)
    where
        F: FnOnce(&mut crate::model::DdCta),
    {
        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No selected section.".to_string();
            return;
        }
        let ni = selected.min(page.nodes.len() - 1);
        let result = match &mut page.nodes[ni] {
            PageNode::Section(section) => {
                normalize_section_columns(section);
                let col_i = selected_column.min(section.columns.len().saturating_sub(1));
                let components = &mut section.columns[col_i].components;
                if let Some(ci) = component_index(components.len(), selected_component) {
                    if let crate::model::SectionComponent::Cta(cta) = &mut components[ci] {
                        mutator(cta);
                        success_message.to_string()
                    } else {
                        "Selected component is not dd-cta.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            _ => "Selected node is not a section.".to_string(),
        };
        self.status = result;
    }

    fn mutate_selected_blockquote<F>(&mut self, mutator: F, success_message: &str)
    where
        F: FnOnce(&mut crate::model::DdBlockquote),
    {
        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No selected section.".to_string();
            return;
        }
        let ni = selected.min(page.nodes.len() - 1);
        let result = match &mut page.nodes[ni] {
            PageNode::Section(section) => {
                normalize_section_columns(section);
                let col_i = selected_column.min(section.columns.len().saturating_sub(1));
                let components = &mut section.columns[col_i].components;
                if let Some(ci) = component_index(components.len(), selected_component) {
                    if let crate::model::SectionComponent::Blockquote(blockquote) =
                        &mut components[ci]
                    {
                        mutator(blockquote);
                        success_message.to_string()
                    } else {
                        "Selected component is not dd-blockquote.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            _ => "Selected node is not a section.".to_string(),
        };
        self.status = result;
    }

    fn mutate_selected_card<F>(&mut self, mutator: F, success_message: &str)
    where
        F: FnOnce(&mut crate::model::DdCard),
    {
        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No selected section.".to_string();
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
                        mutator(card);
                        success_message.to_string()
                    } else {
                        "Selected component is not dd-card.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            _ => "Selected node is not a section.".to_string(),
        };
        self.status = result;
    }

    fn mutate_selected_filmstrip<F>(&mut self, mutator: F, success_message: &str)
    where
        F: FnOnce(&mut crate::model::DdFilmstrip),
    {
        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No selected section.".to_string();
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
                        mutator(filmstrip);
                        success_message.to_string()
                    } else {
                        "Selected component is not dd-filmstrip.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            _ => "Selected node is not a section.".to_string(),
        };
        self.status = result;
    }

    fn mutate_selected_milestones<F>(&mut self, mutator: F, success_message: &str)
    where
        F: FnOnce(&mut crate::model::DdMilestones),
    {
        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No selected section.".to_string();
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
                        mutator(milestones);
                        success_message.to_string()
                    } else {
                        "Selected component is not dd-milestones.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            _ => "Selected node is not a section.".to_string(),
        };
        self.status = result;
    }

    fn mutate_selected_slider<F>(&mut self, mutator: F, success_message: &str)
    where
        F: FnOnce(&mut crate::model::DdSlider),
    {
        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No selected section.".to_string();
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
                        mutator(slider);
                        success_message.to_string()
                    } else {
                        "Selected component is not dd-slider.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            _ => "Selected node is not a section.".to_string(),
        };
        self.status = result;
    }

    fn cycle_accordion_parent_type(&mut self, forward: bool) {
        self.mutate_selected_accordion(
            |a| {
                a.parent_type = next_accordion_type(a.parent_type, forward);
            },
            "Cycled dd-accordion type.",
        );
    }

    fn cycle_accordion_parent_class(&mut self, forward: bool) {
        self.mutate_selected_accordion(
            |a| {
                a.parent_class = next_accordion_class(a.parent_class, forward);
            },
            "Cycled dd-accordion class.",
        );
    }

    fn cycle_accordion_parent_data_aos(&mut self, forward: bool) {
        self.mutate_selected_accordion(
            |a| {
                a.parent_data_aos = next_parent_data_aos(a.parent_data_aos, forward);
            },
            "Cycled dd-accordion data-aos.",
        );
    }

    fn mutate_selected_accordion<F>(&mut self, mutator: F, success_message: &str)
    where
        F: FnOnce(&mut crate::model::DdAccordion),
    {
        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No selected section.".to_string();
            return;
        }
        let ni = selected.min(page.nodes.len() - 1);
        let result = match &mut page.nodes[ni] {
            PageNode::Section(section) => {
                normalize_section_columns(section);
                let col_i = selected_column.min(section.columns.len().saturating_sub(1));
                let components = &mut section.columns[col_i].components;
                if let Some(ci) = component_index(components.len(), selected_component) {
                    if let crate::model::SectionComponent::Accordion(accordion) =
                        &mut components[ci]
                    {
                        mutator(accordion);
                        success_message.to_string()
                    } else {
                        "Selected component is not dd-accordion.".to_string()
                    }
                } else {
                    "Section has no components.".to_string()
                }
            }
            _ => "Selected node is not a section.".to_string(),
        };
        self.status = result;
    }

    fn add_selected_collection_item(&mut self) {
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
                self.status = "Selected component does not support collection items.".to_string();
            }
            None => {
                self.status = "No selected collection component.".to_string();
            }
        }
    }

    fn remove_selected_collection_item(&mut self) {
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
                self.status = "Selected component does not support collection items.".to_string();
            }
            None => {
                self.status = "No selected collection component.".to_string();
            }
        }
    }

    fn add_selected_accordion_item(&mut self) {
        let rows = self.build_page_tree_rows();
        if rows.is_empty() {
            self.status = "No selected section.".to_string();
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
            self.status = "No selected section.".to_string();
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
        self.status = result.1;
    }

    fn remove_selected_accordion_item(&mut self) {
        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let selected_nested_item = self.selected_nested_item;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No selected section.".to_string();
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
        self.status = result.1;
    }

    fn add_selected_alternating_item(&mut self) {
        let rows = self.build_page_tree_rows();
        if rows.is_empty() {
            self.status = "No selected section.".to_string();
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
            self.status = "No selected section.".to_string();
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
        self.status = result.1;
    }

    fn remove_selected_alternating_item(&mut self) {
        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let selected_component = self.selected_component;
        let selected_nested_item = self.selected_nested_item;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No selected section.".to_string();
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
        self.status = result.1;
    }

    fn add_selected_card_item(&mut self) {
        let rows = self.build_page_tree_rows();
        if rows.is_empty() {
            self.status = "No selected section.".to_string();
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
            self.status = "No selected section.".to_string();
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
        self.status = result.1;
    }

    fn remove_selected_card_item(&mut self) {
        let rows = self.build_page_tree_rows();
        if rows.is_empty() {
            self.status = "No selected section.".to_string();
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
            self.status = "No selected section.".to_string();
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
        self.status = result.1;
    }

    fn add_selected_filmstrip_item(&mut self) {
        let rows = self.build_page_tree_rows();
        if rows.is_empty() {
            self.status = "No selected section.".to_string();
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
            self.status = "No selected section.".to_string();
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
        self.status = result.1;
    }

    fn remove_selected_filmstrip_item(&mut self) {
        let rows = self.build_page_tree_rows();
        if rows.is_empty() {
            self.status = "No selected section.".to_string();
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
            self.status = "No selected section.".to_string();
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
        self.status = result.1;
    }

    fn add_selected_milestones_item(&mut self) {
        let rows = self.build_page_tree_rows();
        if rows.is_empty() {
            self.status = "No selected section.".to_string();
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
            self.status = "No selected section.".to_string();
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
        self.status = result.1;
    }

    fn remove_selected_milestones_item(&mut self) {
        let rows = self.build_page_tree_rows();
        if rows.is_empty() {
            self.status = "No selected section.".to_string();
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
            self.status = "No selected section.".to_string();
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
        self.status = result.1;
    }

    fn add_selected_slider_item(&mut self) {
        let rows = self.build_page_tree_rows();
        if rows.is_empty() {
            self.status = "No selected section.".to_string();
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
            self.status = "No selected section.".to_string();
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
        self.status = result.1;
    }

    fn remove_selected_slider_item(&mut self) {
        let rows = self.build_page_tree_rows();
        if rows.is_empty() {
            self.status = "No selected section.".to_string();
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
            self.status = "No selected section.".to_string();
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
        self.status = result.1;
    }

    fn mutate_selected_section<F>(&mut self, mutator: F, success_message: &str)
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
            self.status = "No selected section.".to_string();
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
        self.status = result.1;
    }

    fn add_column(&mut self) {
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

    fn add_column_to_header_section(&mut self) {
        if self.site.header.sections.is_empty() {
            self.status = "No header section available. Add a section first with '/'.".to_string();
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
        self.status = format!("Added column to header section '{}'.", section.id);
    }

    fn remove_selected_column(&mut self) {
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
            self.status = "No selected section.".to_string();
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
        self.status = result.1;
    }

    fn remove_column_from_header_section(&mut self) {
        if self.site.header.sections.is_empty() {
            self.status = "No header sections to modify.".to_string();
            return;
        }
        let section_idx = self
            .selected_header_section
            .min(self.site.header.sections.len().saturating_sub(1));
        let section = &mut self.site.header.sections[section_idx];
        normalize_section_columns(section);
        if section.columns.len() <= 1 {
            self.status = "Header section must keep at least one column.".to_string();
            return;
        }
        let ci = self.selected_header_column.min(section.columns.len() - 1);
        section.columns.remove(ci);
        self.selected_header_column = ci.min(section.columns.len() - 1);
        self.selected_header_component = 0;
        self.status = "Removed column from header section.".to_string();
    }

    fn select_prev_column(&mut self) {
        // Check if we're in Header mode
        if self.selected_region == SelectedRegion::Header {
            let total = match self.selected_header_section_column_total() {
                Some(v) => v,
                None => {
                    self.status = "No header section selected.".to_string();
                    return;
                }
            };
            if total == 0 {
                self.status = "Selected header section has no columns.".to_string();
                return;
            }
            self.selected_header_column = self.selected_header_column.saturating_sub(1);
            self.selected_header_component = 0;
            self.status = format!(
                "Selected header column {} of {}.",
                self.selected_header_column + 1,
                total
            );
            return;
        }

        let total = match self.selected_section_column_total() {
            Some(v) => v,
            None => {
                self.status = "Selected node is not a section.".to_string();
                return;
            }
        };
        if total == 0 {
            self.status = "Selected section has no columns.".to_string();
            return;
        }
        self.selected_column = self.selected_column.saturating_sub(1);
        self.selected_component = 0;
        self.selected_nested_item = 0;
        self.status = format!("Selected column {} of {}.", self.selected_column + 1, total);
    }

    fn select_next_column(&mut self) {
        // Check if we're in Header mode
        if self.selected_region == SelectedRegion::Header {
            let total = match self.selected_header_section_column_total() {
                Some(v) => v,
                None => {
                    self.status = "No header section selected.".to_string();
                    return;
                }
            };
            if total == 0 {
                self.status = "Selected header section has no columns.".to_string();
                return;
            }
            self.selected_header_column = (self.selected_header_column + 1).min(total - 1);
            self.selected_header_component = 0;
            self.status = format!(
                "Selected header column {} of {}.",
                self.selected_header_column + 1,
                total
            );
            return;
        }

        let total = match self.selected_section_column_total() {
            Some(v) => v,
            None => {
                self.status = "Selected node is not a section.".to_string();
                return;
            }
        };
        if total == 0 {
            self.status = "Selected section has no columns.".to_string();
            return;
        }
        self.selected_column = (self.selected_column + 1).min(total - 1);
        self.selected_component = 0;
        self.selected_nested_item = 0;
        self.status = format!("Selected column {} of {}.", self.selected_column + 1, total);
    }

    fn selected_header_section_column_total(&self) -> Option<usize> {
        if self.site.header.sections.is_empty() {
            return None;
        }
        let section_idx = self
            .selected_header_section
            .min(self.site.header.sections.len().saturating_sub(1));
        Some(self.site.header.sections[section_idx].columns.len())
    }

    fn move_selected_column_up(&mut self) {
        // Check if we're in Header mode
        if self.selected_region == SelectedRegion::Header {
            if self.site.header.sections.is_empty() {
                self.status = "No header sections to modify.".to_string();
                return;
            }
            let section_idx = self
                .selected_header_section
                .min(self.site.header.sections.len().saturating_sub(1));
            let section = &mut self.site.header.sections[section_idx];
            normalize_section_columns(section);
            if section.columns.len() < 2 {
                self.status = "Need at least 2 columns.".to_string();
                return;
            }
            let ci = self.selected_header_column.min(section.columns.len() - 1);
            if ci == 0 {
                self.status = "Column is already first.".to_string();
                return;
            }
            section.columns.swap(ci, ci - 1);
            self.selected_header_column = ci - 1;
            self.snap_tree_row_to_header_column(section_idx, ci - 1);
            self.status = "Moved header column up.".to_string();
            return;
        }

        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No selected section.".to_string();
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
        self.status = result.1;
    }

    fn move_selected_column_down(&mut self) {
        // Check if we're in Header mode
        if self.selected_region == SelectedRegion::Header {
            if self.site.header.sections.is_empty() {
                self.status = "No header sections to modify.".to_string();
                return;
            }
            let section_idx = self
                .selected_header_section
                .min(self.site.header.sections.len().saturating_sub(1));
            let section = &mut self.site.header.sections[section_idx];
            normalize_section_columns(section);
            if section.columns.len() < 2 {
                self.status = "Need at least 2 columns.".to_string();
                return;
            }
            let ci = self.selected_header_column.min(section.columns.len() - 1);
            if ci + 1 >= section.columns.len() {
                self.status = "Column is already last.".to_string();
                return;
            }
            section.columns.swap(ci, ci + 1);
            self.selected_header_column = ci + 1;
            self.snap_tree_row_to_header_column(section_idx, ci + 1);
            self.status = "Moved header column down.".to_string();
            return;
        }

        let selected = self.selected_node;
        let selected_column = self.selected_column;
        let Some(page) = self.current_page_mut() else {
            return;
        };
        if page.nodes.is_empty() {
            self.status = "No selected section.".to_string();
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
        self.status = result.1;
    }

    /// After a column swap, force `selected_tree_row` to the Column row for
    /// `(node_idx, column_idx)`. Avoids the permissive Section matcher in
    /// `sync_tree_row_with_selection` falling back to the parent Section.
    fn snap_tree_row_to_column(&mut self, node_idx: usize, column_idx: usize) {
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

    fn snap_tree_row_to_header_column(&mut self, section_idx: usize, column_idx: usize) {
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

    // TODO(rock-19-followup): add row-scoped Left/Right enum cycling for the new
    // component edit flows (dd-image link_target, dd-rich_text/dd-image/dd-navigation
    // data_aos, dd-navigation parent_type/parent_class, dd-header-search/dd-header-menu
    // data_aos, page head robots/schema_type). Current flow uses the multi-field modal
    // which accepts typed enum strings; helpers `next_navigation_type`, `next_navigation_class`,
    // `next_navigation_kind`, `next_robots_directive`, `next_schema_type` are defined and
    // unused, ready to wire up.
    // TODO(rock-19-followup): navigation item/sub-item recursive tree editing (A/Shift+A/X).
    // First-pass stores items[] flat on the DdNavigation; editing of items (kind, label,
    // url, target, css) not yet exposed in the TUI - drop into JSON to edit.
    fn open_page_head_edit_modal(&mut self) {
        let page = &self.site.pages[self.selected_page];
        let head = &page.head;
        let robots = robots_directive_to_str(head.robots).to_string();
        let schema = schema_type_to_str(head.schema_type).to_string();
        let meta = head.meta_description.clone().unwrap_or_default();
        // Default canonical URL to `/<slug>` when unset so authors can see
        // and optionally override the computed canonical.
        let canon = head
            .canonical_url
            .clone()
            .unwrap_or_else(|| format!("/{}", page.slug));
        let slug = page.slug.clone();
        // Default OG Title to the page title when unset so social cards
        // have sensible copy; author can override.
        let og_t = head.og_title.clone().unwrap_or_else(|| head.title.clone());
        let og_d = head.og_description.clone().unwrap_or_default();
        let og_i = head.og_image.clone().unwrap_or_default();
        let mut fields = vec![
            EditField {
                label: "Title".to_string(),
                value: head.title.clone(),
                buffer: head.title.clone(),
                cursor: head.title.len(),
                is_multiline: false,
                rows: 1,
            },
            EditField {
                label: "Meta Description".to_string(),
                value: meta.clone(),
                buffer: meta.clone(),
                cursor: meta.len(),
                is_multiline: true,
                rows: 3,
            },
            EditField {
                label: "Canonical URL".to_string(),
                value: canon.clone(),
                buffer: canon.clone(),
                cursor: canon.len(),
                is_multiline: false,
                rows: 1,
            },
            EditField {
                label: "Robots".to_string(),
                value: robots.clone(),
                buffer: robots.clone(),
                cursor: robots.len(),
                is_multiline: false,
                rows: 1,
            },
            EditField {
                label: "Schema Type".to_string(),
                value: schema.clone(),
                buffer: schema.clone(),
                cursor: schema.len(),
                is_multiline: false,
                rows: 1,
            },
            EditField {
                label: "OG Title".to_string(),
                value: og_t.clone(),
                buffer: og_t.clone(),
                cursor: og_t.len(),
                is_multiline: false,
                rows: 1,
            },
            EditField {
                label: "OG Description".to_string(),
                value: og_d.clone(),
                buffer: og_d.clone(),
                cursor: og_d.len(),
                is_multiline: true,
                rows: 3,
            },
            EditField {
                label: "OG Image".to_string(),
                value: og_i.clone(),
                buffer: og_i.clone(),
                cursor: og_i.len(),
                is_multiline: false,
                rows: 1,
            },
        ];
        // Always show Slug field immediately after Title. Editing it implicitly
        // locks the slug so future title renames won't auto-regenerate it.
        fields.insert(
            1,
            EditField {
                label: "Slug".to_string(),
                value: slug.clone(),
                buffer: slug.clone(),
                cursor: slug.len(),
                is_multiline: false,
                rows: 1,
            },
        );
        self.edit_modal = Some(EditModalState {
            title: "page-head".to_string(),
            fields,
            selected_field: 0,
            scroll_offset: 0,
            visible_fields: 6,
        });
        self.status = "Editing page head. Tab to navigate, Ctrl+S to save, Esc to cancel."
            .to_string();
    }

    fn open_footer_edit_modal(&mut self) {
        let custom = self.site.footer.custom_css.clone().unwrap_or_default();
        let fields = vec![
            EditField {
                label: "Footer ID".to_string(),
                value: self.site.footer.id.clone(),
                buffer: self.site.footer.id.clone(),
                cursor: self.site.footer.id.len(),
                is_multiline: false,
                rows: 1,
            },
            EditField {
                label: "Custom CSS".to_string(),
                value: custom.clone(),
                buffer: custom.clone(),
                cursor: custom.len(),
                is_multiline: false,
                rows: 1,
            },
        ];
        self.edit_modal = Some(EditModalState {
            title: "dd-footer".to_string(),
            fields,
            selected_field: 0,
            scroll_offset: 0,
            visible_fields: 4,
        });
        self.status = "Editing footer. Tab to navigate, Ctrl+S to save, Esc to cancel.".to_string();
    }

    fn open_header_root_edit_modal(&mut self) {
        let custom = self.site.header.custom_css.clone().unwrap_or_default();
        let fields = vec![
            EditField {
                label: "Header ID".to_string(),
                value: self.site.header.id.clone(),
                buffer: self.site.header.id.clone(),
                cursor: self.site.header.id.len(),
                is_multiline: false,
                rows: 1,
            },
            EditField {
                label: "Custom CSS".to_string(),
                value: custom.clone(),
                buffer: custom.clone(),
                cursor: custom.len(),
                is_multiline: false,
                rows: 1,
            },
        ];
        self.edit_modal = Some(EditModalState {
            title: "dd-header-root".to_string(),
            fields,
            selected_field: 0,
            scroll_offset: 0,
            visible_fields: 4,
        });
        self.status = "Editing header root. Tab to navigate, Ctrl+S to save, Esc to cancel."
            .to_string();
    }

    fn begin_edit_selected_component_primary(&mut self) {
        let page = self.current_page();
        if page.nodes.is_empty() {
            self.status = "No nodes to edit.".to_string();
            return;
        }

        let ni = self.selected_node.min(page.nodes.len() - 1);
        let component_to_edit = match &page.nodes[ni] {
            PageNode::Hero(_) => {
                self.status = "Use Enter on hero node for multi-field editing.".to_string();
                return;
            }
            PageNode::Section(section) => {
                let columns = section_columns_ref(section);
                let col_i = self.selected_column.min(columns.len().saturating_sub(1));
                let components = &columns[col_i].components;
                if let Some(ci) = component_index(components.len(), self.selected_component) {
                    match &components[ci] {
                        crate::model::SectionComponent::Banner(banner) => {
                            let fields = vec![
                                EditField {
                                    label: "Banner Class".to_string(),
                                    value: banner_class_to_str(banner.parent_class).to_string(),
                                    buffer: banner_class_to_str(banner.parent_class).to_string(),
                                    cursor: banner_class_to_str(banner.parent_class).len(),
                                    is_multiline: false,
                                    rows: 1,
                                },
                                EditField {
                                    label: "Data AOS".to_string(),
                                    value: parent_data_aos_to_str(banner.parent_data_aos).to_string(),
                                    buffer: parent_data_aos_to_str(banner.parent_data_aos).to_string(),
                                    cursor: parent_data_aos_to_str(banner.parent_data_aos).len(),
                                    is_multiline: false,
                                    rows: 1,
                                },
                                EditField {
                                    label: "Image URL".to_string(),
                                    value: banner.parent_image_url.clone(),
                                    buffer: banner.parent_image_url.clone(),
                                    cursor: banner.parent_image_url.len(),
                                    is_multiline: false,
                                    rows: 1,
                                },
                                EditField {
                                    label: "Image Alt".to_string(),
                                    value: banner.parent_image_alt.clone(),
                                    buffer: banner.parent_image_alt.clone(),
                                    cursor: banner.parent_image_alt.len(),
                                    is_multiline: false,
                                    rows: 1,
                                },
                            ];
                            Some(EditModalState {
                                title: "dd-banner".to_string(),
                                fields,
                                selected_field: 0,
                                scroll_offset: 0,
                                visible_fields: 6,
                            })
                        }
                        crate::model::SectionComponent::Alert(alert) => {
                            let fields = vec![
                                EditField {
                                    label: "Alert Type".to_string(),
                                    value: alert_type_to_str(alert.parent_type).to_string(),
                                    buffer: alert_type_to_str(alert.parent_type).to_string(),
                                    cursor: alert_type_to_str(alert.parent_type).len(),
                                    is_multiline: false,
                                    rows: 1,
                                },
                                EditField {
                                    label: "Alert Class".to_string(),
                                    value: alert_class_to_str(alert.parent_class).to_string(),
                                    buffer: alert_class_to_str(alert.parent_class).to_string(),
                                    cursor: alert_class_to_str(alert.parent_class).len(),
                                    is_multiline: false,
                                    rows: 1,
                                },
                                EditField {
                                    label: "Data AOS".to_string(),
                                    value: parent_data_aos_to_str(alert.parent_data_aos).to_string(),
                                    buffer: parent_data_aos_to_str(alert.parent_data_aos).to_string(),
                                    cursor: parent_data_aos_to_str(alert.parent_data_aos).len(),
                                    is_multiline: false,
                                    rows: 1,
                                },
                                EditField {
                                    label: "Title".to_string(),
                                    value: alert.parent_title.clone().unwrap_or_default(),
                                    buffer: alert.parent_title.clone().unwrap_or_default(),
                                    cursor: alert.parent_title.clone().unwrap_or_default().len(),
                                    is_multiline: false,
                                    rows: 1,
                                },
                                EditField {
                                    label: "Copy".to_string(),
                                    value: alert.parent_copy.clone(),
                                    buffer: alert.parent_copy.clone(),
                                    cursor: alert.parent_copy.len(),
                                    is_multiline: true,
                                    rows: 3, // textarea: 3 rows per dd-alert.md spec
                                },
                            ];
                            Some(EditModalState {
                                title: "dd-alert".to_string(),
                                fields,
                                selected_field: 0,
                                scroll_offset: 0,
                                visible_fields: 6,
                            })
                        }
                        crate::model::SectionComponent::Cta(cta) => {
                            let fields = vec![
                                EditField {
                                    label: "CTA Class".to_string(),
                                    value: cta_class_to_str(cta.parent_class).to_string(),
                                    buffer: cta_class_to_str(cta.parent_class).to_string(),
                                    cursor: cta_class_to_str(cta.parent_class).len(),
                                    is_multiline: false,
                                    rows: 1,
                                },
                                EditField {
                                    label: "Data AOS".to_string(),
                                    value: parent_data_aos_to_str(cta.parent_data_aos).to_string(),
                                    buffer: parent_data_aos_to_str(cta.parent_data_aos).to_string(),
                                    cursor: parent_data_aos_to_str(cta.parent_data_aos).len(),
                                    is_multiline: false,
                                    rows: 1,
                                },
                                EditField {
                                    label: "Image URL".to_string(),
                                    value: cta.parent_image_url.clone(),
                                    buffer: cta.parent_image_url.clone(),
                                    cursor: cta.parent_image_url.len(),
                                    is_multiline: false,
                                    rows: 1,
                                },
                                EditField {
                                    label: "Image Alt".to_string(),
                                    value: cta.parent_image_alt.clone(),
                                    buffer: cta.parent_image_alt.clone(),
                                    cursor: cta.parent_image_alt.len(),
                                    is_multiline: false,
                                    rows: 1,
                                },
                                EditField {
                                    label: "Title".to_string(),
                                    value: cta.parent_title.clone(),
                                    buffer: cta.parent_title.clone(),
                                    cursor: cta.parent_title.len(),
                                    is_multiline: false,
                                    rows: 1,
                                },
                                EditField {
                                    label: "Subtitle".to_string(),
                                    value: cta.parent_subtitle.clone(),
                                    buffer: cta.parent_subtitle.clone(),
                                    cursor: cta.parent_subtitle.len(),
                                    is_multiline: false,
                                    rows: 1,
                                },
                                EditField {
                                    label: "Copy".to_string(),
                                    value: cta.parent_copy.clone(),
                                    buffer: cta.parent_copy.clone(),
                                    cursor: cta.parent_copy.len(),
                                    is_multiline: true,
                                    rows: 5, // textarea: 5 rows per dd-cta.md spec
                                },
                                EditField {
                                    label: "Link URL".to_string(),
                                    value: cta.parent_link_url.clone().unwrap_or_default(),
                                    buffer: cta.parent_link_url.clone().unwrap_or_default(),
                                    cursor: cta.parent_link_url.clone().unwrap_or_default().len(),
                                    is_multiline: false,
                                    rows: 1,
                                },
                                EditField {
                                    label: "Link Target".to_string(),
                                    value: cta
                                        .parent_link_target
                                        .map(child_link_target_to_str)
                                        .unwrap_or("_self")
                                        .to_string(),
                                    buffer: cta
                                        .parent_link_target
                                        .map(child_link_target_to_str)
                                        .unwrap_or("_self")
                                        .to_string(),
                                    cursor: cta
                                        .parent_link_target
                                        .map(child_link_target_to_str)
                                        .unwrap_or("_self")
                                        .len(),
                                    is_multiline: false,
                                    rows: 1,
                                },
                                EditField {
                                    label: "Link Label".to_string(),
                                    value: cta.parent_link_label.clone().unwrap_or_default(),
                                    buffer: cta.parent_link_label.clone().unwrap_or_default(),
                                    cursor: cta.parent_link_label.clone().unwrap_or_default().len(),
                                    is_multiline: false,
                                    rows: 1,
                                },
                            ];
                            Some(EditModalState {
                                title: "dd-cta".to_string(),
                                fields,
                                selected_field: 0,
                                scroll_offset: 0,
                                visible_fields: 6,
                            })
                        }
                        crate::model::SectionComponent::Blockquote(blockquote) => {
                            let fields = vec![
                                EditField {
                                    label: "Data AOS".to_string(),
                                    value: parent_data_aos_to_str(blockquote.parent_data_aos)
                                        .to_string(),
                                    buffer: parent_data_aos_to_str(blockquote.parent_data_aos)
                                        .to_string(),
                                    cursor: parent_data_aos_to_str(blockquote.parent_data_aos).len(),
                                    is_multiline: false,
                                    rows: 1,
                                },
                                EditField {
                                    label: "Image URL".to_string(),
                                    value: blockquote.parent_image_url.clone(),
                                    buffer: blockquote.parent_image_url.clone(),
                                    cursor: blockquote.parent_image_url.len(),
                                    is_multiline: false,
                                    rows: 1,
                                },
                                EditField {
                                    label: "Image Alt".to_string(),
                                    value: blockquote.parent_image_alt.clone(),
                                    buffer: blockquote.parent_image_alt.clone(),
                                    cursor: blockquote.parent_image_alt.len(),
                                    is_multiline: false,
                                    rows: 1,
                                },
                                EditField {
                                    label: "Person Name".to_string(),
                                    value: blockquote.parent_name.clone(),
                                    buffer: blockquote.parent_name.clone(),
                                    cursor: blockquote.parent_name.len(),
                                    is_multiline: false,
                                    rows: 1,
                                },
                                EditField {
                                    label: "Person Title".to_string(),
                                    value: blockquote.parent_role.clone(),
                                    buffer: blockquote.parent_role.clone(),
                                    cursor: blockquote.parent_role.len(),
                                    is_multiline: false,
                                    rows: 1,
                                },
                                EditField {
                                    label: "Copy".to_string(),
                                    value: blockquote.parent_copy.clone(),
                                    buffer: blockquote.parent_copy.clone(),
                                    cursor: blockquote.parent_copy.len(),
                                    is_multiline: true,
                                    rows: 5, // textarea: 5 rows per dd-blockquote.md spec
                                },
                            ];
                            Some(EditModalState {
                                title: "dd-blockquote".to_string(),
                                fields,
                                selected_field: 0,
                                scroll_offset: 0,
                                visible_fields: 6,
                            })
                        }
                        crate::model::SectionComponent::Modal(modal) => {
                            let fields = vec![
                                EditField {
                                    label: "Title".to_string(),
                                    value: modal.parent_title.clone(),
                                    buffer: modal.parent_title.clone(),
                                    cursor: modal.parent_title.len(),
                                    is_multiline: false,
                                    rows: 1,
                                },
                                EditField {
                                    label: "Copy".to_string(),
                                    value: modal.parent_copy.clone(),
                                    buffer: modal.parent_copy.clone(),
                                    cursor: modal.parent_copy.len(),
                                    is_multiline: true,
                                    rows: 5, // textarea: 5 rows per dd-modal.md spec
                                },
                            ];
                            Some(EditModalState {
                                title: "dd-modal".to_string(),
                                fields,
                                selected_field: 0,
                                scroll_offset: 0,
                                visible_fields: 6,
                            })
                        }
                        crate::model::SectionComponent::Filmstrip(filmstrip) => {
                            let fields = vec![
                                EditField {
                                    label: "Filmstrip Type".to_string(),
                                    value: filmstrip_type_to_str(filmstrip.parent_type)
                                        .to_string(),
                                    buffer: filmstrip_type_to_str(filmstrip.parent_type)
                                        .to_string(),
                                    cursor: filmstrip_type_to_str(filmstrip.parent_type).len(),
                                    is_multiline: false,
                                    rows: 1,
                                },
                                EditField {
                                    label: "Data AOS".to_string(),
                                    value: parent_data_aos_to_str(filmstrip.parent_data_aos)
                                        .to_string(),
                                    buffer: parent_data_aos_to_str(filmstrip.parent_data_aos)
                                        .to_string(),
                                    cursor: parent_data_aos_to_str(filmstrip.parent_data_aos).len(),
                                    is_multiline: false,
                                    rows: 1,
                                },
                            ];
                            Some(EditModalState {
                                title: "dd-filmstrip".to_string(),
                                fields,
                                selected_field: 0,
                                scroll_offset: 0,
                                visible_fields: 6,
                            })
                        }
                        crate::model::SectionComponent::Accordion(accordion) => {
                            let fields = vec![
                                EditField {
                                    label: "Accordion Type".to_string(),
                                    value: accordion_type_to_str(accordion.parent_type)
                                        .to_string(),
                                    buffer: accordion_type_to_str(accordion.parent_type)
                                        .to_string(),
                                    cursor: accordion_type_to_str(accordion.parent_type).len(),
                                    is_multiline: false,
                                    rows: 1,
                                },
                                EditField {
                                    label: "Accordion Class".to_string(),
                                    value: accordion_class_to_str(accordion.parent_class)
                                        .to_string(),
                                    buffer: accordion_class_to_str(accordion.parent_class)
                                        .to_string(),
                                    cursor: accordion_class_to_str(accordion.parent_class).len(),
                                    is_multiline: false,
                                    rows: 1,
                                },
                                EditField {
                                    label: "Data AOS".to_string(),
                                    value: parent_data_aos_to_str(accordion.parent_data_aos).to_string(),
                                    buffer: parent_data_aos_to_str(accordion.parent_data_aos).to_string(),
                                    cursor: parent_data_aos_to_str(accordion.parent_data_aos).len(),
                                    is_multiline: false,
                                    rows: 1,
                                },
                                EditField {
                                    label: "Group Name".to_string(),
                                    value: accordion.parent_group_name.clone(),
                                    buffer: accordion.parent_group_name.clone(),
                                    cursor: accordion.parent_group_name.len(),
                                    is_multiline: false,
                                    rows: 1,
                                },
                            ];
                            Some(EditModalState {
                                title: "dd-accordion".to_string(),
                                fields,
                                selected_field: 0,
                                scroll_offset: 0,
                                visible_fields: 6,
                            })
                        }
                        crate::model::SectionComponent::Image(image) => {
                            let url = image.parent_link_url.clone().unwrap_or_default();
                            let target = image
                                .parent_link_target
                                .map(child_link_target_to_str)
                                .unwrap_or("_self")
                                .to_string();
                            let fields = vec![
                                EditField {
                                    label: "Data AOS".to_string(),
                                    value: parent_data_aos_to_str(image.parent_data_aos).to_string(),
                                    buffer: parent_data_aos_to_str(image.parent_data_aos).to_string(),
                                    cursor: parent_data_aos_to_str(image.parent_data_aos).len(),
                                    is_multiline: false,
                                    rows: 1,
                                },
                                EditField {
                                    label: "Image URL".to_string(),
                                    value: image.parent_image_url.clone(),
                                    buffer: image.parent_image_url.clone(),
                                    cursor: image.parent_image_url.len(),
                                    is_multiline: false,
                                    rows: 1,
                                },
                                EditField {
                                    label: "Image Alt".to_string(),
                                    value: image.parent_image_alt.clone(),
                                    buffer: image.parent_image_alt.clone(),
                                    cursor: image.parent_image_alt.len(),
                                    is_multiline: false,
                                    rows: 1,
                                },
                                EditField {
                                    label: "Link URL".to_string(),
                                    value: url.clone(),
                                    buffer: url.clone(),
                                    cursor: url.len(),
                                    is_multiline: false,
                                    rows: 1,
                                },
                                EditField {
                                    label: "Link Target".to_string(),
                                    value: target.clone(),
                                    buffer: target.clone(),
                                    cursor: target.len(),
                                    is_multiline: false,
                                    rows: 1,
                                },
                            ];
                            Some(EditModalState {
                                title: "dd-image".to_string(),
                                fields,
                                selected_field: 0,
                                scroll_offset: 0,
                                visible_fields: 6,
                            })
                        }
                        crate::model::SectionComponent::RichText(rt) => {
                            let class = rt.parent_class.clone().unwrap_or_default();
                            let fields = vec![
                                EditField {
                                    label: "Parent Class".to_string(),
                                    value: class.clone(),
                                    buffer: class.clone(),
                                    cursor: class.len(),
                                    is_multiline: false,
                                    rows: 1,
                                },
                                EditField {
                                    label: "Data AOS".to_string(),
                                    value: parent_data_aos_to_str(rt.parent_data_aos).to_string(),
                                    buffer: parent_data_aos_to_str(rt.parent_data_aos).to_string(),
                                    cursor: parent_data_aos_to_str(rt.parent_data_aos).len(),
                                    is_multiline: false,
                                    rows: 1,
                                },
                                EditField {
                                    label: "Copy".to_string(),
                                    value: rt.parent_copy.clone(),
                                    buffer: rt.parent_copy.clone(),
                                    cursor: rt.parent_copy.len(),
                                    is_multiline: true,
                                    rows: 5,
                                },
                            ];
                            Some(EditModalState {
                                title: "dd-rich_text".to_string(),
                                fields,
                                selected_field: 0,
                                scroll_offset: 0,
                                visible_fields: 6,
                            })
                        }
                        crate::model::SectionComponent::Navigation(nav) => {
                            let type_str = navigation_type_to_str(nav.parent_type).to_string();
                            let class_str = navigation_class_to_str(nav.parent_class).to_string();
                            let aos_str =
                                parent_data_aos_to_str(nav.parent_data_aos).to_string();
                            let fields = vec![
                                EditField {
                                    label: "Nav Type".to_string(),
                                    value: type_str.clone(),
                                    buffer: type_str.clone(),
                                    cursor: type_str.len(),
                                    is_multiline: false,
                                    rows: 1,
                                },
                                EditField {
                                    label: "Nav Class".to_string(),
                                    value: class_str.clone(),
                                    buffer: class_str.clone(),
                                    cursor: class_str.len(),
                                    is_multiline: false,
                                    rows: 1,
                                },
                                EditField {
                                    label: "Data AOS".to_string(),
                                    value: aos_str.clone(),
                                    buffer: aos_str.clone(),
                                    cursor: aos_str.len(),
                                    is_multiline: false,
                                    rows: 1,
                                },
                                EditField {
                                    label: "Parent Width".to_string(),
                                    value: nav.parent_width.clone(),
                                    buffer: nav.parent_width.clone(),
                                    cursor: nav.parent_width.len(),
                                    is_multiline: false,
                                    rows: 1,
                                },
                            ];
                            Some(EditModalState {
                                title: "dd-navigation".to_string(),
                                fields,
                                selected_field: 0,
                                scroll_offset: 0,
                                visible_fields: 6,
                            })
                        }
                        crate::model::SectionComponent::HeaderSearch(hs) => {
                            let aos = parent_data_aos_to_str(hs.parent_data_aos).to_string();
                            let fields = vec![
                                EditField {
                                    label: "Parent Width".to_string(),
                                    value: hs.parent_width.clone(),
                                    buffer: hs.parent_width.clone(),
                                    cursor: hs.parent_width.len(),
                                    is_multiline: false,
                                    rows: 1,
                                },
                                EditField {
                                    label: "Data AOS".to_string(),
                                    value: aos.clone(),
                                    buffer: aos.clone(),
                                    cursor: aos.len(),
                                    is_multiline: false,
                                    rows: 1,
                                },
                            ];
                            Some(EditModalState {
                                title: "dd-header-search".to_string(),
                                fields,
                                selected_field: 0,
                                scroll_offset: 0,
                                visible_fields: 6,
                            })
                        }
                        crate::model::SectionComponent::HeaderMenu(hm) => {
                            let aos = parent_data_aos_to_str(hm.parent_data_aos).to_string();
                            let fields = vec![
                                EditField {
                                    label: "Parent Width".to_string(),
                                    value: hm.parent_width.clone(),
                                    buffer: hm.parent_width.clone(),
                                    cursor: hm.parent_width.len(),
                                    is_multiline: false,
                                    rows: 1,
                                },
                                EditField {
                                    label: "Data AOS".to_string(),
                                    value: aos.clone(),
                                    buffer: aos.clone(),
                                    cursor: aos.len(),
                                    is_multiline: false,
                                    rows: 1,
                                },
                            ];
                            Some(EditModalState {
                                title: "dd-header-menu".to_string(),
                                fields,
                                selected_field: 0,
                                scroll_offset: 0,
                                visible_fields: 6,
                            })
                        }
                        _ => {
                            // Fall back to single-field editing for components with collection items
                            return self.begin_edit_selected_component_single_field();
                        }
                    }
                } else {
                    None
                }
            }
        };

        if let Some(modal) = component_to_edit {
            self.edit_modal = Some(modal);
            self.status = "Multi-field edit: Tab/Up/Down to navigate, type to edit, Ctrl+S to save, Esc to cancel.".to_string();
        } else {
            self.status = "No component selected.".to_string();
        }
    }

    fn begin_edit_selected_component_single_field(&mut self) {
        let selected = {
            let page = self.current_page();
            if page.nodes.is_empty() {
                None
            } else {
                let ni = self.selected_node.min(page.nodes.len() - 1);
                match &page.nodes[ni] {
                    PageNode::Hero(_) => None,
                    PageNode::Section(section) => {
                        let columns = section_columns_ref(section);
                        let col_i = self.selected_column.min(columns.len().saturating_sub(1));
                        let components = &columns[col_i].components;
                        if let Some(ci) = component_index(components.len(), self.selected_component)
                        {
                            match &components[ci] {
                                crate::model::SectionComponent::Cta(cta) => Some((
                                    InputMode::EditCtaClass,
                                    cta_class_to_str(cta.parent_class).to_string(),
                                )),
                                crate::model::SectionComponent::Filmstrip(filmstrip) => Some((
                                    InputMode::EditFilmstripType,
                                    filmstrip_type_to_str(filmstrip.parent_type).to_string(),
                                )),
                                crate::model::SectionComponent::Milestones(milestones) => Some((
                                    InputMode::EditMilestonesDataAos,
                                    parent_data_aos_to_str(milestones.parent_data_aos).to_string(),
                                )),
                                crate::model::SectionComponent::Modal(modal) => {
                                    Some((InputMode::EditModalTitle, modal.parent_title.clone()))
                                }
                                crate::model::SectionComponent::Slider(slider) => {
                                    Some((InputMode::EditSliderTitle, slider.parent_title.clone()))
                                }
                                crate::model::SectionComponent::Card(card) => Some((
                                    InputMode::EditCardType,
                                    card_type_to_str(card.parent_type).to_string(),
                                )),
                                crate::model::SectionComponent::Accordion(acc) => Some((
                                    InputMode::EditAccordionType,
                                    accordion_type_to_str(acc.parent_type).to_string(),
                                )),
                                crate::model::SectionComponent::Blockquote(v) => Some((
                                    InputMode::EditBlockquoteDataAos,
                                    parent_data_aos_to_str(v.parent_data_aos).to_string(),
                                )),
                                crate::model::SectionComponent::Alternating(alt) => Some((
                                    InputMode::EditAlternatingType,
                                    alternating_type_to_str(alt.parent_type).to_string(),
                                )),
                                _ => None,
                            }
                        } else {
                            None
                        }
                    }
                }
            }
        };

        let Some((mode, value)) = selected else {
            self.status = "Component editing not available for this type.".to_string();
            return;
        };
        self.input_mode = Some(mode);
        self.input_buffer = value;
        self.input_cursor = self.input_buffer.chars().count();
        self.status = "Single-field edit: Enter to save, Esc to cancel.".to_string();
    }

    fn selected_section_column_total(&mut self) -> Option<usize> {
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

    fn details_max_scroll(&self) -> usize {
        let visible_rows = self.details_area.height.saturating_sub(2) as usize;
        if visible_rows == 0 {
            return 0;
        }
        let detail_width = self.details_area.width.saturating_sub(2) as usize;
        if detail_width == 0 {
            return 0;
        }
        let total_rows = self.details_text(detail_width).lines().count().max(1);
        total_rows.saturating_sub(visible_rows)
    }

    fn scroll_details_by(&mut self, delta: isize) {
        let max_scroll = self.details_max_scroll() as isize;
        let next = self.details_scroll_row as isize + delta;
        self.details_scroll_row = next.clamp(0, max_scroll) as usize;
    }

    /// Recompute the JSON snapshot of `self.site` and set `dirty` if it
    /// differs from `last_saved_json`. Idempotent: re-calling on an already
    /// dirty app does NOT advance `dirty_since`, preserving the original
    /// debounce anchor.
    fn mark_dirty_if_changed(&mut self) {
        let current = match serde_json::to_string(&self.site) {
            Ok(s) => s,
            Err(_) => return,
        };
        if current != self.last_saved_json {
            if !self.dirty {
                self.dirty_since = Some(std::time::Instant::now());
            }
            self.dirty = true;
        }
    }

    /// Write `self.site` to `path` AND to `<path>.backup`. Both writes share
    /// a single serialization so the two files are guaranteed byte-identical.
    /// Refreshes the saved snapshot and clears the dirty flag on success.
    fn commit_save_with_backup(&mut self, path: &std::path::Path) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(&self.site)?;
        std::fs::write(path, &json)?;
        let backup = backup_path_for(path);
        std::fs::write(&backup, &json)?;
        self.last_saved_json = serde_json::to_string(&self.site).unwrap_or_default();
        self.dirty = false;
        self.dirty_since = None;
        self.path = Some(path.to_path_buf());
        Ok(())
    }

    /// If the site is dirty, has a path, and the debounce window has elapsed,
    /// write `self.site` to the active path and refresh the saved snapshot.
    /// Errors are surfaced as a warning toast and leave `dirty` set so the
    /// next tick can retry.
    fn tick_autosave(&mut self, now: std::time::Instant) {
        if !self.dirty {
            return;
        }
        let Some(since) = self.dirty_since else {
            // Defensive: dirty without a timestamp shouldn't happen; treat as
            // freshly dirty.
            self.dirty_since = Some(now);
            return;
        };
        if now.duration_since(since) < AUTOSAVE_DEBOUNCE {
            return;
        }
        let Some(path) = self.path.clone() else {
            return;
        };
        match crate::storage::save_site(&path, &self.site) {
            Ok(()) => {
                self.last_saved_json =
                    serde_json::to_string(&self.site).unwrap_or_default();
                self.dirty = false;
                self.dirty_since = None;
            }
            Err(e) => {
                let msg = format!("Autosave failed: {}", e);
                self.push_toast(ToastLevel::Warning, msg);
            }
        }
    }
}

fn contains(rect: Rect, x: u16, y: u16) -> bool {
    x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height
}

fn component_index(total: usize, selected_component: usize) -> Option<usize> {
    if total == 0 {
        None
    } else {
        Some(selected_component.min(total - 1))
    }
}

fn section_columns_ref(section: &crate::model::DdSection) -> Vec<SectionColumn> {
    section.columns.clone()
}

fn normalize_section_columns(section: &mut crate::model::DdSection) {
    if section.columns.is_empty() {
        section.columns.push(SectionColumn {
            id: "column-1".to_string(),
            width_class: "dd-u-1-1".to_string(),
            components: Vec::new(),
        });
    }
}

fn nested_index(total: usize, selected_nested_item: usize) -> Option<usize> {
    if total == 0 {
        None
    } else {
        Some(selected_nested_item.min(total - 1))
    }
}

fn section_ascii_map(
    section: &crate::model::DdSection,
    selected_column: usize,
    panel_width: usize,
) -> String {
    const MAX_COMPONENT_ROWS: usize = 4;

    let inner_width = panel_width.saturating_sub(4).max(12);
    let columns = section_columns_ref(section);
    if columns.is_empty() {
        return "(no columns)".to_string();
    }
    let active = selected_column.min(columns.len().saturating_sub(1));

    let mut lines = vec![
        fit_ascii_cell("SECTION", inner_width),
        fit_ascii_cell(&format!("id: {}", section.id), inner_width),
        fit_ascii_cell(
            &format!(
                "title: {}",
                section.section_title.as_deref().unwrap_or("(none)")
            ),
            inner_width,
        ),
        fit_ascii_cell(
            &format!(
                "class: {}",
                section_class_to_str(
                    section
                        .section_class
                        .unwrap_or(crate::model::SectionClass::FullContained)
                )
            ),
            inner_width,
        ),
        fit_ascii_cell("items:", inner_width),
    ];

    let item_boxes = columns
        .iter()
        .enumerate()
        .map(|(idx, col)| {
            let marker = if idx == active { "*" } else { "-" };
            let item_inner_width = section_item_ascii_inner_width(&col.width_class, inner_width);
            let item_border = format!("+{}+", "-".repeat(item_inner_width + 2));
            let mut box_lines = vec![
                item_border.clone(),
                format!(
                    "| {} |",
                    fit_ascii_cell(&format!("{marker} item: {}", col.id), item_inner_width)
                ),
                format!(
                    "| {} |",
                    fit_ascii_cell(&format!("width: {}", col.width_class), item_inner_width)
                ),
            ];
            if col.components.is_empty() {
                box_lines.push(format!(
                    "| {} |",
                    fit_ascii_cell("(empty)", item_inner_width)
                ));
            } else {
                for component in col.components.iter().take(MAX_COMPONENT_ROWS) {
                    match component {
                        crate::model::SectionComponent::Card(card) => {
                            box_lines.push(format!(
                                "| {} |",
                                fit_ascii_cell("- dd-card", item_inner_width)
                            ));
                            for line in card_items_ascii_lines(card, item_inner_width) {
                                box_lines.push(format!(
                                    "| {} |",
                                    fit_ascii_cell(&line, item_inner_width)
                                ));
                            }
                        }
                        _ => {
                            box_lines.push(format!(
                                "| {} |",
                                fit_ascii_cell(
                                    &format!("- {}", component_blueprint_label(component)),
                                    item_inner_width
                                )
                            ));
                        }
                    }
                }
                let more = col.components.len().saturating_sub(MAX_COMPONENT_ROWS);
                if more > 0 {
                    box_lines.push(format!(
                        "| {} |",
                        fit_ascii_cell(&format!("+{more} more"), item_inner_width)
                    ));
                }
            }
            box_lines.push(item_border);
            box_lines
        })
        .collect::<Vec<_>>();

    let item_box_widths = item_boxes
        .iter()
        .map(|item| item.first().map(|s| s.chars().count()).unwrap_or(0))
        .collect::<Vec<_>>();

    let gap = 2usize;
    let mut row_groups: Vec<Vec<usize>> = Vec::new();
    let mut current_row: Vec<usize> = Vec::new();
    let mut current_row_width = 0usize;
    for (idx, width) in item_box_widths.iter().copied().enumerate() {
        let next = if current_row.is_empty() {
            width
        } else {
            current_row_width + gap + width
        };
        if !current_row.is_empty() && next > inner_width {
            row_groups.push(current_row);
            current_row = vec![idx];
            current_row_width = width;
        } else {
            current_row.push(idx);
            current_row_width = next;
        }
    }
    if !current_row.is_empty() {
        row_groups.push(current_row);
    }

    for (row_idx, row) in row_groups.iter().enumerate() {
        if row_idx > 0 {
            lines.push(fit_ascii_cell("", inner_width));
        }
        let max_height = row
            .iter()
            .map(|idx| item_boxes[*idx].len())
            .max()
            .unwrap_or(0);
        for line_idx in 0..max_height {
            let mut composed = String::new();
            for (pos, idx) in row.iter().enumerate() {
                if pos > 0 {
                    composed.push_str("  ");
                }
                let box_lines = &item_boxes[*idx];
                let box_width = item_box_widths[*idx];
                let part = box_lines
                    .get(line_idx)
                    .cloned()
                    .unwrap_or_else(|| " ".repeat(box_width));
                composed.push_str(&part);
            }
            lines.push(fit_ascii_cell(&composed, inner_width));
        }
    }

    let border = format!("+{}+", "-".repeat(inner_width + 2));
    let mut out = Vec::new();
    out.push(border.clone());
    for line in lines {
        out.push(format!("| {} |", line));
    }
    out.push(border);
    out.join("\n")
}

fn header_ascii_map(
    header: &crate::model::DdHeader,
    selected_section: usize,
    selected_column: usize,
    panel_width: usize,
) -> String {
    let inner_width = panel_width.saturating_sub(4).max(12);

    let mut lines = vec![
        fit_ascii_cell("HEADER", inner_width),
        fit_ascii_cell(&format!("id: {}", header.id), inner_width),
        fit_ascii_cell(
            &format!(
                "custom_css: {}",
                header.custom_css.as_deref().unwrap_or("(none)")
            ),
            inner_width,
        ),
        fit_ascii_cell(
            &format!(
                "alert: {}",
                if header.alert.is_some() { "yes" } else { "(none)" }
            ),
            inner_width,
        ),
        fit_ascii_cell("sections:", inner_width),
    ];

    if header.sections.is_empty() {
        lines.push(fit_ascii_cell(
            "(no sections - press '/' to add)",
            inner_width,
        ));
    } else {
        let active_section = selected_section.min(header.sections.len().saturating_sub(1));
        for (s_idx, section) in header.sections.iter().enumerate() {
            let s_marker = if s_idx == active_section { "*" } else { "-" };
            lines.push(fit_ascii_cell(
                &format!("{s_marker} section: {}", section.id),
                inner_width,
            ));

            if section.columns.is_empty() {
                lines.push(fit_ascii_cell("    (no columns)", inner_width));
            } else {
                let active_col = if s_idx == active_section {
                    selected_column.min(section.columns.len().saturating_sub(1))
                } else {
                    0
                };
                for (c_idx, col) in section.columns.iter().enumerate() {
                    let c_marker = if s_idx == active_section && c_idx == active_col {
                        "*"
                    } else {
                        "-"
                    };
                    lines.push(fit_ascii_cell(
                        &format!("    {c_marker} column: {} [{}]", col.id, col.width_class),
                        inner_width,
                    ));
                    if col.components.is_empty() {
                        lines.push(fit_ascii_cell("        (empty)", inner_width));
                    } else {
                        for comp in col.components.iter() {
                            lines.push(fit_ascii_cell(
                                &format!("        - {}", component_label(comp)),
                                inner_width,
                            ));
                        }
                    }
                }
            }
        }
    }

    let border = format!("+{}+", "-".repeat(inner_width + 2));
    let mut out = Vec::new();
    out.push(border.clone());
    for line in lines {
        out.push(format!("| {} |", line));
    }
    out.push(border);
    out.join("\n")
}

fn card_items_ascii_lines(
    card: &crate::model::DdCard,
    container_inner_width: usize,
) -> Vec<String> {
    if card.items.is_empty() {
        return vec![fit_ascii_cell("(empty)", container_inner_width)];
    }

    let child_inner_width = section_item_ascii_inner_width(&card.parent_width, container_inner_width)
        .min(container_inner_width.saturating_sub(6))
        .max(10);
    let child_border = format!("+{}+", "-".repeat(child_inner_width + 2));

    let child_boxes = card
        .items
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            vec![
                child_border.clone(),
                format!(
                    "| {} |",
                    fit_ascii_cell(&format!("card {}:", idx + 1), child_inner_width)
                ),
                format!(
                    "| {} |",
                    fit_ascii_cell(&format!("title: {}", item.child_title), child_inner_width)
                ),
                child_border.clone(),
            ]
        })
        .collect::<Vec<_>>();

    let box_widths = child_boxes
        .iter()
        .map(|b| b.first().map(|s| s.chars().count()).unwrap_or(0))
        .collect::<Vec<_>>();

    let gap = 2usize;
    let mut row_groups: Vec<Vec<usize>> = Vec::new();
    let mut current_row: Vec<usize> = Vec::new();
    let mut current_row_width = 0usize;
    for (idx, width) in box_widths.iter().copied().enumerate() {
        let next = if current_row.is_empty() {
            width
        } else {
            current_row_width + gap + width
        };
        if !current_row.is_empty() && next > container_inner_width {
            row_groups.push(current_row);
            current_row = vec![idx];
            current_row_width = width;
        } else {
            current_row.push(idx);
            current_row_width = next;
        }
    }
    if !current_row.is_empty() {
        row_groups.push(current_row);
    }

    let mut lines = Vec::new();
    for (row_idx, row) in row_groups.iter().enumerate() {
        if row_idx > 0 {
            lines.push(String::new());
        }
        let row_height = row
            .iter()
            .map(|idx| child_boxes[*idx].len())
            .max()
            .unwrap_or(0);
        for line_idx in 0..row_height {
            let mut composed = String::new();
            for (pos, idx) in row.iter().enumerate() {
                if pos > 0 {
                    composed.push_str("  ");
                }
                let part = child_boxes[*idx]
                    .get(line_idx)
                    .cloned()
                    .unwrap_or_else(|| " ".repeat(box_widths[*idx]));
                composed.push_str(&part);
            }
            lines.push(composed);
        }
    }
    lines
}

fn section_item_ascii_inner_width(width_class: &str, section_inner_width: usize) -> usize {
    let min_inner = 12usize;
    // Upper bound chosen so a full-width (ratio 1.0) box renders exactly the
    // same total row width as two half-width (ratio 0.5) boxes + 2-char gap:
    // both resolve to (section_inner_width - 2). Previously inner-10, which
    // left the 1-1 row 4 chars short and misaligned the right edge.
    let max_inner = section_inner_width.saturating_sub(6).max(min_inner);
    let ratio = resolve_dd_u_ratio_for_panel(width_class, section_inner_width)
        .map(|(num, den)| (num as f64 / den as f64).clamp(0.1, 1.0))
        .unwrap_or(1.0);

    // Compute using total box width first so row packing includes border/padding footprint.
    // Box width = inner + 4 (left/right borders + spaces).
    // Subtract a small safety margin to avoid rounding forcing 50/50 items onto separate rows.
    let box_target = ((section_inner_width as f64) * ratio).floor() as isize - 2;
    let inner_target = box_target - 4;
    (inner_target as usize).clamp(min_inner, max_inner)
}

fn resolve_dd_u_ratio_for_panel(width_class: &str, panel_chars: usize) -> Option<(usize, usize)> {
    let current_bp = breakpoint_for_panel_chars(panel_chars);
    let mut base: Option<(usize, usize)> = None;
    let mut sm: Option<(usize, usize)> = None;
    let mut md: Option<(usize, usize)> = None;
    let mut lg: Option<(usize, usize)> = None;
    let mut xl: Option<(usize, usize)> = None;
    let mut xxl: Option<(usize, usize)> = None;

    for token in width_class.split_whitespace() {
        match parse_dd_u_token_ratio(token) {
            Some((ResponsiveBp::Base, ratio)) => base = Some(ratio),
            Some((ResponsiveBp::Sm, ratio)) => sm = Some(ratio),
            Some((ResponsiveBp::Md, ratio)) => md = Some(ratio),
            Some((ResponsiveBp::Lg, ratio)) => lg = Some(ratio),
            Some((ResponsiveBp::Xl, ratio)) => xl = Some(ratio),
            Some((ResponsiveBp::Xxl, ratio)) => xxl = Some(ratio),
            None => {}
        }
    }

    let ordered = [base, sm, md, lg, xl, xxl];
    let idx = current_bp.index();
    for i in (0..=idx).rev() {
        if let Some(ratio) = ordered[i] {
            return Some(ratio);
        }
    }
    for ratio in ordered.iter().skip(idx + 1).flatten() {
        return Some(*ratio);
    }
    None
}

fn parse_dd_u_token_ratio(token: &str) -> Option<(ResponsiveBp, (usize, usize))> {
    let value = token.strip_prefix("dd-u-")?;
    let parts = value.split('-').collect::<Vec<_>>();
    let (bp, num_raw, den_raw) = match parts.as_slice() {
        [num, den] => (ResponsiveBp::Base, *num, *den),
        [bp, num, den] => (
            match *bp {
                "sm" => ResponsiveBp::Sm,
                "md" => ResponsiveBp::Md,
                "lg" => ResponsiveBp::Lg,
                "xl" => ResponsiveBp::Xl,
                "xxl" => ResponsiveBp::Xxl,
                _ => return None,
            },
            *num,
            *den,
        ),
        _ => return None,
    };
    let num = num_raw.parse::<usize>().ok()?;
    let den = den_raw.parse::<usize>().ok()?;
    if den == 0 || num == 0 {
        return None;
    }
    Some((bp, (num.min(den), den)))
}

#[derive(Clone, Copy)]
enum ResponsiveBp {
    Base,
    Sm,
    Md,
    Lg,
    Xl,
    Xxl,
}

impl ResponsiveBp {
    fn index(self) -> usize {
        match self {
            ResponsiveBp::Base => 0,
            ResponsiveBp::Sm => 1,
            ResponsiveBp::Md => 2,
            ResponsiveBp::Lg => 3,
            ResponsiveBp::Xl => 4,
            ResponsiveBp::Xxl => 5,
        }
    }
}

fn breakpoint_for_panel_chars(panel_chars: usize) -> ResponsiveBp {
    if panel_chars >= 180 {
        ResponsiveBp::Xxl
    } else if panel_chars >= 150 {
        ResponsiveBp::Xl
    } else if panel_chars >= 120 {
        ResponsiveBp::Lg
    } else if panel_chars >= 90 {
        ResponsiveBp::Md
    } else if panel_chars >= 60 {
        ResponsiveBp::Sm
    } else {
        ResponsiveBp::Base
    }
}

fn hero_ascii_map(hero: &crate::model::DdHero, panel_width: usize) -> String {
    let inner_width = panel_width.saturating_sub(4).max(8);
    let border = format!("+{}+", "-".repeat(inner_width + 2));
    let lines = [
        fit_ascii_cell("HERO", inner_width),
        fit_ascii_cell(
            &format!(
                "class: {}",
                hero_image_class_to_str(
                    hero.parent_class
                        .unwrap_or(crate::model::HeroImageClass::FullFull)
                ),
            ),
            inner_width,
        ),
        fit_ascii_cell(
            &format!(
                "aos: {}",
                parent_data_aos_to_str(hero.parent_data_aos.unwrap_or(crate::model::HeroAos::FadeIn))
            ),
            inner_width,
        ),
        fit_ascii_cell(
            &format!(
                "custom_css: {}",
                hero.parent_custom_css.as_deref().unwrap_or("(none)")
            ),
            inner_width,
        ),
        fit_ascii_cell(&format!("title: {}", hero.parent_title), inner_width),
        fit_ascii_cell(&format!("subtitle: {}", hero.parent_subtitle), inner_width),
        fit_ascii_cell(
            &format!(
                "cta: {} -> {}",
                hero.link_1_label.as_deref().unwrap_or("(none)"),
                hero.link_1_url.as_deref().unwrap_or("(none)")
            ),
            inner_width,
        ),
        fit_ascii_cell(
            &format!(
                "cta_2: {} -> {}",
                hero.link_2_label.as_deref().unwrap_or("(none)"),
                hero.link_2_url.as_deref().unwrap_or("(none)")
            ),
            inner_width,
        ),
        fit_ascii_cell(&format!("image: {}", hero.parent_image_url), inner_width),
    ];
    let mut out = Vec::new();
    out.push(border.clone());
    for line in lines {
        out.push(format!("| {} |", line));
    }
    out.push(border);
    out.join("\n")
}

fn section_class_to_str(v: crate::model::SectionClass) -> &'static str {
    match v {
        crate::model::SectionClass::Contained => "-contained",
        crate::model::SectionClass::ContainedMd => "-contained-md",
        crate::model::SectionClass::ContainedLg => "-contained-lg",
        crate::model::SectionClass::ContainedXl => "-contained-xl",
        crate::model::SectionClass::ContainedXxl => "-contained-xxl",
        crate::model::SectionClass::FullFull => "-full-full",
        crate::model::SectionClass::FullContained => "-full-contained",
        crate::model::SectionClass::FullContainedMd => "-full-contained-md",
        crate::model::SectionClass::FullContainedLg => "-full-contained-lg",
        crate::model::SectionClass::FullContainedXl => "-full-contained-xl",
        crate::model::SectionClass::FullContainedXxl => "-full-contained-xxl",
    }
}

fn parse_section_class(raw: &str) -> Option<crate::model::SectionClass> {
    match raw.trim() {
        "-contained" => Some(crate::model::SectionClass::Contained),
        "-contained-md" => Some(crate::model::SectionClass::ContainedMd),
        "-contained-lg" => Some(crate::model::SectionClass::ContainedLg),
        "-contained-xl" => Some(crate::model::SectionClass::ContainedXl),
        "-contained-xxl" => Some(crate::model::SectionClass::ContainedXxl),
        "-full-full" => Some(crate::model::SectionClass::FullFull),
        "-full-contained" => Some(crate::model::SectionClass::FullContained),
        "-full-contained-md" => Some(crate::model::SectionClass::FullContainedMd),
        "-full-contained-lg" => Some(crate::model::SectionClass::FullContainedLg),
        "-full-contained-xl" => Some(crate::model::SectionClass::FullContainedXl),
        "-full-contained-xxl" => Some(crate::model::SectionClass::FullContainedXxl),
        _ => None,
    }
}

fn banner_class_to_str(v: crate::model::BannerClass) -> &'static str {
    match v {
        crate::model::BannerClass::BgTopLeft => "-bg-top-left",
        crate::model::BannerClass::BgTopCenter => "-bg-top-center",
        crate::model::BannerClass::BgTopRight => "-bg-top-right",
        crate::model::BannerClass::BgCenterLeft => "-bg-center-left",
        crate::model::BannerClass::BgCenterCenter => "-bg-center-center",
        crate::model::BannerClass::BgCenterRight => "-bg-center-right",
        crate::model::BannerClass::BgBottomLeft => "-bg-bottom-left",
        crate::model::BannerClass::BgBottomCenter => "-bg-bottom-center",
        crate::model::BannerClass::BgBottomRight => "-bg-bottom-right",
    }
}

fn cta_class_to_str(v: crate::model::CtaClass) -> &'static str {
    match v {
        crate::model::CtaClass::TopLeft => "-top-left",
        crate::model::CtaClass::TopCenter => "-top-center",
        crate::model::CtaClass::TopRight => "-top-right",
        crate::model::CtaClass::CenterLeft => "-center-left",
        crate::model::CtaClass::CenterCenter => "-center-center",
        crate::model::CtaClass::CenterRight => "-center-right",
        crate::model::CtaClass::BottomLeft => "-bottom-left",
        crate::model::CtaClass::BottomCenter => "-bottom-center",
        crate::model::CtaClass::BottomRight => "-bottom-right",
    }
}

fn parse_cta_class(raw: &str) -> Option<crate::model::CtaClass> {
    match raw.trim() {
        "-top-left" => Some(crate::model::CtaClass::TopLeft),
        "-top-center" => Some(crate::model::CtaClass::TopCenter),
        "-top-right" => Some(crate::model::CtaClass::TopRight),
        "-center-left" => Some(crate::model::CtaClass::CenterLeft),
        "-center-center" => Some(crate::model::CtaClass::CenterCenter),
        "-center-right" => Some(crate::model::CtaClass::CenterRight),
        "-bottom-left" => Some(crate::model::CtaClass::BottomLeft),
        "-bottom-center" => Some(crate::model::CtaClass::BottomCenter),
        "-bottom-right" => Some(crate::model::CtaClass::BottomRight),
        _ => None,
    }
}

fn next_cta_class(current: crate::model::CtaClass, forward: bool) -> crate::model::CtaClass {
    use crate::model::CtaClass;
    let all = [
        CtaClass::TopLeft,
        CtaClass::TopCenter,
        CtaClass::TopRight,
        CtaClass::CenterLeft,
        CtaClass::CenterCenter,
        CtaClass::CenterRight,
        CtaClass::BottomLeft,
        CtaClass::BottomCenter,
        CtaClass::BottomRight,
    ];
    let idx = all.iter().position(|v| *v == current).unwrap_or(0);
    let next_idx = if forward {
        (idx + 1) % all.len()
    } else if idx == 0 {
        all.len() - 1
    } else {
        idx - 1
    };
    all[next_idx]
}

fn parse_banner_class(raw: &str) -> Option<crate::model::BannerClass> {
    match raw.trim() {
        "-bg-top-left" => Some(crate::model::BannerClass::BgTopLeft),
        "-bg-top-center" => Some(crate::model::BannerClass::BgTopCenter),
        "-bg-top-right" => Some(crate::model::BannerClass::BgTopRight),
        "-bg-center-left" => Some(crate::model::BannerClass::BgCenterLeft),
        "-bg-center-center" => Some(crate::model::BannerClass::BgCenterCenter),
        "-bg-center-right" => Some(crate::model::BannerClass::BgCenterRight),
        "-bg-bottom-left" => Some(crate::model::BannerClass::BgBottomLeft),
        "-bg-bottom-center" => Some(crate::model::BannerClass::BgBottomCenter),
        "-bg-bottom-right" => Some(crate::model::BannerClass::BgBottomRight),
        _ => None,
    }
}

fn next_banner_class(
    current: crate::model::BannerClass,
    forward: bool,
) -> crate::model::BannerClass {
    use crate::model::BannerClass;
    let all = [
        BannerClass::BgTopLeft,
        BannerClass::BgTopCenter,
        BannerClass::BgTopRight,
        BannerClass::BgCenterLeft,
        BannerClass::BgCenterCenter,
        BannerClass::BgCenterRight,
        BannerClass::BgBottomLeft,
        BannerClass::BgBottomCenter,
        BannerClass::BgBottomRight,
    ];
    let idx = all.iter().position(|v| *v == current).unwrap_or(0);
    let next_idx = if forward {
        (idx + 1) % all.len()
    } else if idx == 0 {
        all.len() - 1
    } else {
        idx - 1
    };
    all[next_idx]
}

fn card_type_to_str(v: crate::model::CardType) -> &'static str {
    match v {
        crate::model::CardType::Default => "-default",
        crate::model::CardType::Horizontal => "-horizontal",
    }
}

fn parse_card_type(raw: &str) -> Option<crate::model::CardType> {
    match raw.trim() {
        "-default" => Some(crate::model::CardType::Default),
        "-horizontal" => Some(crate::model::CardType::Horizontal),
        _ => None,
    }
}

fn next_card_type(current: crate::model::CardType, forward: bool) -> crate::model::CardType {
    use crate::model::CardType;
    let all = [CardType::Default, CardType::Horizontal];
    let idx = all.iter().position(|v| *v == current).unwrap_or(0);
    let next_idx = if forward {
        (idx + 1) % all.len()
    } else if idx == 0 {
        all.len() - 1
    } else {
        idx - 1
    };
    all[next_idx]
}

fn filmstrip_type_to_str(v: crate::model::FilmstripType) -> &'static str {
    match v {
        crate::model::FilmstripType::Default => "-default",
        crate::model::FilmstripType::Reverse => "-reverse",
    }
}

fn parse_filmstrip_type(raw: &str) -> Option<crate::model::FilmstripType> {
    match raw.trim() {
        "-default" => Some(crate::model::FilmstripType::Default),
        "-reverse" => Some(crate::model::FilmstripType::Reverse),
        _ => None,
    }
}

fn next_filmstrip_type(
    current: crate::model::FilmstripType,
    forward: bool,
) -> crate::model::FilmstripType {
    use crate::model::FilmstripType;
    let all = [FilmstripType::Default, FilmstripType::Reverse];
    let idx = all.iter().position(|v| *v == current).unwrap_or(0);
    let next_idx = if forward {
        (idx + 1) % all.len()
    } else if idx == 0 {
        all.len() - 1
    } else {
        idx - 1
    };
    all[next_idx]
}

fn child_link_target_to_str(v: crate::model::CardLinkTarget) -> &'static str {
    match v {
        crate::model::CardLinkTarget::SelfTarget => "_self",
        crate::model::CardLinkTarget::Blank => "_blank",
    }
}

fn parse_child_link_target(raw: &str) -> Option<crate::model::CardLinkTarget> {
    match raw.trim() {
        "_self" => Some(crate::model::CardLinkTarget::SelfTarget),
        "_blank" => Some(crate::model::CardLinkTarget::Blank),
        _ => None,
    }
}

fn next_child_link_target(
    current: crate::model::CardLinkTarget,
    forward: bool,
) -> crate::model::CardLinkTarget {
    use crate::model::CardLinkTarget;
    let all = [CardLinkTarget::SelfTarget, CardLinkTarget::Blank];
    let idx = all.iter().position(|v| *v == current).unwrap_or(0);
    let next_idx = if forward {
        (idx + 1) % all.len()
    } else if idx == 0 {
        all.len() - 1
    } else {
        idx - 1
    };
    all[next_idx]
}

fn alternating_type_to_str(v: crate::model::AlternatingType) -> &'static str {
    match v {
        crate::model::AlternatingType::Default => "-default",
        crate::model::AlternatingType::Reverse => "-reverse",
        crate::model::AlternatingType::NoAlternate => "-no-alternate",
    }
}

fn parse_alternating_type(raw: &str) -> Option<crate::model::AlternatingType> {
    match raw.trim() {
        "-default" => Some(crate::model::AlternatingType::Default),
        "-reverse" => Some(crate::model::AlternatingType::Reverse),
        "-no-alternate" => Some(crate::model::AlternatingType::NoAlternate),
        _ => None,
    }
}

fn next_alternating_type(
    current: crate::model::AlternatingType,
    forward: bool,
) -> crate::model::AlternatingType {
    use crate::model::AlternatingType;
    let all = [
        AlternatingType::Default,
        AlternatingType::Reverse,
        AlternatingType::NoAlternate,
    ];
    let idx = all.iter().position(|v| *v == current).unwrap_or(0);
    let next_idx = if forward {
        (idx + 1) % all.len()
    } else if idx == 0 {
        all.len() - 1
    } else {
        idx - 1
    };
    all[next_idx]
}

fn accordion_type_to_str(v: crate::model::AccordionType) -> &'static str {
    match v {
        crate::model::AccordionType::Default => "-default",
        crate::model::AccordionType::Faq => "-faq",
    }
}

fn parse_accordion_type(raw: &str) -> Option<crate::model::AccordionType> {
    match raw.trim() {
        "-default" => Some(crate::model::AccordionType::Default),
        "-faq" => Some(crate::model::AccordionType::Faq),
        _ => None,
    }
}

fn next_accordion_type(
    current: crate::model::AccordionType,
    forward: bool,
) -> crate::model::AccordionType {
    use crate::model::AccordionType;
    let all = [AccordionType::Default, AccordionType::Faq];
    let idx = all.iter().position(|v| *v == current).unwrap_or(0);
    let next_idx = if forward {
        (idx + 1) % all.len()
    } else if idx == 0 {
        all.len() - 1
    } else {
        idx - 1
    };
    all[next_idx]
}

fn accordion_class_to_str(v: crate::model::AccordionClass) -> &'static str {
    match v {
        crate::model::AccordionClass::Borderless => "-borderless",
        crate::model::AccordionClass::Compact => "-compact",
        crate::model::AccordionClass::Primary => "-primary",
        crate::model::AccordionClass::Secondary => "-secondary",
        crate::model::AccordionClass::Tertiary => "-tertiary",
    }
}

fn parse_accordion_class(raw: &str) -> Option<crate::model::AccordionClass> {
    match raw.trim() {
        "-borderless" => Some(crate::model::AccordionClass::Borderless),
        "-compact" => Some(crate::model::AccordionClass::Compact),
        "-primary" => Some(crate::model::AccordionClass::Primary),
        "-secondary" => Some(crate::model::AccordionClass::Secondary),
        "-tertiary" => Some(crate::model::AccordionClass::Tertiary),
        _ => None,
    }
}

fn next_accordion_class(
    current: crate::model::AccordionClass,
    forward: bool,
) -> crate::model::AccordionClass {
    use crate::model::AccordionClass;
    let all = [
        AccordionClass::Borderless,
        AccordionClass::Compact,
        AccordionClass::Primary,
        AccordionClass::Secondary,
        AccordionClass::Tertiary,
    ];
    let idx = all.iter().position(|v| *v == current).unwrap_or(0);
    let next_idx = if forward {
        (idx + 1) % all.len()
    } else if idx == 0 {
        all.len() - 1
    } else {
        idx - 1
    };
    all[next_idx]
}

fn alert_type_to_str(v: crate::model::AlertType) -> &'static str {
    match v {
        crate::model::AlertType::Default => "-default",
        crate::model::AlertType::Info => "-info",
        crate::model::AlertType::Warning => "-warning",
        crate::model::AlertType::Error => "-error",
        crate::model::AlertType::Success => "-success",
    }
}

fn parse_alert_type(raw: &str) -> Option<crate::model::AlertType> {
    match raw.trim() {
        "-default" => Some(crate::model::AlertType::Default),
        "-info" => Some(crate::model::AlertType::Info),
        "-warning" => Some(crate::model::AlertType::Warning),
        "-error" => Some(crate::model::AlertType::Error),
        "-success" => Some(crate::model::AlertType::Success),
        _ => None,
    }
}

fn next_alert_type(current: crate::model::AlertType, forward: bool) -> crate::model::AlertType {
    use crate::model::AlertType;
    let all = [
        AlertType::Default,
        AlertType::Info,
        AlertType::Warning,
        AlertType::Error,
        AlertType::Success,
    ];
    let idx = all.iter().position(|v| *v == current).unwrap_or(0);
    let next_idx = if forward {
        (idx + 1) % all.len()
    } else if idx == 0 {
        all.len() - 1
    } else {
        idx - 1
    };
    all[next_idx]
}

fn alert_class_to_str(v: crate::model::AlertClass) -> &'static str {
    match v {
        crate::model::AlertClass::Default => "-default",
        crate::model::AlertClass::Compact => "-compact",
    }
}

fn parse_alert_class(raw: &str) -> Option<crate::model::AlertClass> {
    match raw.trim() {
        "-default" => Some(crate::model::AlertClass::Default),
        "-compact" => Some(crate::model::AlertClass::Compact),
        _ => None,
    }
}

fn next_alert_class(current: crate::model::AlertClass, forward: bool) -> crate::model::AlertClass {
    use crate::model::AlertClass;
    let all = [AlertClass::Default, AlertClass::Compact];
    let idx = all.iter().position(|v| *v == current).unwrap_or(0);
    let next_idx = if forward {
        (idx + 1) % all.len()
    } else if idx == 0 {
        all.len() - 1
    } else {
        idx - 1
    };
    all[next_idx]
}

fn next_section_class(
    current: crate::model::SectionClass,
    forward: bool,
) -> crate::model::SectionClass {
    use crate::model::SectionClass;
    let all = [
        SectionClass::Contained,
        SectionClass::ContainedMd,
        SectionClass::ContainedLg,
        SectionClass::ContainedXl,
        SectionClass::ContainedXxl,
        SectionClass::FullFull,
        SectionClass::FullContained,
        SectionClass::FullContainedMd,
        SectionClass::FullContainedLg,
        SectionClass::FullContainedXl,
        SectionClass::FullContainedXxl,
    ];
    let idx = all.iter().position(|v| *v == current).unwrap_or(0);
    let next_idx = if forward {
        (idx + 1) % all.len()
    } else if idx == 0 {
        all.len() - 1
    } else {
        idx - 1
    };
    all[next_idx]
}

fn fit_ascii_cell(value: &str, width: usize) -> String {
    let shortened = truncate_ascii(value, width);
    format!("{shortened:<width$}")
}

fn input_lines_preserve(s: &str) -> Vec<String> {
    s.split('\n').map(|line| line.to_string()).collect()
}

fn cursor_from_row_col(lines: &[String], target_row: usize, target_col: usize) -> usize {
    let row = target_row.min(lines.len().saturating_sub(1));
    let mut cursor = 0usize;
    for line in lines.iter().take(row) {
        cursor += line.chars().count() + 1;
    }
    let line_len = lines.get(row).map(|line| line.chars().count()).unwrap_or(0);
    cursor + target_col.min(line_len)
}

fn byte_index_for_char(s: &str, char_idx: usize) -> usize {
    if char_idx == 0 {
        return 0;
    }
    s.char_indices()
        .nth(char_idx)
        .map(|(i, _)| i)
        .unwrap_or_else(|| s.len())
}

fn cursor_row_col(s: &str, cursor: usize) -> (usize, usize) {
    let mut row = 0usize;
    let mut col = 0usize;
    let mut idx = 0usize;
    for ch in s.chars() {
        if idx >= cursor {
            break;
        }
        if ch == '\n' {
            row += 1;
            col = 0;
        } else {
            col += 1;
        }
        idx += 1;
    }
    (row, col)
}

fn truncate_ascii(value: &str, max_chars: usize) -> String {
    let chars = value.chars().collect::<Vec<_>>();
    if chars.len() <= max_chars {
        return value.to_string();
    }
    if max_chars <= 3 {
        return chars.into_iter().take(max_chars).collect();
    }
    let mut out = chars.into_iter().take(max_chars - 3).collect::<String>();
    out.push_str("...");
    out
}

fn component_label(component: &crate::model::SectionComponent) -> &'static str {
    match component {
        crate::model::SectionComponent::Cta(_) => "dd-cta",
        crate::model::SectionComponent::Filmstrip(_) => "dd-filmstrip",
        crate::model::SectionComponent::Milestones(_) => "dd-milestones",
        crate::model::SectionComponent::Slider(_) => "dd-slider",
        crate::model::SectionComponent::Modal(_) => "dd-modal",
        crate::model::SectionComponent::Banner(_) => "dd-banner",
        crate::model::SectionComponent::Card(_) => "dd-card",
        crate::model::SectionComponent::Blockquote(_) => "dd-blockquote",
        crate::model::SectionComponent::Accordion(_) => "dd-accordion",
        crate::model::SectionComponent::Alternating(_) => "dd-alternating",
        crate::model::SectionComponent::Alert(_) => "dd-alert",
        crate::model::SectionComponent::Image(_) => "dd-image",
        crate::model::SectionComponent::RichText(_) => "dd-rich_text",
        crate::model::SectionComponent::Navigation(_) => "dd-navigation",
        crate::model::SectionComponent::HeaderSearch(_) => "dd-header-search",
        crate::model::SectionComponent::HeaderMenu(_) => "dd-header-menu",
    }
}

fn component_blueprint_label(component: &crate::model::SectionComponent) -> String {
    match component {
        crate::model::SectionComponent::Cta(v) => {
            format!("dd-cta | parent_title: {}", v.parent_title)
        }
        crate::model::SectionComponent::Filmstrip(v) => format!(
            "dd-filmstrip | child_title: {}",
            v.items
                .first()
                .map(|i| i.child_title.as_str())
                .unwrap_or("(none)")
        ),
        crate::model::SectionComponent::Milestones(v) => format!(
            "dd-milestones | child_title: {}",
            v.items
                .first()
                .map(|i| i.child_title.as_str())
                .unwrap_or("(none)")
        ),
        crate::model::SectionComponent::Slider(v) => format!(
            "dd-slider | child_title: {}",
            v.items
                .first()
                .map(|i| i.child_title.as_str())
                .unwrap_or("(none)")
        ),
        crate::model::SectionComponent::Modal(v) => {
            format!("dd-modal | parent_title: {}", v.parent_title)
        }
        crate::model::SectionComponent::Accordion(v) => format!(
            "dd-accordion | accordion_title: {}",
            v.items
                .first()
                .map(|i| i.child_title.as_str())
                .unwrap_or("(none)")
        ),
        crate::model::SectionComponent::Alternating(v) => format!(
            "dd-alternating | alternating_title: {}",
            v.items
                .first()
                .map(|i| i.child_title.as_str())
                .unwrap_or("(none)")
        ),
        crate::model::SectionComponent::Card(v) => format!(
            "dd-card | child_title: {}",
            v.items
                .first()
                .map(|i| i.child_title.as_str())
                .unwrap_or("(none)")
        ),
        crate::model::SectionComponent::Blockquote(v) => format!(
            "dd-blockquote | parent_name: {} | parent_role: {}",
            v.parent_name, v.parent_role
        ),
        _ => component_label(component).to_string(),
    }
}

fn component_form(
    component: &crate::model::SectionComponent,
    selected_nested_item: usize,
) -> String {
    match component {
        crate::model::SectionComponent::Cta(v) => format!(
            "fields:\n  cta.class: {}\n  parent_image_url: {}\n  parent_image_alt: {}\n  cta.data_aos: {}\n  parent_title: {}\n  parent_subtitle: {}\n  parent_copy: {}\n  parent_link_url: {}\n  parent_link_target: {}\n  parent_link_label: {}",
            cta_class_to_str(v.parent_class),
            v.parent_image_url,
            v.parent_image_alt,
            parent_data_aos_to_str(v.parent_data_aos),
            v.parent_title,
            v.parent_subtitle,
            v.parent_copy,
            v.parent_link_url.as_deref().unwrap_or("(none)"),
            v.parent_link_target
                .map(child_link_target_to_str)
                .unwrap_or("_self"),
            v.parent_link_label.as_deref().unwrap_or("(none)")
        ),
        crate::model::SectionComponent::Filmstrip(v) => {
            let active = nested_index(v.items.len(), selected_nested_item)
                .map(|i| i + 1)
                .unwrap_or(0);
            let item =
                nested_index(v.items.len(), selected_nested_item).and_then(|i| v.items.get(i));
            format!(
                "fields:\n  parent_type: {}\n  parent_data_aos: {}\n  active_item: {}\n  child_image_url: {}\n  child_image_alt: {}\n  child_title: {}",
                filmstrip_type_to_str(v.parent_type),
                parent_data_aos_to_str(v.parent_data_aos),
                active,
                item.map(|i| i.child_image_url.as_str()).unwrap_or("(none)"),
                item.map(|i| i.child_image_alt.as_str()).unwrap_or("(none)"),
                item.map(|i| i.child_title.as_str()).unwrap_or("(none)")
            )
        }
        crate::model::SectionComponent::Milestones(v) => {
            let active = nested_index(v.items.len(), selected_nested_item)
                .map(|i| i + 1)
                .unwrap_or(0);
            let item =
                nested_index(v.items.len(), selected_nested_item).and_then(|i| v.items.get(i));
            format!(
                "fields:\n  parent_data_aos: {}\n  parent_width: {}\n  active_item: {}\n  child_percentage: {}\n  child_title: {}\n  child_subtitle: {}\n  child_copy: {}\n  child_link_url: {}\n  child_link_target: {}\n  child_link_label: {}",
                parent_data_aos_to_str(v.parent_data_aos),
                v.parent_width,
                active,
                item.map(|i| i.child_percentage.as_str())
                    .unwrap_or("(none)"),
                item.map(|i| i.child_title.as_str()).unwrap_or("(none)"),
                item.map(|i| i.child_subtitle.as_str()).unwrap_or("(none)"),
                item.map(|i| i.child_copy.as_str()).unwrap_or("(none)"),
                item.and_then(|i| i.child_link_url.as_deref())
                    .unwrap_or("(none)"),
                item.and_then(|i| i.child_link_target)
                    .map(child_link_target_to_str)
                    .unwrap_or("_self"),
                item.and_then(|i| i.child_link_label.as_deref())
                    .unwrap_or("(none)")
            )
        }
        crate::model::SectionComponent::Slider(v) => {
            let active = nested_index(v.items.len(), selected_nested_item)
                .map(|i| i + 1)
                .unwrap_or(0);
            let item =
                nested_index(v.items.len(), selected_nested_item).and_then(|i| v.items.get(i));
            format!(
                "fields:\n  parent_title: {}\n  active_item: {}\n  child_title: {}\n  child_copy: {}\n  child_link_url: {}\n  child_link_target: {}\n  child_link_label: {}\n  child_image_url: {}\n  child_image_alt: {}",
                v.parent_title,
                active,
                item.map(|i| i.child_title.as_str()).unwrap_or("(none)"),
                item.map(|i| i.child_copy.as_str()).unwrap_or("(none)"),
                item.and_then(|i| i.child_link_url.as_deref())
                    .unwrap_or("(none)"),
                item.and_then(|i| i.child_link_target)
                    .map(child_link_target_to_str)
                    .unwrap_or("_self"),
                item.and_then(|i| i.child_link_label.as_deref())
                    .unwrap_or("(none)"),
                item.map(|i| i.child_image_url.as_str()).unwrap_or("(none)"),
                item.map(|i| i.child_image_alt.as_str()).unwrap_or("(none)")
            )
        }
        crate::model::SectionComponent::Modal(v) => format!(
            "fields:\n  parent_title: {}\n  parent_copy: {}",
            v.parent_title, v.parent_copy
        ),
        crate::model::SectionComponent::Banner(v) => format!(
            "fields:\n  banner.class: {}\n  banner.data_aos: {}\n  parent_image_url: {}\n  parent_image_alt: {}",
            banner_class_to_str(v.parent_class),
            parent_data_aos_to_str(v.parent_data_aos),
            v.parent_image_url,
            v.parent_image_alt
        ),
        crate::model::SectionComponent::Accordion(v) => {
            let active = nested_index(v.items.len(), selected_nested_item)
                .map(|i| i + 1)
                .unwrap_or(0);
            let title = nested_index(v.items.len(), selected_nested_item)
                .and_then(|i| v.items.get(i))
                .map(|i| i.child_title.as_str())
                .unwrap_or("(none)");
            let content = nested_index(v.items.len(), selected_nested_item)
                .and_then(|i| v.items.get(i))
                .map(|i| i.child_copy.as_str())
                .unwrap_or("(none)");
            format!(
                "fields:\n  parent_type: {}\n  accordion.class: {}\n  accordion.data_aos: {}\n  accordion.parent_group_name: {}\n  active_item: {}\n  accordion_title: {}\n  accordion_copy: {}",
                accordion_type_to_str(v.parent_type),
                accordion_class_to_str(v.parent_class),
                parent_data_aos_to_str(v.parent_data_aos),
                v.parent_group_name,
                active,
                title,
                content
            )
        }
        crate::model::SectionComponent::Blockquote(v) => format!(
            "fields:\n  parent_data_aos: {}\n  parent_image_url: {}\n  parent_image_alt: {}\n  parent_name: {}\n  parent_role: {}\n  parent_copy: {}",
            parent_data_aos_to_str(v.parent_data_aos),
            v.parent_image_url,
            v.parent_image_alt,
            v.parent_name,
            v.parent_role,
            v.parent_copy
        ),
        crate::model::SectionComponent::Alternating(v) => {
            let active = nested_index(v.items.len(), selected_nested_item)
                .map(|i| i + 1)
                .unwrap_or(0);
            let image = nested_index(v.items.len(), selected_nested_item)
                .and_then(|i| v.items.get(i))
                .map(|i| i.child_image_url.as_str())
                .unwrap_or("(none)");
            let image_alt = nested_index(v.items.len(), selected_nested_item)
                .and_then(|i| v.items.get(i))
                .map(|i| i.child_image_alt.as_str())
                .unwrap_or("(none)");
            let title = nested_index(v.items.len(), selected_nested_item)
                .and_then(|i| v.items.get(i))
                .map(|i| i.child_title.as_str())
                .unwrap_or("(none)");
            let copy = nested_index(v.items.len(), selected_nested_item)
                .and_then(|i| v.items.get(i))
                .map(|i| i.child_copy.as_str())
                .unwrap_or("(none)");
            format!(
                "fields:\n  parent_type: {}\n  alternating.class: {}\n  alternating.data_aos: {}\n  active_item: {}\n  alternating_image: {}\n  alternating_image_alt: {}\n  alternating_title: {}\n  alternating_copy: {}",
                alternating_type_to_str(v.parent_type),
                v.parent_class,
                parent_data_aos_to_str(v.parent_data_aos),
                active,
                image,
                image_alt,
                title,
                copy
            )
        }
        crate::model::SectionComponent::Card(v) => {
            let active = nested_index(v.items.len(), selected_nested_item)
                .map(|i| i + 1)
                .unwrap_or(0);
            let item =
                nested_index(v.items.len(), selected_nested_item).and_then(|i| v.items.get(i));
            format!(
                "fields:\n  parent_type: {}\n  parent_data_aos: {}\n  parent_width: {}\n  active_item: {}\n  child_image_url: {}\n  child_image_alt: {}\n  child_title: {}\n  child_subtitle: {}\n  child_copy: {}\n  child_link_url: {}\n  child_link_target: {}\n  child_link_label: {}",
                card_type_to_str(v.parent_type),
                parent_data_aos_to_str(v.parent_data_aos),
                v.parent_width,
                active,
                item.map(|i| i.child_image_url.as_str()).unwrap_or("(none)"),
                item.map(|i| i.child_image_alt.as_str()).unwrap_or("(none)"),
                item.map(|i| i.child_title.as_str()).unwrap_or("(none)"),
                item.map(|i| i.child_subtitle.as_str()).unwrap_or("(none)"),
                item.map(|i| i.child_copy.as_str()).unwrap_or("(none)"),
                item.and_then(|i| i.child_link_url.as_deref())
                    .unwrap_or("(none)"),
                item.and_then(|i| i.child_link_target)
                    .map(child_link_target_to_str)
                    .unwrap_or("_self"),
                item.and_then(|i| i.child_link_label.as_deref())
                    .unwrap_or("(none)")
            )
        }
        crate::model::SectionComponent::Alert(v) => format!(
            "fields:\n  parent_type: {}\n  parent_class: {}\n  parent_data_aos: {}\n  parent_title: {}\n  parent_copy: {}",
            alert_type_to_str(v.parent_type),
            alert_class_to_str(v.parent_class),
            parent_data_aos_to_str(v.parent_data_aos),
            v.parent_title.as_deref().unwrap_or("(none)"),
            v.parent_copy
        ),
        crate::model::SectionComponent::Image(v) => format!(
            "fields:\n  parent_data_aos: {}\n  parent_image_url: {}\n  parent_image_alt: {}\n  parent_link_url: {}\n  parent_link_target: {}",
            parent_data_aos_to_str(v.parent_data_aos),
            v.parent_image_url,
            v.parent_image_alt,
            v.parent_link_url.as_deref().unwrap_or("(none)"),
            v.parent_link_target
                .map(child_link_target_to_str)
                .unwrap_or("_self"),
        ),
        crate::model::SectionComponent::RichText(v) => format!(
            "fields:\n  parent_class: {}\n  parent_data_aos: {}\n  parent_copy: {}",
            v.parent_class.as_deref().unwrap_or("(none)"),
            parent_data_aos_to_str(v.parent_data_aos),
            v.parent_copy
        ),
        crate::model::SectionComponent::Navigation(v) => format!(
            "fields:\n  parent_type: {:?}\n  parent_class: {:?}\n  parent_data_aos: {}\n  parent_width: {}\n  items: {}",
            v.parent_type,
            v.parent_class,
            parent_data_aos_to_str(v.parent_data_aos),
            v.parent_width,
            v.items.len()
        ),
        crate::model::SectionComponent::HeaderSearch(v) => format!(
            "fields:\n  parent_width: {}\n  parent_data_aos: {}",
            v.parent_width,
            parent_data_aos_to_str(v.parent_data_aos),
        ),
        crate::model::SectionComponent::HeaderMenu(v) => format!(
            "fields:\n  parent_width: {}\n  parent_data_aos: {}",
            v.parent_width,
            parent_data_aos_to_str(v.parent_data_aos),
        ),
    }
}

fn hero_image_class_to_str(v: crate::model::HeroImageClass) -> &'static str {
    match v {
        crate::model::HeroImageClass::Contained => "-contained",
        crate::model::HeroImageClass::ContainedMd => "-contained-md",
        crate::model::HeroImageClass::ContainedLg => "-contained-lg",
        crate::model::HeroImageClass::ContainedXl => "-contained-xl",
        crate::model::HeroImageClass::ContainedXxl => "-contained-xxl",
        crate::model::HeroImageClass::FullFull => "-full-full",
        crate::model::HeroImageClass::FullContained => "-full-contained",
        crate::model::HeroImageClass::FullContainedMd => "-full-contained-md",
        crate::model::HeroImageClass::FullContainedLg => "-full-contained-lg",
        crate::model::HeroImageClass::FullContainedXl => "-full-contained-xl",
        crate::model::HeroImageClass::FullContainedXxl => "-full-contained-xxl",
    }
}

fn parent_data_aos_to_str(v: crate::model::HeroAos) -> &'static str {
    match v {
        crate::model::HeroAos::FadeIn => "fade-in",
        crate::model::HeroAos::FadeUp => "fade-up",
        crate::model::HeroAos::FadeRight => "fade-right",
        crate::model::HeroAos::FadeDown => "fade-down",
        crate::model::HeroAos::FadeLeft => "fade-left",
        crate::model::HeroAos::ZoomIn => "zoom-in",
        crate::model::HeroAos::ZoomInUp => "zoom-in-up",
        crate::model::HeroAos::ZoomInDown => "zoom-in-down",
    }
}

fn link_1_target_to_str(v: crate::model::CtaTarget) -> &'static str {
    match v {
        crate::model::CtaTarget::SelfTarget => "_self",
        crate::model::CtaTarget::Blank => "_blank",
        crate::model::CtaTarget::Parent => "_parent",
    }
}

fn parse_hero_image_class(raw: &str) -> Option<crate::model::HeroImageClass> {
    let trimmed = raw.trim();
    let normalized = trimmed.strip_prefix(".dd-hero__image.").unwrap_or(trimmed);
    match normalized {
        "-contained" => Some(crate::model::HeroImageClass::Contained),
        "-contained-md" => Some(crate::model::HeroImageClass::ContainedMd),
        "-contained-lg" => Some(crate::model::HeroImageClass::ContainedLg),
        "-contained-xl" => Some(crate::model::HeroImageClass::ContainedXl),
        "-contained-xxl" => Some(crate::model::HeroImageClass::ContainedXxl),
        "-full-full" => Some(crate::model::HeroImageClass::FullFull),
        "-full-contained" => Some(crate::model::HeroImageClass::FullContained),
        "-full-contained-md" => Some(crate::model::HeroImageClass::FullContainedMd),
        "-full-contained-lg" => Some(crate::model::HeroImageClass::FullContainedLg),
        "-full-contained-xl" => Some(crate::model::HeroImageClass::FullContainedXl),
        "-full-contained-xxl" => Some(crate::model::HeroImageClass::FullContainedXxl),
        _ => None,
    }
}

fn parse_parent_data_aos(raw: &str) -> Option<crate::model::HeroAos> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "fade-in" => Some(crate::model::HeroAos::FadeIn),
        "fade-up" => Some(crate::model::HeroAos::FadeUp),
        "fade-right" => Some(crate::model::HeroAos::FadeRight),
        "fade-down" => Some(crate::model::HeroAos::FadeDown),
        "fade-left" => Some(crate::model::HeroAos::FadeLeft),
        "zoom-in" => Some(crate::model::HeroAos::ZoomIn),
        "zoom-in-up" => Some(crate::model::HeroAos::ZoomInUp),
        "zoom-in-down" => Some(crate::model::HeroAos::ZoomInDown),
        _ => None,
    }
}

fn parse_link_1_target(raw: &str) -> Option<crate::model::CtaTarget> {
    match raw.trim() {
        "_self" => Some(crate::model::CtaTarget::SelfTarget),
        "_blank" => Some(crate::model::CtaTarget::Blank),
        _ => None,
    }
}

fn navigation_type_to_str(v: crate::model::NavigationType) -> &'static str {
    match v {
        crate::model::NavigationType::HeaderNav => "dd-header__navigation",
        crate::model::NavigationType::FooterNav => "dd-footer__navigation",
    }
}

fn parse_navigation_type(raw: &str) -> Option<crate::model::NavigationType> {
    match raw.trim() {
        "dd-header__navigation" | "header" | "HeaderNav" => {
            Some(crate::model::NavigationType::HeaderNav)
        }
        "dd-footer__navigation" | "footer" | "FooterNav" => {
            Some(crate::model::NavigationType::FooterNav)
        }
        _ => None,
    }
}

fn next_navigation_type(
    current: crate::model::NavigationType,
    forward: bool,
) -> crate::model::NavigationType {
    use crate::model::NavigationType;
    let all = [NavigationType::HeaderNav, NavigationType::FooterNav];
    let idx = all.iter().position(|v| *v == current).unwrap_or(0);
    let next = if forward {
        (idx + 1) % all.len()
    } else {
        (idx + all.len() - 1) % all.len()
    };
    all[next]
}

fn navigation_class_to_str(v: crate::model::NavigationClass) -> &'static str {
    match v {
        crate::model::NavigationClass::MainMenu => "-main-menu",
        crate::model::NavigationClass::MenuSecondary => "-menu-secondary",
        crate::model::NavigationClass::MenuTertiary => "-menu-tertiary",
        crate::model::NavigationClass::FooterMenu => "-footer-menu",
        crate::model::NavigationClass::FooterMenuSecondary => "-footer-menu-secondary",
        crate::model::NavigationClass::FooterMenuTertiary => "-footer-menu-tertiary",
        crate::model::NavigationClass::SocialMenu => "-social-menu",
    }
}

fn parse_navigation_class(raw: &str) -> Option<crate::model::NavigationClass> {
    match raw.trim() {
        "-main-menu" => Some(crate::model::NavigationClass::MainMenu),
        "-menu-secondary" => Some(crate::model::NavigationClass::MenuSecondary),
        "-menu-tertiary" => Some(crate::model::NavigationClass::MenuTertiary),
        "-footer-menu" => Some(crate::model::NavigationClass::FooterMenu),
        "-footer-menu-secondary" => Some(crate::model::NavigationClass::FooterMenuSecondary),
        "-footer-menu-tertiary" => Some(crate::model::NavigationClass::FooterMenuTertiary),
        "-social-menu" => Some(crate::model::NavigationClass::SocialMenu),
        _ => None,
    }
}

fn next_navigation_class(
    current: crate::model::NavigationClass,
    forward: bool,
) -> crate::model::NavigationClass {
    use crate::model::NavigationClass;
    let all = [
        NavigationClass::MainMenu,
        NavigationClass::MenuSecondary,
        NavigationClass::MenuTertiary,
        NavigationClass::FooterMenu,
        NavigationClass::FooterMenuSecondary,
        NavigationClass::FooterMenuTertiary,
        NavigationClass::SocialMenu,
    ];
    let idx = all.iter().position(|v| *v == current).unwrap_or(0);
    let next = if forward {
        (idx + 1) % all.len()
    } else {
        (idx + all.len() - 1) % all.len()
    };
    all[next]
}

fn navigation_kind_to_str(v: crate::model::NavigationKind) -> &'static str {
    match v {
        crate::model::NavigationKind::Link => "link",
        crate::model::NavigationKind::Button => "button",
    }
}

fn parse_navigation_kind(raw: &str) -> Option<crate::model::NavigationKind> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "link" => Some(crate::model::NavigationKind::Link),
        "button" => Some(crate::model::NavigationKind::Button),
        _ => None,
    }
}

fn next_navigation_kind(
    current: crate::model::NavigationKind,
    forward: bool,
) -> crate::model::NavigationKind {
    let _ = forward;
    match current {
        crate::model::NavigationKind::Link => crate::model::NavigationKind::Button,
        crate::model::NavigationKind::Button => crate::model::NavigationKind::Link,
    }
}

fn robots_directive_to_str(v: crate::model::RobotsDirective) -> &'static str {
    match v {
        crate::model::RobotsDirective::IndexFollow => "index, follow",
        crate::model::RobotsDirective::NoindexFollow => "noindex, follow",
        crate::model::RobotsDirective::IndexNofollow => "index, nofollow",
        crate::model::RobotsDirective::NoindexNofollow => "noindex, nofollow",
    }
}

fn parse_robots_directive(raw: &str) -> Option<crate::model::RobotsDirective> {
    let normalized: String = raw.trim().to_ascii_lowercase().replace(' ', "");
    match normalized.as_str() {
        "index,follow" => Some(crate::model::RobotsDirective::IndexFollow),
        "noindex,follow" => Some(crate::model::RobotsDirective::NoindexFollow),
        "index,nofollow" => Some(crate::model::RobotsDirective::IndexNofollow),
        "noindex,nofollow" => Some(crate::model::RobotsDirective::NoindexNofollow),
        _ => None,
    }
}

fn next_robots_directive(
    current: crate::model::RobotsDirective,
    forward: bool,
) -> crate::model::RobotsDirective {
    use crate::model::RobotsDirective;
    let all = [
        RobotsDirective::IndexFollow,
        RobotsDirective::NoindexFollow,
        RobotsDirective::IndexNofollow,
        RobotsDirective::NoindexNofollow,
    ];
    let idx = all.iter().position(|v| *v == current).unwrap_or(0);
    let next = if forward {
        (idx + 1) % all.len()
    } else {
        (idx + all.len() - 1) % all.len()
    };
    all[next]
}

fn schema_type_to_str(v: crate::model::SchemaType) -> &'static str {
    match v {
        crate::model::SchemaType::WebPage => "WebPage",
        crate::model::SchemaType::Article => "Article",
        crate::model::SchemaType::AboutPage => "AboutPage",
        crate::model::SchemaType::ContactPage => "ContactPage",
        crate::model::SchemaType::CollectionPage => "CollectionPage",
        crate::model::SchemaType::Organization => "Organization",
        crate::model::SchemaType::LocalBusiness => "LocalBusiness",
        crate::model::SchemaType::Product => "Product",
        crate::model::SchemaType::Service => "Service",
    }
}

fn parse_schema_type(raw: &str) -> Option<crate::model::SchemaType> {
    match raw.trim() {
        "WebPage" => Some(crate::model::SchemaType::WebPage),
        "Article" => Some(crate::model::SchemaType::Article),
        "AboutPage" => Some(crate::model::SchemaType::AboutPage),
        "ContactPage" => Some(crate::model::SchemaType::ContactPage),
        "CollectionPage" => Some(crate::model::SchemaType::CollectionPage),
        "Organization" => Some(crate::model::SchemaType::Organization),
        "LocalBusiness" => Some(crate::model::SchemaType::LocalBusiness),
        "Product" => Some(crate::model::SchemaType::Product),
        "Service" => Some(crate::model::SchemaType::Service),
        _ => None,
    }
}

fn next_schema_type(
    current: crate::model::SchemaType,
    forward: bool,
) -> crate::model::SchemaType {
    use crate::model::SchemaType;
    let all = [
        SchemaType::WebPage,
        SchemaType::Article,
        SchemaType::AboutPage,
        SchemaType::ContactPage,
        SchemaType::CollectionPage,
        SchemaType::Organization,
        SchemaType::LocalBusiness,
        SchemaType::Product,
        SchemaType::Service,
    ];
    let idx = all.iter().position(|v| *v == current).unwrap_or(0);
    let next = if forward {
        (idx + 1) % all.len()
    } else {
        (idx + all.len() - 1) % all.len()
    };
    all[next]
}

fn next_hero_image_class(
    current: crate::model::HeroImageClass,
    forward: bool,
) -> crate::model::HeroImageClass {
    use crate::model::HeroImageClass;
    let all = [
        HeroImageClass::Contained,
        HeroImageClass::ContainedMd,
        HeroImageClass::ContainedLg,
        HeroImageClass::ContainedXl,
        HeroImageClass::ContainedXxl,
        HeroImageClass::FullFull,
        HeroImageClass::FullContained,
        HeroImageClass::FullContainedMd,
        HeroImageClass::FullContainedLg,
        HeroImageClass::FullContainedXl,
        HeroImageClass::FullContainedXxl,
    ];
    let idx = all.iter().position(|v| *v == current).unwrap_or(0);
    let next_idx = if forward {
        (idx + 1) % all.len()
    } else if idx == 0 {
        all.len() - 1
    } else {
        idx - 1
    };
    all[next_idx]
}

fn next_parent_data_aos(current: crate::model::HeroAos, forward: bool) -> crate::model::HeroAos {
    use crate::model::HeroAos;
    let all = [
        HeroAos::FadeIn,
        HeroAos::FadeUp,
        HeroAos::FadeRight,
        HeroAos::FadeDown,
        HeroAos::FadeLeft,
        HeroAos::ZoomIn,
        HeroAos::ZoomInUp,
        HeroAos::ZoomInDown,
    ];
    let idx = all.iter().position(|v| *v == current).unwrap_or(0);
    let next_idx = if forward {
        (idx + 1) % all.len()
    } else if idx == 0 {
        all.len() - 1
    } else {
        idx - 1
    };
    all[next_idx]
}

fn next_hero_link_1_target(
    current: crate::model::CtaTarget,
    forward: bool,
) -> crate::model::CtaTarget {
    use crate::model::CtaTarget;
    let all = [CtaTarget::SelfTarget, CtaTarget::Blank];
    let idx = all.iter().position(|v| *v == current).unwrap_or(0);
    let next_idx = if forward {
        (idx + 1) % all.len()
    } else if idx == 0 {
        all.len() - 1
    } else {
        idx - 1
    };
    all[next_idx]
}

impl AppTheme {
    fn load() -> anyhow::Result<Self> {
        let path = theme_file_candidates()
            .into_iter()
            .find(|candidate| candidate.exists());
        let Some(path) = path else {
            return Ok(Self::default());
        };

        let raw = std::fs::read_to_string(&path)
            .map_err(|e| anyhow::anyhow!("could not read '{}': {}", path.display(), e))?;
        let theme_file: ThemeFile = serde_yaml::from_str(&raw)
            .map_err(|e| anyhow::anyhow!("invalid theme file '{}': {}", path.display(), e))?;
        Self::from_palette(theme_file.colors)
    }

    fn from_palette(p: PaletteFile) -> anyhow::Result<Self> {
        // Core backgrounds
        let background = parse_hex_color(p.base_background.as_str())?;
        let panel_background = parse_hex_color(
            p.body_background
                .as_deref()
                .unwrap_or(p.base_background.as_str()),
        )?;
        let popup_background = parse_hex_color(
            p.modal_background
                .as_deref()
                .unwrap_or(p.base_background.as_str()),
        )?;

        // Text colors
        let foreground = parse_hex_color(p.text_primary.as_str())?;
        let muted = parse_hex_color(p.text_secondary.as_deref().unwrap_or("#9ea3aa"))?;
        let disabled = parse_hex_color(p.text_disabled.as_deref().unwrap_or("#a0a4a8"))?;
        let text_inverse = parse_hex_color(p.text_inverse.as_deref().unwrap_or("#f9fafb"))?;
        let text_labels = parse_hex_color(p.text_labels.as_deref().unwrap_or("#ffaf46"))?;
        let text_active_focus =
            parse_hex_color(p.text_active_focus.as_deref().unwrap_or("#64b4f5"))?;
        let modal_labels = parse_hex_color(p.modal_labels.as_deref().unwrap_or("#64b4f5"))?;
        let modal_text = parse_hex_color(p.modal_text.as_deref().unwrap_or(p.text_primary.as_str()))?;

        // Selection
        let selected_background = parse_hex_color(p.selected_background.as_str())?;

        // Borders
        let border = parse_hex_color(p.border_default.as_str())?;
        let border_active = parse_hex_color(p.border_active.as_deref().unwrap_or("#6ec8ff"))?;

        // Scrollbar
        let scrollbar = parse_hex_color(p.scrollbar.as_deref().unwrap_or("#ffa087"))?;
        let scrollbar_hover =
            parse_hex_color(p.scrollbar_hover.as_deref().unwrap_or("#64b4f5"))?;

        // Input field colors — prefer new split names; fall back to old input_default/input_focus.
        let input_border_default = parse_hex_color(
            p.input_border_default
                .as_deref()
                .or(p.input_default.as_deref())
                .unwrap_or(p.border_default.as_str()),
        )?;
        let input_border_focus = parse_hex_color(
            p.input_border_focus
                .as_deref()
                .or(p.input_focus.as_deref())
                .unwrap_or("#64b4f5"),
        )?;
        let input_text_default = parse_hex_color(
            p.input_text_default
                .as_deref()
                .or(p.input_default.as_deref())
                .unwrap_or(p.text_primary.as_str()),
        )?;
        let input_text_focus = parse_hex_color(
            p.input_text_focus
                .as_deref()
                .or(p.input_focus.as_deref())
                .unwrap_or("#64b4f5"),
        )?;
        let cursor = parse_hex_color(p.cursor.as_deref().unwrap_or("#64b4f5"))?;

        // Back-compat aliases (keep the old semantics for any untouched code paths).
        let input_default = input_border_default;
        let input_focus = input_border_focus;

        // Accents
        let title_seed = p
            .modal_labels
            .as_deref()
            .or(p.text_active_focus.as_deref())
            .or(p.input_border_focus.as_deref())
            .or(p.input_focus.as_deref())
            .unwrap_or(p.text_primary.as_str());
        let title = parse_hex_color(title_seed)?;
        let active = parse_hex_color(p.active.as_deref().unwrap_or("#6ec8ff"))?;

        // Semantic
        let success = parse_hex_color(p.success.as_deref().unwrap_or("#1e8449"))?;
        let warning = parse_hex_color(p.warning.as_deref().unwrap_or("#b9770e"))?;
        let error = parse_hex_color(p.error.as_deref().unwrap_or("#a93226"))?;
        let info = parse_hex_color(p.info.as_deref().unwrap_or("#21618c"))?;

        // File roles (THEME_STRUCTURE_STANDARD.md section 8)
        let folders = parse_hex_color(p.folders.as_deref().unwrap_or("#64b4f5"))?;
        let files = parse_hex_color(p.files.as_deref().unwrap_or("#ffaf46"))?;
        let links = parse_hex_color(p.links.as_deref().unwrap_or("#ffa087"))?;

        Ok(Self {
            background,
            panel_background,
            popup_background,
            foreground,
            muted,
            disabled,
            text_inverse,
            text_labels,
            text_active_focus,
            modal_labels,
            modal_text,
            title,
            active,
            border,
            border_active,
            input_border_default,
            input_border_focus,
            input_text_default,
            input_text_focus,
            cursor,
            scrollbar,
            scrollbar_hover,
            selected_background,
            selected_foreground: foreground,
            success,
            warning,
            error,
            info,
            folders,
            files,
            links,
            input_default,
            input_focus,
        })
    }
}

impl Default for AppTheme {
    fn default() -> Self {
        let border_def = Color::Rgb(245, 246, 247);
        let border_focus = Color::Rgb(100, 180, 245);
        Self {
            background: Color::Rgb(15, 17, 20),
            panel_background: Color::Rgb(42, 45, 49),
            popup_background: Color::Rgb(28, 30, 33),
            foreground: Color::Rgb(245, 246, 247),
            muted: Color::Rgb(158, 163, 170),
            disabled: Color::Rgb(90, 95, 102),
            text_inverse: Color::Rgb(15, 17, 20),
            text_labels: Color::Rgb(255, 175, 70),
            text_active_focus: border_focus,
            modal_labels: border_focus,
            modal_text: Color::Rgb(245, 246, 247),
            title: border_focus,
            active: Color::Rgb(110, 200, 255),
            border: border_def,
            border_active: border_focus,
            input_border_default: border_def,
            input_border_focus: border_focus,
            input_text_default: Color::Rgb(245, 246, 247),
            input_text_focus: border_focus,
            cursor: border_focus,
            scrollbar: Color::Rgb(255, 160, 135),
            scrollbar_hover: border_focus,
            selected_background: Color::Rgb(15, 17, 20),
            selected_foreground: Color::Rgb(245, 246, 247),
            success: Color::Rgb(130, 224, 170),
            warning: Color::Rgb(245, 196, 105),
            error: Color::Rgb(229, 115, 115),
            info: Color::Rgb(93, 173, 226),
            folders: Color::Rgb(100, 180, 245),
            files: Color::Rgb(255, 175, 70),
            links: Color::Rgb(255, 160, 135),
            input_default: border_def,
            input_focus: border_focus,
        }
    }
}

fn theme_file_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    // Per THEME_STRUCTURE_STANDARD.md: project-local override wins over
    // user-global default, which wins over built-in defaults.
    candidates.push(PathBuf::from("dd_staticsite_theme.yml"));
    candidates.push(PathBuf::from("theme.yml"));
    candidates.push(PathBuf::from(".theme.yml"));
    if let Some(home) = std::env::var_os("HOME") {
        let base = Path::new(&home).join(".config").join("ldnddev");
        candidates.push(base.join("dd_staticsite_theme.yml"));
        candidates.push(base.join("dd_staticsite").join(".theme.yml"));
    }
    candidates
}

fn parse_hex_color(raw: &str) -> anyhow::Result<Color> {
    let hex = raw.trim().trim_start_matches('#');
    if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(anyhow::anyhow!(
            "expected hex color like '#RRGGBB', got '{}'",
            raw
        ));
    }
    let r = u8::from_str_radix(&hex[0..2], 16)?;
    let g = u8::from_str_radix(&hex[2..4], 16)?;
    let b = u8::from_str_radix(&hex[4..6], 16)?;
    Ok(Color::Rgb(r, g, b))
}

fn component_search_haystack(kind: ComponentKind) -> String {
    let label = kind.label();
    let underscore = label.replace('-', "_");
    let short = label
        .trim_start_matches("dd-")
        .replace('-', "_")
        .to_string();
    format!("{label} {underscore} {short}")
}

fn fuzzy_score(query: &str, text: &str) -> Option<i32> {
    let q = query.to_ascii_lowercase();
    let t = text.to_ascii_lowercase();
    if q.is_empty() {
        return Some(0);
    }
    if t.contains(&q) {
        return Some(1000 - (t.find(&q).unwrap_or(0) as i32));
    }
    let mut score = 0i32;
    let mut t_chars = t.chars().enumerate();
    let mut last_idx: Option<usize> = None;
    for qc in q.chars() {
        let mut found = None;
        for (idx, tc) in t_chars.by_ref() {
            if tc == qc {
                found = Some(idx);
                break;
            }
        }
        let Some(idx) = found else {
            return None;
        };
        score += 10;
        if let Some(prev) = last_idx {
            if idx == prev + 1 {
                score += 8;
            }
        }
        if idx == 0 {
            score += 6;
        }
        last_idx = Some(idx);
    }
    Some(score)
}

fn next_section_id_for_page(page: &crate::model::Page) -> String {
    let mut used = HashSet::new();
    for node in &page.nodes {
        if let PageNode::Section(section) = node {
            if !section.id.trim().is_empty() {
                used.insert(section.id.clone());
            }
        }
    }
    let mut idx = 1usize;
    loop {
        let candidate = format!("section-{}", idx);
        if !used.contains(&candidate) {
            return candidate;
        }
        idx += 1;
    }
}

fn ensure_page_section_ids(page: &mut crate::model::Page) {
    let mut used = HashSet::new();
    let mut next_idx = 1usize;
    for node in &mut page.nodes {
        let PageNode::Section(section) = node else {
            continue;
        };
        let current = section.id.trim().to_string();
        if !current.is_empty() && !used.contains(&current) {
            used.insert(current);
            continue;
        }
        loop {
            let candidate = format!("section-{}", next_idx);
            next_idx += 1;
            if !used.contains(&candidate) {
                section.id = candidate.clone();
                used.insert(candidate);
                break;
            }
        }
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn component_edit_group_for_mode(mode: InputMode) -> Option<&'static [InputMode]> {
    match mode {
        InputMode::EditHeroImage
        | InputMode::EditHeroClass
        | InputMode::EditHeroAos
        | InputMode::EditHeroCustomCss
        | InputMode::EditHeroTitle
        | InputMode::EditHeroSubtitle
        | InputMode::EditHeroCopy
        | InputMode::EditHeroCtaText
        | InputMode::EditHeroCtaLink
        | InputMode::EditHeroCtaTarget
        | InputMode::EditHeroCtaText2
        | InputMode::EditHeroCtaLink2
        | InputMode::EditHeroCtaTarget2 => Some(&[
            InputMode::EditHeroImage,
            InputMode::EditHeroClass,
            InputMode::EditHeroAos,
            InputMode::EditHeroCustomCss,
            InputMode::EditHeroTitle,
            InputMode::EditHeroSubtitle,
            InputMode::EditHeroCopy,
            InputMode::EditHeroCtaText,
            InputMode::EditHeroCtaLink,
            InputMode::EditHeroCtaTarget,
            InputMode::EditHeroCtaText2,
            InputMode::EditHeroCtaLink2,
            InputMode::EditHeroCtaTarget2,
        ]),
        InputMode::EditSectionId | InputMode::EditSectionTitle | InputMode::EditSectionClass => {
            Some(&[
                InputMode::EditSectionId,
                InputMode::EditSectionTitle,
                InputMode::EditSectionClass,
            ])
        }
        InputMode::EditBannerClass
        | InputMode::EditBannerDataAos
        | InputMode::EditBannerImageUrl
        | InputMode::EditBannerImageAlt => Some(&[
            InputMode::EditBannerClass,
            InputMode::EditBannerDataAos,
            InputMode::EditBannerImageUrl,
            InputMode::EditBannerImageAlt,
        ]),
        InputMode::EditCtaClass
        | InputMode::EditCtaImageUrl
        | InputMode::EditCtaImageAlt
        | InputMode::EditCtaDataAos
        | InputMode::EditCtaTitle
        | InputMode::EditCtaSubtitle
        | InputMode::EditCtaCopy
        | InputMode::EditCtaLinkUrl
        | InputMode::EditCtaLinkTarget
        | InputMode::EditCtaLinkLabel => Some(&[
            InputMode::EditCtaClass,
            InputMode::EditCtaImageUrl,
            InputMode::EditCtaImageAlt,
            InputMode::EditCtaDataAos,
            InputMode::EditCtaTitle,
            InputMode::EditCtaSubtitle,
            InputMode::EditCtaCopy,
            InputMode::EditCtaLinkUrl,
            InputMode::EditCtaLinkTarget,
            InputMode::EditCtaLinkLabel,
        ]),
        InputMode::EditBlockquoteDataAos
        | InputMode::EditBlockquoteImageUrl
        | InputMode::EditBlockquoteImageAlt
        | InputMode::EditBlockquotePersonsName
        | InputMode::EditBlockquotePersonsTitle
        | InputMode::EditBlockquoteCopy => Some(&[
            InputMode::EditBlockquoteDataAos,
            InputMode::EditBlockquoteImageUrl,
            InputMode::EditBlockquoteImageAlt,
            InputMode::EditBlockquotePersonsName,
            InputMode::EditBlockquotePersonsTitle,
            InputMode::EditBlockquoteCopy,
        ]),
        InputMode::EditCardType
        | InputMode::EditCardDataAos
        | InputMode::EditCardWidth
        | InputMode::EditCardItemImageUrl
        | InputMode::EditCardItemImageAlt
        | InputMode::EditCardItemTitle
        | InputMode::EditCardItemSubtitle
        | InputMode::EditCardItemCopy
        | InputMode::EditCardItemLinkUrl
        | InputMode::EditCardItemLinkTarget
        | InputMode::EditCardItemLinkLabel => Some(&[
            InputMode::EditCardType,
            InputMode::EditCardDataAos,
            InputMode::EditCardWidth,
            InputMode::EditCardItemImageUrl,
            InputMode::EditCardItemImageAlt,
            InputMode::EditCardItemTitle,
            InputMode::EditCardItemSubtitle,
            InputMode::EditCardItemCopy,
            InputMode::EditCardItemLinkUrl,
            InputMode::EditCardItemLinkTarget,
            InputMode::EditCardItemLinkLabel,
        ]),
        InputMode::EditFilmstripType
        | InputMode::EditFilmstripDataAos
        | InputMode::EditFilmstripItemImageUrl
        | InputMode::EditFilmstripItemImageAlt
        | InputMode::EditFilmstripItemTitle => Some(&[
            InputMode::EditFilmstripType,
            InputMode::EditFilmstripDataAos,
            InputMode::EditFilmstripItemImageUrl,
            InputMode::EditFilmstripItemImageAlt,
            InputMode::EditFilmstripItemTitle,
        ]),
        InputMode::EditMilestonesDataAos
        | InputMode::EditMilestonesWidth
        | InputMode::EditMilestonesItemPercentage
        | InputMode::EditMilestonesItemTitle
        | InputMode::EditMilestonesItemSubtitle
        | InputMode::EditMilestonesItemCopy
        | InputMode::EditMilestonesItemLinkUrl
        | InputMode::EditMilestonesItemLinkTarget
        | InputMode::EditMilestonesItemLinkLabel => Some(&[
            InputMode::EditMilestonesDataAos,
            InputMode::EditMilestonesWidth,
            InputMode::EditMilestonesItemPercentage,
            InputMode::EditMilestonesItemTitle,
            InputMode::EditMilestonesItemSubtitle,
            InputMode::EditMilestonesItemCopy,
            InputMode::EditMilestonesItemLinkUrl,
            InputMode::EditMilestonesItemLinkTarget,
            InputMode::EditMilestonesItemLinkLabel,
        ]),
        InputMode::EditSliderTitle
        | InputMode::EditSliderItemTitle
        | InputMode::EditSliderItemCopy
        | InputMode::EditSliderItemLinkUrl
        | InputMode::EditSliderItemLinkTarget
        | InputMode::EditSliderItemLinkLabel
        | InputMode::EditSliderItemImageUrl
        | InputMode::EditSliderItemImageAlt => Some(&[
            InputMode::EditSliderTitle,
            InputMode::EditSliderItemTitle,
            InputMode::EditSliderItemCopy,
            InputMode::EditSliderItemLinkUrl,
            InputMode::EditSliderItemLinkTarget,
            InputMode::EditSliderItemLinkLabel,
            InputMode::EditSliderItemImageUrl,
            InputMode::EditSliderItemImageAlt,
        ]),
        InputMode::EditModalTitle | InputMode::EditModalCopy => {
            Some(&[InputMode::EditModalTitle, InputMode::EditModalCopy])
        }
        InputMode::EditAccordionType
        | InputMode::EditAccordionClass
        | InputMode::EditAccordionAos
        | InputMode::EditAccordionGroupName
        | InputMode::EditAccordionFirstTitle
        | InputMode::EditAccordionFirstContent => Some(&[
            InputMode::EditAccordionType,
            InputMode::EditAccordionClass,
            InputMode::EditAccordionAos,
            InputMode::EditAccordionGroupName,
            InputMode::EditAccordionFirstTitle,
            InputMode::EditAccordionFirstContent,
        ]),
        InputMode::EditAlternatingType
        | InputMode::EditAlternatingClass
        | InputMode::EditAlternatingDataAos
        | InputMode::EditAlternatingItemImage
        | InputMode::EditAlternatingItemImageAlt
        | InputMode::EditAlternatingItemTitle
        | InputMode::EditAlternatingItemCopy => Some(&[
            InputMode::EditAlternatingType,
            InputMode::EditAlternatingClass,
            InputMode::EditAlternatingDataAos,
            InputMode::EditAlternatingItemImage,
            InputMode::EditAlternatingItemImageAlt,
            InputMode::EditAlternatingItemTitle,
            InputMode::EditAlternatingItemCopy,
        ]),
        _ => None,
    }
}

fn help_text() -> String {
    [
        "Global:",
        "  F1: Open/close this help",
        "  F3: Validate site (shows errors in a modal)",
        "  Shift+E: Export site to HTML (validates first; prompts for output dir on first use)",
        "  p: Preview current page in browser (validates + exports first)",
        "  Ctrl+Q: Quit",
        "  s: Open save modal and enter file path (also writes a .backup checkpoint)",
        "  Tab / Shift+Tab: Next/previous page",
        "",
        "Autosave: 2s after a change, the active site JSON is rewritten. The .backup is only refreshed by manual `s` saves; on next load the TUI surfaces a toast when site.json and site.json.backup differ.",
        "",
        "Node navigation and edits:",
        "  Up/Down or mouse wheel: Select row in Nodes tree",
        "  PageUp/PageDown: Scroll Details blueprint panel",
        "  Enter: Edit selected row",
        "  Space: Expand/collapse selected section or accordion/alternating/card/filmstrip/milestones/slider items",
        "  /: Open insert fuzzy finder (hero/section/cta/banner/blockquote/accordion/alternating/card/filmstrip/milestones/modal/slider)",
        "  A / X: Add/remove dd-accordion, dd-alternating, dd-card, dd-filmstrip, dd-milestones, or dd-slider item",
        "  d: Delete selected node",
        "",
        "Pages panel ([2] Nodes):",
        "  Shift+A: Add page (title prompt → template picker: Blank / Hero only / Hero + Section / Duplicate)",
        "  Shift+X: Delete current page (confirms; refuses if only 1 page)",
        "  u: Undo last page deletion (session only)",
        "  Shift+J / Shift+K: Move current page down / up (also = sitemap order)",
        "  r: Rename page (auto-slug until first disk save; locked pages expose a Slug field in [HEAD])",
        "",
        "Section layout:",
        "  C / V: Add/remove selected column",
        "  c / v: Select previous/next column",
        "  J / K: Move selected column down/up",
        "  r / f: Edit selected column id / width class",
        "  Details pane shows ASCII blueprint for all page items",
        "",
        "Edit modal:",
        "  Any edit command opens a modal with editable fields",
        "  Tab / Shift+Tab: Next/previous editable field for selected row",
        "  Ctrl+P (in image URL field): Open image picker (./source/images/)",
        "  Ctrl+P (in link URL field): Open page picker (lists site pages, writes /<slug>)",
        "  hero.copy / alternating_copy / accordion_copy / parent_copy / child_copy / child_copy / parent_copy: Up/Down move line, wheel scroll, Enter newline, Ctrl+S save",
        "  Left / Right: Cycle section/hero/cta/banner/accordion/alternating/blockquote/card/filmstrip/milestones/slider option fields when active",
        "  Enter: Confirm edit",
        "  Esc: Cancel edit",
        "  Backspace: Delete character",
    ]
    .join("\n")
}

impl ComponentKind {
    fn all() -> &'static [Self] {
        &[
            Self::Hero,
            Self::Section,
            Self::Cta,
            Self::Banner,
            Self::Blockquote,
            Self::Accordion,
            Self::Alternating,
            Self::Card,
            Self::Filmstrip,
            Self::Milestones,
            Self::Modal,
            Self::Slider,
            Self::Alert,
            Self::Image,
            Self::RichText,
            Self::Navigation,
            Self::HeaderSearch,
            Self::HeaderMenu,
        ]
    }

    fn label(self) -> &'static str {
        match self {
            ComponentKind::Hero => "dd-hero",
            ComponentKind::Section => "dd-section",
            ComponentKind::Cta => "dd-cta",
            ComponentKind::Banner => "dd-banner",
            ComponentKind::Blockquote => "dd-blockquote",
            ComponentKind::Accordion => "dd-accordion",
            ComponentKind::Alternating => "dd-alternating",
            ComponentKind::Card => "dd-card",
            ComponentKind::Filmstrip => "dd-filmstrip",
            ComponentKind::Milestones => "dd-milestones",
            ComponentKind::Modal => "dd-modal",
            ComponentKind::Slider => "dd-slider",
            ComponentKind::Alert => "dd-alert",
            ComponentKind::Image => "dd-image",
            ComponentKind::RichText => "dd-rich_text",
            ComponentKind::Navigation => "dd-navigation",
            ComponentKind::HeaderSearch => "dd-header-search",
            ComponentKind::HeaderMenu => "dd-header-menu",
        }
    }

    fn default_component(self) -> crate::model::SectionComponent {
        match self {
            ComponentKind::Hero | ComponentKind::Section => {
                unreachable!("top-level kinds do not map to section components")
            }
            ComponentKind::Cta => crate::model::SectionComponent::Cta(crate::model::DdCta {
                parent_class: crate::model::CtaClass::TopLeft,
                parent_image_url: "https://dummyimage.com/1920x1080/000000/fff".to_string(),
                parent_image_alt: "Image alt".to_string(),
                parent_data_aos: crate::model::HeroAos::FadeIn,
                parent_title: "Title".to_string(),
                parent_subtitle: "Subtitle".to_string(),
                parent_copy: "Copy".to_string(),
                parent_link_url: Some("/path".to_string()),
                parent_link_target: Some(crate::model::CardLinkTarget::SelfTarget),
                parent_link_label: Some("Learn More".to_string()),
            }),
            ComponentKind::Banner => {
                crate::model::SectionComponent::Banner(crate::model::DdBanner {
                    parent_class: crate::model::BannerClass::BgCenterCenter,
                    parent_data_aos: crate::model::HeroAos::FadeIn,
                    parent_image_url: "https://dummyimage.com/1920x1080/000/fff".to_string(),
                    parent_image_alt: "Banner alt text".to_string(),
                })
            }
            ComponentKind::Blockquote => {
                crate::model::SectionComponent::Blockquote(crate::model::DdBlockquote {
                    parent_data_aos: crate::model::HeroAos::FadeIn,
                    parent_image_url: "https://dummyimage.com/512x512/000/fff".to_string(),
                    parent_image_alt: "blockquote Persons Name".to_string(),
                    parent_name: "blockquote Persons Name".to_string(),
                    parent_role: "blockquote Persons Title".to_string(),
                    parent_copy: "blockquote content".to_string(),
                })
            }
            ComponentKind::Accordion => {
                crate::model::SectionComponent::Accordion(crate::model::DdAccordion {
                    parent_type: crate::model::AccordionType::Default,
                    parent_class: crate::model::AccordionClass::Primary,
                    parent_data_aos: crate::model::HeroAos::FadeIn,
                    parent_group_name: "group1".to_string(),
                    items: vec![crate::model::AccordionItem {
                        child_title: "Accordion Item".to_string(),
                        child_copy: "Accordion content".to_string(),
                    }],
                    multiple: Some(false),
                })
            }
            ComponentKind::Alternating => {
                crate::model::SectionComponent::Alternating(crate::model::DdAlternating {
                    parent_type: crate::model::AlternatingType::Default,
                    parent_class: "-default".to_string(),
                    parent_data_aos: crate::model::HeroAos::FadeIn,
                    items: vec![crate::model::AlternatingItem {
                        child_image_url: "https://dummyimage.com/600x400/000/fff".to_string(),
                        child_image_alt: "Alternating image".to_string(),
                        child_title: "Alternating Item".to_string(),
                        child_copy: "Alternating content".to_string(),
                    }],
                })
            }
            ComponentKind::Card => crate::model::SectionComponent::Card(crate::model::DdCard {
                parent_type: crate::model::CardType::Default,
                parent_data_aos: crate::model::HeroAos::FadeIn,
                parent_width: "dd-u-1-1 dd-u-md-12-24 dd-u-lg-8-24".to_string(),
                items: vec![crate::model::CardItem {
                    child_image_url: "https://dummyimage.com/720x720/000/fff".to_string(),
                    child_image_alt: "Image alt text".to_string(),
                    child_title: "Title".to_string(),
                    child_subtitle: "Subtitle".to_string(),
                    child_copy: "Copy".to_string(),
                    child_link_url: Some("/front".to_string()),
                    child_link_target: Some(crate::model::CardLinkTarget::SelfTarget),
                    child_link_label: Some("Learn More".to_string()),
                }],
            }),
            ComponentKind::Filmstrip => {
                crate::model::SectionComponent::Filmstrip(crate::model::DdFilmstrip {
                    parent_type: crate::model::FilmstripType::Default,
                    parent_data_aos: crate::model::HeroAos::FadeIn,
                    items: vec![crate::model::FilmstripItem {
                        child_image_url: "https://dummyimage.com/256x256/000/fff".to_string(),
                        child_image_alt: "Image alt text".to_string(),
                        child_title: "Title".to_string(),
                    }],
                })
            }
            ComponentKind::Milestones => {
                crate::model::SectionComponent::Milestones(crate::model::DdMilestones {
                    parent_data_aos: crate::model::HeroAos::FadeIn,
                    parent_width: "dd-u-1-1 dd-u-md-12-24".to_string(),
                    items: vec![crate::model::MilestonesItem {
                        child_percentage: "70".to_string(),
                        child_title: "Title".to_string(),
                        child_subtitle: "Subtitle".to_string(),
                        child_copy: "Copy".to_string(),
                        child_link_url: None,
                        child_link_target: Some(crate::model::CardLinkTarget::SelfTarget),
                        child_link_label: None,
                    }],
                })
            }
            ComponentKind::Modal => crate::model::SectionComponent::Modal(crate::model::DdModal {
                parent_title: "Title".to_string(),
                parent_copy: "Copy".to_string(),
            }),
            ComponentKind::Slider => {
                crate::model::SectionComponent::Slider(crate::model::DdSlider {
                    parent_title: String::new(),
                    items: vec![crate::model::SliderItem {
                        child_title: "Title".to_string(),
                        child_copy: "Copy".to_string(),
                        child_link_url: Some("/path".to_string()),
                        child_link_target: Some(crate::model::CardLinkTarget::SelfTarget),
                        child_link_label: Some("Learn More".to_string()),
                        child_image_url: "https://dummyimage.com/720x720/000/fff".to_string(),
                        child_image_alt: "Image alt text".to_string(),
                    }],
                })
            }
            ComponentKind::Alert => crate::model::SectionComponent::Alert(crate::model::DdAlert {
                parent_type: crate::model::AlertType::Default,
                parent_class: crate::model::AlertClass::Default,
                parent_data_aos: crate::model::HeroAos::FadeIn,
                parent_title: Some("Alert Title".to_string()),
                parent_copy: "Alert content".to_string(),
            }),
            ComponentKind::Image => crate::model::SectionComponent::Image(crate::model::DdImage {
                parent_data_aos: crate::model::HeroAos::FadeIn,
                parent_image_url: "https://dummyimage.com/1200x600/000/fff".to_string(),
                parent_image_alt: "Image alt text".to_string(),
                parent_link_url: None,
                parent_link_target: None,
            }),
            ComponentKind::RichText => {
                crate::model::SectionComponent::RichText(crate::model::DdRichText {
                    parent_class: None,
                    parent_data_aos: crate::model::HeroAos::FadeIn,
                    parent_copy: "Copy".to_string(),
                })
            }
            ComponentKind::Navigation => {
                crate::model::SectionComponent::Navigation(crate::model::DdNavigation {
                    parent_type: crate::model::NavigationType::HeaderNav,
                    parent_class: crate::model::NavigationClass::MainMenu,
                    parent_data_aos: crate::model::HeroAos::FadeIn,
                    parent_width: "dd-u-1-1 dd-u-sm-1-1 dd-u-md-1-1 dd-u-lg-18-24".to_string(),
                    items: vec![crate::model::NavigationItem {
                        child_kind: crate::model::NavigationKind::Link,
                        child_link_label: "Home".to_string(),
                        child_link_url: Some("/".to_string()),
                        child_link_target: Some(crate::model::CardLinkTarget::SelfTarget),
                        child_link_css: None,
                        items: Vec::new(),
                    }],
                })
            }
            ComponentKind::HeaderSearch => {
                crate::model::SectionComponent::HeaderSearch(crate::model::DdHeaderSearch {
                    parent_width: "dd-u-3-24 dd-u-sm-3-24 dd-u-md-3-24 dd-u-lg-4-24".to_string(),
                    parent_data_aos: crate::model::HeroAos::FadeIn,
                })
            }
            ComponentKind::HeaderMenu => {
                crate::model::SectionComponent::HeaderMenu(crate::model::DdHeaderMenu {
                    parent_width: "dd-u-3-24 dd-u-sm-3-24 dd-u-md-3-24".to_string(),
                    parent_data_aos: crate::model::HeroAos::FadeIn,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyEvent;

    fn app_with_card() -> App {
        let mut app = App::new(Site::starter(), None, AppTheme::default());
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
        let mut app = App::new(Site::starter(), None, AppTheme::default());
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
        assert!(app.status.contains("must keep at least one item"));
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
        let mut app = App::new(Site::starter(), None, AppTheme::default());
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
        let mut app = App::new(Site::starter(), None, AppTheme::default());
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
        let mut app = App::new(Site::starter(), None, AppTheme::default());
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
        // Cycle parent_data_aos once (focused field 0).
        send_key(&mut app, KeyCode::Right, KeyModifiers::NONE);
        send_key(&mut app, KeyCode::Char('s'), KeyModifiers::CONTROL);
        match &app.site.pages[0].nodes[1] {
            PageNode::Section(s) => match &s.columns[0].components[0] {
                crate::model::SectionComponent::Image(i) => assert_eq!(
                    i.parent_data_aos,
                    crate::model::HeroAos::FadeUp,
                    "image data_aos should advance one step from default"
                ),
                _ => panic!("expected Image"),
            },
            _ => panic!("expected Section"),
        }
    }

    #[test]
    fn tier_a_header_search_form_edit_round_trip() {
        // HeaderSearch only valid in header region, so build a scenario there.
        let mut app = App::new(Site::starter(), None, AppTheme::default());
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
        // parent_data_aos first: cycle once.
        send_key(&mut app, KeyCode::Right, KeyModifiers::NONE);
        send_key(&mut app, KeyCode::Char('s'), KeyModifiers::CONTROL);
        match &app.site.pages[0].nodes[1] {
            PageNode::Section(s) => match &s.columns[0].components[0] {
                crate::model::SectionComponent::Blockquote(bq) => {
                    assert_eq!(bq.parent_data_aos, crate::model::HeroAos::FadeUp);
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
    #[test]
    fn tier_c_hero_form_edit_round_trip() {
        let mut app = App::new(Site::starter(), None, AppTheme::default());
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
        let mut app = App::new(Site::starter(), None, AppTheme::default());
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
        let mut app = App::new(Site::starter(), None, AppTheme::default());
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
        let mut app = App::new(Site::starter(), None, AppTheme::default());
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
        let mut app = App::new(Site::starter(), None, AppTheme::default());
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
        let mut app = App::new(Site::starter(), None, AppTheme::default());
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
        let mut app = App::new(Site::starter(), None, AppTheme::default());
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
        let mut app = App::new(Site::starter(), None, AppTheme::default());
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
        let mut app = App::new(Site::starter(), None, AppTheme::default());
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
        let mut app = App::new(Site::starter(), None, AppTheme::default());
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
        let mut app = App::new(Site::starter(), None, AppTheme::default());
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
        let mut app = App::new(Site::starter(), None, AppTheme::default());
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
        let mut app = App::new(Site::starter(), None, AppTheme::default());
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
        let mut app = App::new(Site::starter(), None, AppTheme::default());
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
        let mut app = App::new(Site::starter(), None, AppTheme::default());
        app.selected_sidebar_section = SidebarSection::Pages;
        app.selected_page = 0;
        send_key(&mut app, KeyCode::Char('J'), KeyModifiers::SHIFT);
        assert_eq!(app.selected_page, 0);
        assert_eq!(app.site.pages[0].head.title, "Home");
    }

    #[test]
    fn pages_panel_r_renames_and_regenerates_slug_when_unlocked() {
        let mut app = App::new(Site::starter(), None, AppTheme::default());
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
        let mut app = App::new(Site::starter(), None, AppTheme::default());
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

    #[test]
    fn page_head_modal_always_shows_slug_field() {
        let mut app = App::new(Site::starter(), None, AppTheme::default());
        // Unlocked starter still exposes the Slug field.
        assert!(!app.site.pages[0].slug_locked);
        app.open_page_head_edit_modal();
        let fields = app
            .edit_modal
            .as_ref()
            .map(|m| m.fields.clone())
            .expect("expected page-head modal");
        let slug_field = fields
            .iter()
            .find(|f| f.label == "Slug")
            .expect("Slug field should always appear");
        assert_eq!(slug_field.value, app.site.pages[0].slug);
    }

    #[test]
    fn page_head_modal_save_writes_slug_and_locks_when_edited() {
        let mut app = App::new(Site::starter(), None, AppTheme::default());
        assert!(!app.site.pages[0].slug_locked);
        app.open_page_head_edit_modal();
        let modal = app.edit_modal.as_mut().unwrap();
        for f in modal.fields.iter_mut() {
            if f.label == "Slug" {
                f.value = "new-slug".to_string();
                f.buffer = "new-slug".to_string();
            }
        }
        let fields = modal.fields.clone();
        let ok = app.save_page_head_changes(&fields);
        assert!(ok);
        assert_eq!(app.site.pages[0].slug, "new-slug");
        assert!(
            app.site.pages[0].slug_locked,
            "editing the slug field must lock the slug"
        );
    }

    #[test]
    fn page_head_modal_save_leaves_slug_unchanged_when_user_did_not_edit_it() {
        let mut app = App::new(Site::starter(), None, AppTheme::default());
        let orig_slug = app.site.pages[0].slug.clone();
        app.open_page_head_edit_modal();
        let fields = app.edit_modal.as_ref().unwrap().fields.clone();
        let ok = app.save_page_head_changes(&fields);
        assert!(ok);
        assert_eq!(app.site.pages[0].slug, orig_slug);
        assert!(!app.site.pages[0].slug_locked, "no slug edit means no lock");
    }

    #[test]
    fn page_head_modal_default_og_title_is_page_title() {
        let mut app = App::new(Site::starter(), None, AppTheme::default());
        assert!(app.site.pages[0].head.og_title.is_none());
        app.open_page_head_edit_modal();
        let og_field = app
            .edit_modal
            .as_ref()
            .unwrap()
            .fields
            .iter()
            .find(|f| f.label == "OG Title")
            .cloned()
            .expect("OG Title field should exist");
        assert_eq!(
            og_field.value, app.site.pages[0].head.title,
            "OG Title should default to the page title when unset"
        );
    }

    #[test]
    fn page_head_modal_default_canonical_is_slug_path() {
        let mut app = App::new(Site::starter(), None, AppTheme::default());
        // Starter page has no canonical_url set.
        assert!(app.site.pages[0].head.canonical_url.is_none());
        app.open_page_head_edit_modal();
        let canon_field = app
            .edit_modal
            .as_ref()
            .unwrap()
            .fields
            .iter()
            .find(|f| f.label == "Canonical URL")
            .cloned()
            .expect("Canonical URL field should exist");
        assert_eq!(
            canon_field.value,
            format!("/{}", app.site.pages[0].slug),
            "canonical should default to /<slug> when unset"
        );
    }

    #[test]
    fn page_head_title_rename_regens_slug_when_unlocked() {
        let mut app = App::new(Site::starter(), None, AppTheme::default());
        assert!(!app.site.pages[0].slug_locked);
        app.open_page_head_edit_modal();
        let modal = app.edit_modal.as_mut().unwrap();
        for f in modal.fields.iter_mut() {
            if f.label == "Title" {
                f.value = "About Us".to_string();
                f.buffer = "About Us".to_string();
            }
        }
        let fields = modal.fields.clone();
        let ok = app.save_page_head_changes(&fields);
        assert!(ok);
        assert_eq!(app.site.pages[0].head.title, "About Us");
        assert_eq!(
            app.site.pages[0].slug, "about-us",
            "slug should regenerate from title when unlocked"
        );
    }

    #[test]
    fn open_validation_modal_on_clean_starter_pushes_success_toast_and_no_modal() {
        let mut app = App::new(Site::starter(), None, AppTheme::default());
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
        let mut app = App::new(Site::starter(), None, AppTheme::default());
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
        let mut app = App::new(Site::starter(), None, AppTheme::default());
        send_key(&mut app, KeyCode::F(3), KeyModifiers::NONE);
        assert!(app.modal.is_none());
        let last = app.toasts.last().expect("expected a success toast");
        assert_eq!(last.level, ToastLevel::Success);
        assert!(last.message.to_lowercase().contains("no validation errors"));
    }

    #[test]
    fn f3_with_validation_errors_opens_modal() {
        let mut app = App::new(Site::starter(), None, AppTheme::default());
        app.site.pages[0].slug = "".to_string();
        send_key(&mut app, KeyCode::F(3), KeyModifiers::NONE);
        assert!(matches!(app.modal, Some(Modal::ValidationErrors { .. })));
    }

    #[test]
    fn f3_then_enter_dismisses_modal() {
        let mut app = App::new(Site::starter(), None, AppTheme::default());
        app.site.pages[0].slug = "".to_string();
        send_key(&mut app, KeyCode::F(3), KeyModifiers::NONE);
        send_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        assert!(app.modal.is_none());
    }

    #[test]
    fn f3_then_j_k_scrolls_error_list() {
        let mut app = App::new(Site::starter(), None, AppTheme::default());
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
    fn begin_export_flow_on_clean_starter_without_export_dir_opens_path_prompt() {
        let mut app = App::new(Site::starter(), None, AppTheme::default());
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
        let mut app = App::new(Site::starter(), None, AppTheme::default());
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
        let json_path = tmp.join("site.json");
        let mut app = App::new(Site::starter(), Some(json_path.clone()), AppTheme::default());
        app.site.export_dir = Some("web".to_string());

        app.begin_export_flow();

        assert!(app.modal.is_none(), "no modal should open — direct export");
        let last = app.toasts.last().expect("expected a toast");
        assert_eq!(last.level, ToastLevel::Success);
        assert!(last.message.to_lowercase().contains("exported"));
        assert!(tmp.join("web").exists(), "export directory should have been created");

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn e_key_with_validation_errors_opens_validation_modal() {
        let mut app = App::new(Site::starter(), None, AppTheme::default());
        app.site.pages[0].slug = "".to_string();
        send_key(&mut app, KeyCode::Char('E'), KeyModifiers::SHIFT);
        assert!(matches!(app.modal, Some(Modal::ValidationErrors { .. })));
    }

    #[test]
    fn e_key_with_clean_site_and_no_export_dir_opens_path_prompt() {
        let mut app = App::new(Site::starter(), None, AppTheme::default());
        send_key(&mut app, KeyCode::Char('E'), KeyModifiers::SHIFT);
        assert!(matches!(app.modal, Some(Modal::ExportPathPrompt { .. })));
    }

    #[test]
    fn fresh_app_is_clean() {
        let app = App::new(Site::starter(), None, AppTheme::default());
        assert!(!app.dirty);
        assert!(app.dirty_since.is_none());
    }

    #[test]
    fn editing_a_page_title_marks_app_dirty() {
        let mut app = App::new(Site::starter(), None, AppTheme::default());
        app.site.pages[0].head.title = "Mutated".to_string();
        app.mark_dirty_if_changed();
        assert!(app.dirty);
        assert!(app.dirty_since.is_some());
    }

    #[test]
    fn unchanged_model_stays_clean() {
        let mut app = App::new(Site::starter(), None, AppTheme::default());
        app.mark_dirty_if_changed();
        assert!(!app.dirty);
        assert!(app.dirty_since.is_none());
    }

    #[test]
    fn dirty_since_does_not_reset_on_subsequent_mutations() {
        let mut app = App::new(Site::starter(), None, AppTheme::default());
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
        let mut app = App::new(Site::starter(), None, AppTheme::default());
        let now = std::time::Instant::now();
        app.tick_autosave(now);
        assert!(!app.dirty);
    }

    #[test]
    fn tick_autosave_does_nothing_when_dirty_but_no_path() {
        let mut app = App::new(Site::starter(), None, AppTheme::default());
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
            App::new(Site::starter(), Some(json_path.clone()), AppTheme::default());
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
        let mut app = App::new(Site::starter(), None, AppTheme::default());
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
            App::new(Site::starter(), Some(json_path.clone()), AppTheme::default());
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
        );
        assert!(app
            .toasts
            .iter()
            .all(|t| !t.message.to_lowercase().contains("differs")));
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn begin_preview_flow_with_invalid_site_opens_validation_modal() {
        let mut app = App::new(Site::starter(), None, AppTheme::default());
        app.site.pages[0].slug = "".to_string();
        app.begin_preview_flow();
        assert!(matches!(app.modal, Some(Modal::ValidationErrors { .. })));
    }

    #[test]
    fn begin_preview_flow_without_export_dir_opens_path_prompt() {
        let mut app = App::new(Site::starter(), None, AppTheme::default());
        app.begin_preview_flow();
        match &app.modal {
            Some(Modal::PreviewPathPrompt { path }) => assert_eq!(path, "./web/"),
            _ => panic!("expected PreviewPathPrompt"),
        }
    }

    #[test]
    fn current_page_slug_for_preview_returns_selected_page_slug() {
        let mut app = App::new(Site::starter(), None, AppTheme::default());
        app.site.pages.push(crate::model::Page::from_template(
            "Contact",
            crate::model::PageTemplate::Blank,
        ));
        app.selected_page = 1;
        assert_eq!(app.current_page_slug_for_preview(), "contact");
    }

    #[test]
    fn p_key_with_validation_errors_opens_validation_modal() {
        let mut app = App::new(Site::starter(), None, AppTheme::default());
        app.site.pages[0].slug = "".to_string();
        send_key(&mut app, KeyCode::Char('p'), KeyModifiers::NONE);
        assert!(matches!(app.modal, Some(Modal::ValidationErrors { .. })));
    }

    #[test]
    fn p_key_with_clean_site_and_no_export_dir_opens_preview_path_prompt() {
        let mut app = App::new(Site::starter(), None, AppTheme::default());
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
        let mut app = App::new(Site::starter(), None, AppTheme::default());
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
        let mut app = App::new(Site::starter(), None, AppTheme::default());
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
}

fn backup_path_for(path: &std::path::Path) -> std::path::PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(".backup");
    std::path::PathBuf::from(s)
}

fn chrono_like_format(t: std::time::SystemTime) -> Option<String> {
    let secs = t
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();
    Some(format!("{}s since epoch", secs))
}

/// Spawn the OS-default opener on the given file path. Returns the spawn
/// error if the command can't be invoked. The browser may take time to
/// open after this returns; we don't wait.
///
/// All three stdio streams are redirected to /dev/null. Without this, any
/// output the opener (or its forked browser) writes to stdout/stderr lands
/// on the same TTY as the TUI in raw mode and scrambles the screen layout.
#[allow(dead_code)]
fn open_in_browser(path: &std::path::Path) -> std::io::Result<()> {
    use std::process::{Command, Stdio};
    let mut cmd: Command;
    #[cfg(target_os = "linux")]
    {
        cmd = Command::new("xdg-open");
        cmd.arg(path);
    }
    #[cfg(target_os = "macos")]
    {
        cmd = Command::new("open");
        cmd.arg(path);
    }
    #[cfg(target_os = "windows")]
    {
        cmd = Command::new("cmd");
        cmd.args(["/C", "start", ""]).arg(path);
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        let _ = path;
        return Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "no known browser opener for this target",
        ));
    }
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(())
}

/// Approximate ratatui Paragraph::wrap line splitting so the help modal can
/// know its total wrapped row count up front (for scroll clamp + scrollbar
/// thumb sizing). Splits on '\n' first, then breaks long lines into
/// `width`-char chunks. Empty lines stay one row tall.
fn wrap_help_lines(text: &str, width: usize) -> usize {
    let w = width.max(1);
    let mut total = 0usize;
    for raw in text.split('\n') {
        if raw.is_empty() {
            total += 1;
            continue;
        }
        let chars = raw.chars().count();
        total += chars.div_ceil(w);
    }
    total
}

fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let target = dst.join(entry.file_name());
        if path.is_dir() {
            copy_dir_recursive(&path, &target)?;
        } else {
            std::fs::copy(&path, &target)?;
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct DirEntryRow {
    name: String,
    is_dir: bool,
}

/// List immediate children of `dir`, sorted: subdirs first (alpha), then
/// files (alpha). Hidden entries (leading dot) are skipped. Returns an
/// empty Vec when the directory is unreadable.
fn list_dir_entries(dir: &std::path::Path) -> Vec<DirEntryRow> {
    let read = match std::fs::read_dir(dir) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    let mut dirs = Vec::new();
    let mut files = Vec::new();
    for entry in read.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
        let row = DirEntryRow { name, is_dir };
        if is_dir {
            dirs.push(row);
        } else {
            files.push(row);
        }
    }
    dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    dirs.extend(files);
    dirs
}

/// Substring filter (case-insensitive). Empty filter passes all entries.
fn filter_entries(entries: &[DirEntryRow], filter: &str) -> Vec<DirEntryRow> {
    if filter.is_empty() {
        return entries.to_vec();
    }
    let needle = filter.to_lowercase();
    entries
        .iter()
        .filter(|e| e.name.to_lowercase().contains(&needle))
        .cloned()
        .collect()
}

/// Substring filter for page (slug, title) pairs. Empty filter passes all.
fn filter_pages(pages: &[(String, String)], filter: &str) -> Vec<(String, String)> {
    if filter.is_empty() {
        return pages.to_vec();
    }
    let needle = filter.to_lowercase();
    pages
        .iter()
        .filter(|(slug, title)| {
            title.to_lowercase().contains(&needle) || slug.to_lowercase().contains(&needle)
        })
        .cloned()
        .collect()
}

/// Strip a leading `./` (and any extra `/`) from a user-supplied relative
/// path so joining against a base of `.` doesn't produce `././foo` paths.
/// Trailing slashes are also trimmed for consistent display.
fn normalize_relative_path(raw: &str) -> String {
    let mut s = raw.trim();
    while let Some(rest) = s.strip_prefix("./") {
        s = rest.trim_start_matches('/');
    }
    s.trim_end_matches('/').to_string()
}

/// Build a clean display path. Prefer `./<rel>` when the export sits inside
/// the site JSON's directory; otherwise fall back to the absolute-ish form.
fn display_relative_path(
    _base: &std::path::Path,
    out: &std::path::Path,
    normalized: &str,
) -> String {
    if normalized.is_empty() {
        out.display().to_string()
    } else {
        format!("./{}/", normalized)
    }
}

impl Modal {
    #[allow(dead_code)]
    fn variant_name(&self) -> &'static str {
        match self {
            Modal::Edit { .. } => "Edit",
            Modal::ComponentPicker { .. } => "ComponentPicker",
            Modal::SavePrompt { .. } => "SavePrompt",
            Modal::SingleField { .. } => "SingleField",
            Modal::FormEdit { .. } => "FormEdit",
            Modal::TemplatePicker { .. } => "TemplatePicker",
            Modal::NewPageTitlePrompt { .. } => "NewPageTitlePrompt",
            Modal::ExportPathPrompt { .. } => "ExportPathPrompt",
            Modal::PreviewPathPrompt { .. } => "PreviewPathPrompt",
            Modal::RenamePagePrompt { .. } => "RenamePagePrompt",
            Modal::ConfirmPrompt { .. } => "ConfirmPrompt",
            Modal::ValidationErrors { .. } => "ValidationErrors",
            Modal::ImagePicker { .. } => "ImagePicker",
            Modal::PagePicker { .. } => "PagePicker",
        }
    }
}
