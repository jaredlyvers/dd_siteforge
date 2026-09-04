//! Modal types, rendering, and event handling.
use super::*;

mod paint;
mod prompts;
mod picker_paint;
mod toasts;
mod form_edit;
mod pickers;
mod events;
mod export;

// UNIFIED MODAL SYSTEM
// ============================================================================

/// All modal types in the application
pub(in crate::tui) enum Modal {
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
pub(in crate::tui) struct DrillFrame {
    pub(in crate::tui) parent_state: editform::EditFormState,
    pub(in crate::tui) parent_cursor_pos: usize,
    pub(in crate::tui) parent_scroll_offset: u16,
    pub(in crate::tui) subform_field_id: String,
    pub(in crate::tui) item_idx: usize,
}

/// Common modal result returned from event handling
pub(in crate::tui) enum ModalResult {
    /// Stay open, continue handling events
    Continue,
    /// Close modal with success
    CloseSuccess,
    /// Close modal with cancel
    CloseCancel,
}

/// The action to execute when a ConfirmPrompt is confirmed.
#[derive(Debug, Clone)]
pub(in crate::tui) enum ConfirmKind {
    DeletePage,
    QuitUnsaved,
}

/// Live state of an open image picker. `root` and `cwd` are absolute
/// paths; `cwd` is always equal to or a descendant of `root`.
#[derive(Debug, Clone)]
pub(in crate::tui) struct ImagePickerState {
    pub(in crate::tui) root: std::path::PathBuf,
    pub(in crate::tui) cwd: std::path::PathBuf,
    pub(in crate::tui) filter: String,
    pub(in crate::tui) selected: usize,
    pub(in crate::tui) binding: ImagePickBinding,
}

#[derive(Debug, Clone)]
pub(in crate::tui) enum ImagePickBinding {
    /// Write back into the FormEdit modal's currently-focused URL field.
    FormEditField { field_id: String },
}

/// Live state of an open page picker. Lists site pages by title; on Enter
/// writes `/<slug>` into the bound URL field.
#[derive(Debug, Clone)]
pub(in crate::tui) struct PagePickerState {
    /// Snapshot of (slug, title) pairs at modal-open time. The picker
    /// doesn't track site mutations while open — it operates on a frozen
    /// list and the underlying site is back-burnered while paused.
    pub(in crate::tui) pages: Vec<(String, String)>,
    pub(in crate::tui) filter: String,
    pub(in crate::tui) selected: usize,
    pub(in crate::tui) binding: PagePickBinding,
}

#[derive(Debug, Clone)]
pub(in crate::tui) enum PagePickBinding {
    /// Write back into the FormEdit modal's currently-focused URL field.
    FormEditField { field_id: String },
}

/// Visual/semantic class of a toast notification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::tui) enum ToastLevel {
    Success,
    Info,
    Warning,
    Error,
}

/// A transient bottom-right notification. Expires ~5s after `shown_at`.
#[derive(Debug, Clone)]
pub(in crate::tui) struct Toast {
    pub(in crate::tui) level: ToastLevel,
    pub(in crate::tui) message: String,
    pub(in crate::tui) shown_at: std::time::Instant,
}

/// Unified modal configuration
pub(in crate::tui) struct ModalConfig {
    pub(in crate::tui) width_percent: u16,
    pub(in crate::tui) height_percent: u16,
    pub(in crate::tui) footer_text: String,
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


impl Modal {
    #[allow(dead_code)]
    pub(in crate::tui) fn variant_name(&self) -> &'static str {
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
