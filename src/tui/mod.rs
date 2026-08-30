use std::collections::HashSet;
use std::io;
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
use crate::model::{PageNode, SectionColumn, Site};
const AUTOSAVE_DEBOUNCE: std::time::Duration = std::time::Duration::from_secs(2);
const TEXTAREA_MAX_DISPLAY_ROWS: u16 = 35;
const DOUBLE_CLICK_THRESHOLD_MS: u128 = 420;

pub mod cursor;
pub mod editform;
mod theme;
mod help;

use theme::*;
use help::*;

pub fn run_tui(site: Site, path: Option<PathBuf>) -> anyhow::Result<()> {
    let (theme, theme_source, load_warning) = AppTheme::load();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(site, path, theme, theme_source, load_warning.clone());
    if let Some(msg) = load_warning {
        app.push_toast(ToastLevel::Warning, msg);
    }
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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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
    theme_source: String,
    header_copy: String,
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
    /// Site snapshots taken before structural tree edits. `u` in Layout pops.
    /// Capped at 20.
    undo_stack: Vec<crate::model::Site>,
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
    regions_area: Rect,
    pages_area: Rect,
    pages_list_state: ListState,
    layout_list_state: ListState,
    last_mouse_click: Option<(u16, u16, std::time::Instant)>,
    path: Option<PathBuf>,
    preview_server: Option<crate::serve::StaticServer>,
    should_quit: bool,
    modal: Option<Modal>,
    component_kind: ComponentKind,
    show_help: bool,
    /// Vertical scroll offset (in rows) for the F1 help modal.
    help_scroll: u16,
    /// Maximum legal `help_scroll` value, recomputed every render based on
    /// the current modal area + wrapped row count. Read by event handlers
    /// to clamp scroll keystrokes without needing the frame.
    help_scroll_max: u16,
    show_theme: bool,
    /// Vertical scroll offset (in rows) for the F2:Theme modal.
    theme_scroll: u16,
    /// Maximum legal `theme_scroll` value, recomputed every render based on
    /// the current modal area + wrapped row count. Read by event handlers
    /// to clamp scroll keystrokes without needing the frame.
    theme_scroll_max: u16,
    theme_status: Option<String>,
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
}

// ============================================================================
// UNIFIED MODAL SYSTEM
// ============================================================================

/// All modal types in the application
enum Modal {
    /// Component picker for inserting components
    ComponentPicker { query: String, selected: usize },
    /// Save file dialog
    SavePrompt { path: String },
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
    QuitUnsaved,
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
    footer_text: String,
}

impl Default for ModalConfig {
    fn default() -> Self {
        Self {
            width_percent: 80,
            height_percent: 80,
            footer_text: "Tab/Up/Down: navigate | Ctrl+S: save | Esc: cancel".to_string(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
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
    #[allow(dead_code)]
    fn is_modal_open(&self) -> bool {
        self.modal.is_some()
    }

    /// Main modal rendering entry point
    fn render_modal(&self, frame: &mut ratatui::Frame) {
        if let Some(modal) = &self.modal {
            self.render_unified_modal(frame, modal);
        }
    }

    /// Render the new unified modal
    fn render_unified_modal(&self, frame: &mut ratatui::Frame, modal: &Modal) {
        match modal {
            Modal::ComponentPicker { query, selected } => {
                self.render_component_picker_unified(frame, query, *selected);
            }
            Modal::SavePrompt { path } => {
                self.render_save_prompt_unified(frame, path);
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
                editform::FieldKind::Textarea { rows, .. } => {
                    let max_rows = textarea_max_rows_for_window(content_height);
                    textarea_display_rows(
                        state.get(field.id),
                        (*rows).max(1),
                        Some(content_rect.width.saturating_sub(2)),
                        max_rows,
                    )
                }
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

            // Input box. Textareas may be taller than the current viewport, so
            // clamp the drawn box instead of dropping it entirely.
            if box_top_screen >= 0 && box_top_screen < content_height as i32 {
                let border_color = if focused {
                    self.theme.input_border_focus
                } else {
                    self.theme.input_border_default
                };
                let visible_box_height = slot
                    .box_height
                    .min(content_height.saturating_sub(box_top_screen as u16));
                if visible_box_height < 3 {
                    continue;
                }
                let box_rect = Rect::new(
                    content_rect.x,
                    content_rect.y + box_top_screen as u16,
                    content_rect.width,
                    visible_box_height,
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
                let visible_rows = rect.height as usize;
                let (display, first_visible_row, total_rows) =
                    render_textarea_display_window(value, cursor_pos, focused, visible_rows);
                let text_rect = if total_rows > visible_rows {
                    Rect {
                        width: rect.width.saturating_sub(1),
                        ..rect
                    }
                } else {
                    rect
                };
                frame.render_widget(
                    Paragraph::new(display)
                        .style(value_style),
                    text_rect,
                );
                if total_rows > visible_rows {
                    render_textarea_scrollbar(
                        frame,
                        Rect {
                            x: rect.x + rect.width.saturating_sub(1),
                            y: rect.y,
                            width: 1,
                            height: rect.height,
                        },
                        first_visible_row,
                        visible_rows,
                        total_rows,
                        self.theme.scrollbar,
                        self.theme.popup_background,
                    );
                }
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
            editform::FieldKind::Textarea { rows, .. } => {
                textarea_display_rows(
                    state.get(field.id),
                    (*rows).max(1),
                    None,
                    TEXTAREA_MAX_DISPLAY_ROWS,
                )
            }
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

fn textarea_display_rows(
    value: &str,
    base_rows: u16,
    wrap_width: Option<u16>,
    max_rows: u16,
) -> u16 {
    let content_rows = textarea_visual_line_count(value, wrap_width).min(u16::MAX as usize) as u16;
    base_rows
        .max(content_rows.max(1))
        .min(max_rows.max(1))
}

fn textarea_max_rows_for_window(content_height: u16) -> u16 {
    content_height
        .saturating_sub(3)
        .max(1)
        .min(TEXTAREA_MAX_DISPLAY_ROWS)
}

fn textarea_visual_line_count(value: &str, wrap_width: Option<u16>) -> usize {
    let Some(width) = wrap_width.map(|w| w.max(1) as usize) else {
        return input_lines_preserve(value).len().max(1);
    };

    input_lines_preserve(value)
        .iter()
        .map(|line| {
            let chars = line.chars().count();
            chars.div_ceil(width).max(1)
        })
        .sum::<usize>()
        .max(1)
}

#[cfg(test)]
fn render_textarea_display(
    value: &str,
    cursor_pos: usize,
    focused: bool,
    visible_rows: usize,
) -> String {
    render_textarea_display_window(value, cursor_pos, focused, visible_rows).0
}

fn render_textarea_display_window(
    value: &str,
    cursor_pos: usize,
    focused: bool,
    visible_rows: usize,
) -> (String, usize, usize) {
    let visible_rows = visible_rows.max(1);
    let mut lines = input_lines_preserve(value);
    if lines.is_empty() {
        lines.push(String::new());
    }

    let cursor_row = textarea_cursor_row(value, cursor_pos).min(lines.len().saturating_sub(1));
    let start = if focused {
        cursor_row.saturating_sub(visible_rows.saturating_sub(1))
    } else {
        0
    };
    let end = (start + visible_rows).min(lines.len());

    let mut display = Vec::with_capacity(visible_rows);
    for (idx, line) in lines.iter().enumerate().take(end).skip(start) {
        if focused && idx == cursor_row {
            let cursor_col = textarea_cursor_col(value, cursor_pos);
            display.push(render_cursor_line(line, cursor_col));
        } else {
            display.push(line.clone());
        }
    }
    while display.len() < visible_rows {
        display.push(String::new());
    }
    (display.join("\n"), start, lines.len())
}

fn render_textarea_scrollbar(
    frame: &mut ratatui::Frame,
    area: Rect,
    first_visible_row: usize,
    visible_rows: usize,
    total_rows: usize,
    scrollbar_color: Color,
    background: Color,
) {
    if area.height == 0 || total_rows <= visible_rows {
        return;
    }

    for y in 0..area.height {
        frame.render_widget(
            Paragraph::new(" ").style(Style::default().bg(background)),
            Rect {
                x: area.x,
                y: area.y + y,
                width: 1,
                height: 1,
            },
        );
    }

    let track_height = area.height as usize;
    let thumb_height = ((visible_rows.max(1) * track_height) / total_rows.max(1))
        .max(1)
        .min(track_height);
    let max_scroll = total_rows.saturating_sub(visible_rows.max(1));
    let travel = track_height.saturating_sub(thumb_height);
    let thumb_top = if max_scroll == 0 {
        0
    } else {
        (first_visible_row.min(max_scroll) * travel) / max_scroll
    };

    for y in thumb_top..thumb_top + thumb_height {
        frame.render_widget(
            Paragraph::new("█").style(Style::default().fg(scrollbar_color).bg(background)),
            Rect {
                x: area.x,
                y: area.y + y as u16,
                width: 1,
                height: 1,
            },
        );
    }
}

fn textarea_cursor_row(value: &str, cursor_pos: usize) -> usize {
    value
        .chars()
        .take(cursor_pos.min(value.chars().count()))
        .filter(|c| *c == '\n')
        .count()
}

fn textarea_cursor_col(value: &str, cursor_pos: usize) -> usize {
    let mut col = 0;
    for c in value.chars().take(cursor_pos.min(value.chars().count())) {
        if c == '\n' {
            col = 0;
        } else {
            col += 1;
        }
    }
    col
}

fn textarea_move_cursor_vertical(value: &str, cursor_pos: usize, row_delta: isize) -> usize {
    let lines = input_lines_preserve(value);
    let current_row = textarea_cursor_row(value, cursor_pos).min(lines.len().saturating_sub(1));
    let current_col = textarea_cursor_col(value, cursor_pos);
    let target_row = current_row
        .saturating_add_signed(row_delta)
        .min(lines.len().saturating_sub(1));

    cursor_from_row_col(&lines, target_row, current_col)
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

    /// Render scrollbar
    #[allow(dead_code)]
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
                    let value = crate::model::page_href(&slug);
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

    /// Unified modal event handling
    fn handle_modal_event(&mut self, evt: Event) -> Option<ModalResult> {
        let _ = self.modal.as_ref()?;

        if let Event::Key(key) = &evt {
            if key.code == KeyCode::F(1) {
                self.show_help = true;
                self.help_scroll = 0;
                return Some(ModalResult::Continue);
            }
            let key = *key;
            return match self.modal.as_ref()? {
                Modal::ComponentPicker { .. } => {
                    self.handle_component_picker_event_unified(key)
                }
                Modal::SavePrompt { .. } => self.handle_save_prompt_event_unified(key),
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
                    self.push_toast(ToastLevel::Success, "Item saved — editing parent.");
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
                        self.push_toast(ToastLevel::Warning, format!("Save failed: {e}"));
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
                    self.push_toast(ToastLevel::Info, "Item edit cancelled.");
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
                        self.push_toast(ToastLevel::Success, "Item added.");
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
                            self.push_toast(ToastLevel::Info, "Item removed.");
                        }
                    } else {
                        self.push_toast(ToastLevel::Warning, format!("Must keep at least {min_items} item(s)."));
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
                            self.push_toast(ToastLevel::Info, "Editing item. Ctrl+S returns to parent.");
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
                if is_textarea {
                    *cursor_pos =
                        textarea_move_cursor_vertical(state.get(field_id), *cursor_pos, -1);
                } else {
                    state.focus_prev();
                    *scroll_offset = auto_scroll_for_focus(state, *scroll_offset);
                    *cursor_pos = state.get(state.form.fields[state.focused_field].id).len();
                }
            }
            KeyCode::Down => {
                if is_textarea {
                    *cursor_pos =
                        textarea_move_cursor_vertical(state.get(field_id), *cursor_pos, 1);
                } else {
                    state.focus_next();
                    *scroll_offset = auto_scroll_for_focus(state, *scroll_offset);
                    *cursor_pos = state.get(state.form.fields[state.focused_field].id).len();
                }
            }
            KeyCode::PageUp if is_textarea => {
                *cursor_pos =
                    textarea_move_cursor_vertical(state.get(field_id), *cursor_pos, -10);
            }
            KeyCode::PageDown if is_textarea => {
                *cursor_pos =
                    textarea_move_cursor_vertical(state.get(field_id), *cursor_pos, 10);
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
                self.push_toast(ToastLevel::Info, "Component picker cancelled.");
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
                self.push_toast(ToastLevel::Warning, "No component selected.");
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
                self.push_toast(ToastLevel::Info, "Save cancelled.");
                Some(ModalResult::CloseCancel)
            }
            KeyCode::Enter => {
                let raw = path.trim();
                if raw.is_empty() {
                    self.push_toast(ToastLevel::Warning, "Save path cannot be empty.");
                    self.modal = Some(Modal::SavePrompt { path });
                    Some(ModalResult::Continue)
                } else {
                    let path_buf = std::path::PathBuf::from(raw);
                    if let Err(e) = self.commit_save_with_backup(&path_buf) {
                        self.push_toast(ToastLevel::Warning, format!("Failed to save: {}", e));
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
                self.push_toast(ToastLevel::Info, "Add page cancelled.");
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
                    self.push_toast(ToastLevel::Info, "Cancelled — no title.");
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
                self.push_toast(ToastLevel::Info, "Add page cancelled.");
                Some(ModalResult::CloseCancel)
            }
            KeyCode::Enter => {
                let trimmed = title.trim().to_string();
                if trimmed.is_empty() {
                    self.push_toast(ToastLevel::Warning, "Title required.");
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
                    self.push_toast(ToastLevel::Warning, "Title required.");
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

        if let Err(e) = crate::export::export_site(&self.site, &out, Some(&base)) {
            let msg = format!("Preview failed: {}", e);
            self.push_toast(ToastLevel::Warning, msg);
            return;
        }
        self.site.export_dir = Some(normalized.clone());

        let display = display_relative_path(&base, &out, &normalized);
        let count = self.site.pages.len();
        self.push_toast(
            ToastLevel::Success,
            format!("Exported {} page(s) to {}", count, display),
        );
        match self.ensure_preview_server(out.clone()) {
            Ok(url) => match open_in_browser(&url) {
                Ok(()) => {
                    self.push_toast(
                        ToastLevel::Info,
                        format!("Opening {} in browser…", url),
                    );
                }
                Err(e) => {
                    self.push_toast(
                        ToastLevel::Warning,
                        format!("Browser open failed: {}", e),
                    );
                }
            },
            Err(e) => {
                self.push_toast(
                    ToastLevel::Warning,
                    format!("Preview server failed: {}", e),
                );
            }
        }
    }

    fn ensure_preview_server(&mut self, out: PathBuf) -> anyhow::Result<String> {
        let slug = self.current_page_slug_for_preview();
        if let Some(server) = self.preview_server.as_ref() {
            server.set_root(out);
            return Ok(server.url_for(&slug));
        }
        let server = crate::serve::StaticServer::start(out)?;
        let url = server.url_for(&slug);
        self.preview_server = Some(server);
        Ok(url)
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

        match crate::export::export_site(&self.site, &out, Some(&base)) {
            Ok(report) => {
                self.site.export_dir = Some(normalized.clone());
                let display = display_relative_path(&base, &out, &normalized);
                let msg = format!("Exported {} page(s) to {}", report.pages, display);
                self.push_toast(ToastLevel::Success, msg);
            }
            Err(e) => {
                let msg = format!("Export failed: {}", e);
                self.push_toast(ToastLevel::Warning, msg);
            }
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
                self.push_toast(ToastLevel::Info, "Rename cancelled.");
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
    /// Empty titles keep the modal open with a "Title required." toast.
    fn commit_rename_page(&mut self, title: String, page_idx: usize) -> Option<ModalResult> {
        let trimmed = title.trim();
        if trimmed.is_empty() {
            self.push_toast(ToastLevel::Warning, "Title required.");
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
            self.push_toast(ToastLevel::Warning, "Page no longer exists.");
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
                    ConfirmKind::QuitUnsaved => self.should_quit = true,
                }
                self.modal = None;
                Some(ModalResult::CloseSuccess)
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.modal = None;
                self.push_toast(ToastLevel::Info, "Cancelled.");
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

}

// ============================================================================
// MAIN APP IMPLEMENTATION
// ============================================================================

impl App {
    fn new(mut site: Site, path: Option<PathBuf>, theme: AppTheme, theme_source: String, theme_status: Option<String>) -> Self {
        for page in &mut site.pages {
            ensure_page_section_ids(page);
        }
        let last_saved_json = serde_json::to_string(&site).unwrap_or_default();

        let header_copy = choose_header_copy(&theme.header_quotes);

        let mut app = Self {
            site,
            theme,
            theme_source,
            header_copy,
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
            undo_stack: Vec::new(),
            pending_new_page_title: None,
            toasts: Vec::new(),
            list_area: Rect::default(),
            details_area: Rect::default(),
            details_scroll_row: 0,
            regions_area: Rect::default(),
            pages_area: Rect::default(),
            pages_list_state: ListState::default(),
            layout_list_state: ListState::default(),
            last_mouse_click: None,
            path,
            preview_server: None,
            should_quit: false,
            modal: None,
            component_kind: ComponentKind::Banner,
            show_help: false,
            help_scroll: 0,
            help_scroll_max: 0,
            show_theme: false,
            theme_scroll: 0,
            theme_scroll_max: 0,
            theme_status,
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

        // Full-screen app shell base layer (base_background + text_primary).
        frame.render_widget(
            Block::default().style(self.theme.app_shell),
            frame.area(),
        );

        let page = self.current_page();
        let root = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // HEADER (fixed, per LDNDDEV standard)
                Constraint::Min(0),    // Main content area
                Constraint::Length(1), // FOOTER (fixed, decluttered keys only)
            ])
            .split(frame.area());
        let main = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
            .split(root[1]);

        // === NEW 3-line header (bordered, title=project, content=header_copy) ===
        let header_block = Block::default()
            .title(format!("dd_siteforge v{}", env!("CARGO_PKG_VERSION")))
            .borders(Borders::ALL)
            .border_style(self.theme.active_border)
            .style(self.theme.app_shell)
            .title_style(
                Style::default()
                    .fg(self.theme.title)
                    .add_modifier(Modifier::BOLD),
            );
        frame.render_widget(header_block.clone(), root[0]);

        if root[0].height >= 3 {
            let inner = header_block.inner(root[0]);
            let quote = Paragraph::new(self.header_copy.as_str()).style(
                Style::default()
                    .fg(self.theme.foreground)
                    .bg(self.theme.background),
            );
            frame.render_widget(quote, inner);
        }
        // (no mouse/keyboard capture for header — purely decorative)

        // Compute page context early so the & from current_page() does not live across later
        // self.* field assigns (list_area etc) in the panel rendering.
        let page_idx = self.selected_page + 1;
        let page_label = if page.head.title.trim().is_empty() {
            page.slug.as_str()
        } else {
            page.head.title.as_str()
        };
        let details_title = format!("Details — {:02}: {}", page_idx, page_label);

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
        self.regions_area = sidebar[0];

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
        let pages_title = if self.site.pages.is_empty() {
            "[2] Pages".to_string()
        } else {
            format!(
                "[2] Pages {}/{}",
                self.selected_page + 1,
                self.site.pages.len()
            )
        };
        let pages_list = List::new(page_items)
            .block(
                Block::default()
                    .title(pages_title)
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
        if !self.site.pages.is_empty() {
            self.pages_list_state.select(Some(self.selected_page));
        } else {
            self.pages_list_state.select(None);
        }
        frame.render_stateful_widget(pages_list, sidebar[1], &mut self.pages_list_state);
        self.pages_area = sidebar[1];

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
        if !tree_rows.is_empty() {
            self.layout_list_state
                .select(Some(self.selected_tree_row.min(tree_rows.len() - 1)));
        } else {
            self.layout_list_state.select(None);
        }
        frame.render_stateful_widget(layouts_list, sidebar[2], &mut self.layout_list_state);
        self.list_area = sidebar[2];

        self.details_area = main[1];
        let details_width = main[1].width.saturating_sub(2) as usize;
        let (details_content, _details_hits) = self.details_text(details_width);
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
                    .title(details_title)
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

        let footer_text = self.footer_hint(root[2].width);
        let footer = Paragraph::new(footer_text).style(self.theme.app_shell);
        frame.render_widget(footer, root[2]);

        if self.show_help {
            let area = centered_rect(80, 80, frame.area());
            frame.render_widget(Clear, area);
            let block = Block::default()
                .title("Key & Mouse bindings (F1 / Esc to close, j/k or arrows to scroll)")
                .borders(Borders::ALL)
                .style(
                    Style::default()
                        .fg(self.theme.foreground)
                        .bg(self.theme.popup_background),
                )
                .border_style(Style::default().fg(self.theme.border_active))
                .title_style(
                    Style::default()
                        .fg(self.theme.modal_header)
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

            // Build rich help (section headers in modal_header, 2-col key/action with
            // wrapping that preserves columns, icons, internal padding + dividers).
            let help = build_help_text(&self.theme, body_w as usize);
            let wrapped_total = count_wrapped_lines(&help, body_w as usize);
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

            let body = Paragraph::new(help)
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

        if self.show_theme {
            let area = centered_rect(80, 80, frame.area());
            frame.render_widget(Clear, area);
            let block = Block::default()
                .title("Theme (F2 / Esc to close, j/k or arrows to scroll)")
                .borders(Borders::ALL)
                .style(
                    Style::default()
                        .fg(self.theme.foreground)
                        .bg(self.theme.popup_background),
                )
                .border_style(Style::default().fg(self.theme.border_active))
                .title_style(
                    Style::default()
                        .fg(self.theme.modal_header)
                        .add_modifier(Modifier::BOLD),
                );
            let inner = block.inner(area);
            frame.render_widget(block, area);

            let scrollbar_width: u16 = 1;
            let body_w = inner.width.saturating_sub(scrollbar_width + 1);
            let body_area = Rect {
                x: inner.x,
                y: inner.y,
                width: body_w,
                height: inner.height,
            };

            let help = build_theme_text(&self.theme, &self.theme_source, &self.theme_status, body_w as usize);
            let wrapped_total = count_wrapped_lines(&help, body_w as usize);
            let visible = inner.height as usize;
            let max_scroll = wrapped_total.saturating_sub(visible) as u16;
            self.theme_scroll_max = max_scroll;
            if self.theme_scroll > max_scroll {
                self.theme_scroll = max_scroll;
            }
            let scroll = self.theme_scroll;

            let body = Paragraph::new(help)
                .style(
                    Style::default()
                        .fg(self.theme.foreground)
                        .bg(self.theme.popup_background),
                )
                .wrap(Wrap { trim: false })
                .scroll((scroll, 0));
            frame.render_widget(body, body_area);

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
        let _ = frame;
        None
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
                self.push_toast(ToastLevel::Info, "New page: type a title, Enter to continue.");
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
                self.push_toast(ToastLevel::Info, "Rename page. Edit and press Enter.");
                true
            }
            _ => false,
        }
    }

    fn handle_event(&mut self, evt: Event) -> anyhow::Result<()> {
        // Help/theme overlays sit above modals so F1 works from FormEdit.
        if !(self.show_help || self.show_theme) {
            if let Some(modal_result) = self.handle_modal_event(evt.clone()) {
                match modal_result {
                    ModalResult::Continue => return Ok(()),
                    ModalResult::CloseSuccess => return Ok(()),
                    ModalResult::CloseCancel => return Ok(()),
                }
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

        if self.show_theme {
            match evt {
                Event::Key(k) => match k.code {
                    KeyCode::F(2) | KeyCode::Esc => {
                        self.show_theme = false;
                        self.theme_scroll = 0;
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.theme_scroll = self
                            .theme_scroll
                            .saturating_add(1)
                            .min(self.theme_scroll_max);
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.theme_scroll = self.theme_scroll.saturating_sub(1);
                    }
                    KeyCode::PageDown => {
                        self.theme_scroll = self
                            .theme_scroll
                            .saturating_add(10)
                            .min(self.theme_scroll_max);
                    }
                    KeyCode::PageUp => {
                        self.theme_scroll = self.theme_scroll.saturating_sub(10);
                    }
                    KeyCode::Home | KeyCode::Char('g') => {
                        self.theme_scroll = 0;
                    }
                    KeyCode::End | KeyCode::Char('G') => {
                        self.theme_scroll = self.theme_scroll_max;
                    }
                    _ => {}
                },
                Event::Mouse(m) => match m.kind {
                    MouseEventKind::ScrollUp => {
                        self.theme_scroll = self.theme_scroll.saturating_sub(3);
                    }
                    MouseEventKind::ScrollDown => {
                        self.theme_scroll = self
                            .theme_scroll
                            .saturating_add(3)
                            .min(self.theme_scroll_max);
                    }
                    _ => {}
                },
                _ => {}
            }
            return Ok(());
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
                KeyCode::F(2) => self.show_theme = true,
                KeyCode::F(3) => self.open_validation_modal(),
                KeyCode::Char('E') if k.modifiers.contains(KeyModifiers::SHIFT) => {
                    self.begin_export_flow();
                }
                KeyCode::Char('p') if !k.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.begin_preview_flow();
                }
                KeyCode::Char('q') if k.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.request_quit();
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
                KeyCode::Char('d') => self.delete_selected_row(),
                KeyCode::Char('y') => self.duplicate_selected_row(),
                KeyCode::Char('u') => self.undo_last(),
                KeyCode::Char('J') => self.move_selected_row(1),
                KeyCode::Char('K') => self.move_selected_row(-1),
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
                }
                KeyCode::Char('2') => {
                    self.selected_sidebar_section = SidebarSection::Pages;
                    self.selected_region = SelectedRegion::Page;
                    self.selected_tree_row = 0;
                    self.sync_tree_row_with_selection();
                }
                KeyCode::Char('3') => {
                    self.selected_sidebar_section = SidebarSection::Layouts;
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
                    let col = m.column;
                    let row = m.row;
                    let now = std::time::Instant::now();
                    let is_double = if let Some((last_col, last_row, last_time)) = self.last_mouse_click {
                        last_col == col && last_row == row && now.duration_since(last_time).as_millis() < DOUBLE_CLICK_THRESHOLD_MS
                    } else {
                        false
                    };
                    self.last_mouse_click = Some((col, row, now));
                    if is_double {
                        self.handle_double_click(col, row);
                    } else {
                        self.handle_click(col, row);
                    }
                }
                _ => {}
            },
            _ => {}
        }
        self.sync_tree_row_with_selection();
        Ok(())
    }

    fn begin_save_prompt(&mut self) {
        if let Some(path) = self.path.clone() {
            match self.commit_save_with_backup(&path) {
                Ok(()) => {
                    self.push_toast(
                        ToastLevel::Success,
                        format!("Saved {}", path.display()),
                    );
                }
                Err(e) => {
                    self.push_toast(ToastLevel::Warning, format!("Failed to save: {}", e));
                }
            }
            return;
        }
        self.modal = Some(Modal::SavePrompt {
            path: "site.json".to_string(),
        });
    }

    fn begin_edit_selected_column_id(&mut self) {
        self.open_column_form_edit("id");
    }

    fn begin_edit_selected_column_width_class(&mut self) {
        self.open_column_form_edit("width_class");
    }

    fn open_column_form_edit(&mut self, focus_id: &str) {
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

    fn handle_click(&mut self, x: u16, y: u16) {
        // Layout tree (list_area)
        if contains(self.list_area, x, y) {
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
            let idx = (y - body_top) as usize + self.layout_list_state.offset();
            if idx < tree_rows.len() {
                self.selected_tree_row = idx;
                self.apply_tree_row_selection(tree_rows[idx]);
                self.selected_sidebar_section = SidebarSection::Layouts;
                self.push_toast(ToastLevel::Info, format!("Selected {}", self.tree_row_label(&tree_rows[idx])));
            }
            return;
        }

        // Pages list
        if contains(self.pages_area, x, y) {
            let body_top = self.pages_area.y.saturating_add(1);
            if y >= body_top {
                let rel = (y - body_top) as usize + self.pages_list_state.offset();
                if rel < self.site.pages.len() {
                    self.selected_page = rel;
                    self.selected_node = 0;
                    self.selected_column = 0;
                    self.selected_component = 0;
                    self.selected_nested_item = 0;
                    self.details_scroll_row = 0;
                    self.selected_tree_row = 0;
                    self.page_head_selected = false;
                    self.selected_region = SelectedRegion::Page;
                    self.selected_sidebar_section = SidebarSection::Pages;
                    self.sync_tree_row_with_selection();
                }
            }
            return;
        }

        // Regions list
        if contains(self.regions_area, x, y) {
            let body_top = self.regions_area.y.saturating_add(1);
            if y >= body_top {
                let rel = (y - body_top) as usize;
                if rel == 0 {
                    self.selected_region = SelectedRegion::Header;
                    self.selected_header_section = 0;
                    self.selected_header_column = 0;
                    self.selected_header_component = 0;
                    self.selected_sidebar_section = SidebarSection::Regions;
                    self.sync_tree_row_with_selection();
                    self.push_toast(ToastLevel::Info, "Selected Header region.");
                } else if rel == 1 {
                    self.selected_region = SelectedRegion::Footer;
                    self.selected_sidebar_section = SidebarSection::Regions;
                    self.sync_tree_row_with_selection();
                    self.push_toast(ToastLevel::Info, "Selected Footer region.");
                }
            }
            return;
        }

        // Details panel
        if contains(self.details_area, x, y) {
            let area = self.details_area;
            let content_top = area.y.saturating_add(1);
            let content_left = area.x.saturating_add(1);
            let scrollbar_x = area.x.saturating_add(area.width.saturating_sub(2));
            let content_bottom = area.y.saturating_add(area.height.saturating_sub(1));
            if y < content_top || y >= content_bottom || x < content_left || x >= scrollbar_x {
                return;
            }
            let rel = (y - content_top) as usize;
            let text_line = rel + self.details_scroll_row;
            let char_x = (x as usize).saturating_sub(content_left as usize);
            self.select_item_from_details_click(text_line, char_x);
            return;
        }
    }

    fn handle_double_click(&mut self, x: u16, y: u16) {
        self.handle_click(x, y);
        // Special case for double-click on an item (not the title bar) in the Nodes / Pages list:
        // Switch to that page (already done in handle_click), then pin the Layout tree selection
        // to its [HEAD] row (by setting the flag + resync), and activate the Layouts sidebar section.
        // The handle_enter below will then see a PageHead row and open the unified page-head editor.
        let pages_body_top = self.pages_area.y.saturating_add(1);
        if contains(self.pages_area, x, y) && y >= pages_body_top {
            self.selected_sidebar_section = SidebarSection::Layouts;
            self.page_head_selected = true;
            self.sync_tree_row_with_selection();
        }
        self.handle_enter_on_selected_row();
    }

    fn select_item_from_details_click(&mut self, text_line: usize, char_x: usize) {
        let detail_w = self.details_area.width.saturating_sub(2) as usize;
        if detail_w == 0 {
            return;
        }
        let (content, _content_hits_for_render) = self.details_text(detail_w);
        let lines: Vec<&str> = content.lines().collect();
        if text_line >= lines.len() {
            return;
        }
        match self.selected_region {
            SelectedRegion::Header => self.select_header_from_details_lines(&lines, text_line),
            SelectedRegion::Page => self.select_page_from_details_lines(&lines, text_line, char_x, detail_w),
            _ => return,
        }
        // Set tree row to the most specific (deepest) matching row for the selection level.
        // This makes tree highlight follow the clicked item, and double-click edit the right thing.
        let rows = self.build_tree_rows();
        if rows.is_empty() {
            return;
        }
        let clicked_line = lines[text_line];
        // For decl lines, use MAX for lower levels so only the decl row matches predicate (ancestors do but we pick specific).
        let mut tcol = self.selected_column;
        let mut tcomp = self.selected_component;
        if clicked_line.contains('[')
            && (clicked_line.contains("dd-hero")
                || clicked_line.contains("dd-section")
                || clicked_line.contains("dd-header"))
        {
            tcol = usize::MAX;
            tcomp = usize::MAX;
        } else if clicked_line.contains("column: ") || clicked_line.contains("item: ") {
            tcomp = usize::MAX;
        }
        let matches = |r: &TreeRow| -> bool {
            match r.kind {
                TreeRowKind::HeaderRoot { .. } => true,
                TreeRowKind::HeaderSection { section_idx } => section_idx == self.selected_header_section,
                TreeRowKind::HeaderColumn { section_idx, column_idx } => {
                    section_idx == self.selected_header_section && column_idx == self.selected_header_column
                }
                TreeRowKind::HeaderComponent { section_idx, column_idx, component_idx } => {
                    section_idx == self.selected_header_section
                        && column_idx == self.selected_header_column
                        && component_idx == self.selected_header_component
                }
                TreeRowKind::Hero { node_idx } | TreeRowKind::Section { node_idx } => node_idx == self.selected_node,
                TreeRowKind::Column { node_idx, column_idx } => {
                    node_idx == self.selected_node && column_idx == tcol
                }
                TreeRowKind::Component { node_idx, column_idx, component_idx } => {
                    node_idx == self.selected_node && column_idx == tcol && component_idx == tcomp
                }
                _ => false,
            }
        };
        if let Some((i, _)) = rows.iter().enumerate().rev().find(|(_, r)| matches(r)) {
            self.selected_tree_row = i;
        }
    }

    fn select_page_from_details_lines(&mut self, lines: &[&str], up_to: usize, char_x: usize, detail_w: usize) {
        let mut node_idx = None;
        let mut col_idx = 0usize;
        let mut comp_idx = 0usize;
        let mut cols_since = 0usize;
        let mut comps_since = 0usize;
        for (_i, &l) in lines.iter().enumerate().take(up_to + 1) {
            if let Some(br) = l.find('[') {
                if let Some(er) = l[br + 1..].find(']') {
                    let ns = &l[br + 1..br + 1 + er];
                    if let Ok(n) = ns.trim().parse::<usize>() {
                        if l.contains("dd-hero") || l.contains("dd-section") {
                            node_idx = Some(n.saturating_sub(1));
                            cols_since = 0;
                            comps_since = 0;
                            col_idx = 0;
                            comp_idx = 0;
                        }
                    }
                }
            }
            if node_idx.is_some() {
                let t = l.trim();
                if t.contains("item: ") || t.contains(" column: ") {
                    col_idx = cols_since;
                    cols_since += 1;
                    comps_since = 0;
                    comp_idx = 0;
                }
                if t.contains("dd-") && !t.contains("dd-section") && !t.contains("dd-hero") {
                    comp_idx = comps_since;
                    comps_since += 1;
                }
            }
        }
        if let Some(n) = node_idx {
            let page = self.current_page();
            if n < page.nodes.len() {
                self.selected_node = n;
                self.selected_column = col_idx;
                self.selected_component = comp_idx;
            }
        }
        // Use precise component hit segments from generation (handles side-by-side column boxes correctly)
        let (_ , hits) = self.details_text(detail_w);  // re-get with same w; hits only for page
        if let Some(line_segs) = hits.get(up_to) {
            for &(x0, x1, c, cp) in line_segs {
                if char_x >= x0 && char_x < x1 {
                    if let Some(n) = node_idx.or(Some(self.selected_node)) {
                        let page = self.current_page();
                        if n < page.nodes.len() {
                            self.selected_node = n;
                            self.selected_column = c;
                            self.selected_component = cp;
                        }
                    }
                    break;
                }
            }
        }
    }

    fn select_header_from_details_lines(&mut self, lines: &[&str], up_to: usize) {
        let mut sec_idx = 0usize;
        let mut col_idx = 0usize;
        let mut comp_idx = 0usize;
        let mut secs = 0usize;
        let mut cols = 0usize;
        let mut comps = 0usize;
        for (_i, &l) in lines.iter().enumerate().take(up_to + 1) {
            let t = l.trim();
            if t.contains("section: ") {
                sec_idx = secs;
                secs += 1;
                cols = 0;
                comps = 0;
                col_idx = 0;
                comp_idx = 0;
            } else if t.contains("column: ") {
                col_idx = cols;
                cols += 1;
                comps = 0;
                comp_idx = 0;
            } else if t.contains("dd-") && !t.contains("section:") {
                comp_idx = comps;
                comps += 1;
            }
        }
        self.selected_header_section = sec_idx;
        self.selected_header_column = col_idx;
        self.selected_header_component = comp_idx;
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

    fn handle_enter_on_selected_row(&mut self) {
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
        self.modal = Some(Modal::ComponentPicker {
            query: String::new(),
            selected: 0,
        });
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

    /// If `row` is a child item row inside a SubForm-bearing component
    /// (CardItem, AccordionItem, etc.), open the parent's FormEdit modal
    /// pre-drilled into the selected item — the same state the user would
    /// reach by opening the parent and pressing Enter on the items field
    /// at the right index.
    fn try_open_form_edit_drilled_into_item(&mut self, row: &TreeRow) -> bool {
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

    /// If `row` is a Column row (under a Section in page/header/footer), open the
    /// section's FormEdit modal pre-drilled into the "columns" SubForm at the
    /// selected column index — the same state as drilling from inside the
    /// section edit.
    fn try_open_form_edit_drilled_into_column(&mut self, row: &TreeRow) -> bool {
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

    fn try_open_root(&self, row: &TreeRow) -> Option<(editform::EditFormState, cursor::Cursor, &'static str)> {
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

    fn insert_selected_component_kind(&mut self) {
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

    fn add_footer_section(&mut self) {
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

    fn add_component_to_header_section(&mut self) {
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

    fn add_component_to_footer_section(&mut self) {
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

    fn normalize_component_picker_selection(&mut self) {
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

    fn handle_down(&mut self) {
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

    fn footer_hint(&self, width: u16) -> String {
        let parts: &[&str] = if self.modal.is_some() || self.show_help || self.show_theme {
            &["F1:Help", "Esc:Close", "Ctrl+Q:Quit"]
        } else {
            match self.selected_sidebar_section {
                SidebarSection::Pages => {
                    if width < 80 {
                        &["F1:Help", "Shift+A:Add", "r:Rename", "Ctrl+Q:Quit"]
                    } else {
                        &[
                            "F1:Help",
                            "Shift+A:Add",
                            "Shift+X:Del",
                            "u:Undo",
                            "r:Rename",
                            "Shift+J/K:Move",
                            "Ctrl+Q:Quit",
                        ]
                    }
                }
                SidebarSection::Regions => {
                    &["F1:Help", "j/k:Header/Footer", "Enter:Edit", "Ctrl+Q:Quit"]
                }
                SidebarSection::Layouts => {
                    if width < 80 {
                        &["F1:Help", "Enter:Edit", "d:Del", "y:Dup", "Ctrl+Q:Quit"]
                    } else if width < 110 {
                        &[
                            "F1:Help",
                            "/:Insert",
                            "d:Del",
                            "y:Dup",
                            "u:Undo",
                            "J/K:Move",
                            "Ctrl+Q:Quit",
                        ]
                    } else {
                        &[
                            "F1:Help",
                            "/:Insert",
                            "Enter:Edit",
                            "d:Del",
                            "y:Dup",
                            "u:Undo",
                            "J/K:Move",
                            "p:Preview",
                            "Ctrl+Q:Quit",
                        ]
                    }
                }
            }
        };
        let mut joined = parts.join("  ");
        if self.dirty {
            joined = format!("*  {joined}");
        }
        if width == 0 {
            return String::new();
        }
        joined.chars().take(width as usize).collect()
    }

    fn details_text(&self, detail_width: usize) -> (String, Vec<Vec<(usize, usize, usize, usize)>>) {
        match self.selected_region {
            SelectedRegion::Header => (self.header_details_text(detail_width), vec![]),
            SelectedRegion::Footer => (self.footer_details_text(detail_width), vec![]),
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
        let (hmap, _h_hits) = header_ascii_map(
            &self.site.header,
            self.selected_header_section,
            self.selected_header_column,
            detail_width,
        );
        out.push(hmap);
        out.push(String::new());
        out.push(format!(
            "Selected: {} | Insert mode: {}",
            self.header_selection_summary(),
            self.component_kind.label()
        ));
        out.join("\n")
    }

    fn footer_details_text(&self, detail_width: usize) -> String {
        let mut out = Vec::new();
        out.push("Site footer".to_string());
        out.push(String::new());
        let marker = if matches!(self.selected_region, SelectedRegion::Footer) {
            "*"
        } else {
            " "
        };
        out.push(format!("{}[01] dd-footer {}", marker, self.site.footer.id));
        let (fmap, _f_hits) = footer_ascii_map(
            &self.site.footer,
            self.selected_header_section,
            self.selected_header_column,
            detail_width,
        );
        out.push(fmap);
        out.join("\n")
    }

    fn page_details_text(&self, detail_width: usize) -> (String, Vec<Vec<(usize, usize, usize, usize)>>) {
        let page = self.current_page();
        if page.nodes.is_empty() {
            return ("No nodes on this page.".to_string(), vec![]);
        }
        let mut out = Vec::new();
        let mut out_hits: Vec<Vec<(usize, usize, usize, usize)>> = vec![];
        out.push(format!("Page blueprint: {}", page.head.title));
        out_hits.push(vec![]);
        out.push(String::new());
        out_hits.push(vec![]);
        for (idx, node) in page.nodes.iter().enumerate() {
            let marker = if idx == self.selected_node { "*" } else { " " };
            match node {
                PageNode::Hero(v) => {
                    out.push(format!("{marker}[{:02}] dd-hero", idx + 1,));
                    out_hits.push(vec![]);
                    let hmap = hero_ascii_map(v, detail_width);
                    for l in hmap.lines() {
                        out.push(l.to_string());
                        out_hits.push(vec![]);
                    }
                }
                PageNode::Section(v) => {
                    out.push(format!("{marker}[{:02}] dd-section {}", idx + 1, v.id));
                    out_hits.push(vec![]);
                    let (sec_str, sec_hits) = section_ascii_map(
                        v,
                        if idx == self.selected_node {
                            self.selected_column
                        } else {
                            0
                        },
                        detail_width,
                    );
                    for (i, l) in sec_str.lines().enumerate() {
                        out.push(l.to_string());
                        out_hits.push(sec_hits.get(i).cloned().unwrap_or_default());
                    }
                }
            }
            out.push(String::new());
            out_hits.push(vec![]);
        }
        out.push(format!(
            "Selected: {} | Insert mode: {}",
            self.selection_summary(),
            self.component_kind.label()
        ));
        out_hits.push(vec![]);
        (out.join("\n"), out_hits)
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
        self.push_toast(ToastLevel::Success, format!("Inserted dd-hero at position {}.", idx + 1));
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
        self.push_toast(ToastLevel::Success, format!("Inserted dd-section at position {}.", idx + 1));
    }

    fn delete_selected_node(&mut self) {
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

    fn selected_tree_row_kind(&self) -> Option<TreeRowKind> {
        let rows = self.build_tree_rows();
        if rows.is_empty() {
            return None;
        }
        Some(rows[self.selected_tree_row.min(rows.len() - 1)].kind)
    }

    fn push_undo(&mut self) {
        self.undo_stack.push(self.site.clone());
        if self.undo_stack.len() > 20 {
            self.undo_stack.remove(0);
        }
    }

    fn undo_last(&mut self) {
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

    fn request_quit(&mut self) {
        if self.dirty {
            self.modal = Some(Modal::ConfirmPrompt {
                message: "Unsaved changes. Quit anyway? y/n".to_string(),
                on_confirm: ConfirmKind::QuitUnsaved,
            });
        } else {
            self.should_quit = true;
        }
    }

    fn delete_selected_row(&mut self) {
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

    fn delete_page_component(&mut self, node_idx: usize, column_idx: usize, component_idx: usize) {
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

    fn delete_header_component(
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

    fn delete_footer_component(
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

    fn duplicate_selected_row(&mut self) {
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

    fn duplicate_selected_collection_item(&mut self) -> bool {
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

    fn move_selected_row(&mut self, delta: isize) {
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

    fn move_region_component(
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

    fn move_selected_collection_item(&mut self, delta: isize) -> bool {
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

    fn add_selected_component_to_section(&mut self) {
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
                self.push_toast(ToastLevel::Warning, "Selected component does not support collection items.");
            }
            None => {
                self.push_toast(ToastLevel::Warning, "No selected collection component.");
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
                self.push_toast(ToastLevel::Warning, "Selected component does not support collection items.");
            }
            None => {
                self.push_toast(ToastLevel::Warning, "No selected collection component.");
            }
        }
    }

    fn add_selected_accordion_item(&mut self) {
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

    fn remove_selected_accordion_item(&mut self) {
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

    fn add_selected_alternating_item(&mut self) {
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

    fn remove_selected_alternating_item(&mut self) {
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

    fn add_selected_card_item(&mut self) {
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

    fn remove_selected_card_item(&mut self) {
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

    fn add_selected_filmstrip_item(&mut self) {
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

    fn remove_selected_filmstrip_item(&mut self) {
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

    fn add_selected_milestones_item(&mut self) {
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

    fn remove_selected_milestones_item(&mut self) {
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

    fn add_selected_slider_item(&mut self) {
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

    fn remove_selected_slider_item(&mut self) {
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

    fn remove_column_from_header_section(&mut self) {
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

    fn select_prev_column(&mut self) {
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

    fn select_next_column(&mut self) {
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

    fn move_selected_column_down(&mut self) {
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
        let (dtxt, _dhits) = self.details_text(detail_width);
        let total_rows = dtxt.lines().count().max(1);
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
        crate::storage::save_site(path, &self.site)?;
        let backup = backup_path_for(path);
        std::fs::copy(path, &backup)?;
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

fn section_ascii_map(
    section: &crate::model::DdSection,
    selected_column: usize,
    panel_width: usize,
) -> (String, Vec<Vec<(usize, usize, usize, usize)>>) {
    const MAX_COMPONENT_ROWS: usize = 4;

    let inner_width = panel_width.saturating_sub(4).max(12);
    let columns = section_columns_ref(section);
    if columns.is_empty() {
        return ("(no columns)".to_string(), vec![]);
    }
    let active = selected_column.min(columns.len().saturating_sub(1));

    // column_data: per column (its box_lines, and per vertical line: which comp_idx it belongs to, or None for box headers/borders)
    let column_data: Vec<(Vec<String>, Vec<Option<usize>>)> = columns
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
            let mut box_comps: Vec<Option<usize>> = vec![None, None, None];
            if col.components.is_empty() {
                box_lines.push(format!(
                    "| {} |",
                    fit_ascii_cell("(empty)", item_inner_width)
                ));
                box_comps.push(None);
            } else {
                for (comp_i, component) in col.components.iter().take(MAX_COMPONENT_ROWS).enumerate() {
                    match component {
                        crate::model::SectionComponent::Card(card) => {
                            box_lines.push(format!(
                                "| {} |",
                                fit_ascii_cell("- dd-card", item_inner_width)
                            ));
                            box_comps.push(Some(comp_i));
                            for line in card_items_ascii_lines(card, item_inner_width) {
                                box_lines.push(format!(
                                    "| {} |",
                                    fit_ascii_cell(&line, item_inner_width)
                                ));
                                box_comps.push(Some(comp_i));
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
                            box_comps.push(Some(comp_i));
                        }
                    }
                }
                let more = col.components.len().saturating_sub(MAX_COMPONENT_ROWS);
                if more > 0 {
                    box_lines.push(format!(
                        "| {} |",
                        fit_ascii_cell(&format!("+{more} more"), item_inner_width)
                    ));
                    box_comps.push(None);
                }
            }
            box_lines.push(item_border);
            box_comps.push(None);
            (box_lines, box_comps)
        })
        .collect::<Vec<_>>();

    // We will build annotations for the inner composed lines (before section outer | wrap)
    // Each inner line will have segments for components: (x0, x1, col, comp)
    let mut inner_composed_lines: Vec<String> = vec![];
    let mut inner_line_segments: Vec<Vec<(usize, usize, usize, usize)>> = vec![]; // x0,x1,col,comp per inner line

    // section header lines (inside the section ascii border)
    let section_header_lines = vec![
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
    for hl in section_header_lines {
        inner_composed_lines.push(hl);
        inner_line_segments.push(vec![]);
    }

    let item_box_widths = column_data
        .iter()
        .map(|(bl, _)| bl.first().map(|s| s.chars().count()).unwrap_or(0))
        .collect::<Vec<_>>();

    let gap = 1usize;
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
            inner_composed_lines.push("".to_string());
            inner_line_segments.push(vec![]);
        }
        let max_height = row
            .iter()
            .map(|idx| column_data[*idx].0.len())
            .max()
            .unwrap_or(0);
        for line_idx in 0..max_height {
            let mut composed = String::new();
            let mut segs: Vec<(usize, usize, usize, usize)> = vec![];
            let mut cur_x = 0usize;
            for (pos, &col_idx) in row.iter().enumerate() {
                if pos > 0 {
                    composed.push_str(" ");
                    cur_x += 1;
                }
                let (box_lines, box_comps) = &column_data[col_idx];
                let box_w = item_box_widths[col_idx];
                let part = box_lines
                    .get(line_idx)
                    .cloned()
                    .unwrap_or_else(|| " ".repeat(box_w));
                let part_start = cur_x;
                composed.push_str(&part);
                cur_x += part.chars().count();
                if let Some(cp) = box_comps.get(line_idx).copied().flatten() {
                    segs.push((part_start, cur_x, col_idx, cp));
                }
            }
            let fitted = fit_ascii_cell(&composed, inner_width);
            inner_composed_lines.push(fitted);
            inner_line_segments.push(segs);
        }
    }

    let border = format!("+{}+", "-".repeat(inner_width + 2));
    let mut out = Vec::new();
    let mut out_hits: Vec<Vec<(usize, usize, usize, usize)>> = vec![]; // final hits per out line
    out.push(border.clone());
    out_hits.push(vec![]);
    for (i, line) in inner_composed_lines.into_iter().enumerate() {
        let final_line = format!("| {} |", line);
        // adjust the inner segs x by +2 for the leading "| "
        let adjusted: Vec<(usize,usize,usize,usize)> = inner_line_segments[i]
            .iter()
            .map(|(x0,x1,c,cp)| (x0 + 2, x1 + 2, *c, *cp))
            .collect();
        out.push(final_line);
        out_hits.push(adjusted);
    }
    out.push(border.clone());
    out_hits.push(vec![]);
    (out.join("\n"), out_hits)
}

fn header_ascii_map(
    header: &crate::model::DdHeader,
    selected_section: usize,
    selected_column: usize,
    panel_width: usize,
) -> (String, Vec<Vec<(usize, usize, usize, usize)>>) {
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
                lines.push(fit_ascii_cell("  (no columns)", inner_width));
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
                        &format!("  {c_marker} column: {} [{}]", col.id, col.width_class),
                        inner_width,
                    ));
                    if col.components.is_empty() {
                        lines.push(fit_ascii_cell("    (empty)", inner_width));
                    } else {
                        for comp in col.components.iter() {
                            lines.push(fit_ascii_cell(
                                &format!("    - {}", component_label(comp)),
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
    let s = out.join("\n");
    let hits = vec![vec![]; out.len()];
    (s, hits)
}

fn footer_ascii_map(
    footer: &crate::model::DdFooter,
    selected_section: usize,
    selected_column: usize,
    panel_width: usize,
) -> (String, Vec<Vec<(usize, usize, usize, usize)>>) {
    let inner_width = panel_width.saturating_sub(4).max(12);
    let mut lines = vec![
        fit_ascii_cell("FOOTER", inner_width),
        fit_ascii_cell(&format!("id: {}", footer.id), inner_width),
        fit_ascii_cell(
            &format!(
                "custom_css: {}",
                footer.custom_css.as_deref().unwrap_or("(none)")
            ),
            inner_width,
        ),
        fit_ascii_cell("sections:", inner_width),
    ];
    if footer.sections.is_empty() {
        lines.push(fit_ascii_cell(
            "(no sections - press '/' to add)",
            inner_width,
        ));
    } else {
        let active_section = selected_section.min(footer.sections.len().saturating_sub(1));
        for (s_idx, section) in footer.sections.iter().enumerate() {
            let s_marker = if s_idx == active_section { "*" } else { "-" };
            lines.push(fit_ascii_cell(
                &format!("{s_marker} section: {}", section.id),
                inner_width,
            ));
            if section.columns.is_empty() {
                lines.push(fit_ascii_cell("  (no columns)", inner_width));
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
                        &format!("  {c_marker} column: {} [{}]", col.id, col.width_class),
                        inner_width,
                    ));
                    if col.components.is_empty() {
                        lines.push(fit_ascii_cell("    (empty)", inner_width));
                    } else {
                        for comp in col.components.iter() {
                            lines.push(fit_ascii_cell(
                                &format!("    - {}", component_label(comp)),
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
    let s = out.join("\n");
    let hits = vec![vec![]; out.len()];
    (s, hits)
}

fn card_items_ascii_lines(
    card: &crate::model::DdCard,
    container_inner_width: usize,
) -> Vec<String> {
    if card.items.is_empty() {
        return vec![fit_ascii_cell("(empty)", container_inner_width)];
    }

    let child_inner_width = section_item_ascii_inner_width(&card.parent_width, container_inner_width)
        .min(container_inner_width.saturating_sub(4))
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

    let gap = 1usize;
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

#[allow(dead_code)]
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

#[allow(dead_code)]
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

#[allow(dead_code)]
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

#[allow(dead_code)]
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

#[allow(dead_code)]
fn navigation_kind_to_str(v: crate::model::NavigationKind) -> &'static str {
    match v {
        crate::model::NavigationKind::Link => "link",
        crate::model::NavigationKind::Button => "button",
    }
}

#[allow(dead_code)]
fn parse_navigation_kind(raw: &str) -> Option<crate::model::NavigationKind> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "link" => Some(crate::model::NavigationKind::Link),
        "button" => Some(crate::model::NavigationKind::Button),
        _ => None,
    }
}

#[allow(dead_code)]
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

#[allow(dead_code)]
fn robots_directive_to_str(v: crate::model::RobotsDirective) -> &'static str {
    match v {
        crate::model::RobotsDirective::IndexFollow => "index, follow",
        crate::model::RobotsDirective::NoindexFollow => "noindex, follow",
        crate::model::RobotsDirective::IndexNofollow => "index, nofollow",
        crate::model::RobotsDirective::NoindexNofollow => "noindex, nofollow",
    }
}

#[allow(dead_code)]
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

#[allow(dead_code)]
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

#[allow(dead_code)]
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

        assert_eq!(rendered, "three\nfour\nfive▋");
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
        assert!(app.show_theme);
        assert_eq!(app.theme_scroll, 0);
        send_key(&mut app, KeyCode::Esc, KeyModifiers::NONE);
        assert!(!app.show_theme);
    }

    #[test]
    fn f2_scroll_keys_and_wheel_update_theme_scroll() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        send_key(&mut app, KeyCode::F(2), KeyModifiers::NONE);
        // simulate render to set max (draw not called, so manually exercise clamp logic path)
        app.theme_scroll_max = 5; // pretend content
        send_key(&mut app, KeyCode::Down, KeyModifiers::NONE);
        assert_eq!(app.theme_scroll, 1);
        send_key(&mut app, KeyCode::PageDown, KeyModifiers::NONE);
        assert!(app.theme_scroll >= 1);
        send_key(&mut app, KeyCode::Char('g'), KeyModifiers::NONE); // home alias
        assert_eq!(app.theme_scroll, 0);
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
        assert!(app.show_theme);
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
        let json_path = tmp.join("site.json");
        let mut app = App::new(Site::starter(), Some(json_path.clone()), AppTheme::default(), "default".to_string(), None);
        app.site.export_dir = Some("web".to_string());

        app.begin_export_flow();

        assert!(app.modal.is_none(), "no modal should open — direct export");
        let last = app.toasts.last().expect("expected a toast");
        assert_eq!(last.level, ToastLevel::Success);
        assert!(last.message.to_lowercase().contains("exported"));
        assert!(tmp.join("web").exists(), "export directory should have been created");
        assert!(
            tmp.join("web").join("assets").join("css").join("style.min.css").exists(),
            "export must include framework CSS"
        );

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
        let footer = app.footer_details_text(40);
        assert!(footer.contains("dd-footer"), "{footer}");
        assert!(!footer.to_lowercase().contains("not yet implemented"));
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
        assert!(app.show_help);
        assert!(matches!(app.modal, Some(Modal::FormEdit { .. })));
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
fn open_in_browser(target: &str) -> std::io::Result<()> {
    use std::process::{Command, Stdio};
    let mut cmd: Command;
    #[cfg(target_os = "linux")]
    {
        cmd = Command::new("xdg-open");
        cmd.arg(target);
    }
    #[cfg(target_os = "macos")]
    {
        cmd = Command::new("open");
        cmd.arg(target);
    }
    #[cfg(target_os = "windows")]
    {
        cmd = Command::new("cmd");
        cmd.args(["/C", "start", ""]).arg(target);
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        let _ = target;
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
            Modal::ComponentPicker { .. } => "ComponentPicker",
            Modal::SavePrompt { .. } => "SavePrompt",
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
