---
component: dd-filmstrip
version: 1
node_scope: section_item   # one of: page_node | section_item
render_behavior: "items[] is rendered twice using identical data; second loop is aria-hidden clone"

insert:
  defaults:
    # parent fields
    parent_type: "-default"
    parent_data_aos: "fade-in"

    # required children collection
    items:
      - child_image_url: "https://dummyimage.com/256x256/000/fff"
        child_image_alt: "Image alt text"
        child_title: "Title"

fields:
  # ---------------------------
  # parent fields
  # ---------------------------
  - id: parent_type
    required: true
    type: enum
    options: ["-default", "-reverse"]
    default: "-default"
    maps_to: ".dd-filmstrip class token"

  - id: parent_data_aos
    required: true
    type: enum
    options: ["fade-in","fade-up","fade-right","fade-down","fade-left","zoom-in","zoom-in-up","zoom-in-down"]
    default: "fade-in"
    maps_to: ".dd-filmstrip[data-aos]"

  # ---------------------------
  # child items[] fields
  # ---------------------------
  - id: items
    required: true
    type: array
    min_items: 1
    item_fields:
      - id: child_image_url
        required: true
        type: string
        maps_to: ".dd-filmstrip__image img[src]"

      - id: child_image_alt
        required: true
        type: string
        maps_to: ".dd-filmstrip__image img[alt]"

      - id: child_title
        required: true
        type: string
        maps_to: ".dd-filmstrip__title"

edit_ui:
  tab_order:
    # parent edit order
    - parent_type
    - parent_data_aos

    # child edit order (used when editing an item row)
    - items[].child_image_url
    - items[].child_image_alt
    - items[].child_title

  navigation_tree:
    parent_row: "dd-filmstrip"
    child_rows: "items[]"
    item_row_label: "item {index}: child_title"
    collapse_expand_key: "Space"

  item_collection:
    add_item_key: "A"
    remove_item_key: "X"
    add_behavior: "insert after selected item row, otherwise append to end"
    min_items: 1

  enter_behavior:
    parent_row: "start parent field editing"
    item_row: "start selected items[].child_image_url editing"

  modal_fields:
    parent_edit_modes:
      - parent_type
      - parent_data_aos
    item_edit_modes:
      - items[].child_image_url
      - items[].child_image_alt
      - items[].child_title
    scope_rule: "when editing an items[] row, parent fields are not editable; when editing parent row, item fields are not editable"
    hide_when_editing_parent_or_child:
      - column.id
      - column.width_class

blueprint:
  label: "dd-filmstrip"
  show_fields:
    - "items[active].child_title"
---

## HTML Template

```html
<div class="dd-filmstrip [parent_type]" data-aos="[parent_data_aos]" data-aos-duration="1000" data-aos-easing="linear" data-aos-anchor-placement="center-center" data-aos-delay="100">
  <ul class="dd-filmstrip__content">
    <!-- repeat: items -->
    <li>
      <img src="[child_image_url]" alt="[child_image_alt]" class="dd-img" loading="lazy">
      <figure class="dd-filmstrip__title">[child_title]</figure>
    </li>
  </ul>

  <ul aria-hidden="true" class="dd-filmstrip__content">
    <!-- repeat: items -->
    <li role="presentation">
      <img src="[child_image_url]" alt="[child_image_alt]" class="dd-img" loading="lazy">
      <figure class="dd-filmstrip__title">[child_title]</figure>
    </li>
  </ul>
</div>

```

## Conditional Markup

- none (this variant intentionally has no optional link fields)
- cloned second list should remain `aria-hidden="true"` and cloned `<li>` should use `role="presentation"`
