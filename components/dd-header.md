---
component: dd-header
version: 4
node_scope: page_node
global: true   # rendered on every page automatically, edited once in site.header

insert:
  defaults:
    id: "header"
    custom_css: ""
    alert: null            # single optional dd-alert, renders above .dd-header__top
    sections:
      - id: "header-section-1"
        section_title: null
        section_class: "-full-contained"
        item_box_class: "l-box"
        custom_css: ""
        columns:
          - id: "column-1"
            width_class: "dd-u-18-24 dd-u-md-18-24"
            components: []
          - id: "column-2"
            width_class: "dd-u-3-24 dd-u-sm-3-24 dd-u-md-3-24 dd-u-lg-4-24"
            components:
              - component_type: "dd-header-search"
          - id: "column-3"
            width_class: "dd-u-3-24 dd-u-sm-3-24 dd-u-md-3-24"
            components:
              - component_type: "dd-header-menu"

fields:
  - id: id
    required: true
    type: string
    maps_to: "header[id reference in builder only]"

  - id: custom_css
    required: false
    type: string
    default: ""
    maps_to: "<header> class token (appended to base dd-header)"

  - id: alert
    required: false
    type: object_or_null
    shape: "dd-alert"
    maps_to: "single dd-alert rendered above .dd-header__top; null when none"

  - id: sections
    required: true
    type: array
    min_items: 1
    shape: "dd-section"
    allowed_column_component_types: ["dd-image", "dd-rich_text", "dd-navigation", "dd-header-search", "dd-header-menu"]
    maps_to: "ordered list of dd-section children; each section uses the standard dd-section shape but its columns are restricted to the allowed_column_component_types above"

edit_ui:
  tab_order:
    - id
    - custom_css

  enter_behavior:
    parent_row: "start header field editing"
    alert_row: "edit or remove the single header alert"
    section_row: "enter standard dd-section editing"

  modal_fields:
    parent_edit_modes:
      - id
      - custom_css

  alert_slot:
    add_key: "A"
    remove_key: "X"
    behavior: "single optional alert; A creates if null, X removes"

  sections_collection:
    add_section_key: "A"
    remove_section_key: "X"
    reorder_keys: ["Up", "Down"]
    min_sections: 1

blueprint:
  label: "dd-header"
  show_fields:
    - id
    - custom_css
    - "alert?"               # ? suffix = may be null
    - "sections[].id"
    - "sections[].columns[].id"
---

## HTML Template

Two structural zones inside the `<header>`:

1. **alert zone** — above `.dd-header__top`, holds the single optional `dd-alert`
2. **top row** — inside `.dd-header__top`, iterates `sections[]` and renders each as a standard `dd-section`

The search panel (`<div class="dd-search">`) is currently hardcoded chrome
that stays at the bottom of the header. TODO: make it toggleable so sites
without search can drop it entirely.

```html
<header class="dd-header [custom_css]">
  <!-- if alert -->
  [render dd-alert here]
  <!-- endif -->

  <div class="dd-header__top">
    <!-- repeat: sections -->
    [render dd-section here — standard section HTML with its columns and nested components]
    <!-- end repeat: sections -->
  </div>

  <!-- hardcoded search panel (always rendered for now) -->
  <div class="dd-search">
    <button class="dd-search__close">- search</button>
    <form action="">
      <label for="name">Search<br />
        <input type="text" id="name">
      </label>
    </form>
  </div>
</header>
```

## Conditional Markup

- `[custom_css]` class token is appended to `<header>` only when `custom_css` is non-empty
- alert zone renders only when `alert` is non-null; when present, the alert HTML is the normal `dd-alert` markup injected above `.dd-header__top`
- `sections[]` renders sequentially inside `.dd-header__top`; each uses standard `dd-section` markup
- header renders once per site and is injected above every page's `<main>` content (see `global: true` flag above)
- `.dd-search` panel is always rendered today (see TODO above)

## Content Restrictions

Header content is bounded by these rules (enforced by the validator and the TUI insert finder):

- `alert`: at most 1 `dd-alert` (single slot, optional)
- `sections[]`: unlimited `dd-section` children
- inside each header `dd-section`'s columns, only the following component types may be placed:
  - `dd-image`
  - `dd-rich_text`
  - `dd-navigation`
  - `dd-header-search`
  - `dd-header-menu`

Other component types (like `dd-card`, `dd-cta`, `dd-banner`, `dd-accordion`, etc.) are valid only in page-level sections, never in header sections.

## Validation Rules

- `id` required and non-empty
- `custom_css` optional; free-form string when provided
- `alert` optional; when non-null, must be a valid `dd-alert` object (fails if it fails dd-alert validation)
- `sections[]` required with at least 1 section; each section must pass standard `dd-section` validation
- every component inside a header section's columns must be one of the `allowed_column_component_types`
- header-scope-only components (`dd-header-search`, `dd-header-menu`) are valid only inside a header `dd-section` column; they must fail validation when placed in page-level sections or in the footer
