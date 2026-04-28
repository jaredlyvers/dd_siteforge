# In-TUI Export Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an `E` key that renders the current site to HTML from inside the TUI. Export is gated by the validation modal (Plan 2) — errors block, clean passes through. Output directory is persisted per-site in the JSON (`site.export_dir`), prompted on first export with a `./web/` default.

**Architecture:** One additive model field (`Site.export_dir: Option<String>`, `#[serde(default)]`). One new `Modal::ExportPathPrompt { path: String }` that mirrors `NewPageTitlePrompt`'s shape. Two helpers on `App`: `begin_export_flow()` (entry point from `E`) and `commit_export_to(path)` (does the work). Existing `crate::renderer::render_site_to_dir` is used unchanged. Source-image copy (`./source/images/` → `<out>/assets/images/`) is best-effort — skipped silently when the source folder doesn't exist, to keep this plan independent of Plan 4 (Image Assets).

**Tech Stack:** Rust 2024, `ratatui`, `crossterm`. Adds `std::fs` recursive copy (hand-rolled, no new dep).

**Plan covers spec section §2 only.** Spec at `docs/superpowers/specs/2026-04-22-pre-1.0-cms-punch-list-design.md`.

---

## Files Map

- **Modify:**
  - `src/model.rs` — add `Site.export_dir: Option<String>` with `#[serde(default)]`.
  - `src/storage.rs` — round-trip test for `export_dir` back-compat.
  - `src/tui.rs` — `Modal::ExportPathPrompt` variant + render/event/debug plumbing; `begin_export_flow`, `commit_export_to`, `copy_source_images_to` helpers; `E` key binding; help text update; integration tests.
- **No changes in:** `src/renderer.rs`, `src/validate.rs`.

---

## Task 1: Add `Site.export_dir` field

**Files:**
- Modify: `src/model.rs`
- Test: `src/storage.rs`

- [ ] **Step 1: Add the field**

In `src/model.rs`, extend `Site`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Site {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub theme: ThemeSettings,
    pub header: DdHeader,
    pub footer: DdFooter,
    pub pages: Vec<Page>,
    /// Persisted export output directory, relative to the site JSON file.
    /// `None` triggers a first-time prompt; user-confirmed value is written back.
    #[serde(default)]
    pub export_dir: Option<String>,
}
```

Add `export_dir: None,` to the literal in `Site::starter()`.

- [ ] **Step 2: Back-compat round-trip test**

Append to `src/storage.rs` `tests`:

```rust
#[test]
fn export_dir_defaults_to_none_on_legacy_json() {
    let json = r##"{
      "schema_version": 1,
      "id": "s",
      "name": "n",
      "theme": {"primary_color":"#000","secondary_color":"#000","tertiary_color":"#000","support_color":"#000"},
      "header": {"id":"h","custom_css":null,"alert":null,"sections":[]},
      "footer": {"id":"f","custom_css":null,"sections":[]},
      "pages": []
    }"##;
    let site: crate::model::Site = serde_json::from_str(json).expect("legacy JSON should load");
    assert!(site.export_dir.is_none(), "legacy sites load with export_dir = None");
}

#[test]
fn export_dir_round_trips_through_save_and_load() {
    let tmp = unique_temp_path("dd_site_export_dir_roundtrip");
    let mut site = crate::model::Site::starter();
    site.export_dir = Some("./web/".to_string());
    save_site(&tmp, &site).expect("save ok");
    let loaded = load_site(&tmp).expect("load ok");
    std::fs::remove_file(&tmp).ok();
    assert_eq!(loaded.export_dir.as_deref(), Some("./web/"));
}
```

- [ ] **Step 3: TDD cycle**

`cargo test -q storage::tests::export_dir 2>&1 | tail -10` — both fail (field missing).
Add the field + `starter()` update.
`cargo test -q storage::tests::export_dir 2>&1 | tail -10` — both pass.
`cargo test -q 2>&1 | tail -3` — full suite green.

- [ ] **Step 4: Commit**

```bash
git add src/model.rs src/storage.rs
git commit -m "model: add Site.export_dir (optional, serde-default) with round-trip tests"
```

---

## Task 2: `Modal::ExportPathPrompt` shell

**Files:**
- Modify: `src/tui.rs`

- [ ] **Step 1: Add the variant**

In `enum Modal`:

```rust
    ExportPathPrompt {
        path: String,
    },
```

- [ ] **Step 2: Wire the 4 plumbing points**

Same pattern as `NewPageTitlePrompt`:
- Render dispatch → `self.render_export_path_prompt(frame, path);`
- Event dispatch → `return self.handle_export_path_prompt_event(key);`
- Debug-name arm → `"ExportPathPrompt"`

- [ ] **Step 3: Implement render using the shared helper**

```rust
fn render_export_path_prompt(&self, frame: &mut ratatui::Frame, path: &str) {
    self.render_single_input_modal(
        frame,
        " Export — output directory ",
        "Path (relative to site JSON)",
        path,
        "Enter or Ctrl+S: export  |  Esc: cancel",
    );
}
```

- [ ] **Step 4: Implement event handler**

Mirror `handle_new_page_title_prompt_event`. Enter (or Ctrl+S) with empty → "Export path required." warning toast and keep modal open. Enter with non-empty → close modal, call `self.commit_export_to(path)`. Esc → close with info toast "Export cancelled.".

```rust
fn handle_export_path_prompt_event(&mut self, key: event::KeyEvent) -> Option<ModalResult> {
    use crossterm::event::KeyCode;
    let path = if let Some(Modal::ExportPathPrompt { path }) = self.modal.take() {
        path
    } else {
        return Some(ModalResult::CloseCancel);
    };
    match key.code {
        KeyCode::Esc => {
            self.push_toast(ToastLevel::Info, "Export cancelled.");
            Some(ModalResult::CloseCancel)
        }
        KeyCode::Enter
        | KeyCode::Char('s') if matches!(key.code, KeyCode::Enter)
            || key.modifiers.contains(KeyModifiers::CONTROL) => {
            let trimmed = path.trim();
            if trimmed.is_empty() {
                self.push_toast(ToastLevel::Warning, "Export path required.");
                self.modal = Some(Modal::ExportPathPrompt { path });
                Some(ModalResult::Continue)
            } else {
                self.commit_export_to(trimmed.to_string());
                Some(ModalResult::CloseSuccess)
            }
        }
        KeyCode::Backspace => {
            let mut new_path = path;
            new_path.pop();
            self.modal = Some(Modal::ExportPathPrompt { path: new_path });
            Some(ModalResult::Continue)
        }
        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            let mut new_path = path;
            new_path.push(c);
            self.modal = Some(Modal::ExportPathPrompt { path: new_path });
            Some(ModalResult::Continue)
        }
        _ => {
            self.modal = Some(Modal::ExportPathPrompt { path });
            Some(ModalResult::Continue)
        }
    }
}
```

The fused `Enter | Char('s')` guard looks noisy; if it doesn't compile cleanly, split into two arms (one `KeyCode::Enter =>`, one `KeyCode::Char('s') if key.modifiers.contains(CONTROL) =>`) that both call the same commit helper.

- [ ] **Step 5: Build and commit**

`cargo check 2>&1 | tail -5` — clean.
`cargo test -q 2>&1 | tail -3` — green.

```bash
git add src/tui.rs
git commit -m "tui: Modal::ExportPathPrompt shell (reuses render_single_input_modal)"
```

---

## Task 3: `begin_export_flow` + `commit_export_to`

**Files:**
- Modify: `src/tui.rs`

- [ ] **Step 1: Implement entry helper**

```rust
/// Entry point for the `E` key. Validates first; opens ValidationErrors
/// modal on failures. Otherwise resolves the output dir (prompting on first
/// use) and either opens the prompt or commits the export directly.
fn begin_export_flow(&mut self) {
    let errors = crate::validate::validate_site(&self.site);
    if !errors.is_empty() {
        self.modal = Some(Modal::ValidationErrors {
            errors,
            scroll_offset: 0,
        });
        return;
    }
    match self.site.export_dir.clone() {
        Some(dir) if !dir.trim().is_empty() => {
            self.commit_export_to(dir);
        }
        _ => {
            self.modal = Some(Modal::ExportPathPrompt {
                path: "./web/".to_string(),
            });
        }
    }
}
```

- [ ] **Step 2: Implement commit helper**

```rust
/// Resolve `rel` against the site JSON's directory (or the current working
/// directory when there's no saved path), run the renderer, best-effort
/// copy source images, and surface the outcome as toasts.
fn commit_export_to(&mut self, rel: String) {
    use std::path::{Path, PathBuf};
    let base = self
        .path
        .as_ref()
        .and_then(|p| p.parent().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."));
    let out = base.join(Path::new(&rel));

    match crate::renderer::render_site_to_dir(&self.site, &out) {
        Ok(()) => {
            self.site.export_dir = Some(rel);
            self.copy_source_images_to(&base, &out);
            let page_count = self.site.pages.len();
            let msg = format!("Exported {} page(s) to {}", page_count, out.display());
            self.push_toast(ToastLevel::Success, msg);
        }
        Err(e) => {
            let msg = format!("Export failed: {}", e);
            self.push_toast(ToastLevel::Warning, msg);
        }
    }
}

/// Recursively copy `base/source/images/` → `<out>/assets/images/` if the
/// source exists. Silently skips when the folder is absent so projects
/// that don't use local images still export cleanly. Any copy failure is
/// surfaced as a warning toast but does not fail the export.
fn copy_source_images_to(&mut self, base: &std::path::Path, out: &std::path::Path) {
    let src = base.join("source").join("images");
    if !src.exists() {
        return;
    }
    let dst = out.join("assets").join("images");
    if let Err(e) = copy_dir_recursive(&src, &dst) {
        let msg = format!("Images copy skipped: {}", e);
        self.push_toast(ToastLevel::Warning, msg);
    }
}
```

- [ ] **Step 3: Add file-level helper (not a method — plain `fn`)**

Add near the bottom of `src/tui.rs`, outside any `impl` block:

```rust
fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name();
        let target = dst.join(&file_name);
        if path.is_dir() {
            copy_dir_recursive(&path, &target)?;
        } else {
            std::fs::copy(&path, &target)?;
        }
    }
    Ok(())
}
```

- [ ] **Step 4: Tests**

Append to `mod tests`:

```rust
#[test]
fn begin_export_flow_on_clean_starter_without_export_dir_opens_path_prompt() {
    let mut app = App::new(Site::starter(), None, AppTheme::default());
    assert!(app.site.export_dir.is_none());
    app.begin_export_flow();
    match &app.modal {
        Some(Modal::ExportPathPrompt { path }) => {
            assert_eq!(path, "./web/");
        }
        _ => panic!("expected ExportPathPrompt, got a different modal or None"),
    }
}

#[test]
fn begin_export_flow_with_invalid_site_opens_validation_modal() {
    let mut app = App::new(Site::starter(), None, AppTheme::default());
    app.site.pages[0].slug = "".to_string();
    app.begin_export_flow();
    assert!(matches!(app.modal, Some(Modal::ValidationErrors { .. })));
}

#[test]
fn begin_export_flow_with_saved_export_dir_commits_directly() {
    // Write site to a temp file so `app.path` + `export_dir` resolve to a
    // writable location. Verify the renderer produced a file and a toast.
    let tmp = std::env::temp_dir().join(format!(
        "dd_export_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&tmp).unwrap();
    let json_path = tmp.join("site.json");
    let mut app = App::new(Site::starter(), Some(json_path.clone()), AppTheme::default());
    app.site.export_dir = Some("web".to_string());

    app.begin_export_flow();

    assert!(app.modal.is_none(), "no modal should open — direct export");
    let last = app.toasts.last().expect("expected a toast");
    assert_eq!(last.level, ToastLevel::Success);
    assert!(last.message.to_lowercase().contains("exported"));
    assert!(tmp.join("web").exists(), "export directory should have been created");

    std::fs::remove_dir_all(&tmp).ok();
}
```

- [ ] **Step 5: TDD + commit**

Confirm all three fail, then pass. Full suite `cargo test -q` should grow by 3.

```bash
git add src/tui.rs
git commit -m "tui: begin_export_flow + commit_export_to + copy_dir_recursive helper"
```

---

## Task 4: `E` key binding + help text

**Files:**
- Modify: `src/tui.rs`

- [ ] **Step 1: Tests**

```rust
#[test]
fn e_key_with_validation_errors_opens_validation_modal() {
    let mut app = App::new(Site::starter(), None, AppTheme::default());
    app.site.pages[0].slug = "".to_string();
    send_key(&mut app, KeyCode::Char('E'), KeyModifiers::SHIFT);
    assert!(matches!(app.modal, Some(Modal::ValidationErrors { .. })));
}

#[test]
fn e_key_with_clean_site_and_no_export_dir_opens_path_prompt() {
    let mut app = App::new(Site::starter(), None, AppTheme::default());
    send_key(&mut app, KeyCode::Char('E'), KeyModifiers::SHIFT);
    assert!(matches!(app.modal, Some(Modal::ExportPathPrompt { .. })));
}
```

Using `Shift+E` rather than lowercase `e` because lowercase is free at the global level but SHIFT reads as the "do it" convention used elsewhere (`Shift+A` / `Shift+X` / `Shift+J` / `Shift+K` in Pages panel). Also avoids future collision with potential in-component-tree lowercase bindings.

- [ ] **Step 2: Implement the binding**

In the top-level `match k.code` inside `handle_event`, next to `KeyCode::F(3)`:

```rust
                KeyCode::Char('E') if k.modifiers.contains(KeyModifiers::SHIFT) => {
                    self.begin_export_flow();
                }
```

- [ ] **Step 3: Help text**

In `help_text()` Global section:

```rust
        "  F3: Validate site (shows errors in a modal)",
        "  Shift+E: Export site to HTML (validates first; prompts for output dir on first use)",
        "  Ctrl+Q: Quit",
```

- [ ] **Step 4: TDD cycle + commit**

```bash
git add src/tui.rs
git commit -m "tui: Shift+E triggers export flow; help text updated"
```

---

## Task 5: Smoke verify

- [ ] **Step 1: Full suite**

`cargo test -q 2>&1 | tail -3` — expect previous + 5 new (58 → 63 or whatever the current count+5 is at that point).

- [ ] **Step 2: Manual smoke**

```bash
cargo run -- tui /tmp/dd_export_smoke.json
```

Walk:
1. `F1` — help lists Shift+E.
2. `Shift+E` on clean starter — prompt opens with `./web/` pre-filled.
3. Enter — success toast reports N pages exported; `site.json` gets saved with `export_dir`; `./web/` contains HTML.
4. `Shift+E` again — exports directly to `./web/` (no prompt) with fresh toast.
5. Introduce a validation error (clear `[HEAD]` Title via page-head modal, Ctrl+S). `Shift+E` — ValidationErrors modal opens instead of exporting.
6. Dismiss (Enter), fix the error, `Shift+E` — succeeds again.

- [ ] **Step 3: (Optional) Verify image copy behaves**

Create `/tmp/<smoke-dir>/source/images/foo.jpg` (a tiny file). Re-export. Confirm `./web/assets/images/foo.jpg` exists.

---

## Self-Review Notes

- **Spec coverage (§2):**
  - `Site.export_dir: Option<String>` persisted with `#[serde(default)]` — Task 1.
  - First-export prompt pre-filled `./web/` — Task 2 (ExportPathPrompt) + Task 3 (begin_export_flow path when `None`).
  - Strict validation gate reusing `Modal::ValidationErrors` — Task 3 `begin_export_flow`.
  - `Shift+E` binds the flow globally — Task 4.
  - `render_site_to_dir` + recursive `./source/imgs → /assets/imgs` copy — Task 3.
  - Success surfaced as a toast with page count — Task 3.

- **Type consistency:** `Modal::ExportPathPrompt`, `begin_export_flow`, `commit_export_to`, `copy_source_images_to`, `copy_dir_recursive`, `site.export_dir` appear consistently across all task steps.

- **Deferred to Plan 4 (Image Assets, §3):**
  - The `Ctrl+P` image picker over `./source/images/`.
  - The validator's `missing-image` check for `assets/images/*` URLs.
  - Plan 3 just mechanically copies whatever is in `./source/images/`; absence is silent.

- **Caveats:**
  - `render_site_to_dir` behavior on errors is pass-through — no partial cleanup. Subsequent exports overwrite.
  - Export path is resolved relative to the site JSON's *directory*. Sites opened without a path (fresh `Site::starter()` session, never saved) resolve relative to the process cwd. That matches the spec and is fine for a small-marketing-site shape.
