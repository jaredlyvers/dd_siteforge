---
component: dd-alert
version: 1
node_scope: section_item   # one of: page_node | section_item
insert:
  defaults:
    alert_type: "-default"
    alert_class: "-default"
    alert_data_aos: "fade-in"
    alert_title: "Alert Title"
    alert_copy: "Alert content"
fields:
  - id: alert_type
    required: true
    type: enum
    options: ["-default", "-info -minor", "-warning -moderate -serious", "-error -critical", "-success"]
    default: "-default"
    maps_to: ".dd-alert class token"
  - id: alert_class
    required: true
    type: enum
    options: ["-default", "-compact"]
    default: "-default"
    maps_to: ".dd-alert class token"
  - id: alert_data_aos
    required: true
    type: enum
    options: ["fade-in","fade-up","fade-right","fade-down","fade-left","zoom-in","zoom-in-up","zoom-in-down"]
    default: "fade-in"
    maps_to: ".dd-alert[data-aos]"
  - id: alert_title
    required: true
    type: string
    maps_to: ".dd-alert__title"
  - id: alert_copy
    required: true
    type: string
    maps_to: ".dd-alert__copy p"
edit_ui:
  tab_order:
    - alert_type
    - alert_class
    - alert_data_aos
    - alert_title
    - alert_copy
  enter_behavior:
    parent_row: "start dd-alert field editing"
  modal_fields:
    parent_edit_modes:
      - alert_type
      - alert_class
      - alert_data_aos
      - alert_title
      - alert_copy
    scope_rule: "dd-alert is a single component with no child rows; all editable fields are available on parent edit"
    hide_when_editing_alert:
      - column.id
      - column.width_class
blueprint:
  label: "dd-alert"
  show_fields:
    - alert_type
    - alert_class
    - alert_title
---

## HTML Template

```html
<div class="dd-alert [alert_type] [alert_class]" role="alert" data-aos="[alert_data_aos]" data-aos-duration="1000" data-aos-easing="linear" data-aos-anchor-placement="center-bottom"
data-aos-delay="100">
  <div class="dd-alert__content dd-g">
    <div class="dd-u-1-1">
      <div class="l-box">
        <div class="dd-alert__title">
          [alert_title]
        </div>
        <div class="dd-alert__copy">
          <p>[alert_copy]</p>
        </div>
      </div>
    </div>
  </div>
</div>
```
