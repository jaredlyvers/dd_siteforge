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
└── export_dir: Option<String>
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
- Static export: `crate::renderer::render_site_to_dir(&site, &out)`. The TUI also copies `<site_dir>/source/images/` → `<out>/assets/images/` after rendering.

## Validation

`validate_site(&Site) → Vec<String>`: structural checks (unique slugs, paired link fields, required fields per component, etc.). The CLI `validate-site` subcommand uses this.

`validate_site_with_root(&Site, Option<&Path>)`: superset that also resolves every `assets/images/*` URL against `<root>/source/images/` and reports missing files as `Missing local image: …`. The TUI calls this from the F3 modal, the export gate, and the preview gate; the CLI does not (no path context).

## TUI Loop

`fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>)`:

```
loop:
  tick_autosave(now)              # write site.json if dirty + 2s elapsed
  terminal.draw(|f| self.draw(f)) # paints chrome + active modal + toasts
  if event::poll(100ms):
    handle_event(evt)             # routes to modal handler or main key dispatch
    mark_dirty_if_changed()       # JSON snapshot diff vs last_saved_json
```

### Key bindings (global)

| Key | Action |
|---|---|
| `F1` | Help (scrollable) |
| `F3` | Validate site → modal on errors, success toast otherwise |
| `Shift+E` | Export site (validate gate → render → copy source/images/) |
| `p` | Preview current page (validate → export → spawn browser) |
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

## Theme

Tokens load from (in order): `./dd_staticsite_theme.yml`, `./theme.yml`, `./.theme.yml`, `~/.config/ldnddev/dd_staticsite_theme.yml`. Fall through to built-in defaults.

Schema follows `THEME_STRUCTURE_STANDARD.md`. Tokens used: `base_background`, `body_background`, `popup_background` (modal), `text_primary`, `text_secondary`/`muted`, `text_labels`, `text_active_focus`, `modal_labels`, `modal_text`, `selected_*`, `border_default`/`border_active`, `input_border_*`, `input_text_*`, `cursor`, `scrollbar`/`scrollbar_hover`, `success`/`info`/`warning`/`error`, `folders`/`files`/`links`.

## Storage + Autosave

- JSON via serde, pretty-printed.
- Dirty-detection compares a serialized snapshot of the site against `last_saved_json` after each event.
- Autosave: 2s debounce → write to current path. Skipped when no path is set.
- Manual `s`: writes `<path>` AND a byte-identical `<path>.backup` (last-known-good checkpoint).
- On load: if `<path>.backup` exists and differs from `<path>`, surface an Info toast.

## Testing

`cargo test -q` — 96 tests across model, storage, validate, and TUI integration paths. Integration tests drive the App via synthesized key events using the in-tree `send_key` helper.
