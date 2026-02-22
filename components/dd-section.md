---
component: dd-section
version: 1
node_scope: page_node
insert:
  defaults:
    id: "section-1"
    section_title: ""
    section_class: "-full-contained"
    item_box_class: "l-box"
    custom_css: ""
    items:
      - item_id: "column-1"
        width_class: "dd-u-1-1"
        components: []
fields:
  - id: id
    required: true
    type: string
    maps_to: "section[id reference in builder only]"
  - id: section_title
    required: false
    type: string
    maps_to: ".dd-section__title"
  - id: section_class
    required: true
    type: enum
    options: ["-contained", "-contained-md", "-contained-lg", "-contained-xl", "-contained-xxl", "-full-full", "-full-contained", "-full-contained-md", "-full-contained-lg", "-full-contained-xl", "-full-contained-xxl"]
    default: "-full-contained"
    maps_to: ".dd-section class token"
  - id: item_box_class
    required: true
    type: enum
    options: ["l-box", "ll-box"]
    default: "l-box"
    maps_to: ".dd-section__item class token"
  - id: custom_css
    required: false
    type: string
    default: ""
    maps_to: ".dd-section class token"
  - id: items
    required: true
    type: array
    min_items: 1
    item_fields:
      - id: item_id
        required: true
        type: string
        maps_to: "builder identifier"
      - id: width_class
        required: true
        type: string
        maps_to: ".dd-section__item width classes (dd-u-*)"
      - id: components
        required: false
        type: array
        maps_to: "nested section components"
edit_ui:
  tab_order:
    - id
    - section_title
    - section_class
    - custom_css
  row_editing:
    section_item:
      - item_id
      - width_class
blueprint:
  label: "dd-section"
  show_fields:
    - id
    - section_title
    - section_class
    - custom_css
    - items[].item_id
    - items[].width_class
---

## HTML Template

```html
<section class="dd-section [section_class]">
  <div class="dd-section__content">
    <!-- if [section_title] --><div class="dd-section__title l-box">[section_title]</div><!-- endif -->
    <div class="dd-section__items dd-g">
      <!-- repeat: items -->
      <div class="dd-section__item [width_class] [item_box_class]">
        <!-- nested components render here -->
      </div>
    </div>
  </div>
</section>
```

## Width Class Guidance

Use `dd-u-*` classes on each `dd-section__item` to control layout per breakpoint, for example:

- `dd-u-1-1`
- `dd-u-sm-1-*`
- `dd-u-md-1-*`
- `dd-u-lg-1-*`
- `dd-u-xl-1-*`
- `dd-u-xxl-1-*`
