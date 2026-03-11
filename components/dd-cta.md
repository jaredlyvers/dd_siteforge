---
component: dd-cta
version: 1
node_scope: section_item   # one of: page_node | section_item

insert:
  defaults:
    parent_class: "-top-left"
    parent_image_url: "https://dummyimage.com/1920x1080/000000/fff"
    parent_image_alt: "Image alt"
    parent_data_aos: "fade-in"
    parent_title: "Title"
    parent_subtitle: "Subtitle"
    parent_copy: "Copy"
    parent_link_url: "/path"
    parent_link_target: "_self"
    parent_link_label: "Learn More"

fields:
  - id: parent_class
    required: true
    type: enum
    options: ["-top-left", "-top-center", "-top-right", "-center-left", "-center-center", "-center-right", "-bottom-left", "-bottom-center", "-bottom-right"]
    default: "-top-left"
    maps_to: ".dd-cta class token"
    
  - id: parent_image_url
    required: true
    type: string
    default: "https://dummyimage.com/1920x1080/000000/fff"
    maps_to: ".dd-cta__image img[src], .dd-cta__image[style background-image]"
    
  - id: parent_image_alt
    required: true
    type: string
    default: "Image alt"
    maps_to: ".dd-cta__image img[alt]"

  - id: parent_data_aos
    required: true
    type: enum
    options: ["fade-in","fade-up","fade-right","fade-down","fade-left","zoom-in","zoom-in-up","zoom-in-down"]
    default: "fade-in"
    maps_to: ".dd-cta__content[data-aos]"

  - id: parent_title
    required: true
    type: string
    maps_to: ".dd-cta__title"

  - id: parent_subtitle
    required: true
    type: string
    maps_to: ".dd-cta__subtitle"

  - id: parent_copy
    required: true
    type: string
    maps_to: ".dd-cta__copy"
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

  - id: parent_link_url
    required: false
    type: string
    maps_to: ".dd-cta__link a[href]"

  - id: parent_link_target
    required: false
    type: enum
    options: ["_self", "_blank"]
    default: "_self"
    maps_to: ".dd-cta__link a[target]"

  - id: parent_link_label
    required: false
    type: string
    maps_to: ".dd-cta__link a"

edit_ui:
  tab_order:
    - parent_class
    - parent_image_url
    - parent_image_alt
    - parent_data_aos
    - parent_title
    - parent_subtitle
    - parent_copy
    - parent_link_url
    - parent_link_target
    - parent_link_label

  enter_behavior:
    parent_row: "start component field editing"

  modal_fields:
    parent_edit_modes:
      - parent_class
      - parent_image_url
      - parent_image_alt
      - parent_data_aos
      - parent_title
      - parent_subtitle
      - parent_copy
      - parent_link_url
      - parent_link_target
      - parent_link_label
    hide_when_editing_component:
      - column.id
      - column.width_class

blueprint:
  label: "dd-cta"
  show_fields:
    - "parent_title"
---

## HTML Template

```html
<div class="dd-cta [parent_class]">
  <div class="dd-cta__image" style="background-image: url([parent_image_url]);">
    <picture>
      <img src="[parent_image_url]" class="dd-img" alt="[parent_image_alt]" />
    </picture>
  </div>
  <div class="dd-cta__content dd-g" data-aos="[parent_data_aos]" data-aos-duration="1000" data-aos-easing="linear" data-aos-anchor-placement="center-center" data-aos-delay="100">
    <div class="dd-cta__copy dd-u-1-1 dd-u-md-12-24">
      <div class="dd-cta__title">
        <h2>[parent_title]</h2>
      </div>
      <div class="dd-cta__subtitle">
        <strong>[parent_subtitle]</strong>
      </div>
      <p>[parent_copy]</p>
      <div class="dd-cta__links dd-g -x-center">
        <div class="dd-cta__link">
          <a href="[parent_link_url]" class="dd-button -primary" target="[parent_link_target]">[parent_link_label]</a>
        </div>
      </div>
    </div>
  </div>
</div>
```

## Conditional Markup

- render `.dd-cta__links` only when both `parent_link_url` and `parent_link_label` are non-empty
- when `parent_link_target` is empty, default to `_self`
