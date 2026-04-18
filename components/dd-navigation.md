---
component: dd-navigation
version: 2
node_scope: section_item   # one of: page_node | section_item

insert:
  defaults:
    # parent fields
    parent_type: "dd-header__navigation"
    parent_class: "-main-menu"
    parent_data_aos: "fade-in"
    parent_width: "dd-u-1-1 dd-u-sm-1-1 dd-u-md-1-1 dd-u-lg-18-24"

    # required children collection (recursive shape)
    items:
      - child_kind: "link"
        child_link_label: "Home"
        child_link_url: "/"
        child_link_target: "_self"
        child_link_css: "menu-item__link"
        items: []

fields:
  # ---------------------------
  # parent fields
  # ---------------------------
  - id: parent_type
    required: true
    type: enum
    options: ["dd-header__navigation", "dd-footer__navigation"]
    default: "dd-header__navigation"
    maps_to: ".dd-navigation class token"

  - id: parent_class
    required: true
    type: enum
    options: ["-main-menu", "-menu-secondary", "-menu-tertiary", "-footer-menu", "-footer-menu-secondary", "-footer-menu-tertiary", "-social-menu"]
    default: "-main-menu"
    maps_to: ".dd-navigation class token"

  - id: parent_data_aos
    required: true
    type: enum
    options: ["fade-in", "fade-up", "fade-right", "fade-down", "fade-left", "zoom-in", "zoom-in-up", "zoom-in-down"]
    default: "fade-in"
    maps_to: ".dd-navigation[data-aos]"

  - id: parent_width
    required: true
    type: string
    default: "dd-u-1-1 dd-u-sm-1-1 dd-u-md-1-1 dd-u-lg-18-24"
    maps_to: ".dd-section__item width class"

  # ---------------------------
  # child items[] fields (recursive tree)
  # each item is either a "link" (clickable, has url) or a "button" (non-clickable
  # grouping header used to hold child items). Any item may itself carry a nested
  # items[] of the same shape to build sub-menus at arbitrary depth.
  # ---------------------------
  - id: items
    required: true
    type: array
    min_items: 1
    item_fields: &nav_item_fields
      - id: child_kind
        required: true
        type: enum
        options: ["link", "button"]
        default: "link"
        maps_to: "render mode: <a> for link, <span> for button"

      - id: child_link_label
        required: true
        type: string
        maps_to: ".menu-item label text (a or span inner text)"

      - id: child_link_url
        required_when: "child_kind == 'link'"
        forbidden_when: "child_kind == 'button'"
        type: string
        maps_to: ".menu-item a[href]"

      - id: child_link_target
        required: false
        forbidden_when: "child_kind == 'button'"
        type: enum
        options: ["_self", "_blank"]
        default: "_self"
        maps_to: ".menu-item a[target]"

      - id: child_link_css
        required: false
        type: string
        maps_to: ".menu-item a/span class attribute"

      - id: items
        required: false
        type: array
        min_items: 0
        recursive: true            # each entry uses the same item_fields shape as the outer items[]
        item_fields: *nav_item_fields
        maps_to: ".sub-menu nested items"

edit_ui:
  tab_order:
    # parent edit order
    - parent_type
    - parent_class
    - parent_data_aos
    - parent_width

    # item edit order (applies at every depth)
    - items[].child_kind
    - items[].child_link_label
    - items[].child_link_url
    - items[].child_link_target
    - items[].child_link_css

  navigation_tree:
    parent_row: "dd-navigation"
    child_rows: "items[] (recursive — each item may have its own items[])"
    item_row_label: "item {path}: [{child_kind}] {child_link_label}"
    collapse_expand_key: "Space"
    indent_per_depth: 2

  item_collection:
    add_sibling_item_key: "A"           # insert item after selected row at the same depth
    add_child_item_key: "Shift+A"       # insert item into selected row's nested items[] (creates sub-menu if empty)
    remove_item_key: "X"
    min_items_root: 1
    min_items_nested: 0

  enter_behavior:
    parent_row: "start parent field editing at parent_type"
    item_row: "start item field editing at child_kind"

  modal_fields:
    parent_edit_modes:
      - parent_type
      - parent_class
      - parent_data_aos
      - parent_width
    item_edit_modes:
      - child_kind
      - child_link_label
      - child_link_url
      - child_link_target
      - child_link_css
    scope_rule: "editing an items[] row hides parent fields; editing parent row hides item fields"
    kind_rule: "when child_kind == 'button', child_link_url and child_link_target are hidden in the modal"

blueprint:
  label: "dd-navigation"
  show_fields:
    - parent_type
    - parent_class
    - "items[].child_kind"
    - "items[].child_link_label"
---

## HTML Template

Rendered as a recursive tree. The `<!-- block: menu-item -->` section is the
recursive unit: at every depth, an item renders either an `<a>` (link) or a
`<span>` (button), followed by a `<ul class="sub-menu">` if and only if the
item carries a non-empty nested `items[]`.

```html
<div class="dd-navigation [parent_class] -y-center">
  <nav itemscope itemtype="https://schema.org/SiteNavigationElement" aria-label="[parent_type] navigation">
    <button class="dd-menu__close fa-regular fa-times" type="button"><span class="visually-hidden">Menu</span></button>
    <ul class="menu">
      <!-- repeat: items -->
      <!-- block: menu-item -->
      <li class="menu-item[#if has_children] -has-children[/if]">
        <!-- if child_kind == 'link' -->
        <a href="[child_link_url]" target="[child_link_target]" class="[child_link_css]">[child_link_label]</a>
        <!-- elif child_kind == 'button' -->
        <span class="[child_link_css]" role="presentation">[child_link_label]</span>
        <!-- endif -->

        <!-- if has_children -->
        <ul class="sub-menu">
          <!-- repeat: items (recursive — reuses block: menu-item) -->
        </ul>
        <!-- endif -->
      </li>
      <!-- endblock -->
    </ul>
  </nav>
</div>
```

## Conditional Markup

- `<a>` renders only when `child_kind == "link"`; `<span>` renders only when `child_kind == "button"`
- when `child_link_target` is empty for a link, default to `_self`
- `<ul class="sub-menu">` renders only when the item's nested `items[]` is non-empty
- `-has-children` modifier on `<li class="menu-item">` is applied only when nested `items[]` is non-empty
- recursion depth is unbounded by spec; the renderer may cap at a sane depth (suggest 4) to prevent runaway templates

## Validation Rules

- root `items[]` requires at least 1 item; nested `items[]` may be empty
- every item: `child_kind` required; `child_link_label` required and non-empty
- `child_kind == "link"`: `child_link_url` required and must pass URL check (`/`, `#`, `http://`, `https://`); `child_link_target` optional (defaults to `_self`)
- `child_kind == "button"`: `child_link_url` must be empty; `child_link_target` must be empty; buttons exist solely to group child items under a non-navigable header
- `child_link_css` optional at every depth
- recursion applies the same rules at every depth
