//! EditForm data model — the backbone of the unified component editor.
//!
//! Each editable component is described by a single `EditForm` literal
//! (a pure data value). One render function draws any form; one dispatch
//! applies field values back to the model. Adding a new component is a
//! data-only change — write an `EditForm`, wire its variant in the save
//! dispatch, and it gains the same editor UX as every other component.
//!
//! Future phases remain additive:
//!   - codegen from `components/dd-*.md` YAML → same `EditForm` values
//!   - runtime-loaded specs → `EditForm` parsed at app startup
//!   - user-authored components → `SectionComponent::Dynamic` variant
//! No further TUI logic is required for those phases; only data plumbing.

use std::collections::HashMap;

/// Static description of an editable component's fields. One instance per
/// component type, stored as a `static` item and referenced by the editor.
#[derive(Debug)]
pub struct EditForm {
    pub title: &'static str,
    pub fields: &'static [FormField],
}

/// One editable field inside an `EditForm`.
#[derive(Debug)]
pub struct FormField {
    pub id: &'static str,
    pub label: &'static str,
    pub kind: FieldKind,
    pub required: bool,
    pub visible_when: Option<FieldPredicate>,
}

/// Shape of a field. The editor renders differently for each variant and
/// the save dispatch decodes values back into typed model fields.
#[derive(Debug)]
pub enum FieldKind {
    /// Single-line text input.
    Text { default: &'static str },
    /// Multi-line text area. `rows` is the rendered height in rows.
    Textarea { rows: u16, default: &'static str },
    /// Single-line text input validated as URL at save time.
    Url { default: &'static str },
    /// Cyclable enum. `options` carries the serde-wire strings (e.g. `-top-left`).
    Enum {
        options: &'static [&'static str],
        default: &'static str,
    },
    /// Three-in-one optional link: url + target + label, rendered as one
    /// logical field with three child inputs. Submits the triple together
    /// only when all three non-empty.
    OptionalLinkTriple {
        url_id: &'static str,
        target_id: &'static str,
        label_id: &'static str,
    },
    // SubForm { template: &'static EditForm, min_items: usize } — added in Tier B.
}

/// Predicate that gates whether a field is visible. The editor skips hidden
/// fields during Tab traversal and the renderer draws them dimmed or not at
/// all.
#[derive(Debug)]
pub enum FieldPredicate {
    FieldEquals {
        other_id: &'static str,
        value: &'static str,
    },
}

/// Live editor state for one form. Held inside `Modal::FormEdit`.
#[derive(Debug, Clone)]
pub struct EditFormState {
    pub form: &'static EditForm,
    pub values: HashMap<String, String>,
    pub focused_field: usize,
    /// (row, col) cursor inside a `Textarea` field; only meaningful when
    /// `focused_field` points at a Textarea.
    pub textarea_cursor: (usize, usize),
}

impl EditFormState {
    /// Build a fresh state with every field initialised to its declared default.
    pub fn new(form: &'static EditForm) -> Self {
        let mut values = HashMap::new();
        for field in form.fields {
            match &field.kind {
                FieldKind::Text { default } | FieldKind::Url { default } => {
                    values.insert(field.id.to_string(), default.to_string());
                }
                FieldKind::Textarea { default, .. } => {
                    values.insert(field.id.to_string(), default.to_string());
                }
                FieldKind::Enum { default, .. } => {
                    values.insert(field.id.to_string(), default.to_string());
                }
                FieldKind::OptionalLinkTriple {
                    url_id,
                    target_id,
                    label_id,
                } => {
                    values.insert(url_id.to_string(), String::new());
                    values.insert(target_id.to_string(), "_self".to_string());
                    values.insert(label_id.to_string(), String::new());
                }
            }
        }
        Self {
            form,
            values,
            focused_field: 0,
            textarea_cursor: (0, 0),
        }
    }

    pub fn get(&self, id: &str) -> &str {
        self.values.get(id).map(String::as_str).unwrap_or("")
    }

    pub fn set(&mut self, id: &str, value: impl Into<String>) {
        self.values.insert(id.to_string(), value.into());
    }

    pub fn field_visible(&self, field: &FormField) -> bool {
        match &field.visible_when {
            None => true,
            Some(FieldPredicate::FieldEquals { other_id, value }) => {
                self.get(other_id) == *value
            }
        }
    }

    /// Indices of visible fields in tab order.
    pub fn visible_field_indices(&self) -> Vec<usize> {
        self.form
            .fields
            .iter()
            .enumerate()
            .filter_map(|(idx, field)| self.field_visible(field).then_some(idx))
            .collect()
    }

    /// Advance `focused_field` to the next visible field, wrapping.
    pub fn focus_next(&mut self) {
        let visible = self.visible_field_indices();
        if visible.is_empty() {
            return;
        }
        let current_pos = visible
            .iter()
            .position(|&i| i == self.focused_field)
            .unwrap_or(0);
        let next_pos = (current_pos + 1) % visible.len();
        self.focused_field = visible[next_pos];
        self.textarea_cursor = (0, 0);
    }

    /// Retreat `focused_field` to the previous visible field, wrapping.
    pub fn focus_prev(&mut self) {
        let visible = self.visible_field_indices();
        if visible.is_empty() {
            return;
        }
        let current_pos = visible
            .iter()
            .position(|&i| i == self.focused_field)
            .unwrap_or(0);
        let prev_pos = if current_pos == 0 {
            visible.len() - 1
        } else {
            current_pos - 1
        };
        self.focused_field = visible[prev_pos];
        self.textarea_cursor = (0, 0);
    }

    pub fn focused(&self) -> Option<&FormField> {
        self.form.fields.get(self.focused_field)
    }

    /// Cycle the focused enum field forward (`forward = true`) or backward.
    /// No-op when the focused field is not an enum.
    pub fn cycle_enum(&mut self, forward: bool) {
        let Some(field) = self.focused() else { return };
        let FieldKind::Enum { options, .. } = &field.kind else {
            return;
        };
        if options.is_empty() {
            return;
        }
        let current = self.get(field.id).to_string();
        let idx = options
            .iter()
            .position(|opt| *opt == current.as_str())
            .unwrap_or(0);
        let next = if forward {
            (idx + 1) % options.len()
        } else if idx == 0 {
            options.len() - 1
        } else {
            idx - 1
        };
        let new_value = options[next].to_string();
        let field_id = field.id;
        self.set(field_id, new_value);
    }
}

// Shared option lists reused by several forms.
const AOS_OPTIONS: &[&str] = &[
    "fade-in",
    "fade-up",
    "fade-right",
    "fade-down",
    "fade-left",
    "zoom-in",
    "zoom-in-up",
    "zoom-in-down",
];

const LINK_TARGET_OPTIONS: &[&str] = &["_self", "_blank"];

// ==================== CTA form (wedge) ====================

pub static CTA_FORM: EditForm = EditForm {
    title: "dd-cta",
    fields: &[
        FormField {
            id: "parent_class",
            label: "Position",
            kind: FieldKind::Enum {
                options: &[
                    "-top-left",
                    "-top-center",
                    "-top-right",
                    "-center-left",
                    "-center-center",
                    "-center-right",
                    "-bottom-left",
                    "-bottom-center",
                    "-bottom-right",
                ],
                default: "-top-left",
            },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "parent_image_url",
            label: "Image URL",
            kind: FieldKind::Url { default: "" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "parent_image_alt",
            label: "Image Alt",
            kind: FieldKind::Text { default: "" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "parent_data_aos",
            label: "Animation",
            kind: FieldKind::Enum {
                options: &[
                    "fade-in",
                    "fade-up",
                    "fade-right",
                    "fade-down",
                    "fade-left",
                    "zoom-in",
                    "zoom-in-up",
                    "zoom-in-down",
                ],
                default: "fade-in",
            },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "parent_title",
            label: "Title",
            kind: FieldKind::Text { default: "" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "parent_subtitle",
            label: "Subtitle",
            kind: FieldKind::Text { default: "" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "parent_copy",
            label: "Copy",
            kind: FieldKind::Textarea {
                rows: 5,
                default: "",
            },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "parent_link_url",
            label: "Link URL (optional)",
            kind: FieldKind::Url { default: "" },
            required: false,
            visible_when: None,
        },
        FormField {
            id: "parent_link_target",
            label: "Link Target",
            kind: FieldKind::Enum {
                options: &["_self", "_blank"],
                default: "_self",
            },
            required: false,
            visible_when: None,
        },
        FormField {
            id: "parent_link_label",
            label: "Link Label (optional)",
            kind: FieldKind::Text { default: "" },
            required: false,
            visible_when: None,
        },
    ],
};

// ==================== Tier A forms ====================

pub static BANNER_FORM: EditForm = EditForm {
    title: "dd-banner",
    fields: &[
        FormField {
            id: "parent_class",
            label: "Background Position",
            kind: FieldKind::Enum {
                options: &[
                    "-bg-top-left",
                    "-bg-top-center",
                    "-bg-top-right",
                    "-bg-center-left",
                    "-bg-center-center",
                    "-bg-center-right",
                    "-bg-bottom-left",
                    "-bg-bottom-center",
                    "-bg-bottom-right",
                ],
                default: "-bg-center-center",
            },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "parent_data_aos",
            label: "Animation",
            kind: FieldKind::Enum {
                options: AOS_OPTIONS,
                default: "fade-in",
            },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "parent_image_url",
            label: "Image URL",
            kind: FieldKind::Url { default: "" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "parent_image_alt",
            label: "Image Alt",
            kind: FieldKind::Text { default: "" },
            required: true,
            visible_when: None,
        },
    ],
};

pub static IMAGE_FORM: EditForm = EditForm {
    title: "dd-image",
    fields: &[
        FormField {
            id: "parent_data_aos",
            label: "Animation",
            kind: FieldKind::Enum {
                options: AOS_OPTIONS,
                default: "fade-in",
            },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "parent_image_url",
            label: "Image URL",
            kind: FieldKind::Url { default: "" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "parent_image_alt",
            label: "Image Alt",
            kind: FieldKind::Text { default: "" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "parent_link_url",
            label: "Link URL (optional)",
            kind: FieldKind::Url { default: "" },
            required: false,
            visible_when: None,
        },
        FormField {
            id: "parent_link_target",
            label: "Link Target",
            kind: FieldKind::Enum {
                options: LINK_TARGET_OPTIONS,
                default: "_self",
            },
            required: false,
            visible_when: None,
        },
    ],
};

pub static HEADER_SEARCH_FORM: EditForm = EditForm {
    title: "dd-header-search",
    fields: &[
        FormField {
            id: "parent_width",
            label: "Width Class",
            kind: FieldKind::Text { default: "" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "parent_data_aos",
            label: "Animation",
            kind: FieldKind::Enum {
                options: AOS_OPTIONS,
                default: "fade-in",
            },
            required: true,
            visible_when: None,
        },
    ],
};

pub static HEADER_MENU_FORM: EditForm = EditForm {
    title: "dd-header-menu",
    fields: &[
        FormField {
            id: "parent_width",
            label: "Width Class",
            kind: FieldKind::Text { default: "" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "parent_data_aos",
            label: "Animation",
            kind: FieldKind::Enum {
                options: AOS_OPTIONS,
                default: "fade-in",
            },
            required: true,
            visible_when: None,
        },
    ],
};

pub static RICH_TEXT_FORM: EditForm = EditForm {
    title: "dd-rich_text",
    fields: &[
        FormField {
            id: "parent_class",
            label: "CSS Class (optional)",
            kind: FieldKind::Text { default: "" },
            required: false,
            visible_when: None,
        },
        FormField {
            id: "parent_data_aos",
            label: "Animation",
            kind: FieldKind::Enum {
                options: AOS_OPTIONS,
                default: "fade-in",
            },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "parent_copy",
            label: "Copy (Markdown)",
            kind: FieldKind::Textarea {
                rows: 6,
                default: "",
            },
            required: true,
            visible_when: None,
        },
    ],
};

pub static ALERT_FORM: EditForm = EditForm {
    title: "dd-alert",
    fields: &[
        FormField {
            id: "parent_type",
            label: "Type",
            kind: FieldKind::Enum {
                options: &["-default", "-info", "-warning", "-error", "-success"],
                default: "-default",
            },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "parent_class",
            label: "Variant",
            kind: FieldKind::Enum {
                options: &["-default", "-compact"],
                default: "-default",
            },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "parent_data_aos",
            label: "Animation",
            kind: FieldKind::Enum {
                options: AOS_OPTIONS,
                default: "fade-in",
            },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "parent_title",
            label: "Title (optional)",
            kind: FieldKind::Text { default: "" },
            required: false,
            visible_when: None,
        },
        FormField {
            id: "parent_copy",
            label: "Copy",
            kind: FieldKind::Textarea {
                rows: 4,
                default: "",
            },
            required: true,
            visible_when: None,
        },
    ],
};

pub static MODAL_FORM: EditForm = EditForm {
    title: "dd-modal",
    fields: &[
        FormField {
            id: "parent_title",
            label: "Title",
            kind: FieldKind::Text { default: "" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "parent_copy",
            label: "Copy",
            kind: FieldKind::Textarea {
                rows: 5,
                default: "",
            },
            required: true,
            visible_when: None,
        },
    ],
};

pub static BLOCKQUOTE_FORM: EditForm = EditForm {
    title: "dd-blockquote",
    fields: &[
        FormField {
            id: "parent_data_aos",
            label: "Animation",
            kind: FieldKind::Enum {
                options: AOS_OPTIONS,
                default: "fade-in",
            },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "parent_image_url",
            label: "Image URL",
            kind: FieldKind::Url { default: "" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "parent_image_alt",
            label: "Image Alt",
            kind: FieldKind::Text { default: "" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "parent_name",
            label: "Name",
            kind: FieldKind::Text { default: "" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "parent_role",
            label: "Role",
            kind: FieldKind::Text { default: "" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "parent_copy",
            label: "Quote",
            kind: FieldKind::Textarea {
                rows: 5,
                default: "",
            },
            required: true,
            visible_when: None,
        },
    ],
};
