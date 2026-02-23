---
component: dd-banner
version: 1
node_scope: section_item
insert:
  defaults:
    banner_class: "-bg-center-center"
    banner_data_aos: "fade-in"
    banner_image_url: "https://dummyimage.com/1920x1080/000/fff"
    banner_image_alt: "Banner alt text"
fields:
  - id: banner_image_url
    required: true
    type: string
    maps_to: ".dd-banner__image img[src]"
  - id: banner_image_alt
    required: true
    type: string
    maps_to: ".dd-banner__image img[alt]"
  - id: banner_class
    required: true
    type: enum
    options: ["-bg-top-left", "-bg-top-center", "-bg-top-right", "-bg-center-left", "-bg-center-center", "-bg-center-right", "-bg-bottom-left", "-bg-bottom-center", "-bg-bottom-right"]
    default: "-bg-center-center"
    maps_to: ".dd-banner class token"
  - id: banner_data_aos
    required: true
    type: enum
    options: ["fade-in", "fade-up", "fade-right", "fade-down", "fade-left", "zoom-in", "zoom-in-up", "zoom-in-down"]
    default: "fade-in"
    maps_to: ".dd-banner[data-aos]"
edit_ui:
  tab_order:
    - banner_class
    - banner_data_aos
    - banner_image_url
    - banner_image_alt
  enter_behavior:
    parent_row: "start dd-banner field editing"
  modal_fields:
    parent_edit_modes:
      - banner_class
      - banner_data_aos
      - banner_image_url
      - banner_image_alt
    scope_rule: "dd-banner is a single component with no child rows; all editable fields are available on parent edit"
    hide_when_editing_banner:
      - column.id
      - column.width_class
blueprint:
  label: "dd-banner"
  show_fields:
    - banner_image_url
    - banner_image_alt
---

## HTML Template

```html
<div class="dd-banner [banner_class]" data-aos="[banner_data_aos]" data-aos-duration="1000" data-aos-easing="linear" data-aos-anchor-placement="center-bottom" data-aos-delay="100" style="background-image: url([banner_image_url]);">
  <div class="dd-banner__image">
    <picture>
      <img src="[banner_image_url]" class="dd-img" alt="[banner_image_alt]" />
    </picture>
  </div>
</div>
```
