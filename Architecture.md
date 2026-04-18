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
3. `dd-alert`
4. `dd-banner`
5. `dd-accordion`
6. `dd-alternating`
7. `dd-blockquote`
8. `dd-card`
9. `dd-cta`
10. `dd-filmstrip`
11. `dd-milestones`
12. `dd-modal`
13. `dd-slider`

### Typed Model

```rust
enum PageNode {
    Hero(DdHero),
    Section(DdSection),
}

enum SectionComponent {
    Alert(DdAlert),
    Cta(DdCta),
    Filmstrip(DdFilmstrip),
    Milestones(DdMilestones),
    Modal(DdModal),
    Slider(DdSlider),
    Banner(DdBanner),
    Card(DdCard),
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
- `dd-card` renders card item collections with optional link output when URL + label are present.
- `dd-cta` renders parent-only CTA content with optional link output when URL + label are present.
- `dd-filmstrip` renders one item collection twice using identical source data; the second loop is aria-hidden.
- `dd-milestones` renders an item grid using parent `data-aos` + width classes and optional per-item links.
- `dd-modal` derives `parent_modal_id` from `parent_title` using a safe HTML id transform at render time.
- `dd-slider` renders a repeating item list with optional per-item links and parent UID derived from `parent_title`; fallback UID is `uid-<random 6 digits>`.
- `dd-hero` and component AOS/class options are rendered from typed enum/string fields.
- `dd-hero.copy` accepts Markdown and HTML; it is converted to HTML at export.

### Validation Rules

Validation runs on create/update/export:
- Site/page basics: non-empty pages, unique slugs.
- Hero: required `title`, paired CTA fields, valid URLs, `image_alt` when image is present.
- Section: unique section and column ids, non-empty `width_class`.
- Alert: required `parent_copy`.
- Banner: required `banner_image_url` and `banner_image_alt`; valid image URL format.
- Accordion: non-empty `group_name`; at least one item; each item needs title/content.
- Alternating: at least one item; each item needs image, alt, title, and copy.
- Blockquote: required image URL/alt, person name/title, and quote copy.
- Card: non-empty `card_width`; at least one item; each item needs image/alt/title/subtitle/copy; optional links must provide URL + label together.
- CTA: required class/image/aos/title/subtitle/copy; optional link fields must provide URL + label together.
- Filmstrip: at least one item; each item needs image URL/alt/title; image URLs must be valid.
- Milestones: non-empty `parent_width`; at least one item; each item needs percentage/title/subtitle/copy; optional links must provide URL + label together.
- Modal: required `parent_title` and `parent_copy`.
- Slider: at least one item; each item needs title/copy/image URL/image alt; optional links must provide URL + label together.

### TUI Editing Contract

- `/` opens fuzzy component insert finder.
- `Enter` starts editing selected node/row.
- `Tab` / `Shift+Tab` moves between editable fields.
- `Left` / `Right` cycles enum-style field options on active fields.
- In multiline textarea fields (`hero.copy`, `alternating_copy`, `accordion_copy`, `blockquote_copy`, `cta_copy`, `card_copy`, `child_copy`, `parent_copy`), `Enter` inserts newline and `Ctrl+S` saves.
- Editing is scoped to selected row type:
  - parent row edits parent fields
  - child row edits child fields

### Storage + Export

- JSON persistence via serde.
- Deterministic save/load for reorder/edit operations.
- HTML export from typed model using handlebars templates.
