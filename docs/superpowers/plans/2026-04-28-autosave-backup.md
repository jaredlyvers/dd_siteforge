# Autosave + Backup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** No more than ~2 seconds of work lost to a crash or mistake. Every model mutation marks the site dirty; a 2-second debounce writes to the active site JSON path. Manual `s` continues to save and additionally writes a byte-identical `<path>.backup` checkpoint. On load, if the backup differs from the current file (i.e. the file is the autosave state, the backup is the last "known good"), surface that fact as a toast so the author can decide whether to roll back via git.

**Architecture:** Three small App fields (`dirty: bool`, `dirty_since: Option<Instant>`, `last_saved_json: String`) and a `tick_autosave(now: Instant)` method called at the top of each main-loop iteration. Mutation detection is snapshot-based: serialize `self.site` to JSON after each event, compare against `last_saved_json`; differences flip dirty true and start the debounce. The 100ms event-poll loop already wakes us up regularly, so the autosave tick has natural cadence with no extra threads. Manual save reuses the existing path on `App.path` and pairs the write with `<path>.backup`. Load-time check runs once in `App::new` when a `path` is provided.

**Tech Stack:** Rust 2024, `serde_json`, `std::fs`, `std::time::Instant`. No new deps. Time injection used only at the test boundary (`tick_autosave(now)` accepts a synthetic `Instant`).

**Plan covers spec section §5 only.** Spec at `docs/superpowers/specs/2026-04-22-pre-1.0-cms-punch-list-design.md`. Image assets (§3) and Preview (§6) follow this.

---

## Files Map

- **Modify:**
  - `src/tui.rs`
    - `App` gains `dirty: bool`, `dirty_since: Option<Instant>`, `last_saved_json: String`.
    - `App::new` seeds `last_saved_json` from the loaded site and runs the load-time backup check.
    - New `mark_dirty_if_changed(&mut self)` recomputes the snapshot and flips dirty.
    - New `tick_autosave(&mut self, now: Instant)` writes when debounce elapses.
    - New `commit_save_with_backup(&mut self, path: &Path)` — manual save + `<path>.backup`.
    - Existing save success branches route through `commit_save_with_backup`.
    - `run` loop: call `mark_dirty_if_changed` after each handled event; call `tick_autosave(Instant::now())` once per loop iteration.
    - Help text adds an `Autosave` line.
- **No changes in:** `src/model.rs`, `src/storage.rs` (already correct shape), `src/validate.rs`, `src/renderer.rs`.

---

## Task 1: App fields + initial snapshot

**Files:**
- Modify: `src/tui.rs`

- [ ] **Step 1: Add fields**

In `struct App`, near `toasts`:

```rust
    /// True when in-memory site differs from `last_saved_json`.
    dirty: bool,
    /// Instant of the first mutation since `last_saved_json` was synced.
    /// `None` while clean.
    dirty_since: Option<std::time::Instant>,
    /// JSON snapshot of the site at the most recent successful disk write.
    /// Used both for dirty detection and for skipping no-op autosaves.
    last_saved_json: String,
```

- [ ] **Step 2: Seed in `App::new`**

After all other field assignments inside `App::new`, before returning the struct (or just compute and pass into the literal — pick what reads cleanly), capture the starting snapshot:

```rust
let last_saved_json =
    serde_json::to_string(&site).unwrap_or_default();
```

Use `last_saved_json` in the struct literal. Initialize `dirty: false, dirty_since: None,`.

- [ ] **Step 3: Build**

`cargo check 2>&1 | tail -5` — clean (dead-code warnings on the new fields are fine until Task 2 reads them).

- [ ] **Step 4: Commit**

```bash
git add src/tui.rs
git commit -m "tui: add App.dirty / dirty_since / last_saved_json (no behavior yet)"
```

---

## Task 2: `mark_dirty_if_changed` + run-loop hook

**Files:**
- Modify: `src/tui.rs`

- [ ] **Step 1: Tests**

Append to `mod tests`:

```rust
#[test]
fn fresh_app_is_clean() {
    let app = App::new(Site::starter(), None, AppTheme::default());
    assert!(!app.dirty);
    assert!(app.dirty_since.is_none());
}

#[test]
fn editing_a_page_title_marks_app_dirty() {
    let mut app = App::new(Site::starter(), None, AppTheme::default());
    app.site.pages[0].head.title = "Mutated".to_string();
    app.mark_dirty_if_changed();
    assert!(app.dirty);
    assert!(app.dirty_since.is_some());
}

#[test]
fn unchanged_model_stays_clean() {
    let mut app = App::new(Site::starter(), None, AppTheme::default());
    app.mark_dirty_if_changed();
    assert!(!app.dirty);
    assert!(app.dirty_since.is_none());
}

#[test]
fn dirty_since_does_not_reset_on_subsequent_mutations() {
    let mut app = App::new(Site::starter(), None, AppTheme::default());
    app.site.pages[0].head.title = "First".to_string();
    app.mark_dirty_if_changed();
    let first = app.dirty_since.expect("dirty_since should be set");
    std::thread::sleep(std::time::Duration::from_millis(5));
    app.site.pages[0].head.title = "Second".to_string();
    app.mark_dirty_if_changed();
    assert_eq!(
        app.dirty_since,
        Some(first),
        "subsequent mutations must NOT push dirty_since forward"
    );
}
```

- [ ] **Step 2: Run tests, verify failure**

`cargo test -q mark_dirty 2>&1 | tail -10` — fail (method missing).

- [ ] **Step 3: Implement**

Add to `impl App`:

```rust
/// Recompute the JSON snapshot of `self.site` and set `dirty` if it
/// differs from `last_saved_json`. Idempotent: re-calling on an already
/// dirty app does NOT advance `dirty_since`, preserving the original
/// debounce anchor.
fn mark_dirty_if_changed(&mut self) {
    let current = match serde_json::to_string(&self.site) {
        Ok(s) => s,
        Err(_) => return,
    };
    if current != self.last_saved_json {
        if !self.dirty {
            self.dirty_since = Some(std::time::Instant::now());
        }
        self.dirty = true;
    }
}
```

- [ ] **Step 4: Wire into `run` loop**

In `fn run<B>` (the loop after `terminal.draw`), after the `handle_event` call, add:

```rust
self.handle_event(evt)?;
self.mark_dirty_if_changed();
```

Don't run `mark_dirty_if_changed` when `event::poll` returned false — there was no event, so no mutation could have happened.

- [ ] **Step 5: Run tests, verify pass**

`cargo test -q 2>&1 | tail -3` — full suite green, +4 tests.

- [ ] **Step 6: Commit**

```bash
git add src/tui.rs
git commit -m "tui: mark_dirty_if_changed via post-event JSON snapshot diff"
```

---

## Task 3: `tick_autosave` with injectable clock

**Files:**
- Modify: `src/tui.rs`

- [ ] **Step 1: Tests** — synthetic `Instant`s, no real delays.

```rust
#[test]
fn tick_autosave_does_nothing_when_clean() {
    let mut app = App::new(Site::starter(), None, AppTheme::default());
    let now = std::time::Instant::now();
    app.tick_autosave(now);
    assert!(!app.dirty);
}

#[test]
fn tick_autosave_does_nothing_when_dirty_but_no_path() {
    let mut app = App::new(Site::starter(), None, AppTheme::default());
    app.site.pages[0].head.title = "x".to_string();
    app.mark_dirty_if_changed();
    let later = app.dirty_since.unwrap()
        + std::time::Duration::from_secs(10);
    app.tick_autosave(later);
    assert!(app.dirty, "no path means no autosave; site stays dirty");
}

#[test]
fn tick_autosave_writes_when_dirty_and_debounce_elapsed() {
    let tmp_dir = std::env::temp_dir().join(format!(
        "dd_autosave_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&tmp_dir).unwrap();
    let json_path = tmp_dir.join("site.json");
    // Seed an initial site on disk so App::new has a stable last_saved_json.
    crate::storage::save_site(&json_path, &Site::starter()).unwrap();

    let mut app =
        App::new(Site::starter(), Some(json_path.clone()), AppTheme::default());
    app.site.pages[0].head.title = "After mutation".to_string();
    app.mark_dirty_if_changed();
    assert!(app.dirty);

    let due = app.dirty_since.unwrap()
        + std::time::Duration::from_millis(2_100);
    app.tick_autosave(due);
    assert!(!app.dirty, "autosave should clear the dirty flag");
    assert!(app.dirty_since.is_none());
    let on_disk = std::fs::read_to_string(&json_path).unwrap();
    assert!(on_disk.contains("After mutation"));
    std::fs::remove_dir_all(&tmp_dir).ok();
}

#[test]
fn tick_autosave_holds_off_within_debounce_window() {
    let mut app = App::new(Site::starter(), None, AppTheme::default());
    app.site.pages[0].head.title = "x".to_string();
    app.mark_dirty_if_changed();
    let still_in_window = app.dirty_since.unwrap()
        + std::time::Duration::from_millis(500);
    app.tick_autosave(still_in_window);
    assert!(app.dirty);
}
```

- [ ] **Step 2: Run tests, verify failure**

`cargo test -q tick_autosave 2>&1 | tail -15` — fail, method missing.

- [ ] **Step 3: Implement**

Add a `const AUTOSAVE_DEBOUNCE: std::time::Duration = std::time::Duration::from_secs(2);` at module scope (top of `src/tui.rs` near other consts) so the value is visible in tests.

```rust
/// If the site is dirty, has a path, and the debounce window has elapsed,
/// write `self.site` to the active path and refresh the saved snapshot.
/// Errors are surfaced as a warning toast and leave `dirty` set so the
/// next tick can retry.
fn tick_autosave(&mut self, now: std::time::Instant) {
    if !self.dirty {
        return;
    }
    let Some(since) = self.dirty_since else {
        // Defensive: dirty without a timestamp shouldn't happen; treat as
        // freshly dirty.
        self.dirty_since = Some(now);
        return;
    };
    if now.duration_since(since) < AUTOSAVE_DEBOUNCE {
        return;
    }
    let Some(path) = self.path.clone() else {
        return;
    };
    match crate::storage::save_site(&path, &self.site) {
        Ok(()) => {
            self.last_saved_json =
                serde_json::to_string(&self.site).unwrap_or_default();
            self.dirty = false;
            self.dirty_since = None;
        }
        Err(e) => {
            let msg = format!("Autosave failed: {}", e);
            self.push_toast(ToastLevel::Warning, msg);
        }
    }
}
```

- [ ] **Step 4: Hook into the run loop**

In `fn run<B>`, before `terminal.draw`:

```rust
self.tick_autosave(std::time::Instant::now());
terminal.draw(|f| self.draw(f))?;
```

Placing it before draw means the dirty flag visible in the (future) status indicator reflects post-tick state.

- [ ] **Step 5: Run tests, verify pass**

`cargo test -q 2>&1 | tail -3` — green, +4 tests.

- [ ] **Step 6: Commit**

```bash
git add src/tui.rs
git commit -m "tui: tick_autosave debounces 2s before writing site.json"
```

---

## Task 4: Manual save also writes `<path>.backup`

**Files:**
- Modify: `src/tui.rs`

- [ ] **Step 1: Tests**

```rust
#[test]
fn manual_save_writes_backup_alongside_main_file() {
    let tmp = std::env::temp_dir().join(format!(
        "dd_backup_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&tmp).unwrap();
    let json_path = tmp.join("site.json");
    let backup_path = tmp.join("site.json.backup");

    let mut app =
        App::new(Site::starter(), Some(json_path.clone()), AppTheme::default());
    app.site.pages[0].head.title = "Pre-save".to_string();

    app.commit_save_with_backup(&json_path)
        .expect("manual save should succeed");

    assert!(json_path.exists(), "main file written");
    assert!(backup_path.exists(), "backup written");
    let main = std::fs::read_to_string(&json_path).unwrap();
    let bak = std::fs::read_to_string(&backup_path).unwrap();
    assert_eq!(main, bak, "backup must be byte-identical to main");
    assert!(!app.dirty, "manual save clears dirty");
    std::fs::remove_dir_all(&tmp).ok();
}
```

- [ ] **Step 2: Run, verify fail**

`cargo test -q manual_save_writes_backup 2>&1 | tail -10`

- [ ] **Step 3: Implement helper**

```rust
/// Write `self.site` to `path` AND to `<path>.backup`. Both writes share
/// a single serialization so the two files are guaranteed byte-identical.
/// Updates the saved snapshot and clears the dirty flag on success.
fn commit_save_with_backup(&mut self, path: &std::path::Path) -> anyhow::Result<()> {
    let json = serde_json::to_string_pretty(&self.site)?;
    std::fs::write(path, &json)?;
    let backup = backup_path_for(path);
    std::fs::write(&backup, &json)?;
    self.last_saved_json = serde_json::to_string(&self.site).unwrap_or_default();
    self.dirty = false;
    self.dirty_since = None;
    self.path = Some(path.to_path_buf());
    Ok(())
}
```

Add a free helper near `copy_dir_recursive`:

```rust
fn backup_path_for(path: &std::path::Path) -> std::path::PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(".backup");
    std::path::PathBuf::from(s)
}
```

- [ ] **Step 4: Route existing save paths through it**

Find every site-write site (`crate::storage::save_site(...)` calls in TUI code) that represents a *manual* save, and replace with `commit_save_with_backup`. The autosave path in `tick_autosave` should remain `storage::save_site` — it must NOT touch the backup, since the backup represents "last manual save".

Specifically check:
- `handle_save_prompt_event_unified` Enter success branch.
- `commit_save_prompt` (legacy SavePrompt path).

Each becomes:

```rust
match self.commit_save_with_backup(&path_buf) {
    Ok(()) => {
        let msg = format!("Saved {}", path_buf.display());
        self.push_toast(ToastLevel::Success, msg);
        // ...result return as before
    }
    Err(e) => {
        let msg = format!("Save failed: {}", e);
        self.push_toast(ToastLevel::Warning, msg);
        // ...result return
    }
}
```

Adapt to the actual return-type contracts in those two methods.

- [ ] **Step 5: Run tests, verify pass + suite still green**

`cargo test -q 2>&1 | tail -3` — green.

- [ ] **Step 6: Commit**

```bash
git add src/tui.rs
git commit -m "tui: manual save writes byte-identical <path>.backup checkpoint"
```

---

## Task 5: Load-time backup divergence toast

**Files:**
- Modify: `src/tui.rs`

- [ ] **Step 1: Test**

```rust
#[test]
fn load_with_diverging_backup_pushes_info_toast() {
    let tmp = std::env::temp_dir().join(format!(
        "dd_loadcheck_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&tmp).unwrap();
    let json_path = tmp.join("site.json");
    let backup_path = tmp.join("site.json.backup");

    // Backup = older known-good content; main = newer autosaved content.
    std::fs::write(&backup_path, "{\"backup\":\"old\"}").unwrap();
    std::fs::write(&json_path, "{\"main\":\"new\"}").unwrap();

    let app = App::new(
        Site::starter(),
        Some(json_path.clone()),
        AppTheme::default(),
    );
    let toast = app
        .toasts
        .iter()
        .find(|t| t.message.to_lowercase().contains("differs from last manual save"));
    assert!(
        toast.is_some(),
        "expected a divergence toast, got: {:?}",
        app.toasts.iter().map(|t| &t.message).collect::<Vec<_>>()
    );
    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn load_with_matching_backup_pushes_no_toast() {
    let tmp = std::env::temp_dir().join(format!(
        "dd_loadcheck_match_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&tmp).unwrap();
    let json_path = tmp.join("site.json");
    let backup_path = tmp.join("site.json.backup");
    std::fs::write(&json_path, "same").unwrap();
    std::fs::write(&backup_path, "same").unwrap();

    let app = App::new(
        Site::starter(),
        Some(json_path.clone()),
        AppTheme::default(),
    );
    assert!(app
        .toasts
        .iter()
        .all(|t| !t.message.to_lowercase().contains("differs")));
    std::fs::remove_dir_all(&tmp).ok();
}
```

- [ ] **Step 2: Run, verify fail**

- [ ] **Step 3: Implement load-time check**

In `App::new`, after the struct literal is constructed and before returning, run:

```rust
if let Some(p) = app.path.as_ref() {
    let backup = backup_path_for(p);
    if backup.exists() && p.exists() {
        if let (Ok(main), Ok(bak)) = (
            std::fs::read_to_string(p),
            std::fs::read_to_string(&backup),
        ) {
            if main != bak {
                let mtime = std::fs::metadata(&backup)
                    .and_then(|m| m.modified())
                    .ok();
                let when = mtime
                    .map(|t| {
                        chrono_like_format(t).unwrap_or_else(|| "unknown".into())
                    })
                    .unwrap_or_else(|| "unknown".into());
                app.push_toast(
                    ToastLevel::Info,
                    format!(
                        "Loaded state differs from last manual save ({}).",
                        when
                    ),
                );
            }
        }
    }
}
```

To avoid pulling in the `chrono` crate, format the mtime with a tiny helper:

```rust
fn chrono_like_format(t: std::time::SystemTime) -> Option<String> {
    let secs = t
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();
    Some(format!("{}s since epoch", secs))
}
```

That's deliberately ugly — me'd rather not invent a date formatter. The test only checks the leading "differs from last manual save" substring, so the time tail is informational. If the team later wants a real timestamp, swap to `time` crate.

You'll need to make `app` mutable: change `Self { ... }` literal at the end of `App::new` to `let mut app = Self { ... };`, then run the check, then `app`.

- [ ] **Step 4: Run tests, verify pass + suite green**

- [ ] **Step 5: Commit**

```bash
git add src/tui.rs
git commit -m "tui: load-time toast when site.json differs from .backup"
```

---

## Task 6: Help text + smoke

**Files:**
- Modify: `src/tui.rs`

- [ ] **Step 1: Help line**

In `help_text()` Global section:

```rust
        "  s: Open save modal and enter file path (also writes a .backup)",
```

Add a separate Autosave note at the bottom of Global:

```rust
        "  Autosave: 2s after a change, the active site JSON is written. The .backup file is only updated by manual `s` saves.",
```

- [ ] **Step 2: Smoke**

```bash
cargo run -- tui /tmp/dd_autosave_smoke.json
```

1. First time: `s` save to `/tmp/dd_autosave_smoke.json`. Confirm both `site.json` and `site.json.backup` exist on disk and are identical.
2. Make any edit (rename a page, swap a field). Wait ~3s. Inspect the file — `site.json` reflects the edit, `site.json.backup` still matches the previous manual save.
3. Edit again, then `s` immediately. Backup file now matches the new state.
4. Manually edit `site.json.backup` to be different (or copy an older version). Quit TUI. Relaunch with the same path. Toast appears: "Loaded state differs from last manual save (...)".
5. `F1` — Autosave note now shows in help.

- [ ] **Step 3: Final commit if anything tweaked during smoke**

---

## Self-Review Notes

- **Spec coverage (§5):**
  - Debounced 2s autosave on every mutation — Tasks 2, 3.
  - Autosave skipped without a saved path — Task 3.
  - Manual `s` writes both files — Task 4.
  - Load-time divergence toast — Task 5.
  - No undo — intentionally absent. Help text reminds the user that git + `<path>.backup` are the rollback tools.

- **Type consistency:** `App.dirty`, `App.dirty_since`, `App.last_saved_json`, `mark_dirty_if_changed`, `tick_autosave`, `commit_save_with_backup`, `backup_path_for`, `AUTOSAVE_DEBOUNCE`, `chrono_like_format` referenced consistently across all 6 tasks.

- **Caveats:**
  - JSON-string snapshot diff is O(N) per event. For 5–20 page marketing sites the JSON is a few KB — fine. If a future site grows large enough that this is measurable, switch to a hash digest of the serialization (still O(N) but with smaller constants and fewer allocations).
  - `chrono_like_format` returns `"<seconds-since-epoch>s since epoch"`, which is functional but ugly. Acceptable for a load-time diagnostic; not for end-user-facing scheduling. Promote to a real formatter if the toast text becomes user-touched anywhere else.
  - Tests use `std::time::Instant` arithmetic to fake the clock without sleeping — `Instant + Duration` is stable across platforms.
  - Autosave intentionally does NOT touch the backup file. The whole point of the backup is "last state the user explicitly committed."
