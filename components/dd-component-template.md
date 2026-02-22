---
component: dd-component
version: 1
node_scope: section_item   # one of: page_node | section_item
insert:
  defaults:
    component_type: "-default"
    component_class: "-primary"
    component_data_aos: "fade-in"
    group_name: "group1"
    items:
      - component_title: "component Item"
        component_copy: "component content"
fields:
  - id: component_type
    required: true
    type: enum
    options: ["-default", "-faq"]
    maps_to: ".dd-component class token"
    affects:
      - "if '-faq', include FAQ ld+json script"
  - id: component_class
    required: true
    type: enum
    options: ["-borderless", "-compact", "-primary", "-secondary", "-tertiary"]
    maps_to: ".dd-component class token"
  - id: component_data_aos
    required: true
    type: enum
    options: ["fade-in","fade-up","fade-right","fade-down","fade-left","zoom-in","zoom-in-up","zoom-in-down"]
    maps_to: ".dd-component[data-aos]"
  - id: group_name
    required: true
    type: string
    maps_to: "details[name]"
  - id: items
    required: true
    type: array
    min_items: 1
    item_fields:
      - id: component_title
        required: true
        type: string
      - id: component_copy
        required: true
        type: string
edit_ui:
  tab_order:
    - component_type
    - component_class
    - component_data_aos
    - group_name
    - items[].component_title
    - items[].component_copy
blueprint:
  label: "dd-component"
  show_fields:
    - "items[active].component_title"
---

## HTML Template

```html
<div class="dd-component" role="component" data-aos="fade-in">
  <div class="dd-component__content dd-g">
    <div class="dd-u-1-1">
      <div class="l-box">
        <div class="dd-component__heading">
          Default component
        </div>
        <div class="dd-component__copy">
          Lorem ipsum dolor sit amet, <a href="#">link</a>
        </div>
      </div>
    </div>
  </div>
</div>

## Conditional Markup

- if component_type == "-faq":

<script type="application/ld+json">
  {
    "@context":"https://schema.org",
    "@type":"FAQPage",
    "mainEntity":[
      {
        "@type":"Question",
        "name":"[component_title]",
        "acceptedAnswer":{"@type":"Answer","text":"[component_copy]"}
      }
    ]
  }
</script>