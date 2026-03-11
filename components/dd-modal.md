---
component: dd-modal
version: 1
node_scope: section_item   # one of: page_node | section_item

insert:
  defaults:
    parent_title: "Title"
    parent_copy: "Copy"

fields:
  - id: parent_title
    required: true
    type: string
    maps_to: ".dd-modal__button-open"

  - id: parent_copy
    required: true
    type: string
    maps_to: ".dd-modal__copy p"
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

derived_fields:
  - id: parent_modal_id
    from: parent_title
    transform: html_id_safe
    fallback: "modal"
    editable: false
    maps_to: ".dd-modal__button-open[data-id], .dd-modal[id], .dd-modal__button-close[data-id]"

transforms:
  html_id_safe:
    steps:
      - "trim"
      - "lowercase"
      - "replace non [a-z0-9_-] with '-'"
      - "collapse consecutive '-'"
      - "trim '-' from start/end"
      - "if starts with digit, prefix 'modal-'"
      - "if empty, use fallback"

edit_ui:
  tab_order:
    - parent_title
    - parent_copy

  enter_behavior:
    parent_row: "start component field editing"

  modal_fields:
    parent_edit_modes:
      - parent_title
      - parent_copy
    hide_when_editing_component:
      - column.id
      - column.width_class

blueprint:
  label: "dd-modal"
  show_fields:
    - "parent_title"
---

## HTML Template

```html
<button class="dd-modal__button-open" data-modal-open data-id="[parent_modal_id]">[parent_title]</button>
<dialog data-modal id="[parent_modal_id]" class="dd-modal">
  <button class="dd-modal__button-close" data-modal-close data-id="[parent_modal_id]" aria-label="close modal window">X</button>
  <div class="dd-modal__copy">
    <p>[parent_copy]</p>
  </div>
</dialog>
```

## Conditional Markup
