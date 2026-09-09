---
component: dd-image
version: 1
node_scope: section_item   # one of: page_node | section_item

insert:
  defaults:
    sal: "fade"
    parent_image_url: "https://dummyimage.com/256x256/000/fff"
    parent_image_url_dark: ""
    parent_image_alt: "Image alt text"
    parent_link_url: "/path"
    parent_link_target: "_self"

fields:
  - id: sal
    required: true
    type: enum
    options: ["fade","slide-up","slide-down","slide-left","slide-right","zoom-in","zoom-out","flip-up","flip-down","flip-left","flip-right"]
    default: "fade"
    maps_to: ".dd-image[data-sal]"

  - id: parent_image_url
    required: true
    type: string
    maps_to: ".dd-image img[src]"

  - id: parent_image_url_dark
    required: false
    type: string
    maps_to: ".dd-image source[srcset]"

  - id: parent_image_alt
    required: true
    type: string
    maps_to: ".dd-image a[title], .dd-image img[alt]"

  - id: parent_link_url
    required: false
    type: string
    maps_to: ".dd-image a[href]"
    
  - id: parent_link_target
    required: false
    type: enum
    options: ["_self", "_blank"]
    default: "_self"
    maps_to: ".dd-image a[target]"

edit_ui:
  tab_order:
    - sal
    - parent_image_url
    - parent_image_url_dark
    - parent_image_alt
    - parent_link_url
    - parent_link_target

  enter_behavior:
    parent_row: "start component field editing"

  modal_fields:
    parent_edit_modes:
      - sal
      - parent_image_url
      - parent_image_url_dark
      - parent_image_alt
      - parent_link_url
      - parent_link_target
    hide_when_editing_component:
      - column.id
      - column.width_class

blueprint:
  label: "dd-image"
  show_fields:
    - "parent_image_alt"
---

## HTML Template

```html
<div class="dd-image" data-sal="[sal]">
  <!-- if parent_link_url -->
  <a href="[parent_link_url]" target="[parent_link_target]" title="[parent_image_alt]">
    <picture>
      <!-- if parent_image_url_dark --><source srcset="[parent_image_url_dark]" media="(prefers-color-scheme: dark)"><!-- endif -->
      <img src="[parent_image_url]" alt="[parent_image_alt]" class="dd-img" loading="lazy" />
    </picture>
  </a>
  <!-- else -->
  <picture>
    <!-- if parent_image_url_dark --><source srcset="[parent_image_url_dark]" media="(prefers-color-scheme: dark)"><!-- endif -->
    <img src="[parent_image_url]" alt="[parent_image_alt]" class="dd-img" loading="lazy" />
  </picture>
  <!-- endif -->
</div>
```

## Conditional Markup

- `<a>` wrapper renders only when `parent_link_url` is non-empty
- when `parent_link_target` is empty, default to `_self`
- `title` attribute on `<a>` uses `parent_image_alt`
- image always renders inside `<picture>`
- `<source media="(prefers-color-scheme: dark)">` renders only when `parent_image_url_dark` is non-empty

## Validation Rules

- `parent_image_url` required and must be a valid URL (`/`, `#`, `http://`, `https://`)
- `parent_image_url_dark` optional; when provided must be a valid URL
- `parent_image_alt` required and non-empty
- when `parent_link_url` is provided, it must be a valid URL
- `parent_link_target` optional; defaults to `_self`
