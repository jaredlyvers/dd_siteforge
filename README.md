# dd_staticsite

A terminal-UI CMS for authoring framework-native static pages. Single Rust binary; edit pages in a TUI, export typed HTML.

```
┌─[1] Regions──┐ ┌─Details──────────────────────┐
│  Header      │ │ class: -full-contained       │
│  Footer      │ │ items:                       │
└──────────────┘ │   +-----------+ +----------+ │
┌─[2] Nodes────┐ │   | column-1  | | column-2 | │
│ 01 Home      │ │   +-----------+ +----------+ │
│ 02 Contact   │ │                              │
└──────────────┘ │                              │
┌─[3] Layout───┐ │                              │
│ [HEAD] Home  │ │                              │
│ 1. dd-hero   │ │                              │
│ 2. dd-section│ │                              │
└──────────────┘ └──────────────────────────────┘
```

## Install

```bash
cargo install --path .
# or
cargo build --release
```

Binary: `target/release/dd_staticsite`.

## Usage

```bash
# Create a starter site at site.json
dd_staticsite init-site site.json

# Edit interactively
dd_staticsite tui site.json

# Validate (exits non-zero on errors)
dd_staticsite validate-site site.json

# Export to ./web/ (or whatever site.export_dir is set to)
dd_staticsite export-html site.json ./web/

# Inspect serialized state
dd_staticsite show-site site.json
```

## TUI cheatsheet

**Global:** `F1` help · `F3` validate · `Shift+E` export · `p` preview in browser · `s` save (+ writes `.backup`) · `/` insert component · `Tab`/`Shift+Tab` next/prev page · `Ctrl+Q` quit.

**Pages panel (`[2] Nodes`):** `Shift+A` add · `Shift+X` delete · `u` undo delete · `Shift+J`/`Shift+K` reorder · `r` rename.

**Layout panel (`[3]`):** `j`/`k` or arrows move · `h`/`l` collapse/expand · `Space` toggle · `Enter` edit · `J`/`K` move column · `C`/`V` add/remove column · `r`/`f` edit column id/width.

**Edit modal:** `Tab` navigate · `Left`/`Right` cycle enums · `Ctrl+S` save · `Esc` cancel · `Ctrl+P` in any URL field opens an image picker (for `*_image_url`) or page picker (for `*_link_url`). Mouse wheel scrolls; click any input box to focus.

## Project layout

```
.
├── components/                    component spec docs (dd-*.md, source of truth)
├── src/
│   ├── main.rs                    CLI entry
│   ├── model.rs                   Site → Page → Node typed tree
│   ├── storage.rs                 JSON load/save
│   ├── validate.rs                structural + missing-image checks
│   ├── renderer.rs                handlebars-driven HTML export
│   ├── tui.rs                     interactive editor (App + Modal)
│   └── tui/{cursor,editform}.rs   form-state + declarative form definitions
├── docs/superpowers/{specs,plans}/  design + implementation plan archive
├── Architecture.md                module map, render/validation rules, key bindings
├── THEME_STRUCTURE_STANDARD.md    theme token schema
├── dd_staticsite_theme.yml        default theme
└── Cargo.toml
```

## Authoring workflow

1. `init-site` → starter `site.json`.
2. Drop image source files in `./source/images/` next to the JSON.
3. `tui` → edit pages, components, head metadata. Autosave writes every 2s; manual `s` makes a checkpoint backup.
4. `Shift+E` to export validates first, then renders to `./web/` (or your configured `export_dir`) and copies `source/images/` to `web/assets/images/`.
5. `p` previews the current page in the system browser.

## Theme

Customize colors by writing one of these (first found wins):
- `./dd_staticsite_theme.yml`
- `./theme.yml` or `./.theme.yml`
- `~/.config/ldnddev/dd_staticsite_theme.yml`

Schema in `THEME_STRUCTURE_STANDARD.md`. Built-in default ships at `dd_staticsite_theme.yml`.

## Tests

```bash
cargo test -q
```

## License

Proprietary; internal ldnddev tooling.
