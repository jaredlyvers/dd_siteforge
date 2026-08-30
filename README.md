# dd_siteforge

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
./install.sh
```

Builds release, drops the binary at `$HOME/.local/bin/dd_siteforge`, and installs the default theme at `$HOME/.config/ldnddev/dd_siteforge_theme.yml` (only when no theme is already there). Override paths via `PREFIX`, `BIN_DIR`, or `CONFIG_DIR` env vars.

Manual alternatives:

```bash
cargo install --path .            # cargo's bin dir (~/.cargo/bin)
cargo build --release             # binary at target/release/dd_siteforge
```

## CSS / JS build

Host Node is the contract. Lando and DDEV are optional wrappers around the same `npm` / `grunt` commands.

```bash
npm install
npx grunt build          # or: npm run build
```

Writes `web/assets/css`, `web/assets/js`, `web/assets/webfonts`, and axe helpers. `npx grunt dev` rebuilds on change and serves `web/` (or proxies `BROWSERSYNC_PROXY` when set).

**Lando** (optional): `lando start` then `lando grunt build`. Site at https://dd-siteforge.lndo.site (also http://localhost:8888).

**DDEV** (optional): `ddev start` then `ddev npm run build`. `ddev launch` opens the site.

**BrowserSync proxy** (optional, for `grunt dev` behind Lando/DDEV):

```bash
BROWSERSYNC_PROXY=https://dd-siteforge.lndo.site npx grunt dev
```

## Usage

```bash
# Create a starter site (prompts for project name; seeds build kit + templates)
dd_siteforge init-site site.json
dd_siteforge init-site site.json --name acme-site

# Re-seed templates (skips existing; --force overwrites)
dd_siteforge init-templates site.json
dd_siteforge init-templates site.json --force
dd_siteforge init-templates site.json --force --name dd-hero

# Re-seed Grunt / source / Lando / DDEV (skips existing; --force overwrites)
dd_siteforge init-scaffold site.json
dd_siteforge init-scaffold site.json --force --name acme-site
dd_siteforge init-scaffold --global

# Edit interactively
dd_siteforge tui site.json

# Validate (exits non-zero on errors; checks local images next to the JSON)
dd_siteforge validate-site site.json

# Export HTML + copy Grunt assets/images (run `npx grunt build` first for css/js)
dd_siteforge export-html site.json ./web/

# Export then serve at http://127.0.0.1:8765/
dd_siteforge serve site.json

# Inspect serialized state
dd_siteforge show-site site.json
```

## TUI cheatsheet

**Global:** `F1` help · `F3` validate · `Shift+E` export · `p` preview in browser · `s` save (+ writes `.backup`) · `/` insert component · `Tab`/`Shift+Tab` next/prev page · `Ctrl+Q` quit (confirms if unsaved).

**Pages panel (`[2] Nodes`):** `Shift+A` add · `Shift+X` delete · `u` undo delete · `Shift+J`/`Shift+K` reorder · `r` rename.

**Layout panel (`[3]`):** `j`/`k` or arrows move · `h`/`l` collapse/expand · `Space` toggle · `Enter` edit · `d` delete · `y` duplicate · `u` undo · `J`/`K` move selected grain · `C`/`V` add/remove column · `r`/`f` edit column id/width.

**Edit modal:** `Tab` navigate · `Left`/`Right` cycle enums · `Ctrl+S` save · `Esc` cancel · `Ctrl+P` in any URL field opens an image picker (for `*_image_url`) or page picker (for `*_link_url`). Mouse wheel scrolls; click any input box to focus.

## Project layout

```
.
├── components/                    component spec docs (dd-*.md, source of truth)
├── src/
│   ├── main.rs                    CLI entry
│   ├── model.rs                   Site → Page → Node typed tree
│   ├── storage.rs                 JSON load/save (atomic)
│   ├── validate.rs                structural + missing-image checks
│   ├── renderer.rs                handlebars-driven HTML export
│   ├── export.rs                  full site export (assets + sitemap/robots/404)
│   ├── serve.rs                   local HTTP preview server
│   └── tui/                       interactive editor (App, theme, help, forms)
├── source/                        framework source (js, scss, favicon, webfonts, images)
├── templates/                     bundled Handlebars (seeded to source/templates on init)
├── Gruntfile.js / package.json    CSS/JS build → web/assets
├── .lando.yml / .ddev/            optional local env (host npm is the contract)
├── docs/SPEC.md                   living product spec
├── Architecture.md                module map, render/validation rules, key bindings
├── LDNDDEV_TUI_VISUAL_STANDARD.md portable TUI theme + shell contract
├── dd_siteforge_theme.yml         default theme
└── Cargo.toml
```

## Authoring workflow

1. `init-site` → starter `site.json`, Grunt/`source/` kit, Lando + DDEV, and `source/templates/`. Pass `--name` to skip the project-name prompt.
2. Drop image source files in `./source/images/` next to the JSON.
3. `tui` → edit pages, components, head metadata. Autosave writes every 2s; manual `s` makes a checkpoint backup.
4. `npx grunt build` then `Shift+E` to export: validates, writes HTML, copies Grunt `web/assets` (css/js/webfonts/favicon) when dest is not already `web/`, copies `source/images/` to `web/assets/images/`, plus `sitemap.xml`, `robots.txt`, `404.html`.
5. `p` exports, starts a local HTTP server, and opens the current page in the system browser. `dd_siteforge serve site.json` does the same from the CLI.

## Theme

Customize colors by writing one of these (first found wins):
- `./dd_siteforge_theme.yml`
- `./theme.yml` or `./.theme.yml`
- `~/.config/ldnddev/dd_siteforge_theme.yml`
- `~/.config/ldnddev/dd_siteforge/.theme.yml`

Schema in `LDNDDEV_TUI_VISUAL_STANDARD.md`. Built-in default ships at `dd_siteforge_theme.yml`.

## Tests

```bash
cargo test -q
```

## License

MIT License. Use, fork, modify, and build from it freely.
