---
component: dd-alert
version: 1
node_scope: section_item   # one of: page_node | section_item
insert:
  defaults:
    alert_type: "-default"
    alert_class: "-default"
    alert_data_aos: "fade-in"
    items:
      - alert_title: "component Item"
        alert_copy: "component content"
        alert_link_target: "_self"
        alert_link_label: "Learn More"
fields:
  - id: alert_type
    required: true
    type: enum
    options: ["-default", "-info -minor" , "-warning -moderate -serious" , "-error -critical" , "-succcess"]
    maps_to: ".dd-alert class token"
  - id: alert_class
    required: true
    type: enum
    options: ["-compact"]
    maps_to: ".dd-alert class token"
  - id: alert_data_aos
    required: true
    type: enum
    options: ["fade-in","fade-up","fade-right","fade-down","fade-left","zoom-in","zoom-in-up","zoom-in-down"]
    maps_to: ".dd-alert[data-aos]"
  - id: items
    required: true
    type: array
    min_items: 1
    max_items: 1
    item_fields:
      - id: alert_title
        required: true
        type: string
      - id: alert_copy
        required: true
        type: string
      - id: alert_link_url
        required: false
        type: string
      - id: alert_link_target
        required: false
        type: enum
        options: ["_self", "_blank"]
      - id: alert_link_label
        type: string
edit_ui:
  tab_order:
    - alert_type
    - alert_class
    - alert_data_aos
    - items[].alert_title
    - items[].alert_copy    
    - items[].alert_link_url
    - items[].alert_link_target
    - items[].alert_link_label
blueprint:
  label: "dd-alert"
  show_fields:
    - "items[active].alert_title"
---

## HTML Template

```html
<div class="dd-alert  [alert_type] [alert_class]" role="alert" data-aos="[accordion_data_aos]" data-aos-duration="1000" data-aos-easing="linear" data-aos-anchor-placement="center-bottom"
data-aos-delay="100">
  <div class="dd-alert__content dd-g">
    <div class="dd-u-1-1">
      <div class="l-box">
        <div class="dd-alert__title">
          [alert_title]
        </div>
        <div class="dd-alert__copy">
          <p>[alert_copy]</p>
          <!-- if [alert_link_url] && [alert_link_label] -->
          <p><a href="[alert_link_url]" target="[alert_link_target]">[alert_link_label]</a></p>
        </div>
      </div>
    </div>
  </div>
</div>
```
