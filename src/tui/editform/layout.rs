//! Hero, section, page-head, header, footer, and navigation forms.
use super::*;

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
            id: "sal",
            label: "Animation",
            kind: FieldKind::Enum { options: SAL_OPTIONS, default: "fade" },
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

pub static PAGE_HEAD_FORM: EditForm = EditForm {
    title: "page-head",
    fields: &[
        FormField {
            id: "title",
            label: "Title",
            kind: FieldKind::Text { default: "" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "slug",
            label: "Slug",
            kind: FieldKind::Text { default: "" },
            required: false,
            visible_when: None,
        },
        FormField {
            id: "meta_title",
            label: "Meta Title",
            kind: FieldKind::Text { default: "" },
            required: false,
            visible_when: None,
        },
        FormField {
            id: "meta_description",
            label: "Meta Description",
            kind: FieldKind::Text { default: "" },
            required: false,
            visible_when: None,
        },
        FormField {
            id: "canonical_url",
            label: "Canonical URL",
            kind: FieldKind::Text { default: "" },
            required: false,
            visible_when: None,
        },
        FormField {
            id: "robots",
            label: "Robots",
            kind: FieldKind::Enum {
                options: ROBOTS_OPTIONS,
                default: "index, follow",
            },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "schema_type",
            label: "Schema Type",
            kind: FieldKind::Enum {
                options: SCHEMA_OPTIONS,
                default: "WebPage",
            },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "og_title",
            label: "OG Title",
            kind: FieldKind::Text { default: "" },
            required: false,
            visible_when: None,
        },
        FormField {
            id: "og_description",
            label: "OG Description",
            kind: FieldKind::Text { default: "" },
            required: false,
            visible_when: None,
        },
        FormField {
            id: "og_image",
            label: "OG Image",
            kind: FieldKind::Text { default: "" },
            required: false,
            visible_when: None,
        },
    ],
};

pub static HEADER_ROOT_FORM: EditForm = EditForm {
    title: "dd-header-root",
    fields: &[
        FormField {
            id: "id",
            label: "Header ID",
            kind: FieldKind::Text { default: "" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "custom_css",
            label: "Custom CSS",
            kind: FieldKind::Text { default: "" },
            required: false,
            visible_when: None,
        },
    ],
};

pub static FOOTER_FORM: EditForm = EditForm {
    title: "dd-footer",
    fields: &[
        FormField {
            id: "id",
            label: "Footer ID",
            kind: FieldKind::Text { default: "" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "custom_css",
            label: "Custom CSS",
            kind: FieldKind::Text { default: "" },
            required: false,
            visible_when: None,
        },
    ],
};

pub static SITE_FORM: EditForm = EditForm {
    title: "Site settings",
    fields: &[
        FormField {
            id: "name",
            label: "Name",
            kind: FieldKind::Text { default: "" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "lang",
            label: "Lang",
            kind: FieldKind::Text { default: "en" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "base_url",
            label: "Base URL",
            kind: FieldKind::Url { default: "" },
            required: false,
            visible_when: None,
        },
        FormField {
            id: "export_dir",
            label: "Export Dir",
            kind: FieldKind::Text { default: "" },
            required: false,
            visible_when: None,
        },
        FormField {
            id: "primary_color",
            label: "Primary Color",
            kind: FieldKind::Text { default: "" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "secondary_color",
            label: "Secondary Color",
            kind: FieldKind::Text { default: "" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "tertiary_color",
            label: "Tertiary Color",
            kind: FieldKind::Text { default: "" },
            required: true,
            visible_when: None,
        },
        FormField {
            id: "support_color",
            label: "Support Color",
            kind: FieldKind::Text { default: "" },
            required: true,
            visible_when: None,
        },
    ],
};

/// NAV_ITEM_FORM is self-referential — its `items` field is a SubForm whose
/// template is `&NAV_ITEM_FORM`. Rust permits this because the address of a
/// `static` is known at compile time.
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
            id: "sal",
            label: "Animation",
            kind: FieldKind::Enum { options: SAL_OPTIONS, default: "fade" },
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
