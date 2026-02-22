---
component: dd-alternating
version: 1
node_scope: section_item   # one of: page_node | section_item
insert:
  defaults:
    alternating_type: "-default"
    alternating_class: "-default"
    alternating_data_aos: "fade-in"
    items:
      - alternating_image: "https://dummyimage.com/600x400/000/fff"
        alternating_image_alt: "Alternating image"
        alternating_title: "Alternating Item"
        alternating_copy: "Alternating content"
fields:
  - id: alternating_type
    required: true
    type: enum
    options: ["-default", "-reverse", "-no-alternate"]
    default: "-default"
    maps_to: ".dd-alternating class token"
  - id: alternating_class
    required: true
    type: string
    default: "-default"
    maps_to: ".dd-alternating class token"
  - id: alternating_data_aos
    required: true
    type: enum
    options: ["fade-in","fade-up","fade-right","fade-down","fade-left","zoom-in","zoom-in-up","zoom-in-down"]
    default: "fade-in"
    maps_to: ".dd-alternating__image[data-aos], .dd-alternating__copy[data-aos]"
  - id: items
    required: true
    type: array
    min_items: 1
    item_fields:
      - id: alternating_image
        required: true
        type: string
        maps_to: ".dd-alternating__image img[src]"
      - id: alternating_image_alt
        required: true
        type: string
        maps_to: ".dd-alternating__image img[alt]"
      - id: alternating_title
        required: true
        type: string
      - id: alternating_copy
        required: true
        type: string
edit_ui:
  tab_order:
    - alternating_type
    - alternating_class
    - alternating_data_aos
    - items[].alternating_image
    - items[].alternating_image_alt
    - items[].alternating_title
    - items[].alternating_copy
  navigation_tree:
    parent_row: "dd-alternating"
    child_rows: "items[]"
    item_row_label: "item {index}: alternating_title"
    collapse_expand_key: "Space"
  item_collection:
    add_item_key: "A"
    remove_item_key: "X"
    add_behavior: "insert after selected item row, otherwise append to end"
    min_items: 1
  enter_behavior:
    parent_row: "start dd-alternating field editing"
    item_row: "start selected items[].alternating_title editing"
  modal_fields:
    parent_edit_modes:
      - alternating_type
      - alternating_class
      - alternating_data_aos
    item_edit_modes:
      - items[].alternating_image
      - items[].alternating_image_alt
      - items[].alternating_title
      - items[].alternating_copy
    scope_rule: "when editing an items[] row, parent alternating fields are not editable; when editing parent row, item fields are not editable"
    hide_when_editing_alternating:
      - column.id
      - column.width_class
      - items[active].alternating_image
      - items[active].alternating_image_alt
      - items[active].alternating_title
      - items[active].alternating_copy
blueprint:
  label: "dd-alternating"
  show_fields:
    - alternating_type
    - alternating_class
    - "items[active].alternating_title"
    - "items[active].alternating_image"
---

## HTML Template

```html
<div class="dd-alternating [alternating_type] [alternating_class]" role="region">
  <div class="dd-alternating__items dd-g">
    <!-- repeat: items -->
    <div class="dd-alternating__item dd-u-1-1">
      <div class="dd-alternating__content dd-g">
        <div class="dd-alternating__image dd-u-1-1 dd-u-sm-1-1 dd-u-md-1-1 dd-u-lg-12-24" data-aos="[alternating_data_aos]" data-aos-duration="1000" data-aos-easing="linear" data-aos-anchor-placement="center-bottom" data-aos-delay="100">
          <picture>
            <img src="[alternating_image]" class="dd-img" alt="[alternating_image_alt]" />
          </picture>
        </div>
        <div class="dd-alternating__copy l-box dd-u-1-1 dd-u-sm-1-1 dd-u-md-1-1 dd-u-lg-12-24" data-aos="[alternating_data_aos]" data-aos-duration="1000" data-aos-easing="linear" data-aos-anchor-placement="center-bottom" data-aos-delay="100">
          <div class="dd-alternating__title">
            <h2>[alternating_title]</h2>
          </div>
          <div class="dd-alternating__body">
            [alternating_copy]
          </div>
        </div>
      </div>
    </div>
  </div>
</div>
```
