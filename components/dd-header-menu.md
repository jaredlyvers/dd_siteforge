---
component: dd-header-menu
version: 1
node_scope: header_item   # header-only chrome component; cannot be used in page sections

insert:
  defaults:
    parent_width: "dd-u-3-24 dd-u-sm-3-24 dd-u-md-3-24"
    parent_data_aos: "fade-in"

fields:
  - id: parent_width
    required: true
    type: string
    default: "dd-u-3-24 dd-u-sm-3-24 dd-u-md-3-24"
    maps_to: ".dd-header__item width class when this component is the only child of a header item (otherwise driven by the parent item)"

  - id: parent_data_aos
    required: true
    type: enum
    options: ["fade-in", "fade-up", "fade-right", "fade-down", "fade-left", "zoom-in", "zoom-in-up", "zoom-in-down"]
    default: "fade-in"
    maps_to: ".dd-header__menu-icon[data-aos]"

edit_ui:
  tab_order:
    - parent_width
    - parent_data_aos

  enter_behavior:
    parent_row: "start component field editing"

  modal_fields:
    parent_edit_modes:
      - parent_width
      - parent_data_aos
    hide_when_editing_component:
      - column.id
      - column.width_class

blueprint:
  label: "dd-header-menu"
  show_fields:
    - parent_width
    - parent_data_aos
---

## HTML Template

```html
<div class="dd-header__menu-icon [parent_width] -y-center -x-center" data-aos="[parent_data_aos]" data-aos-duration="1000" data-aos-easing="linear" data-aos-anchor-placement="center-bottom" data-aos-delay="100">
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
- `parent_data_aos` required; must be one of the enum options
- this component is only valid inside a `dd-section` that is itself a child of `site.header.sections[]`; placing it in a page-level section or in the footer must fail validation
