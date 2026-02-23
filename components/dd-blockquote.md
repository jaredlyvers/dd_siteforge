---
component: dd-blockquote
version: 1
node_scope: section_item   # one of: page_node | section_item
insert:
  defaults:
    blockquote_data_aos: "fade-in"
    blockquote_image_url: "https://dummyimage.com/512x512/000/fff"
    blockquote_image_alt: "blockquote Persons Name"
    blockquote_persons_name: "blockquote Persons Name"
    blockquote_persons_title: "blockquote Persons Title"
    blockquote_copy: "blockquote content"
fields:
  - id: blockquote_data_aos
    required: true
    type: enum
    options: ["fade-in","fade-up","fade-right","fade-down","fade-left","zoom-in","zoom-in-up","zoom-in-down"]
    default: "fade-in"
    maps_to: ".dd-blockquote[data-aos]"
  - id: blockquote_image_url
    required: true
    type: string
    maps_to: ".dd-blockquote__image img[src]"
  - id: blockquote_image_alt
    required: true
    type: string
    maps_to: ".dd-blockquote__image img[alt]"
  - id: blockquote_persons_name
    required: true
    type: string
    maps_to: ".dd-blockquote__name"
  - id: blockquote_persons_title
    required: true
    type: string
    maps_to: ".dd-blockquote__title"
  - id: blockquote_copy
    required: true
    type: string
    maps_to: ".dd-blockquote__comment"
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
edit_ui:
  tab_order:
    - blockquote_data_aos
    - blockquote_image_url
    - blockquote_image_alt
    - blockquote_persons_name
    - blockquote_persons_title
    - blockquote_copy
  enter_behavior:
    parent_row: "start dd-blockquote field editing"
  modal_fields:
    parent_edit_modes:
      - blockquote_data_aos
      - blockquote_image_url
      - blockquote_image_alt
      - blockquote_persons_name
      - blockquote_persons_title
      - blockquote_copy
    scope_rule: "dd-blockquote is a single component with no child rows; all editable fields are available on parent edit"
    hide_when_editing_blockquote:
      - column.id
      - column.width_class
blueprint:
  label: "dd-blockquote"
  show_fields:
    - blockquote_persons_name
    - blockquote_persons_title
---

## HTML Template

```html
<blockquote class="dd-blockquote">
  <div class="dd-blockquote__content dd-g" data-aos="[blockquote_data_aos]" data-aos-duration="1000" data-aos-easing="linear" data-aos-anchor-placement="center-bottom" data-aos-delay="100">
    <div class="dd-blockquote__icon"><svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-quote-icon lucide-quote"><path d="M16 3a2 2 0 0 0-2 2v6a2 2 0 0 0 2 2 1 1 0 0 1 1 1v1a2 2 0 0 1-2 2 1 1 0 0 0-1 1v2a1 1 0 0 0 1 1 6 6 0 0 0 6-6V5a2 2 0 0 0-2-2z"/><path d="M5 3a2 2 0 0 0-2 2v6a2 2 0 0 0 2 2 1 1 0 0 1 1 1v1a2 2 0 0 1-2 2 1 1 0 0 0-1 1v2a1 1 0 0 0 1 1 6 6 0 0 0 6-6V5a2 2 0 0 0-2-2z"/></svg></div>
    <div class="dd-blockquote__person dd-g l-box">
      <div class="dd-blockquote__image">
        <picture>
          <img src="[blockquote_image_url]" class="dd-img" alt="[blockquote_image_alt]" />
        </picture>
      </div>
      <div class="dd-blockquote__name-title">
        <span class="dd-blockquote__name">[blockquote_persons_name]</span>
        <span class="dd-blockquote__title">, [blockquote_persons_title]</span>
      </div>
      <div class="dd-blockquote__comment">
        [blockquote_copy]
      </div>
    </div>
  </div>
</blockquote>
<script type="application/ld+json">
{
  "@context": "https://schema.org/",
  "@type": "Quotation",
  "creator": {
    "@type": "Person",
    "name": "[blockquote_persons_name], [blockquote_persons_title]"
  },
  "text": "[blockquote_copy]"
}
</script>
```
