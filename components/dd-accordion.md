---
component: dd-accordion
version: 1
node_scope: section_item   # one of: page_node | section_item

insert:
  defaults:
    # parent fields
    parent_type: "-default"
    parent_class: "-primary"
    parent_data_aos: "fade-in"
    parent_group_name: "group1"

    # required children collection
    items:
        child_title: "Title"
        child_copy: "Copy"

fields:
  # ---------------------------
  # parent fields
  # ---------------------------
  - id: parent_type
    required: true
    type: enum
    options: ["-default", "-faq"]
    default: "-default"
    maps_to: ".dd-accordion class token"
    affects:
      - "if '-faq', include FAQ ld+json script"

  - id: parent_class
    required: true
    type: enum
    options: ["-borderless", "-compact", "-primary", "-secondary", "-tertiary"]
    default: "-primary"
    maps_to: ".dd-accordion class token"

  - id: parent_data_aos
    required: true
    type: enum
    options: ["fade-in","fade-up","fade-right","fade-down","fade-left","zoom-in","zoom-in-up","zoom-in-down"]
    default: "fade-in"
    maps_to: ".dd-accordion__item[data-aos]"

  - id: parent_group_name
    required: true
    type: string
    maps_to: ".dd-accordion__item class token"

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
        maps_to: ".dd-accordion__title"

      - id: child_copy
        required: true
        type: string
        maps_to: ".dd-accordion__copy"
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
    - parent_group_name

    # child edit order (used when editing an item row)
    - items[].child_title
    - items[].child_copy

  navigation_tree:
    parent_row: "dd-accordion"
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
    item_row: "start selected items[].child_title editing"

  modal_fields:
    parent_edit_modes:
      - parent_type
      - parent_class
      - parent_data_aos
      - parent_group_name
    item_edit_modes:
      - items[].child_title
      - items[].child_copy
    scope_rule: "when editing an items[] row, parent fields are not editable; when editing parent row, item fields are not editable"
    hide_when_editing_parent_or_child:
      - column.id
      - column.width_class

blueprint:
  label: "dd-accordion"
  show_fields:
    - "items[active].child_title"
---

## HTML Template

```html
<div class="dd-accordion [parent_type] [parent_class]">
  <div class="dd-accordion__items dd-g">
    <!-- repeat: items -->
    <details name="[group_name]" class="dd-accordion__item l-box" data-aos="[parent_data_aos]" data-aos-duration="1000" data-aos-easing="linear" data-aos-anchor-placement="center-bottom" data-aos-delay="100">
      <summary class="dd-accordion__header dd-g -y-center">
        <div class="dd-accordion__title dd-u-1-1"><h3>[child_title]</h3></div>
      </summary>
      <div class="dd-accordion__copy"><p>[child_copy]</p></div>
    </details>
  </div>
</div>
```

## Conditional Markup

- if accordion_type == "-faq":

<script type="application/ld+json">
  {
    "@context":"https://schema.org",
    "@type":"FAQPage",
    "mainEntity":[
      {
        "@type":"Question",
        "name":"[accordion_title]",
        "acceptedAnswer":{"@type":"Answer","text":"[accordion_copy]"}
      }
    ]
  }
</script>
```
