# ldnddev TUI Visual Standard

**Portable contract for look, feel, and structure across every ldnddev terminal app.**

Copy this file into a new repo as the visual contract. Do not fork app-specific layout, key lists, or data models into the standard. The body of the app is yours; the chrome, tokens, and interaction habits are shared.

This document is the single source of truth for theming and shell. There is no separate theme-structure guide.

Standard version: **v1**. Bump here and in the theme YAML schema when a change is breaking.

---

## Required vs optional

| Required in every app | Optional (use only if the app needs it) |
|---|---|
| Theme lookup + `version: 1` | File / folder / symlink coloring |
| Canonical color tokens | Tree / file-browser panel |
| 3-line header + 1-line footer | Two-pane source + secondary list |
| `app_shell` + `active_border` | Details / inspector pane |
| F1 Help, F2 Theme | App-specific function keys (F3+) |
| Toasts, modals, themed inputs | Checkboxes, bulk-select, action modes |
| Scrollbars + mouse wheel | |
| Click-to-focus on inputs | |

---

## How to adopt in a new app

1. Copy this file and a sample `<app>_theme.yml` into the new repo.
2. Pick the app name (`dd_ftp`, `dd_siteforge`, …). Use it for the theme filename, header title, and config path.
3. Implement the shell first: full-screen `app_shell`, fixed 3-line header (app name + random tagline), 1-line adaptive footer starting with `F1:Help`.
4. Load themes in the lookup order below. Reject files that are not `version: 1`.
5. Map every painted cell to a canonical token. Do not hard-code colors after load.
6. Ship F1 Help (full key + mouse list) and F2 Theme (source + status + sampled tokens).
7. Toasts for non-blocking messages; modals for errors and forms.
8. Test narrow (<80 cols) and wide terminals. No overflow, no clipped chrome.
9. Body layout is app-defined. See §6. Do not copy another app's panels unless the product actually needs them.
10. New shared pattern? Update this document first, then implement.

---

## Goals

- One visual language and interaction habit across the family
- Same token → same intent in every app
- Local override, then global, then built-in defaults
- Easy to implement (Ratatui + crossterm today; token names stay framework-agnostic)
- Playful header, quiet chrome, dense body
- Keyboard and mouse both first-class

---

## 1. Theme system

### Lookup order (every app, exact)

1. `./<APP>_theme.yml` — project-local override
2. `~/.config/ldnddev/<APP>_theme.yml` — user global
3. Built-in defaults inside the binary

`<APP>` is the crate / binary name (`dd_ftp`, `dd_siteforge`, …).

### Schema version

Every theme file must declare:

```yaml
version: 1
```

On load: accept `1`. Any other value or a missing field → skip that file, fall back, and surface a **warning** toast (or startup warning). Do not silently ignore a bad version.

### Canonical tokens

Required keys under `colors:`:

```yaml
version: 1

colors:
  base_background: "#0F1114"
  body_background: "#2A2D31"
  modal_background: "#1C1E21"

  text_primary: "#F5F6F7"
  text_secondary: "#9EA3AA"
  text_labels: "#FFAF46"
  text_active_focus: "#64B4F5"
  modal_labels: "#64B4F5"
  modal_text: "#F5F6F7"
  modal_header: "#64B4F5"

  selected_background: "#0F1114"

  border_default: "#F5F6F7"
  border_active: "#64B4F5"
  scrollbar: "#FFA087"
  scrollbar_hover: "#64B4F5"

  input_border_default: "#F5F6F7"
  input_border_focus: "#64B4F5"
  input_text_default: "#F5F6F7"
  input_text_focus: "#64B4F5"
  cursor: "#64B4F5"

  success: "#82e0aa"
  warning: "#f5c469"
  error: "#e57373"
  info: "#5dade2"

  folders: "#64B4F5"
  files: "#FFAF46"
  links: "#FFA087"
```

Hex values above are the family default palette. Apps may ship a different palette; they must keep the **key names**.

### Optional keys

```yaml
header_quotes:
  - "Short one-line tagline."
  - "Another one."

colors:
  text_disabled: "#A0A4A8"
  text_inverse: "#F9FAFB"
```

- `header_quotes`: non-empty list replaces the app's built-in rotating header lines. Omit → built-ins.
- `text_disabled` / `text_inverse`: use if the UI has disabled or inverted text; otherwise skip.

Do not invent other keys in an app theme. If a new UI element needs a color, add the token here first.

### Mapping rules

**Backgrounds**

- `base_background` → app shell, header, footer
- `body_background` → main content panes (lists, trees, tables, editors)
- `modal_background` → every modal / dialog

**Text**

- `text_primary` → primary content
- `text_secondary` → muted / secondary
- `text_labels` → labels at rest
- `text_active_focus` → focused / selected labels
- `modal_labels` / `modal_text` → labels and body inside modals
- `modal_header` → section headers inside modals (F1 cards, F2 sections); bold

**Selection and chrome**

- `selected_background` → highlighted row (pair with `text_active_focus` for the row text)
- `border_default` → idle panel borders
- `border_active` → focused panel border

Derived styles every app should expose on the theme struct:

- `app_shell` = `base_background` + `text_primary`
- `active_border` = `border_active` as a border style

**Inputs and scrollbars**

- Every editable field uses `input_border_*`, `input_text_*`, and `cursor`
- Paint a 1-cell cursor overlay with `bg(cursor)` on top of the terminal cursor
- Every scrollbar uses `scrollbar` / `scrollbar_hover`

**Semantic and file roles**

- `success` / `warning` / `error` / `info` → toasts, alerts, status
- `folders` / `files` / `links` → trees, pickers, path lists (even if the app has no full file browser)

Never hard-code a color after the theme is loaded. Always pull from the theme struct.

### Theme status

- Track source: `local` / `global` / `default`
- Warn at startup (toast) if a file was skipped
- F2 Theme shows source, schema version, load status, and sampled tokens with hex

### Validation checklist (per app, before ship)

1. Local theme path loads
2. Missing local → global loads
3. Missing both → built-in defaults
4. Bad / missing `version` falls back with a visible warning
5. Every required key parses
6. Every token is mapped to a real widget (no dead names except unused file-role colors in apps with no files)
7. Loaded values win; built-in hex is only the fallback
8. Focus states use `*_focus` / `text_active_focus` / `border_active`, not the default tokens

### Anti-patterns

- Hard-coded colors in render paths after load
- One token used for unrelated intents (`warning` as body text)
- Missing focus mapping on labels, borders, or inputs
- App-specific token names that were never added here
- Persistent theme-health text in the footer

---

## 2. Shell layout

Every app uses this vertical split. Heights are fixed.

```rust
let outer = Layout::default()
    .direction(Direction::Vertical)
    .constraints([
        Constraint::Length(3), // header
        Constraint::Min(0),    // body (app-defined)
        Constraint::Length(1), // footer
    ])
    .split(frame.area());
```

- Full-screen `Block` with `app_shell` as the base layer
- Header always 3 lines (including borders)
- Footer always 1 line
- Body is whatever the product is (one pane, two panes, three panes — see §6)

Do not make header or footer height dynamic.

---

## 3. Header

### Structure

- Bordered `Block`
- `.title("<app>")` — binary / product name
- `.borders(Borders::ALL)`
- `.border_style(theme.active_border)`
- `.style(theme.app_shell)`
- Inner content: one line — the current tagline

### Taglines

Pick one string at startup (time-based seed XOR pid is fine: stable for the run, different each launch).

Each app ships its own short built-in list. Users override via `header_quotes` in the theme file.

Taglines: one line, short, match the app's personality. The standard does not mandate a shared quote list.

### Behavior

Decorative only. No mouse or keyboard. Changes only on restart.

---

## 4. Footer

### Structure

- Borderless `Paragraph` with `.style(theme.app_shell)`
- Fixed 1 line
- One width-adaptive line of key hints

### Rules

- Always start with `F1:Help`
- Next global keys: `F2:Theme`, then quit, then the app's highest-value actions
- Narrow terminals: terse abbreviations
- Wide terminals: slightly longer labels; mouse reminder only on the widest band
- Full authoritative list lives in F1 Help, not the footer
- No persistent status / theme-health / progress text. Those are toasts.

Example *shape* (keys are illustrative — replace with the app's own):

```text
# narrow
F1:Help  F2:Theme  q:Quit  j/k:Nav

# medium
F1: Help   F2: Theme   j/k: Move   Enter: Edit   q: Quit

# wide
F1: Help   F2: Theme   j/k: Move   Enter: Edit   q: Quit   (mouse: click/scroll)
```

---

## 5. Shared chrome

### F1 Help

- Centered modal, `modal_background` + `modal_text` / `modal_labels`
- Section headers use `modal_header` (bold)
- Full keyboard list + mouse list + app notes
- Text wraps. Content scrolls (keys + mouse wheel). Scrollbar when it overflows.

### F2 Theme

- Same chrome and scroll mechanics as F1
- Must show: theme source (`local` / `global` / `default`), schema version, load status, sampled tokens with hex
- Not a credits / about screen. If the app wants credits, that is a separate modal.

### Toasts

- Four semantic colors only: `success`, `warning`, `error`, `info`
- Bottom-right, auto-dismiss ~5s
- Non-blocking messages go here. Blocking errors go to a modal.

### Modals

- Centered. Clear the dimmed area underneath.
- `modal_background` + `modal_text` / `modal_labels` / `modal_header`
- Esc cancels unless the modal is an error that must be dismissed

### Inputs

- Full `input_*` + `cursor` set
- Click an input to focus it
- Tab / Shift+Tab (or Up/Down) moves between fields

### Scrollbars

- Right-edge, vertical, no decorative symbols required
- Driven by the widget's offset
- Mouse: wheel over the pane, click/drag on the bar
- Capture the pane `Rect` every frame for hit-testing

### Mouse (global)

Every interactive pane:

1. Store its `Rect` during draw
2. Wheel over the pane scrolls that pane
3. Click focuses / selects
4. Drag on the scrollbar proportional-scrolls

---

## 6. Body layout (app-defined)

The middle band is the product. This standard does **not** require a file-browser, a source/dest split, or any other app's panel set.

Shared rules for whatever you put there:

- Panes use `body_background`
- Idle border: `border_default`. Focused pane: `border_active`
- Selected row: `selected_background` + `text_active_focus`
- Titles may include counts or filter state
- Capture `*_area` rects for mouse
- Overflow → scrollbar
- Keyboard nav (`j`/`k` or arrows, `g`/`G` jump) on any list or tree

Common shapes (pick one; do not implement the others "for consistency"):

- **Single pane** — one list, editor, or log
- **Master / detail** — list or tree on the left, inspector on the right
- **Browser** — tree + secondary list (only if the product is actually a browser)

---

## 7. Optional recipes

Use these only when the product needs them. Token names stay canonical; the widget structure is a recipe, not a mandate.

### File / path coloring

- Directories → `folders`
- Regular files → `files`
- Symlinks / URL-like entries → `links`

### Tree panel

- Unicode prefixes (`├─`, `└─`, `│  `)
- `h`/`l` or arrows expand/collapse
- Enter activates the current row
- `/` filters if the list can get long
- Mouse: click row to select, click glyph zone to expand, double-click to activate, Shift+click only if the app has multi-select
- Capture the tree `Rect` every frame

Do not copy another app's `Node` / `NodeKind` / checkbox / LINK-COPY columns unless this app has that domain.

### Inspector / details pane

- Shows the current selection
- Title may include context (`Details — current item`)
- Clickable regions inside the pane should hit-test against stored rects

---

## 8. Implementation rules

- Hard-code shell heights (header 3, footer 1)
- Capture pane rects at draw time
- Footer is adaptive key hints only
- Load and validate the theme as specified
- Update this document before adding a shared visual or interaction pattern
- Ratatui snippets in this file are examples. Other frameworks must keep the same token names, heights, and behavior

---

Follow this and the apps feel like a family. Body content can differ. Chrome, color, and habits should not.
