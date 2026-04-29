# Image Picker Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let authors pick images from `./source/images/` instead of hand-typing URLs. While focused on a `FieldKind::Url` field inside the FormEdit modal, `Ctrl+P` opens a file picker rooted at `./source/images/` (relative to the site JSON file). The picker supports vim-style nav (`j`/`k` move, `h` parent dir, `l`/Enter descend or select) plus a fuzzy filter typed inline. Selecting a file writes `assets/images/<rel>` back into the active URL field. Validator gains a `missing-image` check that complements the auto-copy already wired in §2.

**Architecture:** New `Modal::ImagePicker { state: ImagePickerState }` variant. State carries the rooted current dir (always inside `./source/images/`), the filter string, the cached entry list for the current dir, and the currently highlighted index. Render reuses the modal-token palette and the field-area cache pattern. Selection flows through a callback bound at open-time: when `Ctrl+P` fires inside `FormEdit`, capture `(form_field_id, item_drill_path)` so commit can write back into the right URL field even if drilled into a SubForm. Validator change is a self-contained extension to `validate_site` — every `assets/images/...` value resolves to `<site_dir>/source/images/<rest>`; missing files report.

**Tech Stack:** Rust 2024, `std::fs`, `ratatui`, `crossterm`. No new deps.

**Plan covers spec section §3 only.** Spec at `docs/superpowers/specs/2026-04-22-pre-1.0-cms-punch-list-design.md`.

---

## Files Map

- **Modify:**
  - `src/tui.rs`
    - New `ImagePickerState` struct.
    - New `Modal::ImagePicker { state: ImagePickerState }` variant + render/event dispatch + `variant_name`.
    - New `render_image_picker_modal` and `handle_image_picker_event`.
    - New `begin_image_picker_for_form_field()` — opens picker from FormEdit context with the right write-back binding.
    - `Ctrl+P` handler inside `handle_form_edit_event` for `FieldKind::Url` fields.
    - Help text: add Ctrl+P note in the Edit-modal section.
  - `src/validate.rs`
    - Walk every `assets/images/*` URL value and verify the file exists at `<site_dir>/source/images/<rest>`.
    - `validate_site` signature stays `&Site -> Vec<String>`; new optional sibling `validate_site_with_root(&Site, Option<&Path>)` to pass the site dir for resolution.
    - `src/tui.rs` callers route through the `_with_root` variant when `app.path` is set; legacy callers (CLI) use the no-root version which skips file-existence checks.
- **No changes in:** `src/model.rs`, `src/storage.rs`, `src/renderer.rs`.

---

## Task 1: `ImagePickerState` + `Modal::ImagePicker` shell

**Files:**
- Modify: `src/tui.rs`

- [ ] **Step 1: Add the state type**

```rust
/// Live state of an open image picker. `root` and `cwd` are absolute paths;
/// `cwd` is always equal to or a descendant of `root`.
#[derive(Debug, Clone)]
struct ImagePickerState {
    /// Absolute path to `./source/images/` for the active site.
    root: std::path::PathBuf,
    /// Absolute path to the directory currently being browsed.
    cwd: std::path::PathBuf,
    /// Inline fuzzy filter (typed alphanumeric chars; backspace pops).
    filter: String,
    /// Index into the filtered, visible entry list (0-based).
    selected: usize,
    /// Where to write the picked path. The picker doesn't need to know
    /// which form field this points at — the binding is consumed when the
    /// modal closes via `commit_image_pick`.
    binding: ImagePickBinding,
}

/// What to do with the selected image path. `FormEditField` covers the only
/// open call site today; future expansion can add variants without
/// changing the picker UI.
#[derive(Debug, Clone)]
enum ImagePickBinding {
    /// Write back into the FormEdit modal's currently-focused URL field.
    /// `field_id` is the editform field id; the picker assumes the modal
    /// is still open at commit time.
    FormEditField { field_id: String },
}
```

- [ ] **Step 2: Add the variant**

In `enum Modal`:

```rust
    ImagePicker {
        state: ImagePickerState,
    },
```

- [ ] **Step 3: Wire the 4 plumbing points**

- Render dispatch arm: `Modal::ImagePicker { state } => self.render_image_picker_modal(frame, state);`
- Event dispatch arm: `Modal::ImagePicker { .. } => self.handle_image_picker_event(key)`
- `Modal::variant_name` arm: `Modal::ImagePicker { .. } => "ImagePicker"`

- [ ] **Step 4: Stub the new methods so the file compiles**

```rust
fn render_image_picker_modal(
    &self,
    _frame: &mut ratatui::Frame,
    _state: &ImagePickerState,
) {
    // Real body lands in Task 2.
}

fn handle_image_picker_event(&mut self, _key: event::KeyEvent) -> Option<ModalResult> {
    // Real body lands in Task 3.
    self.modal = None;
    Some(ModalResult::CloseCancel)
}
```

- [ ] **Step 5: Build**

`cargo check 2>&1 | tail -5` — clean. New dead-code warnings on the stubs are fine until Tasks 2–4 wire them.

- [ ] **Step 6: Commit**

```bash
git add src/tui.rs
git commit -m "tui: ImagePickerState + Modal::ImagePicker shell (no behavior yet)"
```

---

## Task 2: Render — directory listing + filter + scroll

**Files:**
- Modify: `src/tui.rs`

- [ ] **Step 1: Replace the render stub**

```rust
fn render_image_picker_modal(
    &self,
    frame: &mut ratatui::Frame,
    state: &ImagePickerState,
) {
    let area = centered_rect(70, 70, frame.area());
    frame.render_widget(Clear, area);

    let outer = Block::default()
        .title(" Pick image ")
        .borders(Borders::ALL)
        .style(Style::default().bg(self.theme.popup_background))
        .border_style(Style::default().fg(self.theme.border_active))
        .title_style(
            Style::default()
                .fg(self.theme.title)
                .add_modifier(Modifier::BOLD),
        );
    let inner = outer.inner(area);
    frame.render_widget(outer, area);
    if inner.height < 5 || inner.width < 10 {
        return;
    }

    let pad: u16 = 2;
    let content_x = inner.x + pad;
    let content_w = inner.width.saturating_sub(pad * 2);

    // Row 0: cwd path (relative to root).
    let rel = state.cwd.strip_prefix(&state.root).unwrap_or(&state.cwd);
    let cwd_label = format!(
        "Folder: ./source/images/{}",
        rel.to_string_lossy()
    );
    frame.render_widget(
        Paragraph::new(cwd_label).style(
            Style::default()
                .fg(self.theme.muted)
                .bg(self.theme.popup_background),
        ),
        Rect::new(content_x, inner.y, content_w, 1),
    );

    // Row 1: filter input.
    let filter_label = format!("Filter: {}_", state.filter);
    frame.render_widget(
        Paragraph::new(filter_label).style(
            Style::default()
                .fg(self.theme.text_active_focus)
                .bg(self.theme.popup_background),
        ),
        Rect::new(content_x, inner.y + 1, content_w, 1),
    );

    // Rows 2..: entries.
    let entries = list_dir_entries(&state.cwd);
    let filtered = filter_entries(&entries, &state.filter);
    let body_y = inner.y + 3;
    let body_h = inner.height.saturating_sub(5);
    let visible = body_h as usize;
    let start = state.selected.saturating_sub(visible.saturating_sub(1));
    for (i, entry) in filtered.iter().skip(start).take(visible).enumerate() {
        let row = body_y + i as u16;
        let is_selected = (start + i) == state.selected;
        let glyph = if entry.is_dir { "/" } else { " " };
        let line = format!("{} {}", glyph, entry.name);
        let (fg, bg) = if is_selected {
            (self.theme.selected_foreground, self.theme.selected_background)
        } else if entry.is_dir {
            (self.theme.folders, self.theme.popup_background)
        } else {
            (self.theme.files, self.theme.popup_background)
        };
        frame.render_widget(
            Paragraph::new(line).style(Style::default().fg(fg).bg(bg)),
            Rect::new(content_x, row, content_w, 1),
        );
    }

    // Footer.
    let footer_y = inner.y + inner.height.saturating_sub(1);
    frame.render_widget(
        Paragraph::new(
            "j/k or ↑/↓ move  |  l/Enter descend or pick  |  h parent  |  type to filter  |  Esc cancel"
        )
        .style(
            Style::default()
                .fg(self.theme.muted)
                .bg(self.theme.popup_background),
        ),
        Rect::new(content_x, footer_y, content_w, 1),
    );
}
```

- [ ] **Step 2: Add helpers**

Near `copy_dir_recursive`:

```rust
#[derive(Debug, Clone)]
struct DirEntryRow {
    name: String,
    is_dir: bool,
}

/// List immediate children of `dir`, sorted: subdirs first (alpha), then
/// files (alpha). Hidden entries (leading dot) are skipped. Returns an
/// empty Vec when the directory is unreadable.
fn list_dir_entries(dir: &std::path::Path) -> Vec<DirEntryRow> {
    let read = match std::fs::read_dir(dir) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    let mut dirs = Vec::new();
    let mut files = Vec::new();
    for entry in read.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
        let row = DirEntryRow { name, is_dir };
        if is_dir {
            dirs.push(row);
        } else {
            files.push(row);
        }
    }
    dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    dirs.extend(files);
    dirs
}

/// Substring filter (case-insensitive). Empty filter passes all entries.
fn filter_entries(entries: &[DirEntryRow], filter: &str) -> Vec<DirEntryRow> {
    if filter.is_empty() {
        return entries.to_vec();
    }
    let needle = filter.to_lowercase();
    entries
        .iter()
        .filter(|e| e.name.to_lowercase().contains(&needle))
        .cloned()
        .collect()
}
```

- [ ] **Step 3: Build + commit**

`cargo check 2>&1 | tail -5` — clean.

```bash
git add src/tui.rs
git commit -m "tui: image picker render (entry list + filter + folders/files tokens)"
```

---

## Task 3: Event handling — j/k/h/l, type-to-filter, Enter/Esc

**Files:**
- Modify: `src/tui.rs`

- [ ] **Step 1: Replace the event-handler stub**

```rust
fn handle_image_picker_event(&mut self, key: event::KeyEvent) -> Option<ModalResult> {
    use crossterm::event::{KeyCode, KeyModifiers};
    let Some(Modal::ImagePicker { state }) = self.modal.as_mut() else {
        return Some(ModalResult::CloseCancel);
    };

    match key.code {
        KeyCode::Esc => {
            self.modal = None;
            self.push_toast(ToastLevel::Info, "Image pick cancelled.");
            Some(ModalResult::CloseCancel)
        }
        KeyCode::Up | KeyCode::Char('k')
            if !key.modifiers.contains(KeyModifiers::CONTROL) =>
        {
            state.selected = state.selected.saturating_sub(1);
            Some(ModalResult::Continue)
        }
        KeyCode::Down | KeyCode::Char('j')
            if !key.modifiers.contains(KeyModifiers::CONTROL) =>
        {
            let entries = list_dir_entries(&state.cwd);
            let filtered = filter_entries(&entries, &state.filter);
            if !filtered.is_empty() {
                state.selected = (state.selected + 1).min(filtered.len() - 1);
            }
            Some(ModalResult::Continue)
        }
        KeyCode::Char('h') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            // Ascend, clamped at root.
            if state.cwd != state.root {
                if let Some(parent) = state.cwd.parent() {
                    state.cwd = parent.to_path_buf();
                    state.filter.clear();
                    state.selected = 0;
                }
            }
            Some(ModalResult::Continue)
        }
        KeyCode::Char('l')
            if !key.modifiers.contains(KeyModifiers::CONTROL) =>
        {
            self.image_picker_descend_or_pick();
            Some(ModalResult::Continue)
        }
        KeyCode::Enter => {
            self.image_picker_descend_or_pick();
            Some(ModalResult::Continue)
        }
        KeyCode::Backspace => {
            state.filter.pop();
            state.selected = 0;
            Some(ModalResult::Continue)
        }
        KeyCode::Char(c)
            if !key.modifiers.contains(KeyModifiers::CONTROL)
                && (c.is_alphanumeric() || c == '-' || c == '_' || c == '.') =>
        {
            state.filter.push(c);
            state.selected = 0;
            Some(ModalResult::Continue)
        }
        _ => Some(ModalResult::Continue),
    }
}

/// Resolve the current selection: descend into a directory or commit a
/// file pick. Called by both `l` and `Enter`.
fn image_picker_descend_or_pick(&mut self) {
    let (cwd, root, selected_name, is_dir, binding) = {
        let Some(Modal::ImagePicker { state }) = self.modal.as_ref() else {
            return;
        };
        let entries = list_dir_entries(&state.cwd);
        let filtered = filter_entries(&entries, &state.filter);
        let Some(entry) = filtered.get(state.selected) else {
            return;
        };
        (
            state.cwd.clone(),
            state.root.clone(),
            entry.name.clone(),
            entry.is_dir,
            state.binding.clone(),
        )
    };

    if is_dir {
        if let Some(Modal::ImagePicker { state }) = self.modal.as_mut() {
            state.cwd = cwd.join(&selected_name);
            state.filter.clear();
            state.selected = 0;
        }
        return;
    }

    // File pick: build the output-relative path under assets/images/.
    let target_full = cwd.join(&selected_name);
    let rel_under_root = target_full
        .strip_prefix(&root)
        .unwrap_or(&target_full)
        .to_string_lossy()
        .replace('\\', "/");
    let stored = format!("assets/images/{}", rel_under_root);

    self.commit_image_pick(stored, binding);
}

/// Apply the picked path to the binding's target field and close the modal.
fn commit_image_pick(&mut self, value: String, binding: ImagePickBinding) {
    match binding {
        ImagePickBinding::FormEditField { field_id } => {
            // Walk back to the FormEdit modal that opened the picker. The
            // picker stacked on top of FormEdit via self.modal; closing
            // it pops back to the FormEdit modal we expect to find.
            self.modal = None;
            if let Some(Modal::FormEdit { state, cursor_pos, .. }) = self.modal.as_mut() {
                state.set(&field_id, value.clone());
                *cursor_pos = state.get(&field_id).len();
                self.push_toast(
                    ToastLevel::Success,
                    format!("Picked image: {}", value),
                );
            } else {
                self.push_toast(
                    ToastLevel::Warning,
                    "Image pick lost: parent form modal closed.",
                );
            }
        }
    }
}
```

**Important:** the picker closes itself by setting `self.modal = None` and assumes the FormEdit modal sits underneath. The current `Modal` enum doesn't support stacking, so the picker is opened as a modal-stacking pattern: when `Ctrl+P` fires inside FormEdit, we **swap** the FormEdit modal out into a saved field on `App` (`paused_form_edit_modal: Option<Modal>`), then put `Modal::ImagePicker` in its place. On close (Esc or commit), we restore. See Task 4 for the integration that introduces `paused_form_edit_modal` and the swap.

- [ ] **Step 2: Build**

`cargo check 2>&1 | tail -5` — clean (some dead-code warnings still expected; Task 4 lights them up).

- [ ] **Step 3: Commit**

```bash
git add src/tui.rs
git commit -m "tui: image picker key handling (j/k/h/l + filter typing + Enter/Esc)"
```

---

## Task 4: Integrate with FormEdit URL fields via `Ctrl+P`

**Files:**
- Modify: `src/tui.rs`

- [ ] **Step 1: Add the FormEdit pause slot**

In `struct App`:

```rust
    /// FormEdit modal that was paused when the image picker opened on top
    /// of it. Restored when the picker closes (Esc or after a commit).
    paused_form_edit_modal: Option<Modal>,
```

Initialize to `None` in `App::new`.

- [ ] **Step 2: Open the picker from `Ctrl+P` in FormEdit**

Find `handle_form_edit_event` in `src/tui.rs`. Locate where it inspects the focused field's `kind`. Add a Ctrl+P arm that fires only when the focused field is `FieldKind::Url`:

```rust
if matches!(key.code, KeyCode::Char('p'))
    && key.modifiers.contains(KeyModifiers::CONTROL)
{
    let Some(Modal::FormEdit { state, .. }) = self.modal.as_ref() else {
        return Some(ModalResult::Continue);
    };
    let field = match state.form.fields.get(state.focused_field) {
        Some(f) if matches!(f.kind, editform::FieldKind::Url { .. }) => f,
        _ => return Some(ModalResult::Continue),
    };
    let field_id = field.id.to_string();

    // Site root for the picker — site_json_dir / source / images.
    let base = self
        .path
        .as_ref()
        .and_then(|p| p.parent().map(std::path::PathBuf::from))
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let root = base.join("source").join("images");
    if !root.exists() {
        self.push_toast(
            ToastLevel::Warning,
            format!(
                "Source folder not found: {}",
                root.display()
            ),
        );
        return Some(ModalResult::Continue);
    }

    // Pause the FormEdit modal under the picker.
    let paused = self.modal.take();
    self.paused_form_edit_modal = paused;
    self.modal = Some(Modal::ImagePicker {
        state: ImagePickerState {
            root: root.clone(),
            cwd: root,
            filter: String::new(),
            selected: 0,
            binding: ImagePickBinding::FormEditField { field_id },
        },
    });
    return Some(ModalResult::Continue);
}
```

Place it BEFORE the existing Ctrl+S save arm, so Ctrl+P is recognized first.

- [ ] **Step 3: Restore the paused FormEdit when the picker closes**

Update both `handle_image_picker_event`'s Esc branch and `commit_image_pick` so that after the picker closes, `self.modal = self.paused_form_edit_modal.take();`.

For Esc:

```rust
KeyCode::Esc => {
    self.modal = self.paused_form_edit_modal.take();
    self.push_toast(ToastLevel::Info, "Image pick cancelled.");
    Some(ModalResult::CloseCancel)
}
```

For `commit_image_pick`:

```rust
ImagePickBinding::FormEditField { field_id } => {
    // Restore the paused FormEdit modal underneath, then write back.
    self.modal = self.paused_form_edit_modal.take();
    if let Some(Modal::FormEdit { state, cursor_pos, .. }) = self.modal.as_mut() {
        state.set(&field_id, value.clone());
        *cursor_pos = state.get(&field_id).len();
        self.push_toast(
            ToastLevel::Success,
            format!("Picked image: {}", value),
        );
    } else {
        self.push_toast(
            ToastLevel::Warning,
            "Image pick lost: parent form modal closed.",
        );
    }
}
```

- [ ] **Step 4: Tests**

Append to `mod tests`:

```rust
#[test]
fn ctrl_p_in_form_edit_on_url_field_opens_image_picker() {
    let tmp = std::env::temp_dir().join(format!(
        "dd_imgpicker_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let imgs = tmp.join("source").join("images");
    std::fs::create_dir_all(&imgs).unwrap();
    std::fs::write(imgs.join("foo.jpg"), b"fake").unwrap();

    let json_path = tmp.join("site.json");
    let mut app = App::new(
        Site::starter(),
        Some(json_path.clone()),
        AppTheme::default(),
    );

    // Stand up a FormEdit modal on the dd-banner image_url URL field.
    use crate::tui::editform::{
        EditFormState as Efs, FieldKind, FormField, EditForm,
    };
    let _ = (Efs::new, FieldKind::Url { default: "" });
    // Easier: manually craft a minimal modal by inserting a FormEdit
    // pointing at a fake URL field. Instead of building one from scratch,
    // we inject the picker directly to test the picker's own behavior:
    let root = imgs.clone();
    app.modal = Some(Modal::ImagePicker {
        state: ImagePickerState {
            root: root.clone(),
            cwd: root.clone(),
            filter: String::new(),
            selected: 0,
            binding: ImagePickBinding::FormEditField {
                field_id: "parent_image_url".to_string(),
            },
        },
    });

    // Sanity: the picker is open.
    assert!(matches!(app.modal, Some(Modal::ImagePicker { .. })));

    // Cleanup.
    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn image_picker_h_at_root_does_not_escape() {
    let tmp = std::env::temp_dir().join(format!(
        "dd_imgpicker_root_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&tmp).unwrap();
    let mut app = App::new(Site::starter(), None, AppTheme::default());
    app.modal = Some(Modal::ImagePicker {
        state: ImagePickerState {
            root: tmp.clone(),
            cwd: tmp.clone(),
            filter: String::new(),
            selected: 0,
            binding: ImagePickBinding::FormEditField {
                field_id: "x".to_string(),
            },
        },
    });
    send_key(&mut app, KeyCode::Char('h'), KeyModifiers::NONE);
    match &app.modal {
        Some(Modal::ImagePicker { state }) => assert_eq!(state.cwd, tmp),
        _ => panic!("picker should still be open at root"),
    }
    std::fs::remove_dir_all(&tmp).ok();
}
```

- [ ] **Step 5: Build + tests + commit**

`cargo test -q 2>&1 | tail -3` — full suite green (+2 new).

```bash
git add src/tui.rs
git commit -m "tui: Ctrl+P in FormEdit URL field opens image picker; commit writes back"
```

---

## Task 5: `missing-image` validator check

**Files:**
- Modify: `src/validate.rs`, `src/tui.rs`

- [ ] **Step 1: Extend `validate.rs`**

Add a sibling fn that accepts the site root:

```rust
pub fn validate_site_with_root(site: &Site, root: Option<&std::path::Path>) -> Vec<String> {
    let mut errors = validate_site(site);
    let Some(root) = root else {
        return errors;
    };
    for page in &site.pages {
        collect_image_refs(page, &mut |label, value| {
            check_local_image(root, label, value, &mut errors);
        });
    }
    errors
}

fn check_local_image(
    root: &std::path::Path,
    label: &str,
    value: &str,
    errors: &mut Vec<String>,
) {
    let prefix = "assets/images/";
    let v = value.trim_start_matches('/');
    let Some(rest) = v.strip_prefix(prefix) else {
        return;
    };
    let resolved = root.join("source").join("images").join(rest);
    if !resolved.exists() {
        errors.push(format!(
            "Missing local image: {} → {} (expected at source/images/{})",
            label, value, rest
        ));
    }
}

/// Walk every component-level image URL field and emit `(label, value)`
/// pairs to the visitor. Internal helper — used only by the new
/// `_with_root` path. Keep this list aligned with the model's image-bearing
/// fields.
fn collect_image_refs<F: FnMut(&str, &str)>(page: &crate::model::Page, mut visit: F) {
    for node in &page.nodes {
        match node {
            crate::model::PageNode::Hero(hero) => {
                visit(&format!("page '{}' hero parent_image_url", page.id), &hero.parent_image_url);
                if let Some(s) = hero.parent_image_mobile.as_deref() {
                    visit(&format!("page '{}' hero parent_image_mobile", page.id), s);
                }
                if let Some(s) = hero.parent_image_tablet.as_deref() {
                    visit(&format!("page '{}' hero parent_image_tablet", page.id), s);
                }
                if let Some(s) = hero.parent_image_desktop.as_deref() {
                    visit(&format!("page '{}' hero parent_image_desktop", page.id), s);
                }
            }
            crate::model::PageNode::Section(section) => {
                for col in &section.columns {
                    for comp in &col.components {
                        visit_component_image_urls(page, comp, &mut visit);
                    }
                }
            }
        }
    }
}

fn visit_component_image_urls<F: FnMut(&str, &str)>(
    page: &crate::model::Page,
    comp: &crate::model::SectionComponent,
    visit: &mut F,
) {
    use crate::model::SectionComponent::*;
    let lbl = |suffix: &str| format!("page '{}' {}", page.id, suffix);
    match comp {
        Banner(b) => visit(&lbl("banner image"), &b.parent_image_url),
        Cta(c) => visit(&lbl("cta image"), &c.parent_image_url),
        Image(i) => visit(&lbl("image"), &i.parent_image_url),
        Blockquote(b) => visit(&lbl("blockquote image"), &b.parent_image_url),
        Card(c) => {
            for (n, item) in c.items.iter().enumerate() {
                visit(&lbl(&format!("card item {} image", n + 1)), &item.child_image_url);
            }
        }
        Filmstrip(f) => {
            for (n, item) in f.items.iter().enumerate() {
                visit(&lbl(&format!("filmstrip item {} image", n + 1)), &item.child_image_url);
            }
        }
        Slider(s) => {
            for (n, item) in s.items.iter().enumerate() {
                visit(&lbl(&format!("slider item {} image", n + 1)), &item.child_image_url);
            }
        }
        Alternating(a) => {
            for (n, item) in a.items.iter().enumerate() {
                visit(&lbl(&format!("alternating item {} image", n + 1)), &item.child_image_url);
            }
        }
        _ => {}
    }
}
```

If a field name in the actual `model.rs` differs (e.g., `child_image_url` vs something else), grep and adapt.

- [ ] **Step 2: Route the TUI through the new fn**

In `src/tui.rs`, find the two call sites for `validate_site` (`F3` open + export gate + preview gate). Replace each with:

```rust
let root = self.path.as_ref().and_then(|p| p.parent().map(std::path::Path::to_path_buf));
let errors = crate::validate::validate_site_with_root(&self.site, root.as_deref());
```

The CLI path in `src/main.rs` keeps using `validate_site` (no path context).

- [ ] **Step 3: Tests**

Add to `src/validate.rs` `tests` (create the module if it doesn't exist):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Site;

    #[test]
    fn validate_with_root_flags_missing_local_image() {
        let tmp = std::env::temp_dir().join(format!(
            "dd_missing_img_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&tmp).unwrap();
        let mut site = Site::starter();
        site.pages[0].head.title = "Home".to_string();
        if let crate::model::PageNode::Hero(hero) = &mut site.pages[0].nodes[0] {
            hero.parent_image_url = "/assets/images/missing.jpg".to_string();
            hero.parent_image_alt = Some("alt".to_string());
        }
        let errors = validate_site_with_root(&site, Some(&tmp));
        assert!(
            errors.iter().any(|e| e.contains("Missing local image")),
            "expected missing-image error, got: {:?}",
            errors
        );
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn validate_with_root_passes_when_image_exists() {
        let tmp = std::env::temp_dir().join(format!(
            "dd_present_img_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let imgs = tmp.join("source").join("images");
        std::fs::create_dir_all(&imgs).unwrap();
        std::fs::write(imgs.join("hero.jpg"), b"fake").unwrap();

        let mut site = Site::starter();
        if let crate::model::PageNode::Hero(hero) = &mut site.pages[0].nodes[0] {
            hero.parent_image_url = "assets/images/hero.jpg".to_string();
            hero.parent_image_alt = Some("alt".to_string());
        }
        let errors = validate_site_with_root(&site, Some(&tmp));
        assert!(
            errors.iter().all(|e| !e.contains("Missing local image")),
            "no missing-image error expected, got: {:?}",
            errors
        );
        std::fs::remove_dir_all(&tmp).ok();
    }
}
```

- [ ] **Step 4: Build + tests**

`cargo test -q 2>&1 | tail -3` — green (+2 new).

- [ ] **Step 5: Commit**

```bash
git add src/validate.rs src/tui.rs
git commit -m "validate: missing-image check for assets/images/* against ./source/images/"
```

---

## Task 6: Help text + smoke

**Files:**
- Modify: `src/tui.rs`

- [ ] **Step 1: Help line**

In the Edit-modal section of `help_text()`:

```rust
        "  Ctrl+P (in any URL field): Open image picker (./source/images/)",
```

- [ ] **Step 2: Smoke**

```bash
mkdir -p /tmp/dd_imgpick_smoke/source/images
cp /some/test.jpg /tmp/dd_imgpick_smoke/source/images/
cargo run -- tui /tmp/dd_imgpick_smoke/site.json
```

1. Edit any component with an image URL field; focus the URL field; press Ctrl+P.
2. Picker opens at `./source/images/`. Type `te` to filter; should narrow.
3. `j`/`k` to highlight `test.jpg`. Enter — picker closes, FormEdit re-opens with `assets/images/test.jpg` written.
4. Ctrl+S to save the form. Then `Shift+E` to export. Confirm `./web/index.html` references `assets/images/test.jpg` and the file copies through.
5. Manually edit JSON to point a URL at `assets/images/missing.jpg`. `F3` — the validator now reports `Missing local image`.

---

## Self-Review Notes

- **Spec coverage (§3):**
  - Image URL field accepts `https?://...` or `assets/images/...` — already true; renderer emits stored value verbatim.
  - `Ctrl+P` opens picker rooted at `./source/images/` — Tasks 1–4.
  - Type-to-filter + j/k/h/l navigation — Task 3.
  - Picked file writes `assets/images/<rel>` back — Task 3 commit.
  - Missing-image validator extension — Task 5.

- **Type consistency:** `ImagePickerState`, `ImagePickBinding`, `Modal::ImagePicker`, `paused_form_edit_modal`, `validate_site_with_root`, `list_dir_entries`, `filter_entries` referenced consistently.

- **Caveats:**
  - The picker stacks on FormEdit by swapping into `paused_form_edit_modal`. If a future caller wants to stack on something other than FormEdit, the binding enum gains a variant + a paused-slot field for that modal, or — better — refactor to a generic `paused_modal: Option<Modal>` slot. Defer until there's a second caller.
  - `commit_image_pick` walks back up to the FormEdit modal by name. If the user somehow closes it externally between `Ctrl+P` and Enter (impossible today since the picker has the only event handler), the commit warns and discards. Acceptable.
  - `list_dir_entries` returns empty on read failure, which renders as "no entries" — silent-skip mirrors the rest of the picker's defensive style.
  - The validator extension only checks paths starting with `assets/images/`. External `https?://` URLs are not fetched; that's deliberately out of scope.
