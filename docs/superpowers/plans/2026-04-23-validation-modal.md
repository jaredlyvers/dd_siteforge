# Validation Modal Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an `F3` key that runs `validate_site` and opens a scrollable modal listing every error. The same modal is later reused by the in-TUI export flow (Plan 3) as a blocking gate.

**Architecture:** One new `Modal::ValidationErrors { errors: Vec<String>, scroll_offset: usize }` variant, rendered with the same unified token palette used by PageHead (border_active, popup_background, text_labels/text_active_focus, scrollbar tokens). `F3` globally triggers a helper `open_validation_modal()` that calls `crate::validate::validate_site(&self.site)` and opens the modal, or shows a green status line when there are no errors. Nothing new in `src/validate.rs`; the existing `pub fn validate_site(&Site) -> Vec<String>` is already the right shape.

**Tech Stack:** Rust 2024, `ratatui`, `crossterm`. No new deps.

**Plan covers spec section §4 only.** Spec at `docs/superpowers/specs/2026-04-22-pre-1.0-cms-punch-list-design.md`. Export (§2) in the next plan.

---

## Files Map

- **Modify:**
  - `src/tui.rs` — new `Modal::ValidationErrors` variant, `render_validation_errors_modal`, `handle_validation_errors_event`, `open_validation_modal` helper, `F3` top-level key binding, help-text section, debug-name arm, integration tests.
- **No changes in:** `src/model.rs`, `src/storage.rs`, `src/validate.rs`, `src/renderer.rs`.

---

## Task 1: `Modal::ValidationErrors` variant + modal plumbing shell

**Files:**
- Modify: `src/tui.rs` — `enum Modal`, render dispatch, event dispatch, debug-name match, new render + event methods.

- [ ] **Step 1: Add the variant**

In `enum Modal` add (placement beside other small modals like `ConfirmPrompt`):

```rust
    ValidationErrors {
        errors: Vec<String>,
        scroll_offset: usize,
    },
```

- [ ] **Step 2: Wire render dispatch**

In the render-dispatch match, add:

```rust
            Modal::ValidationErrors { errors, scroll_offset } => {
                self.render_validation_errors_modal(frame, errors, *scroll_offset);
            }
```

- [ ] **Step 3: Wire event dispatch**

In the modal-event dispatch, add:

```rust
                Modal::ValidationErrors { .. } => return self.handle_validation_errors_event(key),
```

- [ ] **Step 4: Wire debug-name arm**

In `Modal::variant_name`, add:

```rust
            Modal::ValidationErrors { .. } => "ValidationErrors",
```

- [ ] **Step 5: Implement `render_validation_errors_modal`**

Add to `impl App`:

```rust
fn render_validation_errors_modal(
    &self,
    frame: &mut ratatui::Frame,
    errors: &[String],
    scroll_offset: usize,
) {
    let area = centered_rect(70, 60, frame.area());
    frame.render_widget(Clear, area);

    let outer_title = format!(" Validation — {} error(s) ", errors.len());
    let modal_block = Block::default()
        .title(outer_title)
        .borders(Borders::ALL)
        .style(Style::default().bg(self.theme.popup_background))
        .border_style(Style::default().fg(self.theme.border_active))
        .title_style(
            Style::default()
                .fg(self.theme.title)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_widget(modal_block.clone(), area);
    let inner = modal_block.inner(area);

    if inner.width < 4 || inner.height < 3 {
        return;
    }

    let padding_x: u16 = 2;
    let content_x = inner.x + padding_x;
    let content_w = inner.width.saturating_sub(padding_x * 2);
    let footer_height: u16 = 1;
    let list_height = inner.height.saturating_sub(footer_height);

    // Render numbered error lines with word wrap. Lines that wrap count as
    // one logical entry scrolled by scroll_offset; the line itself may wrap
    // across multiple screen rows.
    let wrapped_lines = self.wrap_validation_lines(errors, content_w as usize);
    let visible = wrapped_lines
        .iter()
        .skip(scroll_offset)
        .take(list_height as usize)
        .cloned()
        .collect::<Vec<_>>();

    let body = Paragraph::new(visible.join("\n"))
        .style(
            Style::default()
                .fg(self.theme.foreground)
                .bg(self.theme.popup_background),
        );
    frame.render_widget(
        body,
        Rect {
            x: content_x,
            y: inner.y,
            width: content_w,
            height: list_height,
        },
    );

    // Footer hint row
    let footer_y = inner.y + inner.height.saturating_sub(footer_height);
    let footer_area = Rect {
        x: content_x,
        y: footer_y,
        width: content_w,
        height: 1,
    };
    let footer_text = if wrapped_lines.len() > list_height as usize {
        "j / k or ↑ / ↓ to scroll  |  Enter or Esc to dismiss"
    } else {
        "Enter or Esc to dismiss"
    };
    let footer = Paragraph::new(footer_text).style(
        Style::default()
            .fg(self.theme.muted)
            .bg(self.theme.popup_background),
    );
    frame.render_widget(footer, footer_area);
}

/// Turn raw validator messages into a numbered, pre-wrapped list. Each
/// message becomes one or more visible rows depending on `width`.
fn wrap_validation_lines(&self, errors: &[String], width: usize) -> Vec<String> {
    let mut out = Vec::with_capacity(errors.len());
    for (i, err) in errors.iter().enumerate() {
        let prefix = format!("{}. ", i + 1);
        let indent = " ".repeat(prefix.len());
        let body_w = width.saturating_sub(prefix.len()).max(1);
        let mut first = true;
        let mut remaining = err.as_str();
        while !remaining.is_empty() {
            // Naive char-wrap — validator messages are short, so no
            // word-splitting needed for a readable first pass.
            let take = remaining.chars().take(body_w).count();
            let (chunk, rest) = remaining.split_at(
                remaining
                    .char_indices()
                    .nth(take)
                    .map(|(i, _)| i)
                    .unwrap_or(remaining.len()),
            );
            let line = if first {
                format!("{}{}", prefix, chunk)
            } else {
                format!("{}{}", indent, chunk)
            };
            out.push(line);
            remaining = rest;
            first = false;
        }
    }
    out
}
```

- [ ] **Step 6: Implement `handle_validation_errors_event`**

```rust
fn handle_validation_errors_event(&mut self, key: event::KeyEvent) -> Option<ModalResult> {
    use crossterm::event::KeyCode;
    let (errors_len, scroll) = match &self.modal {
        Some(Modal::ValidationErrors { errors, scroll_offset }) => {
            (errors.len(), *scroll_offset)
        }
        _ => return Some(ModalResult::CloseCancel),
    };
    match key.code {
        KeyCode::Enter | KeyCode::Esc => {
            self.modal = None;
            Some(ModalResult::CloseSuccess)
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if let Some(Modal::ValidationErrors { scroll_offset, .. }) = self.modal.as_mut() {
                *scroll_offset = scroll_offset.saturating_sub(1);
            }
            Some(ModalResult::Continue)
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let Some(Modal::ValidationErrors { scroll_offset, .. }) = self.modal.as_mut() {
                if scroll + 1 < errors_len.max(1) {
                    *scroll_offset += 1;
                }
            }
            Some(ModalResult::Continue)
        }
        KeyCode::PageUp => {
            if let Some(Modal::ValidationErrors { scroll_offset, .. }) = self.modal.as_mut() {
                *scroll_offset = scroll_offset.saturating_sub(5);
            }
            Some(ModalResult::Continue)
        }
        KeyCode::PageDown => {
            if let Some(Modal::ValidationErrors { scroll_offset, .. }) = self.modal.as_mut() {
                *scroll_offset = (scroll + 5).min(errors_len.saturating_sub(1));
            }
            Some(ModalResult::Continue)
        }
        _ => Some(ModalResult::Continue),
    }
}
```

- [ ] **Step 7: Build**

Run: `cargo check 2>&1 | tail -5`
Expected: clean. `dead_code` warnings for the two new fns are OK until Task 2 invokes them.

- [ ] **Step 8: Commit**

```bash
git add src/tui.rs
git commit -m "tui: Modal::ValidationErrors shell (render + j/k/arrow/PgUp/PgDn scroll)"
```

---

## Task 2: `open_validation_modal` helper

**Files:**
- Modify: `src/tui.rs` — new helper method on `impl App`.

- [ ] **Step 1: Write the failing tests**

Append to `mod tests`:

```rust
#[test]
fn open_validation_modal_on_clean_starter_sets_status_and_no_modal() {
    let mut app = App::new(Site::starter(), None, AppTheme::default());
    app.open_validation_modal();
    assert!(app.modal.is_none(), "no modal should open when validation is clean");
    assert!(
        app.status.to_lowercase().contains("no validation errors"),
        "status should confirm clean validation, got: {:?}",
        app.status
    );
}

#[test]
fn open_validation_modal_with_errors_opens_modal_with_error_list() {
    let mut app = App::new(Site::starter(), None, AppTheme::default());
    // Force an error: empty slug.
    app.site.pages[0].slug = "".to_string();
    app.open_validation_modal();
    match &app.modal {
        Some(Modal::ValidationErrors { errors, scroll_offset }) => {
            assert!(!errors.is_empty());
            assert_eq!(*scroll_offset, 0);
            assert!(
                errors.iter().any(|e| e.contains("empty slug")),
                "expected empty-slug error, got: {:?}",
                errors
            );
        }
        other => panic!("expected Modal::ValidationErrors, got {:?}", other),
    }
}
```

- [ ] **Step 2: Run tests — verify fail**

Run: `cargo test -q open_validation_modal 2>&1 | tail -10`
Expected: fail — `open_validation_modal` method doesn't exist.

- [ ] **Step 3: Implement the helper**

Add to `impl App`:

```rust
/// Run `validate_site` on the current site. Open `Modal::ValidationErrors`
/// if any errors; otherwise set a green status and leave no modal open.
fn open_validation_modal(&mut self) {
    let errors = crate::validate::validate_site(&self.site);
    if errors.is_empty() {
        self.status = "No validation errors.".to_string();
    } else {
        self.status = format!("Validation: {} error(s).", errors.len());
        self.modal = Some(Modal::ValidationErrors {
            errors,
            scroll_offset: 0,
        });
    }
}
```

- [ ] **Step 4: Run tests — verify pass**

Run: `cargo test -q open_validation_modal 2>&1 | tail -10`
Expected: both pass.

Full suite: `cargo test -q 2>&1 | tail -3`
Expected: previous count + 2.

- [ ] **Step 5: Commit**

```bash
git add src/tui.rs
git commit -m "tui: open_validation_modal helper runs validate_site and opens modal"
```

---

## Task 3: `F3` global key binding

**Files:**
- Modify: `src/tui.rs` — top-level `match k.code` inside `handle_event`.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn f3_on_clean_starter_shows_no_error_status() {
    let mut app = App::new(Site::starter(), None, AppTheme::default());
    send_key(&mut app, KeyCode::F(3), KeyModifiers::NONE);
    assert!(app.modal.is_none());
    assert!(app.status.to_lowercase().contains("no validation errors"));
}

#[test]
fn f3_with_validation_errors_opens_modal() {
    let mut app = App::new(Site::starter(), None, AppTheme::default());
    app.site.pages[0].slug = "".to_string();
    send_key(&mut app, KeyCode::F(3), KeyModifiers::NONE);
    assert!(matches!(app.modal, Some(Modal::ValidationErrors { .. })));
}

#[test]
fn f3_then_enter_dismisses_modal() {
    let mut app = App::new(Site::starter(), None, AppTheme::default());
    app.site.pages[0].slug = "".to_string();
    send_key(&mut app, KeyCode::F(3), KeyModifiers::NONE);
    send_key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    assert!(app.modal.is_none());
}

#[test]
fn f3_then_j_k_scrolls_error_list() {
    let mut app = App::new(Site::starter(), None, AppTheme::default());
    // Generate multiple errors: no pages.
    app.site.pages.clear();
    send_key(&mut app, KeyCode::F(3), KeyModifiers::NONE);
    let initial_errors = match &app.modal {
        Some(Modal::ValidationErrors { errors, .. }) => errors.len(),
        _ => 0,
    };
    if initial_errors > 1 {
        send_key(&mut app, KeyCode::Char('j'), KeyModifiers::NONE);
        match &app.modal {
            Some(Modal::ValidationErrors { scroll_offset, .. }) => {
                assert_eq!(*scroll_offset, 1);
            }
            _ => panic!("modal closed unexpectedly"),
        }
        send_key(&mut app, KeyCode::Char('k'), KeyModifiers::NONE);
        match &app.modal {
            Some(Modal::ValidationErrors { scroll_offset, .. }) => {
                assert_eq!(*scroll_offset, 0);
            }
            _ => panic!("modal closed unexpectedly"),
        }
    }
}
```

- [ ] **Step 2: Run tests — verify fail**

Run: `cargo test -q f3_ 2>&1 | tail -10`
Expected: fail — F3 does nothing.

- [ ] **Step 3: Implement**

In the top-level `match k.code` block inside `handle_event` (not the Pages-panel dispatcher — this is a global key), add an arm next to the existing `KeyCode::F(1) => self.show_help = true,`:

```rust
                KeyCode::F(3) => self.open_validation_modal(),
```

- [ ] **Step 4: Run tests — verify pass**

Run: `cargo test -q f3_ 2>&1 | tail -10`
Expected: all four tests pass.

Full suite: `cargo test -q 2>&1 | tail -3`
Expected: previous count + 4.

- [ ] **Step 5: Commit**

```bash
git add src/tui.rs
git commit -m "tui: F3 opens validation errors modal"
```

---

## Task 4: Help text update

**Files:**
- Modify: `src/tui.rs` — the Global section of `help_text()`.

- [ ] **Step 1: Add the line**

Locate `fn help_text()` and the `"Global:"` section. Add `F3` after `F1`:

```rust
        "  F1: Open/close this help",
        "  F3: Validate site (shows errors in a modal)",
```

- [ ] **Step 2: Build and run tests**

Run: `cargo test -q 2>&1 | tail -3`
Expected: all tests still pass.

- [ ] **Step 3: Commit**

```bash
git add src/tui.rs
git commit -m "tui: document F3 in help text"
```

---

## Task 5: Manual smoke + final verify

- [ ] **Step 1: Full test suite**

Run: `cargo test -q 2>&1 | tail -3`
Expected: green.

- [ ] **Step 2: Manual TUI smoke**

```bash
cargo run -- tui /tmp/dd_validate_smoke.json
```

Walk through:
1. `F1` — confirm help now lists F3.
2. `F3` on the clean starter — status bar should read "No validation errors." and no modal opens.
3. `[3]` panel → pick a page → open the PageHead modal → blank out the Title → Ctrl+S to save.
4. `F3` — modal should open showing the missing title error; `j`/`k` scrolls if there are multiple; `Enter` dismisses.

- [ ] **Step 3: Confirm feature complete**

No empty commit needed. Four commits on this feature, each with clear message. Branch ready to merge or stack on top for Plan 3 (Export).

---

## Self-Review Notes

- **Spec coverage (spec §4):**
  - `F3` key (rebound from plan's `V` to avoid Layout-panel conflict) — Task 3.
  - Numbered error list modal — Task 1 + `wrap_validation_lines` helper.
  - `j/k` + PgUp/PgDn scroll, Enter/Esc dismiss — Task 1 event handler + Task 3 tests.
  - Same modal reused later by export gate — Plan 3 will construct `Modal::ValidationErrors` directly.

- **Type consistency:** `Modal::ValidationErrors`, `open_validation_modal`, `render_validation_errors_modal`, `handle_validation_errors_event`, `wrap_validation_lines` referenced consistently across all 5 tasks.

- **Token compliance:** Reuses the same tokens as `render_edit_modal_unified` and `render_single_input_modal`:
  - Outer border: `border_active`
  - Modal bg: `popup_background`
  - Title: `title`
  - Body text: `foreground`
  - Footer: `muted`
  No new tokens introduced.

- **Caveats:**
  - `wrap_validation_lines` uses naive char-wrap. Validator messages are one-line sentences under ~100 chars today, so word-boundary wrapping is overkill. Revisit when any validator message routinely exceeds the modal width.
  - Scroll bound uses `errors_len` (logical entries), not wrapped-row count. That's intentional — one `j` press advances one error, not one screen row. If messages start wrapping heavily this may feel sluggish and can be swapped to wrapped-line count.
