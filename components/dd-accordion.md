---
component: dd-accordion
version: 1
node_scope: section_item   # one of: page_node | section_item
insert:
  defaults:
    accordion_type: "-default"
    accordion_class: "-primary"
    accordion_data_aos: "fade-in"
    group_name: "group1"
    items:
      - accordion_title: "Accordion Item"
      - accordion_copy: "Accordion content"
fields:
  - id: accordion_type
    required: true
    type: enum
    options: ["-default", "-faq"]
    default: "-default"
    maps_to: ".dd-accordion class token"
    affects:
      - "if '-faq', include FAQ ld+json script"
  - id: accordion_class
    required: true
    type: enum
    options: ["-borderless", "-compact", "-primary", "-secondary", "-tertiary"]
    default: "-primary"
    maps_to: ".dd-accordion class token"
  - id: accordion_data_aos
    required: true
    type: enum
    options: ["fade-in","fade-up","fade-right","fade-down","fade-left","zoom-in","zoom-in-up","zoom-in-down"]
    default: "fade-in"
    maps_to: ".dd-accordion[data-aos]"
  - id: group_name
    required: true
    type: string
    maps_to: "details[name]"
  - id: items
    required: true
    type: array
    min_items: 1
    item_fields:
      - id: accordion_title
        required: true
        type: string
      - id: accordion_copy
        required: true
        type: string
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
    - accordion_type
    - accordion_class
    - accordion_data_aos
    - group_name
    - items[].accordion_title
    - items[].accordion_copy
  navigation_tree:
    parent_row: "dd-accordion"
    child_rows: "items[]"
    item_row_label: "item {index}: accordion_title"
    collapse_expand_key: "Space"
  item_collection:
    add_item_key: "A"
    remove_item_key: "X"
    add_behavior: "insert after selected item row, otherwise append to end"
    min_items: 1
  enter_behavior:
    parent_row: "start dd-accordion field editing"
    item_row: "start selected items[].accordion_title editing"
  modal_fields:
    parent_edit_modes:
      - accordion_type
      - accordion_class
      - accordion_data_aos
      - group_name
    item_edit_modes:
      - items[].accordion_title
      - items[].accordion_copy
    scope_rule: "when editing an items[] row, parent accordion fields are not editable; when editing parent row, item fields are not editable"
    hide_when_editing_accordion:
      - column.id
      - column.width_class
      - items[active].accordion_title
      - items[active].accordion_copy
blueprint:
  label: "dd-accordion"
  show_fields:
    - "items[active].accordion_title"
---

## HTML Template

```html
<div class="dd-accordion [accordion_type] [accordion_class]" data-aos="[accordion_data_aos]" data-aos-duration="1000" data-aos-easing="linear" data-aos-anchor-placement="center-bottom" data-aos-delay="100">
  <div class="dd-accordion__items">
    <!-- repeat: items -->
    <details name="[group_name]" class="dd-accordion__item">
      <summary class="dd-accordion__header dd-g -y-center">
        <div class="dd-accordion__title dd-u-1-1">[accordion_title]</div>
      </summary>
      <div class="dd-accordion__copy"><p>[accordion_copy]</p></div>
    </details>
  </div>
</div>

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
