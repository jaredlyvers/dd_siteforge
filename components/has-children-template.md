---
component: dd-your-component
version: 1
node_scope: section_item   # one of: page_node | section_item

insert:
  defaults:
    # parent fields
    parent_type: "-default"
    parent_class: "-primary"
    parent_data_aos: "fade-in"
    parent_width: "dd-u-1-1 dd-u-md-12-24"

    # required children collection
    items:
      - child_image_url: "https://dummyimage.com/720x720/000/fff"
        child_image_alt: "Image alt text"
        child_title: "Title"
        child_subtitle: "Subtitle"
        child_copy: "Copy"
        child_link_url: "/path"
        child_link_target: "_self"
        child_link_label: "Learn More"

fields:
  # ---------------------------
  # parent fields
  # ---------------------------
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

  - id: parent_data_aos
    required: true
    type: enum
    options: ["fade-in","fade-up","fade-right","fade-down","fade-left","zoom-in","zoom-in-up","zoom-in-down"]
    default: "fade-in"
    maps_to: ".dd-your-component[data-aos] OR child[data-aos]"

  - id: parent_width
    required: true
    type: string
    default: "dd-u-1-1 dd-u-md-12-24"
    maps_to: ".dd-your-component__item class token"

  # ---------------------------
  # child items[] fields
  # ---------------------------
  - id: items
    required: true
    type: array
    min_items: 1
    item_fields:
      - id: child_image_url
        required: true
        type: string
        maps_to: ".dd-your-component__image img[src]"

      - id: child_image_alt
        required: true
        type: string
        maps_to: ".dd-your-component__image img[alt]"

      - id: child_title
        required: true
        type: string
        maps_to: ".dd-your-component__title"

      - id: child_subtitle
        required: true
        type: string
        maps_to: ".dd-your-component__subtitle"

      - id: child_copy
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

      - id: child_link_url
        required: false
        type: string
        maps_to: ".dd-your-component__link a[href]"

      - id: child_link_target
        required: false
        type: enum
        options: ["_self", "_blank"]
        default: "_self"
        maps_to: ".dd-your-component__link a[target]"

      - id: child_link_label
        required: false
        type: string
        maps_to: ".dd-your-component__link a"

edit_ui:
  tab_order:
    # parent edit order
    - parent_type
    - parent_class
    - parent_data_aos
    - parent_width

    # child edit order (used when editing an item row)
    - items[].child_image_url
    - items[].child_image_alt
    - items[].child_title
    - items[].child_subtitle
    - items[].child_copy
    - items[].child_link_url
    - items[].child_link_target
    - items[].child_link_label

  navigation_tree:
    parent_row: "dd-your-component"
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
      - parent_type
      - parent_class
      - parent_data_aos
      - parent_width
    item_edit_modes:
      - items[].child_image_url
      - items[].child_image_alt
      - items[].child_title
      - items[].child_subtitle
      - items[].child_copy
      - items[].child_link_url
      - items[].child_link_target
      - items[].child_link_label
    scope_rule: "when editing an items[] row, parent fields are not editable; when editing parent row, item fields are not editable"
    hide_when_editing_parent_or_child:
      - column.id
      - column.width_class

blueprint:
  label: "dd-your-component"
  show_fields:
    - "items[active].child_title"
---

## HTML Template

```html
<div class="dd-your-component [parent_type] [parent_class]">
  <div class="dd-your-component__items dd-g">
    <!-- repeat: items -->
    <div class="dd-your-component__item l-box [parent_width]" data-aos="[parent_data_aos]">
      <div class="dd-your-component__body dd-g">
        <div class="dd-your-component__image">
          <img src="[child_image_url]" alt="[child_image_alt]" class="dd-img" loading="lazy">
        </div>
        <div class="dd-your-component__copy l-box">
          <div class="dd-your-component__title"><h3>[child_title]</h3></div>
          <div class="dd-your-component__subtitle"><strong>[child_subtitle]</strong></div>
          <p>[child_copy]</p>
          <div class="dd-your-component__links dd-g">
            <div class="dd-your-component__link">
              <a href="[child_link_url]" target="[child_link_target]" class="dd-button -primary">[child_link_label]</a>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</div>
```

## Conditional Markup

- render `.dd-your-component__links` only when both `child_link_url` and `child_link_label` are non-empty
- when `child_link_target` is empty, default to `_self`

