# TDD Plan: TUI Header + Shell Consistency (LDNDDEV Visual Standard)

**Date:** 2026-06-19  
**Topic:** Make dd_siteforge header (and necessary coupled shell changes) match the header section + overall layout rules in `LDNDDEV_TUI_VISUAL_STANDARD.md`.  
**Branch:** mouse-control (already active)  
**Goal:** Fixed 3-line bordered header using project title "dd_siteforge" + random tagline (from defaults or `header_quotes` override), full-screen `app_shell` base, 1-line borderless adaptive-key footer, strict theme lookup + `version: 1` enforcement + source tracking + `header_quotes` support, move page context to Details panel, convert former `self.status` instructionals to toasts, update docs + tests. All per the standard + CLAUDE.md conventions.  
**Non-goals (out of scope for this batch):** F2 Credits modal (even though source plumbing is added), changes to other apps.

**Preconditions / invariants to maintain:**
- No code changes until this plan is committed.
- One commit per task below.
- Use `cargo check` and `cargo test -q` after relevant tasks.
- Drive behavior via `send_key` in tests where possible.
- New fields on model/theme get proper serde defaults where applicable (here for theme).
- All colors/styles from `self.theme.*` (new `app_shell` / `active_border` convenience Styles added to match the standard document language for easy porting).
- Header is purely decorative: no interaction, chosen once at startup using time ^ pid, only changes on restart.
- Footer always starts with `F1:Help`, width-adaptive, no persistent long status text.
- Hard-coded shell heights: header `Length(3)`, footer `Length(1)`.
- Use correct project spelling `dd_siteforge_theme.yml` (user confirmed typo in query response).
- Update `Architecture.md` and `CLAUDE.md`.
- Add/update tests.
- Taglines provided by user (exact):
  1. "Drafts are just commits that lost their nerve."
  2. "Saved. Probably. Hopefully. Definitely. (It saved.)."
  3. "This post is live, which means it's officially out of your hands."
  4. "Scheduled for later — future you can deal with the typos."
  5. "Deleted. We won't talk about it again. (We both saw it.)"

**Randomization helper (to be added):**
Use `SystemTime` unix secs XOR `std::process::id()` (as u64), then `% len`.

**Footer adaptive strings (approved "these or very similar"; using close variants for fit/accuracy):**
- <75 cols: `"F1:Help  q:Quit  s:Save  /:Insert  Enter:Edit  j/k:Nav  Spc:Toggle  1/2/3:Focus"`
- <110 cols: `"F1: Help   s: Save   /: Insert   Enter: Edit   Tab: Switch page   Shift+E: Export   p: Preview   q: Quit"`
- else: `"F1: Help   s: Save   /: Insert component   Enter: Edit   Tab/Shift+Tab: switch page   Shift+E: Export   p: Preview   F3: Validate   Ctrl+Q: Quit   (mouse: click/scroll/drag)"`

**Tagline alignment inside header:** left (default Paragraph).

**Theme load warning policy:** only surface visible Warning toast on version issues, parse errors, or fallback (not on clean local/global loads). Source tracked on App for future-proofing.

**Commit style:** plain prefixes (`tui:`, `docs:`, `test:`). Co-author trailer for AI if applicable.

After the plan commit, implement task-by-task. Each task = minimal complete change + check/test + commit.

---

## Task 1: Extend theme types + implement strict load with version + header_quotes (tui.rs)

**Goal:** Update `ThemeFile`, `AppTheme` (add `app_shell`, `active_border`, `header_quotes`), `from_palette`, `Default`, add `default_header_quotes`, rewrite `load()` (and remove old candidates fn usage) to follow exact lookup + version:1 enforcement + pick quotes (file wins only if non-empty after version ok). Always succeeds, returns source + optional warning.

**Exact changes:**

1. Add fields to `struct AppTheme` (after `input_focus: Color,`):

```rust
    app_shell: Style,
    active_border: Style,
    header_quotes: Vec<String>,
```

2. Update `struct ThemeFile`:

```rust
#[derive(Debug, Deserialize)]
struct ThemeFile {
    #[serde(default)]
    version: Option<u32>,
    #[serde(default)]
    header_quotes: Vec<String>,
    colors: PaletteFile,
}
```

3. Add the default quotes helper (place it near `parse_hex_color`, before or after `theme_file_candidates` removal):

```rust
fn default_header_quotes() -> Vec<String> {
    vec![
        "Drafts are just commits that lost their nerve.".to_string(),
        "Saved. Probably. Hopefully. Definitely. (It saved.).".to_string(),
        "This post is live, which means it's officially out of your hands.".to_string(),
        "Scheduled for later — future you can deal with the typos.".to_string(),
        "Deleted. We won't talk about it again. (We both saw it.)".to_string(),
    ]
}
```

4. Rewrite `impl AppTheme` load / from_palette / default. Full replacement for the load-related functions (approx lines 17827-18026 area):

```rust
impl AppTheme {
    fn load() -> (Self, String, Option<String>) {
        let candidates: Vec<(PathBuf, &'static str)> = {
            let mut c = vec![
                (PathBuf::from("dd_siteforge_theme.yml"), "local"),
            ];
            if let Some(home) = std::env::var_os("HOME") {
                let base = Path::new(&home).join(".config").join("ldnddev");
                c.push((base.join("dd_siteforge_theme.yml"), "global"));
            }
            c
        };

        let mut warning: Option<String> = None;

        for (path, src) in candidates {
            if !path.exists() {
                continue;
            }
            let raw = match std::fs::read_to_string(&path) {
                Ok(r) => r,
                Err(e) => {
                    warning = Some(format!("could not read '{}': {}", path.display(), e));
                    continue;
                }
            };
            let theme_file: ThemeFile = match serde_yaml::from_str(&raw) {
                Ok(f) => f,
                Err(e) => {
                    warning = Some(format!("invalid theme file '{}': {}", path.display(), e));
                    continue;
                }
            };

            // Strict version enforcement per LDNDDEV_TUI_VISUAL_STANDARD.md
            match theme_file.version {
                Some(1) => {}
                Some(v) => {
                    warning = Some(format!(
                        "theme '{}' declares version {} (expected 1); using built-in defaults",
                        path.display(),
                        v
                    ));
                    continue;
                }
                None => {
                    warning = Some(format!(
                        "theme '{}' is missing required 'version: 1'; using built-in defaults",
                        path.display()
                    ));
                    continue;
                }
            }

            let quotes = if !theme_file.header_quotes.is_empty() {
                theme_file.header_quotes
            } else {
                default_header_quotes()
            };

            match Self::from_palette(theme_file.colors, quotes) {
                Ok(t) => return (t, src.to_string(), warning),
                Err(e) => {
                    warning = Some(format!(
                        "theme '{}' color parse error: {}; using defaults",
                        path.display(),
                        e
                    ));
                    continue;
                }
            }
        }

        // Built-in fallback
        (Self::default(), "default".to_string(), warning)
    }

    fn from_palette(
        p: PaletteFile,
        header_quotes: Vec<String>,
    ) -> anyhow::Result<Self> {
        // (existing core background/text/selection/border/scrollbar/input/accent/semantic/file color parsing unchanged up to `let links = ...`)

        let links = parse_hex_color(p.links.as_deref().unwrap_or("#ffa087"))?;

        let app_shell = Style::default()
            .bg(background)
            .fg(foreground);
        let active_border = Style::default().fg(border_active);

        Ok(Self {
            background,
            panel_background,
            popup_background,
            foreground,
            muted,
            disabled,
            text_inverse,
            text_labels,
            text_active_focus,
            modal_labels,
            modal_text,
            title,
            active,
            border,
            border_active,
            input_border_default,
            input_border_focus,
            input_text_default,
            input_text_focus,
            cursor,
            scrollbar,
            scrollbar_hover,
            selected_background,
            selected_foreground: foreground,
            success,
            warning,
            error,
            info,
            folders,
            files,
            links,
            input_default,
            input_focus,
            app_shell,
            active_border,
            header_quotes,
        })
    }
}

impl Default for AppTheme {
    fn default() -> Self {
        let border_def = Color::Rgb(245, 246, 247);
        let border_focus = Color::Rgb(100, 180, 245);
        Self {
            background: Color::Rgb(15, 17, 20),
            panel_background: Color::Rgb(42, 45, 49),
            popup_background: Color::Rgb(28, 30, 33),
            foreground: Color::Rgb(245, 246, 247),
            muted: Color::Rgb(158, 163, 170),
            disabled: Color::Rgb(90, 95, 102),
            text_inverse: Color::Rgb(15, 17, 20),
            text_labels: Color::Rgb(255, 175, 70),
            text_active_focus: border_focus,
            modal_labels: border_focus,
            modal_text: Color::Rgb(245, 246, 247),
            title: border_focus,
            active: Color::Rgb(110, 200, 255),
            border: border_def,
            border_active: border_focus,
            input_border_default: border_def,
            input_border_focus: border_focus,
            input_text_default: Color::Rgb(245, 246, 247),
            input_text_focus: border_focus,
            cursor: border_focus,
            scrollbar: Color::Rgb(255, 160, 135),
            scrollbar_hover: border_focus,
            selected_background: Color::Rgb(15, 17, 20),
            selected_foreground: Color::Rgb(245, 246, 247),
            success: Color::Rgb(130, 224, 170),
            warning: Color::Rgb(245, 196, 105),
            error: Color::Rgb(229, 115, 115),
            info: Color::Rgb(93, 173, 226),
            folders: Color::Rgb(100, 180, 245),
            files: Color::Rgb(255, 175, 70),
            links: Color::Rgb(255, 160, 135),
            input_default: border_def,
            input_focus: border_focus,
            app_shell: Style::default()
                .bg(Color::Rgb(15, 17, 20))
                .fg(Color::Rgb(245, 246, 247)),
            active_border: Style::default().fg(border_focus),
            header_quotes: default_header_quotes(),
        }
    }
}
```

5. Delete (or comment out and leave for now) the old `fn theme_file_candidates()` body/usage. (It will become dead after load rewrite.)

6. Remove any old direct calls to the old `from_palette` (none outside load).

**Verification:**
- `cargo check`
- In a test or manually, confirm `AppTheme::default().header_quotes.len() == 5`

**Commit message:** `tui: add app_shell/active_border/header_quotes + strict versioned load to AppTheme per LDNDDEV standard`

---

## Task 2: Wire theme source + header_copy into App; remove status field + "Ready." init (tui.rs)

**Goal:** Add `theme_source: String, header_copy: String` to `App`. Update `App::new` signature + body (compute copy using helper, init new fields). Remove `status: String` entirely from struct + new + all direct display use (display removal in later task). Add the choose helper.

**Exact changes:**

1. In `struct App` (around line 74, after `theme: AppTheme,`):

```rust
    theme: AppTheme,
    theme_source: String,
    header_copy: String,
```

(Remove the `status: String,` line entirely later in same edit pass.)

2. Add choose helper (near `default_header_quotes` or other small fns):

```rust
fn choose_header_copy(quotes: &[String]) -> String {
    if quotes.is_empty() {
        return "Drafts are just commits that lost their nerve.".to_string();
    }
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
        ^ (std::process::id() as u64);
    quotes[(seed as usize) % quotes.len()].clone()
}
```

3. Change `fn new` signature + body (around 4281):

```rust
    fn new(mut site: Site, path: Option<PathBuf>, theme: AppTheme, theme_source: String) -> Self {
        for page in &mut site.pages {
            ensure_page_section_ids(page);
        }
        let last_saved_json = serde_json::to_string(&site).unwrap_or_default();

        let header_copy = choose_header_copy(&theme.header_quotes);

        let mut app = Self {
            site,
            theme,
            theme_source,
            header_copy,
            // ... all other fields same, but WITHOUT the old status line ...
            deleted_pages: Vec::new(),
            // ...
            // status line deleted
            // ...
            header_column_expanded: true,
            dirty: false,
            dirty_since: None,
            last_saved_json,
        };
        // existing backup toast logic stays (it uses push_toast)
        if let Some(p) = app.path.as_ref() {
            // ... unchanged ...
        }
        app
    }
```

4. Update the `App` struct literal init sites inside new if any other copies (none).

**Verification:** `cargo check`

**Commit message:** `tui: add theme_source + header_copy to App, remove status field, implement choose_header_copy`

---

## Task 3: Update run_tui + all App::new call sites (tui.rs + tests)

**Goal:** Adapt the theme load call site in `run_tui`. Update every `App::new(..., AppTheme::default())` (and the one with json_path) to pass the 4th arg `"default".to_string()`. (There are ~30 in the big `#[cfg(test)]` block.)

**Exact changes:**

1. In `pub fn run_tui` (top of file, ~29):

```rust
pub fn run_tui(site: Site, path: Option<PathBuf>) -> anyhow::Result<()> {
    let (theme, theme_source, load_warning) = AppTheme::load();

    enable_raw_mode()?;
    // ... stdout setup unchanged ...

    let mut app = App::new(site, path, theme, theme_source);
    if let Some(msg) = load_warning {
        app.push_toast(ToastLevel::Warning, msg);
    }
    let run_res = app.run(&mut terminal);
    // ... rest unchanged ...
}
```

(Old match/err eprintln path is now fully handled inside load() + toast.)

2. Global search in the test module for `App::new(Site::starter(), None, AppTheme::default())` and similar with `Some(json_path.clone()), AppTheme::default()` — change every one to pass `, "default".to_string()` as 4th arg.

   Representative before/after (repeat for all ~30 occurrences; use unique context for each search_replace if strings collide):

```rust
// before
let mut app = App::new(Site::starter(), None, AppTheme::default());
// after
let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string());
```

   Same for the ones with path:
```rust
let mut app = App::new(Site::starter(), Some(json_path.clone()), AppTheme::default(), "default".to_string());
```

   (Exact count from earlier greps: many in the 186xx-202xx range.)

**Verification:** `cargo check` (should be clean; tests will still compile).

**Commit message:** `tui: wire new App::new signature through run_tui and all test sites`

---

## Task 4: Rewrite main draw layout + render new header + new footer + move page info to Details (tui.rs)

**Goal:** At start of `draw`:
- Render full-screen base `Block` with `app_shell`.
- Change root Layout to Length(3) / Min(0) / Length(1).
- Render the bordered 3-line header Block (title "dd_siteforge", active_border, app_shell, title_style) + left-aligned quote Paragraph from `header_copy` in its inner.
- Update the Details panel title (around the "Details" block) to include page context e.g. "Details — 03: Home".
- At the bottom, replace the entire old 3-line bordered "Status" + long `footer_text + status` with a single borderless 1-line Paragraph using the adaptive `footer_text` + `.style(self.theme.app_shell)`.
- Remove the old 1-line top "dd | Page: ..." render code.

**Exact changes (large but localized to draw):**

Replace the beginning of `fn draw` (current ~4386-4410 area) with:

```rust
    fn draw(&mut self, frame: &mut ratatui::Frame) {
        self.prune_toasts();
        self.multiline_value_area = None;

        // Full-screen app shell base layer (base_background + text_primary).
        frame.render_widget(
            Block::default().style(self.theme.app_shell),
            frame.area(),
        );

        let page = self.current_page();
        let root = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // HEADER (fixed, per LDNDDEV standard)
                Constraint::Min(0),    // Main content area
                Constraint::Length(1), // FOOTER (fixed, decluttered keys only)
            ])
            .split(frame.area());
        let main = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
            .split(root[1]);

        // === NEW 3-line header (bordered, title=project, content=header_copy) ===
        let header_block = Block::default()
            .title("dd_siteforge")
            .borders(Borders::ALL)
            .border_style(self.theme.active_border)
            .style(self.theme.app_shell)
            .title_style(
                Style::default()
                    .fg(self.theme.title)
                    .add_modifier(Modifier::BOLD),
            );
        frame.render_widget(header_block.clone(), root[0]);

        if root[0].height >= 3 {
            let inner = header_block.inner(root[0]);
            let quote = Paragraph::new(self.header_copy.as_str()).style(
                Style::default()
                    .fg(self.theme.foreground)
                    .bg(self.theme.background),
            );
            frame.render_widget(quote, inner);
        }
        // (no mouse/keyboard capture for header — purely decorative)
```

Then, **later in the same draw** (around the old details block ~4624), change the title:

```rust
        let page_idx = self.selected_page + 1;
        let page_label = if page.head.title.trim().is_empty() {
            page.slug.as_str()
        } else {
            page.head.title.as_str()
        };
        let details_title = format!("Details — {:02}: {}", page_idx, page_label);

        let details = Paragraph::new(details_content)
            ...
            .block(
                Block::default()
                    .title(details_title)   // <--- changed
                    .borders(Borders::ALL)
                    ...
```

Finally, **replace the old bottom status block** (the `let footer_text = format!( "F1 help | ... | {}", self.status); ... frame.render_widget(footer, root[2]);` block around 4695-4721) with the new adaptive 1-line footer:

```rust
        // === NEW 1-line adaptive footer (borderless, keys only, app_shell, always F1:Help first) ===
        let footer_text = if root[2].width < 75 {
            "F1:Help  q:Quit  s:Save  /:Insert  Enter:Edit  j/k:Nav  Spc:Toggle  1/2/3:Focus"
        } else if root[2].width < 110 {
            "F1: Help   s: Save   /: Insert   Enter: Edit   Tab: Switch page   Shift+E: Export   p: Preview   q: Quit"
        } else {
            "F1: Help   s: Save   /: Insert component   Enter: Edit   Tab/Shift+Tab: switch page   Shift+E: Export   p: Preview   F3: Validate   Ctrl+Q: Quit   (mouse: click/scroll/drag)"
        };
        let footer = Paragraph::new(footer_text).style(self.theme.app_shell);
        frame.render_widget(footer, root[2]);
```

Delete the old top 1-line header render (the `let header_text = format!("dd | Page: {}"...); frame.render_widget(header, root[0]);` that we replaced).

**Verification:** `cargo check`

**Commit message:** `tui: draw new 3-line header + 1-line footer + full app_shell base + Details page context (LDNDDEV shell)`

---

## Task 5: Convert all `self.status =` to toasts + clean up the one test assert + related comment (tui.rs)

**Goal:** Every former instructional `self.status =` now becomes a `push_toast` (Info for normal instructions, Warning for guards/failures like "must keep", "Title required", "Failed to save", Success for mutations like "Item saved", "Inserted", "Moved"). Existing explicit `push_toast` calls are untouched. Remove all references to the deleted field. Update the single test that read `app.status`.

**Process (exact for agent):**
- Use `grep -n 'self\.status =' src/tui.rs` (or the tool) repeatedly.
- For each unique assignment, use `search_replace` with a sufficiently unique `old_string` containing the line + surrounding context to make it unique.
- Examples of the mapping (apply similar for all):

Example 1 (simple instruction):
```rust
// before
self.status = "Rename page. Edit and press Enter.".to_string();
// after
self.push_toast(ToastLevel::Info, "Rename page. Edit and press Enter.");
```

Example 2 (format + guard failure):
```rust
// before (in min guard)
self.status = format!("Must keep at least {min_items} item(s).");
// after
self.push_toast(ToastLevel::Warning, format!("Must keep at least {min_items} item(s)."));
```

Example 3 (success mutation):
```rust
// before
self.status = "Item saved — editing parent.".to_string();
// after
self.push_toast(ToastLevel::Success, "Item saved — editing parent.");
```

Example 4 (failed save):
```rust
// before
self.status = format!("Failed to save: {}", e);
// after
self.push_toast(ToastLevel::Warning, format!("Failed to save: {}", e));
```

- "Editing ..." / "Switched to ..." / "Press Enter to edit." → Info
- "No ... selected." / "Title required." / "Cancelled." → Warning or Info (prefer Warning when it was a refusal)
- "Inserted ...", "Moved ...", "Deleted node ..." → Success
- "Save prompt opened." etc. → Info

- After all replacements, `cargo check` will show any missed `self.status` reads/writes → fix the last ones.
- Update the test (around 18694):

```rust
        send_key(&mut app, KeyCode::Char('X'), KeyModifiers::SHIFT);
        assert_eq!(selected_card(&app).items.len(), 1);
        // was: assert!(app.status.contains("must keep at least one item"));
        let last = app.toasts.last().expect("expected a warning toast for min guard");
        assert!(last.message.contains("must keep at least one item"));
        assert_eq!(last.level, ToastLevel::Warning);
```

- Update the comment that mentioned status (around 4047): change to reflect toasts now carry the messages.

- Delete the now-unused `status: String,` declaration and its init (already done in Task 2, but verify zero references remain).

**Verification:** `cargo test -q --test tui` or full `cargo test -q` (the affected test must still pass via toast).

**Commit message:** `tui: replace all self.status instructionals with toasts; remove status field usage; update affected test`

---

## Task 6: Update the project's theme file (dd_siteforge_theme.yml)

**Goal:** Add `version: 1` at top. Add commented `header_quotes` guidance. Rewrite the lookup order comment block to exactly match the new standard (and the load code).

**Exact new top of file:**

```yaml
# dd_siteforge TUI Theme Configuration
# Lookup order (per LDNDDEV_TUI_VISUAL_STANDARD.md):
# 1. ./dd_siteforge_theme.yml (local override)
# 2. ~/.config/ldnddev/dd_siteforge_theme.yml (global)
# 3. Built-in defaults inside the app
#
# All files must declare:
# version: 1
#
# Optional top-level header_quotes (non-empty list replaces built-in taglines):
# header_quotes:
#   - "Your first override."
#   - "Second override."
#
# Colors follow the canonical tokens. Do not invent new ones.

version: 1

# header_quotes: []   # uncomment + populate to customize the rotating header banner
colors:
  # ... rest of file unchanged ...
```

**Verification:** The file still parses (will be exercised by load in tests).

**Commit message:** `docs: update dd_siteforge_theme.yml for version + header_quotes + strict lookup`

---

## Task 7: Update Architecture.md

**Goal:** Revise the TUI layout description and the entire Theme section to describe the new standard shell + header + footer + theme load rules. Mention `header_copy`, randomization, `app_shell`/`active_border`, Details panel now carries page context, status messages via toasts.

**Exact additions/changes (high level + snippets):**

- In TUI Loop / chrome description: replace old layout notes with:
  "Shell is always fixed: 3-line top header (bordered Block title=\"dd_siteforge\" + random tagline from defaults or theme `header_quotes`), main 50/25/75 panels, 1-line bottom footer (borderless adaptive key hints always starting F1:Help). Full app_shell base Block is rendered first."

- Replace the old Theme section (~105-109) with:

```markdown
## Theme + Visual Shell

Theme load is strict (LDNDDEV_TUI_VISUAL_STANDARD.md):
1. `./dd_siteforge_theme.yml` (local)
2. `~/.config/ldnddev/dd_siteforge_theme.yml` (global)
3. Built-in defaults

Every theme file must contain `version: 1` at the top level (validated on load; bad/missing version falls back with a Warning toast).

`header_quotes` (optional top-level list) overrides the 5 built-in rotating header taglines (chosen once at App::new using time ^ pid).

The TUI now exposes `theme.app_shell` and `theme.active_border` (Style) for the standard header/footer.

See `src/tui.rs` (draw, AppTheme::load, choose_header_copy, default_header_quotes) for the concrete implementation.
The Details panel title now includes current page context ("Details — 03: Home").
Former persistent status messages are delivered as toasts.
```

- Keep other accurate parts of Architecture (autosave, testing, etc.).

**Verification:** Read the file; no code impact.

**Commit message:** `docs: update Architecture.md for new LDNDDEV header/shell layout + theme rules`

---

## Task 8: Update CLAUDE.md

**Goal:** Update the "User preferences" bullet about status bar (now footer is keys-only per visual standard; instructionals use toasts). Update any theme lookup references. Mention LDNDDEV_TUI_VISUAL_STANDARD.md as the governing doc for shell.

**Exact edits:**

In the "User preferences captured during the v1.0 push" section:

- Change the status bar bullet to:
  "Footer is now a strict 1-line adaptive key-hint bar (borderless, app_shell style, always starts F1:Help; no long status text). Instructional / in-progress messages use toasts (Info level)."

- In the Theme tokens or "What lives where" section, add a note:
  "Shell layout and header/footer follow LDNDDEV_TUI_VISUAL_STANDARD.md (3-line header with app name + tagline, 1-line footer). Theme loading also follows the standard (version + header_quotes + strict two-file + default lookup)."

- Update the old THEME_STRUCTURE_STANDARD.md references if they appear in context of loading (they do in a couple places).

**Verification:** `git diff` on the file.

**Commit message:** `docs: update CLAUDE.md for new footer semantics + LDNDDEV shell standard`

---

## Task 9: Add / update tests for the new header behavior (tui.rs)

**Goal:** Add at least one dedicated test that the header_copy is chosen from the default list. Optionally strengthen an existing integration test to also assert something about the new layout (e.g. that after new, header_copy is present). Keep all existing tests passing.

**Exact test to add** (place inside the `#[cfg(test)] mod tests` block, near other App construction tests):

```rust
    #[test]
    fn app_selects_header_copy_from_defaults_at_construction() {
        let app = App::new(
            Site::starter(),
            None,
            AppTheme::default(),
            "default".to_string(),
        );
        let defs = default_header_quotes();
        assert!(
            defs.iter().any(|q| q == &app.header_copy),
            "header_copy '{}' should be one of the defaults",
            app.header_copy
        );
        assert_eq!(app.theme_source, "default");
    }
```

(Also update the one status->toast test from Task 5 if not already landed.)

If `default_header_quotes` is not visible, it is (same module).

**Verification:** `cargo test -q` (the new test + all prior ones must pass; the card min-guard test must now pass via toasts).

**Commit message:** `test: add header_copy selection test + ensure status->toast conversions are covered`

---

## Task 10: Full verification + any final cleanups

**Steps (run in order):**
1. `cargo check`
2. `cargo test -q`
3. If any warnings or the old `theme_file_candidates` fn is now dead, remove it (and its old comment).
4. Optionally: `cargo run -- tui /tmp/scratch-header-test.json` (interactive smoke — verify 3-line header with one of the 5 taglines appears at top with blue active border, 1-line keys at bottom, Details title has page number+name, theme load toast if you temporarily corrupt the yml version).
5. `git status` — only expected files changed.
6. If anything left (old root[0] references? unlikely), fix.

**Commit message (only if extra cleanups needed):** `tui: final cleanups after header shell changes`

If no extra changes, the Task 9 commit is the last.

---

**Post-plan notes for implementer:**
- This plan is self-contained; a fresh agent should be able to follow task-by-task using only the blocks + `cargo` commands + grep for remaining `self.status`.
- After all commits on the branch, follow normal "when the user says ship" flow (smoke, fast-forward merge if needed, tag, etc.).
- The visual result: consistent ldnddev family header across apps.

**End of plan.**
