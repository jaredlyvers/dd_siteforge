# Architecture

Terminal-UI CMS for authoring framework-native static pages. Built in Rust on `ratatui` (rendering), `crossterm` (terminal events), `serde`/`serde_json` (state persistence), and `handlebars` (HTML export). Single-binary, no server, no database.

## Crate Layout

```
src/
  main.rs          CLI entry: init-site / show-site / validate-site / export-html / tui
  model.rs         Site → Page → PageNode → SectionComponent typed tree (serde)
  storage.rs       JSON load/save
  validate.rs      validate_site() + validate_site_with_root() (missing-image)
  renderer.rs      typed-model → HTML via handlebars templates
  tui.rs           interactive editor (App + Modal enum + render/event loop)
  tui/cursor.rs    component → form-state mapping for the unified editor
  tui/editform.rs  declarative form definitions (FormEdit values)
```

## Content Hierarchy

```
Site
├── header (DdHeader)         always present
├── footer (DdFooter)         always present
├── pages: Vec<Page>
│   ├── head (DdHead)         per-page SEO + slug
│   └── nodes: Vec<PageNode>  ordered top-level blocks
│       ├── Hero(DdHero)      standalone, no wrapper
│       └── Section(DdSection)
│           └── columns → components
├── export_dir: Option<String>
├── base_url: Option<String>  origin for canonical / OG / sitemap
└── lang: String              <html lang>, default "en"
```

## Components

**Top-level (Page node):** `dd-hero`, `dd-section`.

**Section components:** `dd-alert`, `dd-banner`, `dd-blockquote`, `dd-card`, `dd-cta`, `dd-filmstrip`, `dd-image`, `dd-milestones`, `dd-modal`, `dd-rich_text`, `dd-slider`, `dd-alternating`, `dd-accordion`, `dd-navigation`.

**Header / Footer slots:** same component set as section components plus `dd-header-search`, `dd-header-menu`.

Each component spec lives in `components/dd-*.md` (single source of truth for fields, render rules, validation).

## Renderer

- Iterates `site.pages` in order; each page emits one `<slug>.html` to `<export_dir>/`.
- `dd-hero` / `dd-section` / each section component has a dedicated `render_*` fn in `src/renderer.rs`.
- Special cases:
  - `dd-accordion` emits FAQ JSON-LD only when `parent_type == -faq`.
  - `dd-blockquote` emits Quotation JSON-LD.
  - `dd-modal` derives `parent_modal_id` from `parent_title` (HTML-id-safe).
  - `dd-slider` derives `parent_uid` from `parent_title`; `uid-<random6>` fallback.
  - `dd-hero.copy` accepts Markdown or HTML, converted at export.
- Static export: `crate::export::export_site(&site, &out, site_root)`. Writes `{slug}.html`, bundled `assets/` (css/js/favicon/webfonts), copies `<site_dir>/source/images/` → `<out>/assets/images/`, plus `sitemap.xml`, `robots.txt`, and `404.html` when no author 404 page exists.
- Asset and page hrefs are same-directory relative (`assets/css/style.min.css`, `contact.html`). `p` / `serve` start a local HTTP server so those paths resolve.

## Validation

`validate_site(&Site) → Vec<String>`: structural checks (unique slugs, paired link fields, required fields per component, etc.).

`validate_site_with_root(&Site, Option<&Path>)`: superset that also resolves every `assets/images/*` URL against `<root>/source/images/` (pages, header, footer, `og_image`) and reports missing files as `Missing local image: …`. CLI `validate-site` / `export-html` / `serve` and the TUI F3/export/preview gates all use this when a site path is known.

## TUI Loop

`fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>)`:

```
loop:
  tick_autosave(now)              # write site.json if dirty + 2s elapsed
  terminal.draw(|f| self.draw(f)) # paints fixed 3-line header (dd_siteforge + random tagline) + panels + 1-line adaptive footer keys + modals + toasts (per LDNDDEV standard)
  if event::poll(100ms):
    handle_event(evt)             # routes to modal handler or main key dispatch
    mark_dirty_if_changed()       # JSON snapshot diff vs last_saved_json
```

### Key bindings (global)

| Key | Action |
|---|---|
| `F1` | Help (scrollable) |
| `F2` | Theme info modal (source + status + color details; same layout as F1) |
| `F3` | Validate site → modal on errors, success toast otherwise |
| `Shift+E` | Export site (validate gate → render → copy source/images/) |
| `p` | Preview current page (validate → export → local HTTP server → browser) |
| `s` | Save (writes `<path>` + `<path>.backup`) |
| `/` | Insert component fuzzy picker |
| `Tab` / `Shift+Tab` | Next/prev page |
| `1` / `2` / `3` | Sidebar focus: Regions / Pages / Layout |
| `Ctrl+Q` | Quit |

### Pages panel (`[2] Nodes`)

`Shift+A` add (template picker) · `Shift+X` delete (confirm + session trash) · `u` undo delete · `Shift+J/K` reorder · `r` rename.

### Layout panel (`[3]`)

`Up/Down` or `j/k` move row · `g`/`G` first/last · `h`/`l` collapse/expand · `Space` toggle expand · `Enter` edit row · `J/K` move column up/down · `C/V` add/remove column · `c/v` prev/next column · `r/f` edit column id/width-class.

### Edit modal

`Tab` / `Up/Down` navigate fields · `Left/Right` cycle enum values · `Ctrl+S` save · `Esc` cancel · `Ctrl+P` (in URL field) opens image picker (image fields) or page picker (link fields). Click any input box to focus it. Mouse wheel scrolls the field list.

### Image / Page pickers

`↑/↓` move · `←` parent dir (image only) · `→`/`Enter` descend or pick · type to filter · `Esc` cancel.

## Theme + Visual Shell

Theme load is strict (LDNDDEV_TUI_VISUAL_STANDARD.md):
1. `./dd_siteforge_theme.yml` (local)
2. `~/.config/ldnddev/dd_siteforge_theme.yml` (global)
3. Built-in defaults

Every theme file must contain `version: 1` at the top level (validated on load; bad/missing version falls back with a Warning toast).

`header_quotes` (optional top-level list) overrides the 5 built-in rotating header taglines (chosen once at App::new using time ^ pid).

The TUI now exposes `theme.app_shell` and `theme.active_border` (Style) for the standard header/footer.
`theme.modal_header` colors inner section headers (e.g. cards in F1 help modal).
F2 opens the Theme info modal (source, load status, sampled color tokens with hex) using identical chrome and scroll mechanics to the F1 help modal.

See `src/tui.rs` (draw, AppTheme::load, choose_header_copy, default_header_quotes) for the concrete implementation.
The Details panel title now includes current page context ("Details — 03: Home").
Former persistent status messages are delivered as toasts.

## Storage + Autosave

- JSON via serde, pretty-printed.
- Dirty-detection compares a serialized snapshot of the site against `last_saved_json` after each event.
- Autosave: 2s debounce → write to current path. Skipped when no path is set.
- Manual `s`: writes `<path>` AND a byte-identical `<path>.backup` (last-known-good checkpoint).
- On load: if `<path>.backup` exists and differs from `<path>`, surface an Info toast.

## Testing

`cargo test -q` — 96 tests across model, storage, validate, and TUI integration paths. Integration tests drive the App via synthesized key events using the in-tree `send_key` helper.
