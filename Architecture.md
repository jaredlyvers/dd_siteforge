### Architecture

**Overview:**
Terminal User Interface (TUI) app for building framework-native pages through a CMS-like workflow. Built in Rust using `ratatui` (mouse + keyboard), `crossterm` (terminal events), `serde/serde_json` (state persistence), and `handlebars` (template rendering). Output targets ldnddev framework markup and component templates, not generic flex-only HTML.

Primary content hierarchy:
- `Site`
- `Page`
- `Node` (ordered)
  - `dd-hero` (top-level component, no `dd-section` wrapper required)
  - `dd-section` wrapper containing one or more section-compatible components

This model enforces framework rules, required parameters, and valid layout options at authoring time before export.

**Goals:**
- Produce HTML that matches framework component contracts and class patterns.
- Prevent invalid component configs with schema-based validation.
- Support responsive and accessible authoring defaults.
- Export deterministic, production-ready static pages.

### Framework-Native Component Model

Represent components as typed variants with explicit fields instead of generic key/value maps.

```rust
struct Site {
    id: String,
    name: String,
    theme: ThemeSettings,
    pages: Vec<Page>,
}

struct Page {
    id: String,
    slug: String,
    title: String,
    meta_description: Option<String>,
    nodes: Vec<PageNode>, // Ordered render list
}

enum PageNode {
    Hero(DdHero),
    Section(DdSection),
}

struct DdHero {
    image: String,
    hero_class: HeroImageClass,        // -contained|-contained-md|...|-full-contained-xxl
    hero_aos: HeroAos,                 // fade-in|fade-up|fade-right|fade-down|fade-left|zoom-in|zoom-in-up|zoom-in-down
    title: String,
    subtitle: String,
    copy: Option<String>,
    cta_text: Option<String>,
    cta_link: Option<String>,
    cta_text_2: Option<String>,
    cta_link_2: Option<String>,
    cta_target: Option<CtaTarget>,
    cta_target_2: Option<CtaTarget>,
    image_alt: Option<String>,
    image_mobile: Option<String>,
    image_tablet: Option<String>,
    image_desktop: Option<String>,
    image_class: Option<HeroImageClass>, // dd-hero__image class token
}

struct DdSection {
    id: String,
    section_title: Option<String>,
    section_class: SectionClass, // -contained|-contained-md|...|-full-contained-xxl
    item_box_class: SectionItemBoxClass, // l-box|ll-box
    columns: Vec<SectionColumn>,
}

enum SectionComponent {
    Card(DdCard),
    Alert(DdAlert),
    Banner(DdBanner),
    Tabs(DdTabs),
    Accordion(DdAccordion),
    Cta(DdCta),
    Modal(DdModal),
    Slider(DdSlider),
    Spacer(DdSpacer),
    Timeline(DdTimeline),
}
```

### Components and Contracts

All components below are available and should be first-class in the editor.

1. `dd-hero`
- Required: `title`
- Optional: `image`, `image_alt` (required when `image` is present), `subtitle`, `copy`, `cta_text`, `cta_link`, `cta_target`, `cta_text_2`, `cta_link_2`, `cta_target_2`, `image_mobile`, `image_tablet`, `image_desktop`, `hero_class`, `hero_aos`, `image_class`
- Placement: top-level page node (no `dd-section` wrapper)

2. `dd-card`
- Required: `title`, `image`
- Optional: `subtitle`, `copy`, `cta_text`, `cta_link`, `image_alt`, `columns(2|3|4)`, `animate(fade-up|fade-in|slide-up)`
- Placement: inside `dd-section`

3. `dd-section` (wrapper)
- Required: `content`
- Optional layout params: `section_title`, `section_class`, `item_box_class`
- Placement: top-level page node wrapping section-compatible components

4. `dd-alert`
- Required: `type(success|error|warning|info)`, `message`
- Optional: `title`, `dismissible`

5. `dd-banner`
- Required: `message`, `background`
- Optional: `link_text`, `link_url`, `dismissible`

6. `dd-tabs`
- Required: `tabs[]` (`title`, `content`)
- Optional: `default_tab`, `orientation(horizontal|vertical)`

7. `dd-accordion`
- Required: `items[]` (`title`, `content`)
- Optional: `multiple`

8. `dd-cta`
- Required: `title`, `copy`, `cta_text`, `cta_link`

9. `dd-modal`
- Required: `trigger_text`, `title`, `content`

10. `dd-slider`
- Required: `slides[]` (`image`, `title`, `copy`)
- Optional: `autoplay`, `speed`

11. `dd-spacer`
- Required: `height(sm|md|lg|xl|xxl)`

12. `dd-timeline`
- Required: `events[]` (`date`, `title`, `description`)

### Layout Options and Utilities

Expose these options directly in TUI controls (dropdown/select lists) and validate enumerations:

- `dd-section.section_class`: `-contained`, `-contained-md`, `-contained-lg`, `-contained-xl`, `-contained-xxl`, `-full-full`, `-full-contained`, `-full-contained-md`, `-full-contained-lg`, `-full-contained-xl`, `-full-contained-xxl`
- `dd-section.item_box_class`: `l-box`, `ll-box`
- `dd-card.columns`: `2`, `3`, `4`
- `dd-card.animate`: `fade-up`, `fade-in`, `slide-up`
- `dd-tabs.orientation`: `horizontal`, `vertical`
- `dd-hero.hero_class`: `-contained`, `-contained-md`, `-contained-lg`, `-contained-xl`, `-contained-xxl`, `-full-full`, `-full-contained`, `-full-contained-md`, `-full-contained-lg`, `-full-contained-xl`, `-full-contained-xxl`
- `dd-hero.hero_aos`: `fade-in`, `fade-up`, `fade-right`, `fade-down`, `fade-left`, `zoom-in`, `zoom-in-up`, `zoom-in-down`

Framework utility classes to preserve during output where applicable:
- Grid: `dd-u-1-1`, `dd-u-md-12-24`, `dd-u-lg-8-24`, `dd-u-xl-6-24`
- Spacing: `l-box`, `ll-box`
- Text align: `-text-center`, `-text-left`, `-text-right`
- Visibility: `-scrn-reader-only`

### Rendering Rules

- Render `Page.nodes` in order.
- `dd-hero` renders as a standalone block.
- `dd-hero` root class is rendered as `dd-hero {hero_class}`.
- `dd-hero__content` renders configurable AOS attributes:
  - `data-aos="{hero_aos}"`
  - `data-aos-duration="1000"`
  - `data-aos-easing="linear"`
  - `data-aos-anchor-placement="center-bottom"`
  - `data-aos-delay="100"`
- All other components render inside `dd-section` wrappers.
- Use framework template structure and class names (for example `dd-section`, `dd-section__container`, `dd-section__item dd-u-1-1`).
- Prefer template-backed output for each component:
  - `/web/templates/components/dd-hero.html`
  - `/web/templates/components/dd-card.html`
  - `/web/templates/components/dd-section.html`
  - `/web/templates/components/dd-alert.html`
  - `/web/templates/components/dd-banner.html`
  - `/web/templates/components/dd-tabs.html`
  - `/web/templates/components/dd-accordion.html`
  - `/web/templates/components/dd-cta.html`
  - `/web/templates/components/dd-modal.html`
  - `/web/templates/components/dd-slider.html`
  - `/web/templates/components/dd-spacer.html`
  - `/web/templates/components/dd-timeline.html`

### Validation Layer

Validation runs on create, update, and export.

1. Schema validation
- Required fields must exist per component.
- Enum fields must be valid framework values.
- Nested arrays (`tabs`, `items`, `slides`, `events`) must contain required child fields.

2. Placement validation
- `dd-hero` allowed at top level.
- Section components must live inside `dd-section`.

3. Accessibility validation
- Require meaningful `image_alt` when image is informational.
- Enforce semantic headings and section labels where appropriate.
- Ensure interactive controls have discernible text.

4. Link/media validation
- URL format checks for CTA and links.
- Image path existence checks when local assets are used.

### UI/TUI Architecture

**UI Layer (ratatui + crossterm):**
- Left panel: page tree (`Hero`, `Section`, section components)
- Center panel: property editor for selected node
- Right panel: validation/errors and quick help
- Event model:
  - Keyboard: arrows, tab/shift-tab, enter, esc
  - Mouse: click-select, click-edit, wheel-scroll

Hero editing controls currently implemented:
- Select hero row, press `Enter` to open edit mode.
- `Tab` / `Shift+Tab` cycles hero fields:
  - `image`, `class`, `data_aos`, `title`, `subtitle`, `copy`, `link_1_text`, `link_1_url`, `link_2_text`, `link_2_url`
- While editing hero:
  - `Left` / `Right` cycles `hero.class` when that field is active.
  - `Left` / `Right` cycles `hero.data_aos` when that field is active.

**Application Layer:**
- Command handlers: add/move/delete node, update fields, clone component, reorder sections/components
- Undo/redo stack for editor operations
- Deterministic serialization for stable diffs

**Storage Layer:**
- JSON persistence for full `Site` model
- Autosave + explicit save
- Versioned schema for future migrations

### Build and Export Workflow

All framework build commands must use `lando`:

```bash
lando grunt build
lando grunt dev
lando grunt sync
```

Export flow:
1. Load site JSON.
2. Validate full model.
3. Render page HTML via component templates.
4. Write output files and copy referenced assets.
5. Run framework build command for final assets.

### Implementation Plan

**Phase 1: Foundation**
- Set up Rust project and terminal shell.
- Implement typed model (`Site`, `Page`, `PageNode`, framework component structs).
- Add JSON load/save and schema versioning.

**Phase 2: Framework-Aware Editor**
- Build tree navigation and property forms.
- Add all 12 framework components to insert menu.
- Add constrained selectors for all layout enums.

**Phase 3: Validation Engine**
- Implement required-field, enum, placement, and nested-item validation.
- Surface inline validation errors in UI.
- Block export on critical errors.

**Phase 4: Renderer**
- Implement template mapping for each component.
- Enforce wrapper rules (`dd-hero` standalone, others in `dd-section`).
- Generate complete page HTML with semantic structure.

**Phase 5: Accessibility and Responsiveness**
- Add checks for alt text, heading order guidance, and link text quality.
- Support responsive image fields (`image_mobile/tablet/desktop`) in hero.
- Preserve framework utility classes in output.

**Phase 6: Integration and Delivery**
- Integrate `lando` build hooks.
- Add tests for validation and render snapshots.
- Document authoring workflow, keyboard/mouse controls, and troubleshooting.

### Testing Strategy

- Unit tests: schema validation, placement rules, enum parsing, serialization.
- Snapshot tests: rendered HTML per component and mixed-page compositions.
- Integration tests: JSON import -> edit operations -> export pipeline.
- Manual QA: keyboard + mouse interaction, long-form content, responsive layouts.

### Risks and Mitigations

- Risk: Drift between app structs and framework templates.
  - Mitigation: central component registry with shared metadata for required fields/options.

- Risk: Invalid user-authored JSON.
  - Mitigation: strict parser + migration layer + clear repair messages.

- Risk: Template changes in framework.
  - Mitigation: version pinning and regression snapshots for generated HTML.

### Success Criteria

- Users can build pages using every available framework component.
- Exported HTML follows framework structure and passes validation.
- Layout option controls are constrained to framework-supported values.
- Generated output is responsive, accessible, and build-ready with `lando`.
