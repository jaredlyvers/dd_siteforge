---
component: dd-banner
version: 1
node_scope: page_node
insert:
  defaults:
    banner_class: "-bg-center-center"
    banner_data_aos: "fade-in"
    banner_image: "https://dummyimage.com/1920x1080/000/fff"
    banner_image_alt: "Banner alt text"
fields:
  - id: banner_image
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
    maps_to: ".dd-banner__content[data-aos]"
edit_ui:
  tab_order:
    - banner_data_aos
    - banner_image
    - banner_image_alt
blueprint:
  label: "dd-banner"
  show_fields:
    - banner_image
---

## HTML Template

```html
<div class="dd-banner [banner_class]" data-aos="[banner_data_aos]" data-aos-duration="1000" data-aos-easing="linear" data-aos-anchor-placement="center-bottom" data-aos-delay="100">
  <div class="dd-banner__image">
    <picture>
      <img src="[banner_image]" class="dd-img" alt="[banner_image_alt]" />
    </picture>
  </div>
</section>
```
