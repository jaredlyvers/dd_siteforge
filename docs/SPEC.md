# dd_siteforge — Spec

Living product spec. Replaces the old dated design/plan archive. Update this file when behavior or conventions change.

Companion docs:

- `Architecture.md` — crate map, render/validation rules, key bindings
- `LDNDDEV_TUI_VISUAL_STANDARD.md` — portable TUI theme + shell contract (copy into any new ldnddev TUI)
- `components/dd-*.md` — per-component fields, render rules, validation
- `README.md` — install and usage

---

## Product

Terminal-UI CMS for authoring framework-native static pages. Single Rust binary. Author edits a typed site tree in the TUI, exports HTML, hosts anywhere static.

Target: small marketing sites (roughly 5–20 pages, one editor). No multi-user, no live CMS, no database.

Workflow:

1. `init-site` → starter `site.json`, build kit (`source/`, Grunt, Lando/DDEV), and `source/templates/`
2. Images in `./source/images/` next to the JSON
3. TUI edits pages/components/head. Autosave every 2s; `s` writes a `.backup`
4. `npx grunt build` then export: HTML + Grunt `web/assets` + images + sitemap/robots/404
5. `p` / `serve` starts a local HTTP server so relative `assets/` paths resolve

Current crate version: see `Cargo.toml`.

---

## Shipped surface

### CLI

`init-site` (`--name`) · `init-templates` (`--force`, `--name`) · `init-scaffold` (`--force`, `--global`, `--name`) · `tui` · `validate-site` · `export-html` · `serve` · `show-site`

### Content model

`Site` → always-present `header` / `footer` → `pages[]` → per-page `head` + `nodes[]` (`dd-hero` or `dd-section` with columns of components). Optional `export_dir`, `base_url`, `lang`.

New fields on `Site` / `Page` take `#[serde(default)]` so legacy JSON still loads.

Animation attribute is SAL (`data-sal`). JSON still accepts the old `parent_data_aos` alias.

### TUI

Fixed 3-line header + body + 1-line adaptive footer per the visual standard.

Sidebar: `[1]` Regions · `[2]` Pages · `[3]` Layout. Right pane: Details (click-to-select, ascii maps).

Toasts for success / info / warning. Modals for errors and forms.

F1 Help (wrap + scroll). F2 Theme (source, status, color samples; same chrome as F1). F3 Validate. `Shift+E` Export. `p` Preview. `s` Save. `/` insert. `Ctrl+Q` quit (confirm if dirty).

Pages panel: add / delete / undo / reorder / rename. Layout: nav, expand, edit, duplicate, columns.

Edit forms: Tab between fields, click-to-focus, mouse wheel, `Ctrl+P` image or page picker on URL fields.

### Export + assets

- Handlebars from crate `templates/` plus `source/templates/` overrides. Seed on `init-site` only. Re-seed with `init-templates --force`. Export never writes templates.
- Build kit (Gruntfile, package.json, `.lando.yml`, `.ddev/`, `source/` except author images and templates) is embedded in the binary. `init-site` copies it once (skip existing). Optional house overlay: `~/.config/ldnddev/dd_siteforge/` (dump with `init-scaffold --global`). Re-seed a site with `init-scaffold --force`.
- `init-site` asks for a project name (or `--name` / folder default when stdin is not a TTY) and stamps that slug into Lando, DDEV, and `package.json`.
- CSS/JS come from Grunt (`source/{js,scss}` → `web/assets`). Host `npm install && npx grunt build` is the contract. Lando and DDEV are optional wrappers.
- Export copies Grunt `web/assets/{css,js,webfonts,favicon,vendors}` unless dest is already that tree (does not clobber a local grunt build). Fills missing webfonts/favicon from `source/`. Copies `source/images/` → `<out>/assets/images/`.

### Validation

`validate_site` — structural. `validate_site_with_root` — also missing local images. CLI export/serve and TUI F3/export/preview all gate on this.

---

## Non-goals

- Multi-user, auth, or a server-side CMS
- Remote image CDN beyond pasting an external URL
- Mixing template seed with the Grunt pipeline
- Inventing one-off theme tokens (promote them in the visual standard first)

---

## Conventions

### Branch + commit

Feature work on `feat/<short-name>` off `master`. Commits use plain prefixes: `tui:`, `model:`, `validate:`, `docs:`, `test:`. Tags: annotated `vMAJOR.MINOR.PATCH`. Fast-forward merges. Do not re-tag.

### Tests

`#[cfg(test)] mod tests` at the bottom of the module. Drive TUI via in-tree `send_key`, not by poking state when a key path exists. `cargo test -q`.

### New component

1. Spec in `components/dd-*.md` (copy a `components/*-template.md` scaffold)
2. Types in `src/model.rs`
3. Renderer in `src/renderer.rs` + `templates/dd-*.hbs`
4. FormEdit in `src/tui/editform/`
5. Route in `src/tui/cursor.rs`

### New modal

Four-point plumbing: enum variant + render dispatch + event dispatch + `Modal::variant_name` arm. Multi-field forms go through `render_edit_modal_unified` or `render_form_edit_modal`. Single prompts share `render_single_input_modal`. Render fns are `&self`; values the event loop needs from draw go through `RefCell` or pre-publish into `&mut self`.

### Theme

Always `self.theme.*`. Labels: `text_labels` → `text_active_focus` when focused. Input borders/text/cursor from the `input_*` + `cursor` tokens. Folders/files/links in pickers. Modal section headers: `modal_header` (bold).

### UX prefs

- Footer is a 1-line adaptive key bar. Always starts with `F1:Help`. No long status text.
- All scrollable surfaces: mouse wheel + keyboard.
- Path display: strip leading `./` and trailing `/`.
- Browser launch pins stdio to `/dev/null` so raw-mode TUI stays intact.

### Local files — never commit

`site.json`, `site.json.backup`, `web/`, `source/images/*` (keep `.gitkeep`), `.kilo/`.

### Ship

Smoke test → fast-forward merge → push → annotated tag matching `Cargo.toml` version → push tag. No PR unless asked.

---

## Anti-patterns

- Features that were not requested
- Proactive `cargo fmt` / `cargo fix`
- Bypassing git hooks
- Back-compat shims for paths with no old consumers
- Docstrings on every function (one-liner only when the *why* is non-obvious)
- Overwriting existing user files on template seed without `--force`
