---
component: dd-banner
version: 1
node_scope: section_item   # one of: page_node | section_item

insert:
  defaults:
    parent_class: "-bg-center-center"
    parent_data_aos: "fade-in"
    parent_image_url: "https://dummyimage.com/1920x1080/000/fff"
    parent_image_alt: "Banner alt text"

fields:
  - id: parent_class
    required: true
    type: enum
    options: ["-bg-top-left", "-bg-top-center", "-bg-top-right", "-bg-center-left", "-bg-center-center", "-bg-center-right", "-bg-bottom-left", "-bg-bottom-center", "-bg-bottom-right"]
    default: "-bg-center-center"
    maps_to: ".dd-banner class token"

  - id: parent_data_aos
    required: true
    type: enum
    options: ["fade-in","fade-up","fade-right","fade-down","fade-left","zoom-in","zoom-in-up","zoom-in-down"]
    default: "fade-in"
    maps_to: ".dd-banner[data-aos]"

  - id: parent_image_url
    required: true
    type: string
    maps_to: ".dd-banner__image img[src]"

  - id: parent_image_alt
    required: true
    type: string
    maps_to: ".dd-banner__image img[alt]"

edit_ui:
  tab_order:
    - parent_class
    - parent_data_aos
    - parent_image_url
    - parent_image_alt

  enter_behavior:
    parent_row: "start component field editing"

  modal_fields:
    parent_edit_modes:
      - parent_class
      - parent_data_aos
      - parent_image_url
      - parent_image_alt
    hide_when_editing_component:
      - column.id
      - column.width_class

blueprint:
  label: "dd-banner"
  show_fields:
    - "parent_image_alt"
---

## HTML Template

```html
<div class="dd-banner [parent_class]" data-aos="[parent_data_aos]" data-aos-duration="1000" data-aos-easing="linear" data-aos-anchor-placement="center-bottom" data-aos-delay="100" style="background-image: url([banner_image_url]);">
  <div class="dd-banner__image">
    <img src="[parent_image_url]" class="dd-img" alt="[parent_image_alt]" loading="lazy" />
  </div>
</div>
```

## Conditional Markup

- none (this variant intentionally has no optional link fields)
