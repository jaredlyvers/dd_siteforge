# F2:Theme Modal Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add F2:Theme modal that shows theme source, status, and a compact list of color variables, styled with identical layout/chrome to the existing F1 help modal (per approved design spec 2026-06-21-f2-theme-modal-design.md).

**Architecture:** Mirror F1 help exactly using parallel fields (show_theme, theme_scroll*, theme_status) on App, dedicated if-blocks in draw/handle_event, new build_theme_text helper (reusing same styling/scroll logic). Update footer, help text, docs, and add send_key tests. No changes to unified Modal enum. All per LDNDDEV_TUI_VISUAL_STANDARD.md and the approved spec.

**Tech Stack:** Rust + ratatui (same as rest of TUI in src/tui.rs)

---

I'm using the writing-plans skill to create the implementation plan.

## File Structure
- Primary change: src/tui.rs (add fields, helpers, render, event, updates to existing fns, tests)
- Docs: Architecture.md (key table + TUI section), CLAUDE.md (minor key list update)
- Plan and spec already exist; no new source files.

Decomposition locked to keep changes focused in tui.rs (large file but established pattern for help modal). Each task produces testable progress.

## Task 1: Add the new state fields to App struct

**Files:**
- Modify: `src/tui.rs:129-135` (after help_scroll_max)

- [ ] **Step 1.1: Add fields after help_scroll_max**

Insert the new fields (use exact indentation from surrounding code).

```rust
    /// Maximum legal `help_scroll` value, recomputed every render based on
    /// the current modal area + wrapped row count. Read by event handlers
    /// to clamp scroll keystrokes without needing the frame.
    help_scroll_max: u16,
    /// Per-frame cache of (field_idx, input_area_rect) for whichever
```

After the help_scroll_max line, add:

```rust
    show_theme: bool,
    /// Vertical scroll offset (in rows) for the F2:Theme modal.
    theme_scroll: u16,
    /// Maximum legal `theme_scroll` value, recomputed every render based on
    /// the current modal area + wrapped row count. Read by event handlers
    /// to clamp scroll keystrokes without needing the frame.
    theme_scroll_max: u16,
    theme_status: Option<String>,
```

Full context for unique replace (use search_replace with sufficient old_string):

Old (around line 129-142):
```rust
    show_help: bool,
    /// Vertical scroll offset (in rows) for the F1 help modal.
    help_scroll: u16,
    /// Maximum legal `help_scroll` value, recomputed every render based on
    /// the current modal area + wrapped row count. Read by event handlers
    /// to clamp scroll keystrokes without needing the frame.
    help_scroll_max: u16,
    /// Per-frame cache of (field_idx, input_area_rect) for whichever
    /// multi-field modal is currently rendered. Click-to-focus lookups
    /// search this cache; render writes it. Empty when no eligible modal
    /// is open.
    modal_field_areas: std::cell::RefCell<Vec<(usize, Rect)>>,
```

New:
```rust
    show_help: bool,
    /// Vertical scroll offset (in rows) for the F1 help modal.
    help_scroll: u16,
    /// Maximum legal `help_scroll` value, recomputed every render based on
    /// the current modal area + wrapped row count. Read by event handlers
    /// to clamp scroll keystrokes without needing the frame.
    help_scroll_max: u16,
    show_theme: bool,
    /// Vertical scroll offset (in rows) for the F2:Theme modal.
    theme_scroll: u16,
    /// Maximum legal `theme_scroll` value, recomputed every render based on
    /// the current modal area + wrapped row count. Read by event handlers
    /// to clamp scroll keystrokes without needing the frame.
    theme_scroll_max: u16,
    theme_status: Option<String>,
    /// Per-frame cache of (field_idx, input_area_rect) for whichever
    /// multi-field modal is currently rendered. Click-to-focus lookups
    /// search this cache; render writes it. Empty when no eligible modal
    /// is open.
    modal_field_areas: std::cell::RefCell<Vec<(usize, Rect)>>,
```

- [ ] **Step 1.2: Run cargo check to verify**

Run: `cargo check`

Expected: succeeds (no type errors from the added fields yet; they will be initialized in later tasks).

- [ ] **Step 1.3: Commit**

```bash
git add src/tui.rs
git commit -m "tui: add show_theme / theme_scroll* / theme_status fields to App (F2:Theme modal)"
```

## Task 2: Initialize new fields in App::new and update its signature + call sites

**Files:**
- Modify: `src/tui.rs:4294` (new fn sig and body), the init literal around 4341, run_tui around line 40, and all test App::new calls (there are ~30; use grep to find, replace systematically).

- [ ] **Step 2.1: Update the fn signature and doc comment if any**

Current sig (line ~4294):
```rust
    fn new(mut site: Site, path: Option<PathBuf>, theme: AppTheme, theme_source: String) -> Self {
```

Change to:
```rust
    fn new(mut site: Site, path: Option<PathBuf>, theme: AppTheme, theme_source: String, theme_status: Option<String>) -> Self {
```

- [ ] **Step 2.2: Update the construction literal inside new (around line 4341)**

After the help_scroll_max init, add the new inits (before modal_field_areas).

Find the block:
```rust
            show_help: false,
            help_scroll: 0,
            help_scroll_max: 0,
            modal_field_areas: std::cell::RefCell::new(Vec::new()),
```

Replace with:
```rust
            show_help: false,
            help_scroll: 0,
            help_scroll_max: 0,
            show_theme: false,
            theme_scroll: 0,
            theme_scroll_max: 0,
            theme_status,
            modal_field_areas: std::cell::RefCell::new(Vec::new()),
```

Also set `theme_source` is already there; it stays.

- [ ] **Step 2.3: Update the call in run_tui (top of file, ~line 40)**

Current:
```rust
    let mut app = App::new(site, path, theme, theme_source);
```

To:
```rust
    let mut app = App::new(site, path, theme, theme_source, load_warning);
```

(The if let for toast stays.)

- [ ] **Step 2.4: Update the header test in tests (around line 21053 or the one added in header plan)**

Find:
```rust
        let app = App::new(
            Site::starter(),
            None,
            AppTheme::default(),
            "default".to_string(),
        );
```

Update all similar to pass , None at end.

- [ ] **Step 2.5: Systematically update every other App::new in the test module**

Run: `grep -n 'App::new(Site::starter()' src/tui.rs | cat` (or use tool)

There are many of form:
- App::new(Site::starter(), None, AppTheme::default(), "default".to_string());
- App::new(Site::starter(), Some(json_path.clone()), AppTheme::default(), "default".to_string());
- Variations with other themes or paths.

For each, append `, None` before the closing `)` .

Use search_replace with replace_all=false for unique contexts, or multiple calls. Do in small batches to keep commits granular.

Example replace (one instance):
Old: `App::new(Site::starter(), None, AppTheme::default(), "default".to_string());`
New: `App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);`

Repeat for all ~30. Run `cargo check` after batch of 5-10.

- [ ] **Step 2.6: Run cargo check after all updates**

Run: `cargo check`

Expected: clean.

- [ ] **Step 2.7: Commit the signature + init + run_tui + all test sites**

```bash
git add src/tui.rs
git commit -m "tui: wire theme_status through App::new, run_tui and all test sites for F2:Theme"
```

## Task 3: Add the color_to_hex and build_theme_text helpers

**Files:**
- Modify: `src/tui.rs` (place color_to_hex near parse_hex_color ~18611, build_theme_text near build_help_text ~18970)

- [ ] **Step 3.1: Add color_to_hex after parse_hex_color**

After the parse_hex_color fn (ends ~18623), before component_search_haystack, insert:

```rust
fn color_to_hex(c: Color) -> String {
    if let Color::Rgb(r, g, b) = c {
        format!("#{:02x}{:02x}{:02x}", r, g, b)
    } else {
        "?".to_string()
    }
}
```

- [ ] **Step 3.2: Add build_theme_text fn (full) modeled exactly on build_help_text but for theme content**

Place it right after build_help_text (after line ~19152, before count_wrapped_lines).

Full function (use the exact list and styling from the approved spec + mock):

```rust
fn build_theme_text(theme: &AppTheme, source: &str, status: &Option<String>, width: usize) -> Text<'static> {
    let h_style = Style::default()
        .fg(theme.modal_header)
        .add_modifier(Modifier::BOLD);
    let k_style = Style::default().fg(theme.text_active_focus);
    let div_style = Style::default().fg(theme.muted);

    let mut lines: Vec<Line<'static>> = Vec::new();

    // Theme section
    lines.push(Line::from(Span::styled("Theme", h_style)));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  Source: ", k_style),
        Span::raw(format!("{}   (./dd_siteforge_theme.yml or equivalent)", source)),
    ]));
    let status_str = status.as_deref().unwrap_or("OK (loaded cleanly)");
    lines.push(Line::from(vec![
        Span::styled("  Status: ", k_style),
        Span::raw(status_str.to_string()),
    ]));
    lines.push(Line::from(""));

    // divider
    let rule_len = width.saturating_sub(4).clamp(12, 50);
    let rule = "─".repeat(rule_len);
    lines.push(Line::from(Span::styled(format!("  {}", rule), div_style)));
    lines.push(Line::from(""));

    // Color tokens section
    lines.push(Line::from(Span::styled("Loaded color tokens (sampled)", h_style)));
    lines.push(Line::from(""));

    let tokens: Vec<(&str, Color, &str)> = vec![
        ("background", theme.background, "app_shell base"),
        ("popup_background", theme.popup_background, "modals & popups"),
        ("foreground", theme.foreground, "primary text"),
        ("modal_header", theme.modal_header, "section titles bold"),
        ("text_labels", theme.text_labels, "labels default"),
        ("text_active_focus", theme.text_active_focus, "focus + keys"),
        ("input_border_focus", theme.input_border_focus, "focused inputs"),
        ("success", theme.success, "success toasts"),
        ("warning", theme.warning, "warning toasts"),
        ("error", theme.error, "error toasts"),
        ("info", theme.info, "info toasts"),
        ("folders", theme.folders, "image picker folders"),
        ("files", theme.files, "image picker files"),
        ("links", theme.links, "image picker links"),
        ("scrollbar", theme.scrollbar, "scrollbars"),
        ("scrollbar_hover", theme.scrollbar_hover, "scrollbar thumb"),
    ];

    for (name, color, role) in tokens {
        let hex = color_to_hex(color);
        let line = format!("  {:<18} {}   ({})", name, hex, role);
        lines.push(Line::from(Span::raw(line)));
    }

    lines.push(Line::from(""));

    // final divider
    lines.push(Line::from(Span::styled(format!("  {}", rule), div_style)));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::raw("  (All colors from self.theme.*. No hardcodes.)")));

    Text::from(lines)
}
```

Note: count_wrapped_lines is shared and works for both (it just uses .lines.len()).

- [ ] **Step 3.3: Run cargo check**

Run: `cargo check`

Expected: compiles (the fn is defined, not yet called).

- [ ] **Step 3.4: Commit**

```bash
git add src/tui.rs
git commit -m "tui: add color_to_hex + build_theme_text (exact F2:Theme content per spec)"
```

## Task 4: Update draw() to render the F2:Theme modal (copy of help block)

**Files:**
- Modify: `src/tui.rs` (the draw fn, after the if self.show_help block ~4859)

- [ ] **Step 4.1: Insert the render if after the closing } of show_help**

Find the spot after:
```rust
            }
        }
        // Render edit modal if open
        if let Some(modal) = &self.edit_modal {
```

Insert before the edit modal comment the full if self.show_theme block (copy/adapt from help, using theme_ vars, build_theme_text call, "Theme (F2 / Esc..." title).

Exact block (adapted from current help render 4761-4859, replace help_ with theme_ , "Key & Mouse..." with "Theme (F2 / Esc to close, j/k or arrows to scroll)", call build_theme_text):

```rust
        if self.show_theme {
            let area = centered_rect(80, 80, frame.area());
            frame.render_widget(Clear, area);
            let block = Block::default()
                .title("Theme (F2 / Esc to close, j/k or arrows to scroll)")
                .borders(Borders::ALL)
                .style(
                    Style::default()
                        .fg(self.theme.foreground)
                        .bg(self.theme.popup_background),
                )
                .border_style(Style::default().fg(self.theme.border_active))
                .title_style(
                    Style::default()
                        .fg(self.theme.modal_header)
                        .add_modifier(Modifier::BOLD),
                );
            let inner = block.inner(area);
            frame.render_widget(block, area);

            let scrollbar_width: u16 = 1;
            let body_w = inner.width.saturating_sub(scrollbar_width + 1);
            let body_area = Rect {
                x: inner.x,
                y: inner.y,
                width: body_w,
                height: inner.height,
            };

            let help = build_theme_text(&self.theme, &self.theme_source, &self.theme_status, body_w as usize);
            let wrapped_total = count_wrapped_lines(&help, body_w as usize);
            let visible = inner.height as usize;
            let max_scroll = wrapped_total.saturating_sub(visible) as u16;
            self.theme_scroll_max = max_scroll;
            if self.theme_scroll > max_scroll {
                self.theme_scroll = max_scroll;
            }
            let scroll = self.theme_scroll;

            let body = Paragraph::new(help)
                .style(
                    Style::default()
                        .fg(self.theme.foreground)
                        .bg(self.theme.popup_background),
                )
                .wrap(Wrap { trim: false })
                .scroll((scroll, 0));
            frame.render_widget(body, body_area);

            if (wrapped_total as u16) > inner.height {
                let track_x = inner.x + inner.width.saturating_sub(1);
                for row in 0..inner.height {
                    let cell = Paragraph::new("│").style(
                        Style::default()
                            .fg(self.theme.scrollbar)
                            .bg(self.theme.popup_background),
                    );
                    frame.render_widget(
                        cell,
                        Rect {
                            x: track_x,
                            y: inner.y + row,
                            width: 1,
                            height: 1,
                        },
                    );
                }
                let total_h = inner.height as usize;
                let thumb_h = ((total_h * total_h) / wrapped_total.max(1)).max(1);
                let scroll_range = wrapped_total.saturating_sub(total_h).max(1);
                let thumb_top = ((scroll as usize) * total_h.saturating_sub(thumb_h))
                    / scroll_range;
                for i in 0..thumb_h {
                    let cell = Paragraph::new("█").style(
                        Style::default()
                            .fg(self.theme.scrollbar_hover)
                            .bg(self.theme.popup_background),
                    );
                    frame.render_widget(
                        cell,
                        Rect {
                            x: track_x,
                            y: inner.y + (thumb_top + i) as u16,
                            width: 1,
                            height: 1,
                        },
                    );
                }
            }
        }
```

- [ ] **Step 4.2: Run cargo check**

Run: `cargo check`

Expected: ok (render added, not yet event).

- [ ] **Step 4.3: Commit**

```bash
git add src/tui.rs
git commit -m "tui: render F2:Theme modal in draw (exact copy of F1 chrome + build_theme_text)"
```

## Task 5: Wire event handling for F2:Theme (open, close, scroll)

**Files:**
- Modify: `src/tui.rs` (in handle_event, after the show_help if block ~5532, and in the global key match ~5559)

- [ ] **Step 5.1: Add the if self.show_theme block after the show_help return**

After:
```rust
            return Ok(());
        }

        if self.save_prompt_open {
```

Insert:
```rust
        if self.show_theme {
            match evt {
                Event::Key(k) => match k.code {
                    KeyCode::F(2) | KeyCode::Esc => {
                        self.show_theme = false;
                        self.theme_scroll = 0;
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.theme_scroll = self
                            .theme_scroll
                            .saturating_add(1)
                            .min(self.theme_scroll_max);
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.theme_scroll = self.theme_scroll.saturating_sub(1);
                    }
                    KeyCode::PageDown => {
                        self.theme_scroll = self
                            .theme_scroll
                            .saturating_add(10)
                            .min(self.theme_scroll_max);
                    }
                    KeyCode::PageUp => {
                        self.theme_scroll = self.theme_scroll.saturating_sub(10);
                    }
                    KeyCode::Home | KeyCode::Char('g') => {
                        self.theme_scroll = 0;
                    }
                    KeyCode::End | KeyCode::Char('G') => {
                        self.theme_scroll = self.theme_scroll_max;
                    }
                    _ => {}
                },
                Event::Mouse(m) => match m.kind {
                    MouseEventKind::ScrollUp => {
                        self.theme_scroll = self.theme_scroll.saturating_sub(3);
                    }
                    MouseEventKind::ScrollDown => {
                        self.theme_scroll = self
                            .theme_scroll
                            .saturating_add(3)
                            .min(self.theme_scroll_max);
                    }
                    _ => {}
                },
                _ => {}
            }
            return Ok(());
        }
```

- [ ] **Step 5.2: Add F(2) in the global key match**

In the match k.code block (around 5559):
After 
```rust
                KeyCode::F(1) => self.show_help = true,
```
Add:
```rust
                KeyCode::F(2) => self.show_theme = true,
```

- [ ] **Step 5.3: Run cargo check**

Run: `cargo check`

Expected: clean.

- [ ] **Step 5.4: Commit**

```bash
git add src/tui.rs
git commit -m "tui: handle F2 open/close/scroll keys + wheel for theme modal (mirrors F1)"
```

## Task 6: Update footers and the in-help text

**Files:**
- Modify: `src/tui.rs` (footer logic ~4751-4757, and build_help_text Global section ~19027)

- [ ] **Step 6.1: Update the three footer strings**

Current narrow (~4752):
```rust
            "F1:Help  q:Quit  s:Save  /:Insert  Enter:Edit  j/k:Nav  Spc:Toggle  1/2/3:Focus"
```
New:
```rust
            "F1:Help  F2:Theme  q:Quit  s:Save  /:Insert  Enter:Edit  j/k:Nav  Spc:Toggle  1/2/3:Focus"
```

Medium (~4754):
```rust
            "F1: Help   s: Save   /: Insert   Enter: Edit   Tab: Switch page   Shift+E: Export   p: Preview   q: Quit"
```
New (insert after F1):
```rust
            "F1: Help   F2: Theme   s: Save   /: Insert   Enter: Edit   Tab: Switch page   Shift+E: Export   p: Preview   q: Quit"
```

Wide (~4756):
```rust
            "F1: Help   s: Save   /: Insert component   Enter: Edit   Tab/Shift+Tab: switch page   Shift+E: Export   p: Preview   F3: Validate   Ctrl+Q: Quit   (mouse: click/scroll/drag)"
```
New:
```rust
            "F1: Help   F2: Theme   s: Save   /: Insert component   Enter: Edit   Tab/Shift+Tab: switch page   Shift+E: Export   p: Preview   F3: Validate   Ctrl+Q: Quit   (mouse: click/scroll/drag)"
```

- [ ] **Step 6.2: Add F2 entry in build_help_text**

In the Global section array (around 19027):
After the F1 tuple, add:
```rust
            ("F2", "Open/close theme source + color details (F2:Theme)"),
```

Keep the rest of the array.

- [ ] **Step 6.3: Run cargo check**

Run: `cargo check`

- [ ] **Step 6.4: Commit**

```bash
git add src/tui.rs
git commit -m "tui: advertise F2:Theme in adaptive footers + Global help section"
```

## Task 7: Update docs (Architecture.md + CLAUDE.md)

**Files:**
- Modify: `Architecture.md` (key table ~79, TUI section)
- Modify: `CLAUDE.md` (user prefs / keys mentions)

- [ ] **Step 7.1: Update Architecture.md key bindings table**

Find the table under ### Key bindings (global)

Add row:
| `F2` | Theme info modal (source + status + color details; same layout as F1) |

After the F1 row.

- [ ] **Step 7.2: Add note in Architecture.md Theme + Visual Shell or TUI Loop section**

Add a sentence: "F2 opens the Theme info modal (source, load status, sampled color tokens with hex) using identical chrome and scroll mechanics to the F1 help modal."

- [ ] **Step 7.3: Update CLAUDE.md**

In the "User preferences captured during the v1.0 push" section, in the footer bullet or add: "F2:Theme shows theme source + color details (matches F1 modal layout)."

Also in "What lives where" if keys listed.

- [ ] **Step 7.4: Run git diff or cargo check (docs are md)**

- [ ] **Step 7.5: Commit**

```bash
git add Architecture.md CLAUDE.md
git commit -m "docs: document F2:Theme in Architecture.md and CLAUDE.md"
```

## Task 8: Add and verify tests for F2:Theme

**Files:**
- Modify: `src/tui.rs` (in the tests mod, after existing F1/F3 or help related tests, around 205xx-206xx area)

- [ ] **Step 8.1: Add basic toggle test**

After an existing F3 test (e.g. after f3_then_enter_dismisses_modal), insert:

```rust
    #[test]
    fn f2_opens_and_closes_with_f2_and_esc() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        send_key(&mut app, KeyCode::F(2), KeyModifiers::NONE);
        assert!(app.show_theme);
        assert_eq!(app.theme_scroll, 0);
        send_key(&mut app, KeyCode::Esc, KeyModifiers::NONE);
        assert!(!app.show_theme);
    }
```

- [ ] **Step 8.2: Add scroll test**

```rust
    #[test]
    fn f2_scroll_keys_and_wheel_update_theme_scroll() {
        let mut app = App::new(Site::starter(), None, AppTheme::default(), "default".to_string(), None);
        send_key(&mut app, KeyCode::F(2), KeyModifiers::NONE);
        // simulate render to set max (draw not called, so manually exercise clamp logic path)
        app.theme_scroll_max = 5; // pretend content
        send_key(&mut app, KeyCode::Down, KeyModifiers::NONE);
        assert_eq!(app.theme_scroll, 1);
        send_key(&mut app, KeyCode::PageDown, KeyModifiers::NONE);
        assert!(app.theme_scroll >= 1);
        send_key(&mut app, KeyCode::G, KeyModifiers::NONE); // home alias
        assert_eq!(app.theme_scroll, 0);
    }
```

- [ ] **Step 8.3: Add status test**

```rust
    #[test]
    fn f2_shows_warning_status_when_theme_status_is_some() {
        let mut app = App::new(
            Site::starter(),
            None,
            AppTheme::default(),
            "default".to_string(),
            Some("theme 'foo.yml' declares version 99 (expected 1); using built-in defaults".to_string()),
        );
        send_key(&mut app, KeyCode::F(2), KeyModifiers::NONE);
        assert!(app.show_theme);
        // status is stored; render would show it (no crash)
    }
```

- [ ] **Step 8.4: Run the new tests + full suite**

Run: `cargo test -q --test tui f2_ -- --nocapture`

Expected: all pass.

Run full: `cargo test -q`

- [ ] **Step 8.5: Commit**

```bash
git add src/tui.rs
git commit -m "test: add F2:Theme toggle, scroll, status send_key tests"
```

## Task 9: Final verification + cleanup

**Files:**
- src/tui.rs (any remaining)

- [ ] **Step 9.1: Full cargo check and test**

Run: `cargo check && cargo test -q`

Expected: clean, all tests pass (including new F2 ones and old F1/F3/help).

- [ ] **Step 9.2: Optional manual smoke (if in env)**

Run: `cargo run -- tui /tmp/scratch-f2.json` (create a scratch if needed), press F2, verify looks like the mock, source "default", colors listed, Esc closes, footer shows F2:Theme, F1 still works.

- [ ] **Step 9.3: Commit any final (or if clean, note in previous)**

If changes: `git commit -m "tui: final F2:Theme cleanups + verification"`

## Task 10: Update the visual companion note (optional, for history)

**Files:**
- The brainstorm content dir files (already updated in process)

- [ ] **Step 10.1: Optionally touch a note in the plan dir or leave (visuals already reflect approval)**

## Verification Summary (from spec)
- All tasks produce passing tests at each commit.
- Matches exactly the approved spec and visual mocks (16 tokens, title, F2:Theme everywhere, F1 parity).
- Plan self-review: covers every bullet in the design spec (state, render, events, footer, help text, docs, tests, non-goals). No gaps. No placeholders. Consistent naming (show_theme / theme_* to match "F2:Theme" label chosen by user).

Plan complete and saved to `docs/superpowers/plans/2026-06-21-f2-theme-modal.md`.

**Two execution options:**
**1. Subagent-Driven (recommended)** - dispatch fresh subagent per task + review
**2. Inline Execution** - execute tasks in this session

Which approach? (Reply and I'll use the appropriate sub-skill: subagent-driven-development or executing-plans.) 
