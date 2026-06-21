# Design Spec: F2:Theme Modal (F1 layout parity)

**Date:** 2026-06-21  
**Status:** All sections approved in brainstorming. Ready for spec commit + user review gate.  
**Related:** LDNDDEV_TUI_VISUAL_STANDARD.md (F2 Credits/Theme modal requirement), previous header shell plan (F2 was deferred non-goal), user request for "F2 option" showing theme source + color variable details, styled identically to F1 help modal.

## Summary of Request
Restore an F2 key (previously existed in spirit) that opens a scrollable modal showing:
- Which theme source is active (`local`, `global`, or `default`)
- Theme status / load health (the warning message if any, or "OK")
- Details about the color variables (a compact sampled list of the most visible tokens with live hex values + roles)

The modal must have **exactly the same layout and chrome** as the existing F1 help modal (80%×80% centered, `modal_header` section titles, `text_active_focus` key labels, muted dividers, custom right-edge scrollbar, identical title phrasing pattern, wheel + keyboard scrolling, etc.).

Footer must advertise it (always start with F1:Help, adaptive). Help text must document it. No changes to unified `Modal` enum or other modals.

## Approved Design Choices (from brainstorming visual companion)
- **Label / naming:** "F2:Theme" (title bar, footer hints, help text entry)
- **Content volume:** A — compact list of 16 key tokens with color swatches (in visual), live `#rrggbb`, short role description
- **Plumbing:** Approach 1 (mirror F1 exactly) — parallel `show_theme` / `theme_scroll` / `theme_scroll_max` + `theme_status: Option<String>` on `App`. Dedicated `if` blocks in `handle_event` + `draw` (after the `show_help` blocks). New `build_theme_text` helper parallel to `build_help_text`. No `Modal` variant.
- **Supporting changes:** 3 adaptive footer strings, one entry in `build_help_text` Global section, `Architecture.md` + `CLAUDE.md` updates, `send_key`-driven tests, small `color_to_hex` helper, remove `#[allow(dead_code)]` on `theme_source`.

## State & API Changes
Add to `struct App` (near the existing `show_help` / `help_*` fields):

```rust
show_theme: bool,
/// Vertical scroll offset (in rows) for the F2:Theme modal.
theme_scroll: u16,
/// Maximum legal `theme_scroll` value, recomputed every render.
theme_scroll_max: u16,
theme_status: Option<String>,  // the load_warning from AppTheme::load(), or None for clean
```

`theme_source: String` already exists (remove the `#[allow(dead_code)]` attr once rendered).

Update `App::new` signature and all call sites (run_tui + ~30 test helpers):

```rust
fn new(..., theme_source: String, theme_status: Option<String>) -> Self { ... }
```

In `run_tui`:

```rust
let (theme, theme_source, load_warning) = AppTheme::load();
...
let mut app = App::new(site, path, theme, theme_source, load_warning);
if let Some(msg) = load_warning {
    app.push_toast(ToastLevel::Warning, msg);
}
```

In `build` / new construction, initialize the new bool/scroll fields to false/0 and store `theme_status`.

Add private helper (near `build_help_text` and `color_to_hex` will live near `parse_hex_color`):

```rust
fn color_to_hex(c: Color) -> String {
    if let Color::Rgb(r, g, b) = c {
        format!("#{:02x}{:02x}{:02x}", r, g, b)
    } else {
        "?".to_string()
    }
}

fn build_theme_text(
    theme: &AppTheme,
    source: &str,
    status: &Option<String>,
    width: usize,
) -> Text<'static> { ... }
```

Inside `build_theme_text`: use the same `h_style`, `k_style`, `div_style`, section + divider pattern (or direct `Line` pushes for the color list). Hard-code the 16-token list (order roughly follows AppTheme struct + visible roles from CLAUDE.md / LDNDDEV):

- background, popup_background, foreground, modal_header, text_labels, text_active_focus, input_border_focus,
- success, warning, error, info,
- folders, files, links,
- scrollbar, scrollbar_hover

Each line: `  name                #rrggbb   (short role)`

Status line: `if let Some(w) = status { w } else { "OK (loaded cleanly)" }`

## Rendering & Event Handling
In `fn draw(...)` — after the entire `if self.show_help { ... }` block:

```rust
if self.show_theme {
    // near-identical copy of the help block:
    let area = centered_rect(80, 80, frame.area());
    frame.render_widget(Clear, area);
    let block = Block::default()
        .title("Theme (F2 / Esc to close, j/k or arrows to scroll)")
        .borders(Borders::ALL)
        .style(Style::default().fg(self.theme.foreground).bg(self.theme.popup_background))
        .border_style(Style::default().fg(self.theme.border_active))
        .title_style(Style::default().fg(self.theme.modal_header).add_modifier(Modifier::BOLD));
    ...
    let help = build_theme_text(&self.theme, &self.theme_source, &self.theme_status, body_w as usize);
    ... (same wrapped_total, max_scroll publish to self.theme_scroll_max, clamp, Paragraph scroll, custom │/█ scrollbar paint using theme.scrollbar / scrollbar_hover)
}
```

In `fn handle_event(...)`:

After the `if self.show_help { ... return Ok(()); }` block, add:

```rust
if self.show_theme {
    match evt {
        Event::Key(k) => match k.code {
            KeyCode::F(2) | KeyCode::Esc => {
                self.show_theme = false;
                self.theme_scroll = 0;
            }
            KeyCode::Down | KeyCode::Char('j') => { self.theme_scroll = ...clamp... }
            ... (exact same Up/k, PageDown, PageUp, Home/g, End/G, mouse ScrollUp/ScrollDown as help)
        },
        ... (mouse)
    }
    return Ok(());
}
```

In the global key match (only reached when no unified Modal is open and help/theme not active):

```rust
KeyCode::F(1) => self.show_help = true,
KeyCode::F(2) => self.show_theme = true,
KeyCode::F(3) => self.open_validation_modal(),
```

F2 while a FormEdit / other Modal is open is swallowed (returns Continue from handle_modal_event) — identical to current F1 behavior.

## Footer & In-Help Documentation
Update the three `footer_text` strings (the if/else if/else on `root[2].width`):

- <75: `... F1:Help  F2:Theme  q:Quit ...`
- <110: `F1: Help   F2: Theme   s: Save ...`
- wide: `F1: Help   F2: Theme   ... F3: Validate ...`

Add to `build_help_text` under the first "Global" section (right after the F1 line):

```rust
("F2", "Open/close theme source + color details (F2:Theme)"),
```

## Docs Updates (part of the change)
- Architecture.md: update the "Key bindings (global)" table to include F2; add a one-sentence note in the TUI Loop or Theme section.
- CLAUDE.md: ensure the captured user prefs / key list mentions F2:Theme alongside F1/F3 (minor).

## Tests
All inside `#[cfg(test)] mod tests`, driving via `send_key`:

- `f2_opens_and_closes_with_f2_and_esc`
- `f2_scroll_keys_and_wheel_update_theme_scroll`
- `f2_shows_warning_status_when_theme_status_is_some`
- Ensure existing F3 / help tests still pass; add a quick "F2 does not appear while FormEdit modal is open" guard if easy.

Also exercise `App::new(..., Some("some warning".into()))` in at least one construction test.

## Non-Goals / Explicit Scope (tight per CLAUDE.md)
- No `Modal::ThemeInfo` variant or integration with the unified modal system.
- No live theme reload / file picker / editor.
- No new common scrollable-modal helper or refactoring of the existing F1 help code.
- Duplication of the render/event blocks (~80 + 25 lines) is accepted for this small feature.
- No changes to theme loading, AppTheme, or `dd_siteforge_theme.yml`.
- No bump to Cargo version or tag.

## Files Touched (high level)
- src/tui.rs (App struct, new(), run_tui, draw, handle_event, build_theme_text, color_to_hex, footer strings, build_help_text, tests)
- docs/Architecture.md
- CLAUDE.md (light)
- The spec itself + this visual companion history

## Verification After Implementation
- `cargo check`
- `cargo test -q` (all 96+ tests, new F2 ones pass)
- Manual smoke: `cargo run -- tui /tmp/scratch.json`, press F2, verify source/status/colors, scrolling, Esc, footer hint, F1 still works, etc.
- With a temp bad theme file (wrong version) to see the warning text appear in F2.

All decisions above were reviewed and approved section-by-section in the visual companion (http://localhost:56461) during the 2026-06-21 brainstorming session. The right-hand terminal mock in the companion exactly matches the intended final F2:Theme content and chrome.

---

**Next (after user review gate):** TDD plan via writing-plans skill, one commit per task, send_key-driven tests, etc.
