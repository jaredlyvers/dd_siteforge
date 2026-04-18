---
component: dd-footer
version: 2
node_scope: page_node
global: true   # rendered on every page automatically, edited once in site.footer

insert:
  defaults:
    id: "footer"
    custom_css: ""
    sections:
      - id: "footer-section-1"
        section_title: null
        section_class: "-full-contained"
        item_box_class: "l-box"
        custom_css: ""
        columns:
          - id: "column-1"
            width_class: "dd-u-1-1"
            components: []

fields:
  - id: id
    required: true
    type: string
    maps_to: "footer[id reference in builder only]"

  - id: custom_css
    required: false
    type: string
    default: ""
    maps_to: "<footer> class token (appended to base dd-footer)"

  - id: sections
    required: true
    type: array
    min_items: 1
    shape: "dd-section"
    allowed_column_component_types: ["dd-image", "dd-rich_text", "dd-navigation"]
    maps_to: "ordered list of dd-section children; each section uses the standard dd-section shape but its columns are restricted to the allowed_column_component_types above"

edit_ui:
  tab_order:
    - id
    - custom_css

  enter_behavior:
    parent_row: "start footer field editing"
    section_row: "enter standard dd-section editing"

  modal_fields:
    parent_edit_modes:
      - id
      - custom_css

  sections_collection:
    add_section_key: "A"
    remove_section_key: "X"
    reorder_keys: ["Up", "Down"]
    min_sections: 1

blueprint:
  label: "dd-footer"
  show_fields:
    - id
    - custom_css
    - "sections[].id"
    - "sections[].columns[].id"
---

## HTML Template

`sections[]` render sequentially inside `.dd-footer__content`. Each uses
standard `dd-section` markup. No alert zone and no search chrome in footer
(keep it lean).

```html
<footer class="dd-footer [custom_css]">
  <div class="dd-footer__content">
    <!-- repeat: sections -->
    [render dd-section here — standard section HTML with its columns and nested components]
    <!-- end repeat: sections -->
  </div>
</footer>
```

## Conditional Markup

- `[custom_css]` class token is appended to `<footer>` only when `custom_css` is non-empty
- `sections[]` renders sequentially inside `.dd-footer__content`; each uses standard `dd-section` markup
- footer renders once per site and is injected after every page's `</main>` (see `global: true` flag above)

## Content Restrictions

Footer content is bounded by these rules (enforced by the validator and the TUI insert finder):

- `sections[]`: unlimited `dd-section` children
- inside each footer `dd-section`'s columns, only the following component types may be placed:
  - `dd-image`
  - `dd-rich_text`
  - `dd-navigation`

Header-scope-only components (`dd-header-search`, `dd-header-menu`) are not allowed in the footer. Other component types (like `dd-card`, `dd-cta`, `dd-banner`, `dd-accordion`, `dd-alert`, etc.) are valid only in page-level sections, never in footer sections.

## Validation Rules

- `id` required and non-empty
- `custom_css` optional; free-form string when provided
- `sections[]` required with at least 1 section; each section must pass standard `dd-section` validation
- every component inside a footer section's columns must be one of the `allowed_column_component_types` (`dd-image`, `dd-rich_text`, `dd-navigation`)
