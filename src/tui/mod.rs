use std::collections::HashSet;
use std::io;
use std::path::PathBuf;
use std::time::Duration;

pub(super) use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers, MouseButton,
    MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
pub(super) use ratatui::layout::{Constraint, Direction, Layout, Rect};
pub(super) use ratatui::style::{Color, Modifier, Style};
pub(super) use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Terminal;
pub(super) use crate::model::{PageNode, SectionColumn, Site};
pub(super) const AUTOSAVE_DEBOUNCE: std::time::Duration = std::time::Duration::from_secs(2);
pub(super) const TEXTAREA_MAX_DISPLAY_ROWS: u16 = 35;
pub(super) const DOUBLE_CLICK_THRESHOLD_MS: u128 = 420;

pub mod cursor;
pub mod editform;
mod theme;
mod help;
mod component_kind;
mod form_textarea;
mod scrollbar;
mod util;
mod details;
mod tree;
mod modals;
mod draw;
mod events;
#[cfg(test)]
mod tests;

use theme::*;
use help::*;
use component_kind::*;
use form_textarea::*;
use scrollbar::*;
use util::*;
use details::*;
use tree::*;
use modals::*;

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
pub(super) enum SidebarSection {
    Regions,
    Pages,
    Layouts,
    Details,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum SelectedRegion {
    Page,
    Site,
    Header,
    Footer,
}

/// F1 Help / F2 Theme. Lives beside `modal`, never inside it, so Esc
/// cannot drop ImagePicker or a paused FormEdit.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Overlay {
    Help { scroll: u16 },
    Theme { scroll: u16 },
}

pub(super) struct App {
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
    /// Draw-time hit segments for the Details pane, one row per painted line.
    /// Each segment is `(x0, x1, col, comp)` in content-cell coordinates.
    details_hits: Vec<Vec<(usize, usize, usize, usize)>>,
    details_scrollbar_track: ScrollbarTrack,
    layout_scrollbar_track: ScrollbarTrack,
    help_scrollbar_track: ScrollbarTrack,
    theme_scrollbar_track: ScrollbarTrack,
    form_scrollbar_track: std::cell::RefCell<ScrollbarTrack>,
    scrollbar_drag: Option<ScrollbarDrag>,
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
    overlay: Option<Overlay>,
    /// Maximum legal overlay scroll, recomputed every render from the
    /// current overlay area + line count so key/wheel handlers can clamp.
    overlay_scroll_max: u16,
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


impl App {
    pub(super) fn new(mut site: Site, path: Option<PathBuf>, theme: AppTheme, theme_source: String, theme_status: Option<String>) -> Self {
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
            details_hits: Vec::new(),
            details_scrollbar_track: ScrollbarTrack::default(),
            layout_scrollbar_track: ScrollbarTrack::default(),
            help_scrollbar_track: ScrollbarTrack::default(),
            theme_scrollbar_track: ScrollbarTrack::default(),
            form_scrollbar_track: std::cell::RefCell::new(ScrollbarTrack::default()),
            scrollbar_drag: None,
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
            overlay: None,
            overlay_scroll_max: 0,
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

    pub(super) fn run<B: ratatui::backend::Backend>(
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

    pub(super) fn begin_save_prompt(&mut self) {
        if let Some(path) = self.path.clone() {
            match self.commit_save_with_backup(&path) {
                Ok(()) => {
                    self.push_toast(
                        ToastLevel::Success,
                        format!("Saved {}", path.display()),
                    );
                }
                Err(e) => {
                    self.push_toast(ToastLevel::Error, format!("Failed to save: {}", e));
                }
            }
            return;
        }
        self.modal = Some(Modal::SavePrompt {
            path: "site.json".to_string(),
        });
    }

    pub(super) fn current_page(&self) -> &crate::model::Page {
        &self.site.pages[self.selected_page]
    }

    pub(super) fn current_page_mut(&mut self) -> Option<&mut crate::model::Page> {
        self.site.pages.get_mut(self.selected_page)
    }

    pub(super) fn selected_index_for_page(page: &crate::model::Page, selected_node: usize) -> Option<usize> {
        if page.nodes.is_empty() {
            None
        } else {
            Some(selected_node.min(page.nodes.len() - 1))
        }
    }

    /// Recompute the JSON snapshot of `self.site` and set `dirty` if it
    /// differs from `last_saved_json`. Idempotent: re-calling on an already
    /// dirty app does NOT advance `dirty_since`, preserving the original
    /// debounce anchor.
    pub(super) fn mark_dirty_if_changed(&mut self) {
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
    pub(super) fn commit_save_with_backup(&mut self, path: &std::path::Path) -> anyhow::Result<()> {
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
    /// Errors are surfaced as an error toast and leave `dirty` set so the
    /// next tick can retry.
    pub(super) fn tick_autosave(&mut self, now: std::time::Instant) {
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
                self.push_toast(ToastLevel::Error, msg);
            }
        }
    }
}

