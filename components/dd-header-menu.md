---
component: dd-header-menu
version: 1
node_scope: header_item   # header-only chrome component; cannot be used in page sections

insert:
  defaults:
    parent_width: "dd-u-3-24 dd-u-sm-3-24 dd-u-md-3-24"
    sal: "fade"

fields:
  - id: parent_width
    required: true
    type: string
    default: "dd-u-3-24 dd-u-sm-3-24 dd-u-md-3-24"
    maps_to: ".dd-header__item width class when this component is the only child of a header item (otherwise driven by the parent item)"

  - id: sal
    required: true
    type: enum
    options: ["fade","slide-up","slide-down","slide-left","slide-right","zoom-in","zoom-out","flip-up","flip-down","flip-left","flip-right"]
    default: "fade"
    maps_to: ".dd-header__menu-icon[data-sal]"

edit_ui:
  tab_order:
    - parent_width
    - sal

  enter_behavior:
    parent_row: "start component field editing"

  modal_fields:
    parent_edit_modes:
      - parent_width
      - sal
    hide_when_editing_component:
      - column.id
      - column.width_class

blueprint:
  label: "dd-header-menu"
  show_fields:
    - parent_width
    - sal
---

## HTML Template

```html
<div class="dd-header__menu-icon [parent_width] -y-center -x-center" data-sal="[sal]">
  <button class="dd-menu__toggle fa-regular fa-bars" type="button">
    <span class="visually-hidden">Menu</span>
  </button>
</div>
```

## Conditional Markup

- always renders when present in a header item's `components[]`
- toggles the mobile/overlay menu (behavior handled by frontend JS, not spec)

## Validation Rules

- `parent_width` required and non-empty
- `sal` required; must be one of the enum options
- this component is only valid inside a `dd-section` that is itself a child of `site.header.sections[]`; placing it in a page-level section or in the footer must fail validation
