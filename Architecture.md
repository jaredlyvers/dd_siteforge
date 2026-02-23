### Architecture

**Overview:**
Terminal User Interface (TUI) app for building framework-native pages through a CMS-like workflow. Built in Rust using `ratatui` (mouse + keyboard), `crossterm` (terminal events), `serde/serde_json` (state persistence), and `handlebars` (template rendering).

Primary content hierarchy:
- `Site`
- `Page`
- `Node` (ordered)
  - `dd-hero` (top-level component, no `dd-section` wrapper required)
  - `dd-section` wrapper containing section components

### Supported Components

Only these components are supported:
1. `dd-hero`
2. `dd-section`
3. `dd-banner`
4. `dd-accordion`
5. `dd-alternating`
6. `dd-blockquote`

### Typed Model

```rust
enum PageNode {
    Hero(DdHero),
    Section(DdSection),
}

enum SectionComponent {
    Banner(DdBanner),
    Accordion(DdAccordion),
    Alternating(DdAlternating),
    Blockquote(DdBlockquote),
}
```

### Rendering Rules

- Render `Page.nodes` in order.
- `dd-hero` renders standalone.
- `dd-section` renders a column grid and each section component inside the selected columns.
- `dd-accordion` FAQ schema (`ld+json`) is rendered only when `accordion_type` is `-faq`.
- `dd-blockquote` renders quotation markup + `ld+json` quotation schema.
- `dd-hero` and component AOS/class options are rendered from typed enum/string fields.
- `dd-hero.copy` accepts Markdown and HTML; it is converted to HTML at export.

### Validation Rules

Validation runs on create/update/export:
- Site/page basics: non-empty pages, unique slugs.
- Hero: required `title`, paired CTA fields, valid URLs, `image_alt` when image is present.
- Section: unique section and column ids, non-empty `width_class`.
- Banner: required `banner_image_url` and `banner_image_alt`; valid image URL format.
- Accordion: non-empty `group_name`; at least one item; each item needs title/content.
- Alternating: at least one item; each item needs image, alt, title, and copy.
- Blockquote: required image URL/alt, person name/title, and quote copy.

### TUI Editing Contract

- `/` opens fuzzy component insert finder.
- `Enter` starts editing selected node/row.
- `Tab` / `Shift+Tab` moves between editable fields.
- `Left` / `Right` cycles enum-style field options on active fields.
- In multiline textarea fields (`hero.copy`, `alternating_copy`, `accordion_copy`, `blockquote_copy`), `Enter` inserts newline and `Ctrl+S` saves.
- Editing is scoped to selected row type:
  - parent row edits parent fields
  - child row edits child fields

### Storage + Export

- JSON persistence via serde.
- Deterministic save/load for reorder/edit operations.
- HTML export from typed model using handlebars templates.
