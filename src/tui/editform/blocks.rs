//! Simple block component forms.
use super::*;

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
            id: "sal",
            label: "Animation",
            kind: FieldKind::Enum {
                options: SAL_OPTIONS,
                default: "fade",
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
            id: "sal",
            label: "Animation",
            kind: FieldKind::Enum {
                options: SAL_OPTIONS,
                default: "fade",
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
            id: "sal",
            label: "Animation",
            kind: FieldKind::Enum {
                options: SAL_OPTIONS,
                default: "fade",
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
            id: "sal",
            label: "Animation",
            kind: FieldKind::Enum {
                options: SAL_OPTIONS,
                default: "fade",
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
            id: "sal",
            label: "Animation",
            kind: FieldKind::Enum {
                options: SAL_OPTIONS,
                default: "fade",
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
            id: "sal",
            label: "Animation",
            kind: FieldKind::Enum {
                options: SAL_OPTIONS,
                default: "fade",
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
            id: "sal",
            label: "Animation",
            kind: FieldKind::Enum {
                options: SAL_OPTIONS,
                default: "fade",
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
            id: "sal",
            label: "Animation",
            kind: FieldKind::Enum {
                options: SAL_OPTIONS,
                default: "fade",
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
