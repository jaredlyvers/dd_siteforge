---
component: dd-your-component
version: 1
node_scope: section_item   # one of: page_node | section_item

insert:
  defaults:
    parent_type: "-default"
    parent_class: "-primary"
    sal: "fade"
    parent_title: "Title"
    parent_subtitle: "Subtitle"
    parent_copy: "Copy"

fields:
  - id: parent_type
    required: true
    type: enum
    options: ["-default", "-alt"]
    default: "-default"
    maps_to: ".dd-your-component class token"

  - id: parent_class
    required: true
    type: enum
    options: ["-primary", "-secondary"]
    default: "-primary"
    maps_to: ".dd-your-component class token"

  - id: sal
    required: true
    type: enum
    options: ["fade","slide-up","slide-down","slide-left","slide-right","zoom-in","zoom-out","flip-up","flip-down","flip-left","flip-right"]
    default: "fade"
    maps_to: ".dd-your-component[data-sal]"

  - id: parent_title
    required: true
    type: string
    maps_to: ".dd-your-component__title"

  - id: parent_subtitle
    required: true
    type: string
    maps_to: ".dd-your-component__subtitle"

  - id: parent_copy
    required: true
    type: string
    maps_to: ".dd-your-component__copy"
    ui:
      control: textarea
      rows: 5
      multiline: true
      keyboard:
        enter: "insert newline"
        ctrl_s: "save"
        up_down: "move cursor line"
        left_right: "move cursor character"
      mouse:
        wheel: "scroll lines"

edit_ui:
  tab_order:
    - parent_type
    - parent_class
    - sal
    - parent_title
    - parent_subtitle
    - parent_copy

  enter_behavior:
    parent_row: "start component field editing"

  modal_fields:
    parent_edit_modes:
      - parent_type
      - parent_class
      - sal
      - parent_title
      - parent_subtitle
      - parent_copy
    hide_when_editing_component:
      - column.id
      - column.width_class

blueprint:
  label: "dd-your-component"
  show_fields:
    - "parent_title"
---

## HTML Template

```html
<div class="dd-your-component [parent_type] [parent_class]" data-sal="[sal]">
  <div class="dd-your-component__title"><h3>[parent_title]</h3></div>
  <div class="dd-your-component__subtitle"><strong>[parent_subtitle]</strong></div>
  <div class="dd-your-component__copy"><p>[parent_copy]</p></div>
</div>
```

## Conditional Markup

- none (this variant intentionally has no optional link fields)
