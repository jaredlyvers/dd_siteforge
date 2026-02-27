---
component: dd-slider
version: 1
node_scope: section_item   # one of: page_node | section_item

insert:
  defaults:
    # parent fields
    parent_title: ""

    # required children collection
    items:
      - child_title: "Title"
        child_copy: "Copy"
        child_link_url: "/path"
        child_link_target: "_self"
        child_link_label: "Learn More"
        child_image_url: "https://dummyimage.com/720x720/000/fff"
        child_image_alt: "Image alt text"

fields:
  # ---------------------------
  # parent fields
  # ---------------------------
  - id: parent_title
    required: false
    type: string
    maps_to: ".dd-slider__title"

  # ---------------------------
  # child items[] fields
  # ---------------------------
  - id: items
    required: true
    type: array
    min_items: 1
    item_fields:
      - id: child_title
        required: true
        type: string
        maps_to: ".dd-slider__title"

      - id: child_copy
        required: true
        type: string
        maps_to: ".dd-slider__copy"
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

      - id: child_link_url
        required: false
        type: string
        maps_to: ".dd-slider__link a[href]"

      - id: child_link_target
        required: false
        type: enum
        options: ["_self", "_blank"]
        default: "_self"
        maps_to: ".dd-slider__link a[target]"

      - id: child_link_label
        required: false
        type: string
        maps_to: ".dd-slider__link a"
        
      - id: child_image_url
        required: true
        type: string
        maps_to: ".dd-slider__image img[src]"

      - id: child_image_alt
        required: true
        type: string
        maps_to: ".dd-slider__image img[alt]"

derived_fields:
  - id: parent_uid
    from: parent_title
    transform: html_id_safe
    fallback:
      mode: random_numeric
      prefix: "uid-"
      digits: 6
    editable: false
    maps_to: ".dd-slider__item[data-id]"

transforms:
  html_id_safe:
    steps:
      - "trim"
      - "lowercase"
      - "replace non [a-z0-9_-] with '-'"
      - "collapse consecutive '-'"
      - "trim '-' from start/end"
      - "if starts with digit, prefix 'uid-'"
      - "if empty, use fallback"
      
edit_ui:
  tab_order:
    # parent edit order
    - parent_title

    # child edit order (used when editing an item row)
    - items[].child_title
    - items[].child_copy
    - items[].child_link_url
    - items[].child_link_target
    - items[].child_link_label
    - items[].child_image_url
    - items[].child_image_alt

  navigation_tree:
    parent_row: "dd-slider"
    child_rows: "items[]"
    item_row_label: "item {index}: child_title"
    collapse_expand_key: "Space"

  item_collection:
    add_item_key: "A"
    remove_item_key: "X"
    add_behavior: "insert after selected item row, otherwise append to end"
    min_items: 1

  enter_behavior:
    parent_row: "start parent field editing"
    item_row: "start selected items[].child_image_url editing"

  modal_fields:
    parent_edit_modes:
      - parent_title
    item_edit_modes:
      - items[].child_title
      - items[].child_copy
      - items[].child_link_url
      - items[].child_link_target
      - items[].child_link_label
      - items[].child_image_url
      - items[].child_image_alt
    scope_rule: "when editing an items[] row, parent fields are not editable; when editing parent row, item fields are not editable"
    hide_when_editing_parent_or_child:
      - column.id
      - column.width_class

blueprint:
  label: "dd-slider"
  show_fields:
    - "items[active].child_title"
---

## HTML Template

```html
<div class="dd-slider">
  <div class="dd-slider__title">
    <h2>[parent_title]</h2>
  </div>
  <ul class="dd-slider__items -nostyle">
    <!-- repeat: items -->
    <li class="dd-slider__item" data-id="[parent_uid]">
      <div class="dd-slider__content">
        <div class="dd-g">
          <!-- Start body -->
          <div class="dd-slider__body dd-u-1-1 dd-u-sm-1-1 dd-u-md-1-1 dd-u-lg-12-24 l-box">
            <div class="dd-slider__title">
              [child_title]
            </div>
            <div class="dd-slider__copy">
              [child_copy]
              <div class="dd-slider__links">
                <div class="dd-slider__link">
                  <a href="[child_link_url]" target="[child_link_target]" class="dd-button -primary">[child_link_label]</a>
                </div>
              </div>
            </div>
          </div>
          <!-- End body -->
          <!-- Start image or video -->
          <div class="dd-slider__image dd-u-1-1 dd-u-sm-1-1 dd-u-md-1-1 dd-u-lg-12-24">
            <img
              src="[child_image_url]"
              alt="[child_image_alt]"
            />
          </div>
          <!-- End image or video -->
        </div>
      </div>
    </li>
  </ul>
  <div class="dd-slider__navigation">
    <button id="dd-slider__previous"><span class="-scrn-reader-only">Previous slide</span> < </button>
    <ul class="dd-slider__tabs -nostyle"></ul>
    <button id="dd-slider__next"><span class="-scrn-reader-only">Next slide</span> > </button>
  </div>
</div>
```

## Conditional Markup

- render `.dd-slider__title` only when `parent_title` is not empty
- render `.dd-slider__links` only when both `child_link_url` and `child_link_label` are non-empty
- when `child_link_target` is empty, default to `_self`
