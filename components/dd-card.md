---
component: dd-card
version: 1
node_scope: section_item   # one of: page_node | section_item
insert:
  defaults:
    card_type: "-default"
    card_data_aos: "fade-in"
    card_width: "dd-u-1-1 dd-u-md-12-24 dd-u-lg-8-24"
    items:
      - card_image_url: "https://dummyimage.com/720x720/000/fff"
      - card_image_alt: "Image alt text"
      - card_title: "Title"
      - card_subtitle: "Subtitle"
      - card_copy: "Copy"
      - card_link_url: "/front"
      - card_link_target: "_self"
      - card_link_label: "Learn More"
fields:
  - id: card_type
    required: true
    type: enum
    options: ["-default", "-horizontal"]
    default: "-default"
    maps_to: ".dd-card class token"
  - id: card_data_aos
    required: true
    type: enum
    options: ["fade-in","fade-up","fade-right","fade-down","fade-left","zoom-in","zoom-in-up","zoom-in-down"]
    default: "fade-in"
    maps_to: ".dd-card__item[data-aos]"
  - id: items
    required: true
    type: array
    min_items: 1
  - id: card_width
    required: true
    type: string
    maps_to: ".dd-card__item class token"
    item_fields:
      - id: card_image_url
        required: true
        type: string
        maps_to: ".dd-card__image img[src]"
      - id: card_image_alt
        required: true
        type: string
        maps_to: ".dd-card__image img[alt]"
      - id: card_title
        required: true
        type: string
        maps_to: ".dd-card__title"
      - id: card_copy
        required: true
        type: string
        maps_to: ".dd-card__copy"
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
      - id: card_link_url
        required: true
        type: string
        maps_to: ".dd-card__link a[href]"
      - id: card_link_target
        required: true
        type: enum
        options: ["_self", "_blank"]
        maps_to: ".dd-card__link a[target]"
      - id: card_link_label
        required: true
        type: string
        maps_to: ".dd-card__link a"
edit_ui:
  tab_order:
    - card_type
    - card_class
    - card_data_aos
    - group_name
    - items[].card_title
    - items[].card_copy
  navigation_tree:
    parent_row: "dd-card"
    child_rows: "items[]"
    item_row_label: "item {index}: card_title"
    collapse_expand_key: "Space"
  item_collection:
    add_item_key: "A"
    remove_item_key: "X"
    add_behavior: "insert after selected item row, otherwise append to end"
    min_items: 1
  enter_behavior:
    parent_row: "start dd-card field editing"
    item_row: "start selected items[].card_title editing"
  modal_fields:
    parent_edit_modes:
      - card_type
      - card_class
      - card_data_aos
      - group_name
    item_edit_modes:
      - items[].card_title
      - items[].card_copy
    scope_rule: "when editing an items[] row, parent card fields are not editable; when editing parent row, item fields are not editable"
    hide_when_editing_card:
      - column.id
      - column.width_class
      - items[active].card_title
      - items[active].card_copy
blueprint:
  label: "dd-card"
  show_fields:
    - "items[active].card_title"
---

## HTML Template

```html
<div class="dd-card">
  <div class="dd-card__items dd-g"><!-- cards loop inside items -->
    <div class="dd-card__item l-box dd-u-1-1 dd-u-md-12-24" data-aos="fade-up">
      <div class="dd-card__body dd-g">
        <div class="dd-card__image">
          <img src="[card_image_url]" alt="[card_image_alt]" class="dd-image" loading="lazy">
        </div>
        <div class="dd-card__copy l-box">
          <div class="dd-card__title">
            <h3>[card_title]</h3>
          </div>
          <div class="dd-card__subtitle">
            <strong>[card_subtitle]</strong>
          </div>
          <p>[card_copy]</p>
          <div class="dd-card__links dd-g">
            <div class="dd-card__link">
              <a href="[card_link_url]" target="[card_link_target]" class="dd-button -primary">[card_link_label]</a>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</div>
```
