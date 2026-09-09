//! Collection component forms and their item templates.
use super::*;

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
            id: "child_subtitle",
            label: "Subtitle (optional)",
            kind: FieldKind::Text { default: "Subtitle" },
            required: false,
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
            id: "sal",
            label: "Animation",
            kind: FieldKind::Enum { options: SAL_OPTIONS, default: "fade" },
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
            id: "sal",
            label: "Animation",
            kind: FieldKind::Enum { options: SAL_OPTIONS, default: "fade" },
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
            id: "sal",
            label: "Animation",
            kind: FieldKind::Enum { options: SAL_OPTIONS, default: "fade" },
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
            id: "sal",
            label: "Animation",
            kind: FieldKind::Enum { options: SAL_OPTIONS, default: "fade" },
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
            id: "sal",
            label: "Animation",
            kind: FieldKind::Enum { options: SAL_OPTIONS, default: "fade" },
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
