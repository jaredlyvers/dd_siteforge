---
component: dd-milestones
version: 1
node_scope: section_item   # one of: page_node | section_item

insert:
  defaults:
    # parent fields
    parent_data_aos: "fade-in"
    parent_width: "dd-u-1-1 dd-u-md-12-24"

    # required children collection
    items:
      - child_percentage: "70"
        child_title: "Title"
        child_subtitle: "Subtitle"
        child_copy: "Copy"
        child_link_url: ""
        child_link_target: "_self"
        child_link_label: ""

fields:
  # ---------------------------
  # parent fields
  # ---------------------------
  - id: parent_data_aos
    required: true
    type: enum
    options: ["fade-in","fade-up","fade-right","fade-down","fade-left","zoom-in","zoom-in-up","zoom-in-down"]
    default: "fade-in"
    maps_to: ".dd-milestones__item[data-aos]"

  - id: parent_width
    required: true
    type: string
    default: "dd-u-1-1 dd-u-md-12-24"
    maps_to: ".dd-milestones__item class token"

  # ---------------------------
  # child items[] fields
  # ---------------------------
  - id: items
    required: true
    type: array
    min_items: 1
    item_fields:

      - id: child_percentage
        required: true
        type: string
        maps_to: ".dd-milestones__percentage[data-number], .dd-milestones__percentage.number"

      - id: child_title
        required: true
        type: string
        maps_to: ".dd-milestones__title"

      - id: child_subtitle
        required: true
        type: string
        maps_to: ".dd-milestones__subtitle"

      - id: child_copy
        required: true
        type: string
        maps_to: ".dd-milestones__copy"
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
        maps_to: ".dd-milestones__link a[href]"

      - id: child_link_target
        required: false
        type: enum
        options: ["_self", "_blank"]
        default: "_self"
        maps_to: ".dd-milestones__link a[target]"

      - id: child_link_label
        required: false
        type: string
        maps_to: ".dd-milestones__link a"

edit_ui:
  tab_order:
    # parent edit order
    - parent_data_aos
    - parent_width

    # child edit order (used when editing an item row)
    - items[].child_percentage
    - items[].child_title
    - items[].child_subtitle
    - items[].child_copy
    - items[].child_link_url
    - items[].child_link_target
    - items[].child_link_label

  navigation_tree:
    parent_row: "dd-milestones"
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
    item_row: "start selected items[].child_percentage editing"

  modal_fields:
    parent_edit_modes:
      - parent_data_aos
      - parent_width
    item_edit_modes:
      - items[].child_percentage
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
  label: "dd-milestones"
  show_fields:
    - "items[active].child_percentage"
    - "items[active].child_title"
---

## HTML Template

```html
<div class="dd-milestones">
  <div class="dd-milestones__content">
    <div class="dd-milestones__items dd-g">
      <!-- repeat: items -->
      <div class="dd-milestones__item l-box [parent_width]" data-aos="[parent_data_aos]" data-aos-duration="1000" data-aos-easing="linear" data-aos-anchor-placement="center-center" data-aos-delay="100">
        <div class="dd-milestones__body l-box">
          <div class="dd-milestones__percentage" data-number="[child_percentage]"><span class="number">[child_percentage]</span>%</div>
          <div>
            <div class="dd-milestones__title"><h2>[child_title]</h2></div>
            <div class="dd-milestones__subtitle"><strong>[child_subtitle]</strong></div>
            <div class="dd-milestones__copy">[child_copy]</div>
            <div class="dd-milestones__links">
              <div class="dd-milestones__link">
                <a href="[child_link_url]" target="[child_link_target]" class="dd-button -primary">[child_link_label]</a>
              </div>
            </div>
          </div>
        </div>
      </div>

    </div>
  </div>
</div>
```

## Conditional Markup

- render `.dd-milestones__links` only when both `child_link_url` and `child_link_label` are non-empty
- when `child_link_target` is empty, default to `_self`
