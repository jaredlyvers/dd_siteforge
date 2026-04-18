---
component: dd-alternating
version: 1
node_scope: section_item   # one of: page_node | section_item

insert:
  defaults:
    # parent fields
    parent_type: "-default"
    parent_class: "-default"
    parent_data_aos: "fade-in"

    # required children collection
    items:
      - child_image_url: "https://dummyimage.com/720x720/000/fff"
        child_image_alt: "Image alt text"
        child_title: "Title"
        child_subtitle: "Subtitle"
        child_copy: "Copy"

fields:
  # ---------------------------
  # parent fields
  # ---------------------------
  - id: parent_type
    required: true
    type: enum
    options: ["-default", "-reverse", "-no-alternate"]
    default: "-default"
    maps_to: ".dd-alternating class token"

  - id: parent_class
    required: true
    type: enum
    options: ["-primary", "-secondary"]
    default: "-primary"
    maps_to: ".dd-alternating class token"

  - id: parent_data_aos
    required: true
    type: enum
    options: ["fade-in","fade-up","fade-right","fade-down","fade-left","zoom-in","zoom-in-up","zoom-in-down"]
    default: "fade-in"
    maps_to: ".dd-alternating[data-aos] OR child[data-aos]"

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
        maps_to: ".dd-alternating__image img[src]"

      - id: child_image_alt
        required: true
        type: string
        maps_to: ".dd-alternating__image img[alt]"

      - id: child_title
        required: true
        type: string
        maps_to: ".dd-alternating__title"

      - id: child_subtitle
        required: true
        type: string
        maps_to: ".dd-alternating__subtitle"

      - id: child_copy
        required: true
        type: string
        maps_to: ".dd-alternating__copy"
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
    # parent edit order
    - parent_type
    - parent_class
    - parent_data_aos

    # child edit order (used when editing an item row)
    - items[].child_image_url
    - items[].child_image_alt
    - items[].child_title
    - items[].child_subtitle
    - items[].child_copy

  navigation_tree:
    parent_row: "dd-alternating"
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
    item_edit_modes:
      - items[].child_image_url
      - items[].child_image_alt
      - items[].child_title
      - items[].child_subtitle
      - items[].child_copy
    scope_rule: "when editing an items[] row, parent fields are not editable; when editing parent row, item fields are not editable"
    hide_when_editing_parent_or_child:
      - column.id
      - column.width_class

blueprint:
  label: "dd-alternating"
  show_fields:
    - "items[active].child_title"
---

## HTML Template

```html
<div class="dd-alternating [parent_type] [parent_class]" role="region">
  <div class="dd-alternating__items dd-g">
    <!-- repeat: items -->
    <div class="dd-alternating__item dd-u-1-1">
      <div class="dd-alternating__body dd-g">
        <div class="dd-alternating__image dd-u-1-1 dd-u-sm-1-1 dd-u-md-1-1 dd-u-lg-12-24" data-aos="[parent_data_aos]" data-aos-duration="1000" data-aos-easing="linear" data-aos-anchor-placement="center-bottom" data-aos-delay="100">
          <img src="[child_image_url]" alt="[child_image_alt]" class="dd-img" loading="lazy">
        </div>
        <div class="dd-alternating__copy l-box dd-u-1-1 dd-u-sm-1-1 dd-u-md-1-1 dd-u-lg-12-24" data-aos="[parent_data_aos]" data-aos-duration="1000" data-aos-easing="linear" data-aos-anchor-placement="center-bottom" data-aos-delay="100">
          <div class="dd-alternating__title"><h2>[child_title]</h2></div>
          <div class="dd-alternating__subtitle"><strong>[child_subtitle]</strong></div>
          <div class="dd-alternating__body">
            <p>[child_copy]</p>
          </div>
        </div>
      </div>
    </div>
  </div>
</div>
```

## Conditional Markup

- none (this variant intentionally has no optional link fields)
