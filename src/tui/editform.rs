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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    OptionalLinkTriple {
        url_id: &'static str,
        target_id: &'static str,
        label_id: &'static str,
    },
    /// Collection of sub-items. Each item follows `template`'s shape. The
    /// editor renders the collection as a summary list and drills into a
    /// nested FormEdit for individual-item editing. `summary_field_id` tells
    /// the parent renderer which field of the item to surface as the row
    /// summary (usually `child_title` or equivalent).
    SubForm {
        template: &'static EditForm,
        min_items: usize,
        summary_field_id: &'static str,
    },
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
    /// For each `SubForm` field (keyed by its id), the live state of every
    /// item in the collection. Each item is itself an `EditFormState` whose
    /// `form` is the SubForm's item template.
    pub sub_state: HashMap<String, Vec<EditFormState>>,
    /// For each `SubForm` field, which item index is currently highlighted
    /// in the summary list (used by Up/Down/A/X/Enter when focus is on the
    /// SubForm field).
    pub selected_sub_item: HashMap<String, usize>,
    pub focused_field: usize,
    /// (row, col) cursor inside a `Textarea` field; only meaningful when
    /// `focused_field` points at a Textarea.
    pub textarea_cursor: (usize, usize),
}

impl EditFormState {
    /// Build a fresh state with every field initialised to its declared default.
    pub fn new(form: &'static EditForm) -> Self {
        let mut values = HashMap::new();
        let mut sub_state: HashMap<String, Vec<EditFormState>> = HashMap::new();
        let mut selected_sub_item: HashMap<String, usize> = HashMap::new();
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
                FieldKind::SubForm { .. } => {
                    sub_state.insert(field.id.to_string(), Vec::new());
                    selected_sub_item.insert(field.id.to_string(), 0);
                }
            }
        }
        Self {
            form,
            values,
            sub_state,
            selected_sub_item,
            focused_field: 0,
            textarea_cursor: (0, 0),
        }
    }

    /// Make an item-level state for adding a new item to the given SubForm.
    /// Returns None if the field isn't a SubForm.
    pub fn new_sub_item(&self, subform_field_id: &str) -> Option<EditFormState> {
        for field in self.form.fields {
            if field.id == subform_field_id {
                if let FieldKind::SubForm { template, .. } = &field.kind {
                    return Some(EditFormState::new(*template));
                }
            }
        }
        None
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

const HERO_TARGET_OPTIONS: &[&str] = &["_self", "_blank", "_parent"];

const HERO_CLASS_OPTIONS: &[&str] = &[
    "-contained",
    "-contained-md",
    "-contained-lg",
    "-contained-xl",
    "-contained-xxl",
    "-full-full",
    "-full-contained",
    "-full-contained-md",
    "-full-contained-lg",
    "-full-contained-xl",
    "-full-contained-xxl",
];

const SECTION_CLASS_OPTIONS: &[&str] = HERO_CLASS_OPTIONS;

const ITEM_BOX_CLASS_OPTIONS: &[&str] = &["l-box", "ll-box"];

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

// ==================== Tier B item templates ====================
// Item templates are defined before the parent forms that reference them.

pub static CARD_ITEM_FORM: EditForm = EditForm {
    title: "dd-card item",
    fields: &[
        FormField {
            id: "child_image_url",
            label: "Image URL",
            kind: FieldKind::Url { default: "" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "child_image_alt",
            label: "Image Alt",
            kind: FieldKind::Text { default: "" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "child_title",
            label: "Title",
            kind: FieldKind::Text { default: "" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "child_subtitle",
            label: "Subtitle",
            kind: FieldKind::Text { default: "" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "child_copy",
            label: "Copy",
            kind: FieldKind::Textarea { rows: 4, default: "" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "child_link_url",
            label: "Link URL (optional)",
            kind: FieldKind::Url { default: "" },
            required: false,
            visible_when: None,
        },
        FormField {
            id: "child_link_target",
            label: "Link Target",
            kind: FieldKind::Enum { options: LINK_TARGET_OPTIONS, default: "_self" },
            required: false,
            visible_when: None,
        },
        FormField {
            id: "child_link_label",
            label: "Link Label (optional)",
            kind: FieldKind::Text { default: "" },
            required: false,
            visible_when: None,
        },
    ],
};

pub static FILMSTRIP_ITEM_FORM: EditForm = EditForm {
    title: "dd-filmstrip item",
    fields: &[
        FormField {
            id: "child_image_url",
            label: "Image URL",
            kind: FieldKind::Url { default: "" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "child_image_alt",
            label: "Image Alt",
            kind: FieldKind::Text { default: "" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "child_title",
            label: "Title",
            kind: FieldKind::Text { default: "" },
            required: true,
            visible_when: None,
        },
    ],
};

pub static MILESTONES_ITEM_FORM: EditForm = EditForm {
    title: "dd-milestones item",
    fields: &[
        FormField {
            id: "child_percentage",
            label: "Percentage",
            kind: FieldKind::Text { default: "" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "child_title",
            label: "Title",
            kind: FieldKind::Text { default: "" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "child_subtitle",
            label: "Subtitle",
            kind: FieldKind::Text { default: "" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "child_copy",
            label: "Copy",
            kind: FieldKind::Textarea { rows: 4, default: "" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "child_link_url",
            label: "Link URL (optional)",
            kind: FieldKind::Url { default: "" },
            required: false,
            visible_when: None,
        },
        FormField {
            id: "child_link_target",
            label: "Link Target",
            kind: FieldKind::Enum { options: LINK_TARGET_OPTIONS, default: "_self" },
            required: false,
            visible_when: None,
        },
        FormField {
            id: "child_link_label",
            label: "Link Label (optional)",
            kind: FieldKind::Text { default: "" },
            required: false,
            visible_when: None,
        },
    ],
};

pub static SLIDER_ITEM_FORM: EditForm = EditForm {
    title: "dd-slider item",
    fields: &[
        FormField {
            id: "child_title",
            label: "Title",
            kind: FieldKind::Text { default: "" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "child_copy",
            label: "Copy",
            kind: FieldKind::Textarea { rows: 4, default: "" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "child_image_url",
            label: "Image URL",
            kind: FieldKind::Url { default: "" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "child_image_alt",
            label: "Image Alt",
            kind: FieldKind::Text { default: "" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "child_link_url",
            label: "Link URL (optional)",
            kind: FieldKind::Url { default: "" },
            required: false,
            visible_when: None,
        },
        FormField {
            id: "child_link_target",
            label: "Link Target",
            kind: FieldKind::Enum { options: LINK_TARGET_OPTIONS, default: "_self" },
            required: false,
            visible_when: None,
        },
        FormField {
            id: "child_link_label",
            label: "Link Label (optional)",
            kind: FieldKind::Text { default: "" },
            required: false,
            visible_when: None,
        },
    ],
};

pub static ACCORDION_ITEM_FORM: EditForm = EditForm {
    title: "dd-accordion item",
    fields: &[
        FormField {
            id: "child_title",
            label: "Title",
            kind: FieldKind::Text { default: "" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "child_copy",
            label: "Content",
            kind: FieldKind::Textarea { rows: 5, default: "" },
            required: true,
            visible_when: None,
        },
    ],
};

pub static ALTERNATING_ITEM_FORM: EditForm = EditForm {
    title: "dd-alternating item",
    fields: &[
        FormField {
            id: "child_image_url",
            label: "Image URL",
            kind: FieldKind::Url { default: "" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "child_image_alt",
            label: "Image Alt",
            kind: FieldKind::Text { default: "" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "child_title",
            label: "Title",
            kind: FieldKind::Text { default: "" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "child_copy",
            label: "Copy",
            kind: FieldKind::Textarea { rows: 5, default: "" },
            required: true,
            visible_when: None,
        },
    ],
};

// ==================== Tier B parent forms ====================

pub static CARD_FORM: EditForm = EditForm {
    title: "dd-card",
    fields: &[
        FormField {
            id: "parent_type",
            label: "Layout",
            kind: FieldKind::Enum { options: &["-default", "-horizontal"], default: "-default" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "parent_data_aos",
            label: "Animation",
            kind: FieldKind::Enum { options: AOS_OPTIONS, default: "fade-in" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "parent_width",
            label: "Width Classes",
            kind: FieldKind::Text { default: "dd-u-1-1 dd-u-md-12-24 dd-u-lg-8-24" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "items",
            label: "Items",
            kind: FieldKind::SubForm {
                template: &CARD_ITEM_FORM,
                min_items: 1,
                summary_field_id: "child_title",
            },
            required: true,
            visible_when: None,
        },
    ],
};

pub static FILMSTRIP_FORM: EditForm = EditForm {
    title: "dd-filmstrip",
    fields: &[
        FormField {
            id: "parent_type",
            label: "Direction",
            kind: FieldKind::Enum { options: &["-default", "-reverse"], default: "-default" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "parent_data_aos",
            label: "Animation",
            kind: FieldKind::Enum { options: AOS_OPTIONS, default: "fade-in" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "items",
            label: "Items",
            kind: FieldKind::SubForm {
                template: &FILMSTRIP_ITEM_FORM,
                min_items: 1,
                summary_field_id: "child_title",
            },
            required: true,
            visible_when: None,
        },
    ],
};

pub static MILESTONES_FORM: EditForm = EditForm {
    title: "dd-milestones",
    fields: &[
        FormField {
            id: "parent_data_aos",
            label: "Animation",
            kind: FieldKind::Enum { options: AOS_OPTIONS, default: "fade-in" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "parent_width",
            label: "Width Classes",
            kind: FieldKind::Text { default: "dd-u-1-1 dd-u-md-12-24" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "items",
            label: "Items",
            kind: FieldKind::SubForm {
                template: &MILESTONES_ITEM_FORM,
                min_items: 1,
                summary_field_id: "child_title",
            },
            required: true,
            visible_when: None,
        },
    ],
};

pub static SLIDER_FORM: EditForm = EditForm {
    title: "dd-slider",
    fields: &[
        FormField {
            id: "parent_title",
            label: "Slider Title",
            kind: FieldKind::Text { default: "" },
            required: false,
            visible_when: None,
        },
        FormField {
            id: "items",
            label: "Items",
            kind: FieldKind::SubForm {
                template: &SLIDER_ITEM_FORM,
                min_items: 1,
                summary_field_id: "child_title",
            },
            required: true,
            visible_when: None,
        },
    ],
};

pub static ACCORDION_FORM: EditForm = EditForm {
    title: "dd-accordion",
    fields: &[
        FormField {
            id: "parent_type",
            label: "Type",
            kind: FieldKind::Enum { options: &["-default", "-faq"], default: "-default" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "parent_class",
            label: "Variant",
            kind: FieldKind::Enum {
                options: &["-borderless", "-compact", "-primary", "-secondary", "-tertiary"],
                default: "-primary",
            },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "parent_data_aos",
            label: "Animation",
            kind: FieldKind::Enum { options: AOS_OPTIONS, default: "fade-in" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "parent_group_name",
            label: "Group Name",
            kind: FieldKind::Text { default: "group1" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "items",
            label: "Items",
            kind: FieldKind::SubForm {
                template: &ACCORDION_ITEM_FORM,
                min_items: 1,
                summary_field_id: "child_title",
            },
            required: true,
            visible_when: None,
        },
    ],
};

pub static ALTERNATING_FORM: EditForm = EditForm {
    title: "dd-alternating",
    fields: &[
        FormField {
            id: "parent_type",
            label: "Alternation",
            kind: FieldKind::Enum {
                options: &["-default", "-reverse", "-no-alternate"],
                default: "-default",
            },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "parent_class",
            label: "CSS Class",
            kind: FieldKind::Text { default: "-default" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "parent_data_aos",
            label: "Animation",
            kind: FieldKind::Enum { options: AOS_OPTIONS, default: "fade-in" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "items",
            label: "Items",
            kind: FieldKind::SubForm {
                template: &ALTERNATING_ITEM_FORM,
                min_items: 1,
                summary_field_id: "child_title",
            },
            required: true,
            visible_when: None,
        },
    ],
};

// ==================== Tier C: dd-hero ====================

pub static HERO_FORM: EditForm = EditForm {
    title: "dd-hero",
    fields: &[
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
            label: "Copy (Markdown)",
            kind: FieldKind::Textarea { rows: 5, default: "" },
            required: false,
            visible_when: None,
        },
        FormField {
            id: "parent_class",
            label: "Hero Class",
            kind: FieldKind::Enum { options: HERO_CLASS_OPTIONS, default: "-full-full" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "parent_data_aos",
            label: "Animation",
            kind: FieldKind::Enum { options: AOS_OPTIONS, default: "fade-in" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "parent_custom_css",
            label: "Custom CSS (optional)",
            kind: FieldKind::Text { default: "" },
            required: false,
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
            label: "Image Alt (optional)",
            kind: FieldKind::Text { default: "" },
            required: false,
            visible_when: None,
        },
        FormField {
            id: "parent_image_class",
            label: "Image Class",
            kind: FieldKind::Enum { options: HERO_CLASS_OPTIONS, default: "-full-full" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "parent_image_mobile",
            label: "Image (mobile, optional)",
            kind: FieldKind::Url { default: "" },
            required: false,
            visible_when: None,
        },
        FormField {
            id: "parent_image_tablet",
            label: "Image (tablet, optional)",
            kind: FieldKind::Url { default: "" },
            required: false,
            visible_when: None,
        },
        FormField {
            id: "parent_image_desktop",
            label: "Image (desktop, optional)",
            kind: FieldKind::Url { default: "" },
            required: false,
            visible_when: None,
        },
        FormField {
            id: "link_1_label",
            label: "Link 1 Label (optional)",
            kind: FieldKind::Text { default: "" },
            required: false,
            visible_when: None,
        },
        FormField {
            id: "link_1_url",
            label: "Link 1 URL (optional)",
            kind: FieldKind::Url { default: "" },
            required: false,
            visible_when: None,
        },
        FormField {
            id: "link_1_target",
            label: "Link 1 Target",
            kind: FieldKind::Enum { options: HERO_TARGET_OPTIONS, default: "_self" },
            required: false,
            visible_when: None,
        },
        FormField {
            id: "link_2_label",
            label: "Link 2 Label (optional)",
            kind: FieldKind::Text { default: "" },
            required: false,
            visible_when: None,
        },
        FormField {
            id: "link_2_url",
            label: "Link 2 URL (optional)",
            kind: FieldKind::Url { default: "" },
            required: false,
            visible_when: None,
        },
        FormField {
            id: "link_2_target",
            label: "Link 2 Target",
            kind: FieldKind::Enum { options: HERO_TARGET_OPTIONS, default: "_self" },
            required: false,
            visible_when: None,
        },
    ],
};

// ==================== Tier C: dd-section (and header/footer section) ====================

pub static COLUMN_ITEM_FORM: EditForm = EditForm {
    title: "column",
    fields: &[
        FormField {
            id: "id",
            label: "Column ID",
            kind: FieldKind::Text { default: "" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "width_class",
            label: "Width Class (dd-u-*)",
            kind: FieldKind::Text { default: "dd-u-1-1" },
            required: true,
            visible_when: None,
        },
    ],
};

pub static SECTION_FORM: EditForm = EditForm {
    title: "dd-section",
    fields: &[
        FormField {
            id: "id",
            label: "Section ID",
            kind: FieldKind::Text { default: "" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "section_title",
            label: "Section Title (optional)",
            kind: FieldKind::Text { default: "" },
            required: false,
            visible_when: None,
        },
        FormField {
            id: "section_class",
            label: "Section Class",
            kind: FieldKind::Enum { options: SECTION_CLASS_OPTIONS, default: "-full-contained" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "item_box_class",
            label: "Item Box Class",
            kind: FieldKind::Enum { options: ITEM_BOX_CLASS_OPTIONS, default: "l-box" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "columns",
            label: "Columns",
            kind: FieldKind::SubForm {
                template: &COLUMN_ITEM_FORM,
                min_items: 1,
                summary_field_id: "id",
            },
            required: true,
            visible_when: None,
        },
    ],
};

// ==================== Tier D: dd-navigation (recursive) ====================
//
// NAV_ITEM_FORM is self-referential — its `items` field is a SubForm whose
// template is `&NAV_ITEM_FORM`. Rust permits this because the address of a
// `static` is known at compile time.

pub static NAV_ITEM_FORM: EditForm = EditForm {
    title: "nav item",
    fields: &[
        FormField {
            id: "child_kind",
            label: "Kind",
            kind: FieldKind::Enum { options: &["link", "button"], default: "link" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "child_link_label",
            label: "Label",
            kind: FieldKind::Text { default: "" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "child_link_url",
            label: "URL",
            kind: FieldKind::Url { default: "" },
            required: false,
            visible_when: Some(FieldPredicate::FieldEquals {
                other_id: "child_kind",
                value: "link",
            }),
        },
        FormField {
            id: "child_link_target",
            label: "Target",
            kind: FieldKind::Enum { options: LINK_TARGET_OPTIONS, default: "_self" },
            required: false,
            visible_when: Some(FieldPredicate::FieldEquals {
                other_id: "child_kind",
                value: "link",
            }),
        },
        FormField {
            id: "child_link_css",
            label: "CSS Class (optional)",
            kind: FieldKind::Text { default: "" },
            required: false,
            visible_when: None,
        },
        FormField {
            id: "items",
            label: "Nested items",
            kind: FieldKind::SubForm {
                template: &NAV_ITEM_FORM,
                min_items: 0,
                summary_field_id: "child_link_label",
            },
            required: false,
            visible_when: None,
        },
    ],
};

pub static NAVIGATION_FORM: EditForm = EditForm {
    title: "dd-navigation",
    fields: &[
        FormField {
            id: "parent_type",
            label: "Type",
            kind: FieldKind::Enum {
                options: &["dd-header__navigation", "dd-footer__navigation"],
                default: "dd-header__navigation",
            },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "parent_class",
            label: "Menu Style",
            kind: FieldKind::Enum {
                options: &[
                    "-main-menu",
                    "-menu-secondary",
                    "-menu-tertiary",
                    "-footer-menu",
                    "-footer-menu-secondary",
                    "-footer-menu-tertiary",
                    "-social-menu",
                ],
                default: "-main-menu",
            },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "parent_data_aos",
            label: "Animation",
            kind: FieldKind::Enum { options: AOS_OPTIONS, default: "fade-in" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "parent_width",
            label: "Width Classes",
            kind: FieldKind::Text {
                default: "dd-u-1-1 dd-u-sm-1-1 dd-u-md-1-1 dd-u-lg-18-24",
            },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "items",
            label: "Menu Items",
            kind: FieldKind::SubForm {
                template: &NAV_ITEM_FORM,
                min_items: 1,
                summary_field_id: "child_link_label",
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
