# CLAUDE.md

Guidance for Claude / agent sessions working on this repo.

## What this is

`dd_staticsite` is a Rust terminal-UI CMS shipped at `v1.0.0`. The pre-1.0 punch list (page CRUD, validation modal, in-TUI export, autosave + backup, preview, image picker, theme schema) is complete. The code is on `master`, tagged `v1.0.0`. Architecture lives in `Architecture.md`; per-feature design + implementation history lives under `docs/superpowers/`.

## Project conventions

### Branch + commit style
- Feature work happens on `feat/<short-name>` branches off `master` (or `dd-header` if that's where active work sits).
- Commits use plain prefixes: `tui:`, `model:`, `validate:`, `docs:`, `test:`. No conventional-commits trailers.
- Co-author trailer for AI commits is `Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>`.
- Merges into long-lived branches use regular fast-forward when possible (the user has confirmed this style).
- Tags: `vMAJOR.MINOR.PATCH` annotated, summarizing what shipped vs the prior tag.

### Docs convention
- `Architecture.md` — concise module map, render/validation rules, key bindings. Update when surface area changes.
- `docs/superpowers/specs/` — design specs (one file per feature batch). Don't rewrite history here; if a spec drifts, revise the language but preserve the intent.
- `docs/superpowers/plans/` — implementation plans (TDD task list per feature). One per feature.
- `components/dd-*.md` — single source of truth for each component's fields, render rules, validation. Update these when a component spec changes; the model/renderer/validator follow.

### Code style
- Tests live in `#[cfg(test)] mod tests` blocks at the bottom of each module. Drive TUI behavior via the in-tree `send_key` helper rather than directly poking state where possible.
- New fields on `model::Site` / `model::Page` get `#[serde(default)]` so legacy JSON keeps loading.
- New TUI modal variants follow the four-point plumbing: enum variant + render dispatch + event dispatch + `Modal::variant_name` arm.
- Multi-field modal fields render through `render_edit_modal_unified` (Modal::Edit) or `render_form_edit_modal` (Modal::FormEdit). Single-input prompts share `render_single_input_modal`.
- Render functions are `&self`. State that the event loop needs from render (e.g. cached field rects, `help_scroll_max`) goes through `RefCell` or pre-publish-into-`&mut self` fields.

### Theme tokens
Always source colors from `self.theme.*`. Standard mappings (per `THEME_STRUCTURE_STANDARD.md`):
- Labels: `text_labels` default → `text_active_focus` when the field is focused
- Input borders: `input_border_default` / `input_border_focus`
- Input text: `input_text_default` / `input_text_focus`
- Cursor: paint a 1-cell overlay with `bg(cursor)` on top of the placed terminal cursor
- Folders / files / links: `folders` / `files` / `links` (used in the image picker)

### User preferences captured during the v1.0 push
- Toasts (success / info / warning) for non-blocking messages; modals for errors.
- Status bar reserved for instructional/in-progress text ("Rename page. Edit and press Enter."), not transient confirmations.
- All scrollable surfaces support mouse wheel + keyboard.
- All multi-field modal inputs support click-to-focus.
- Help modal text wraps + scrolls + has a scrollbar.
- Path display normalized: strip leading `./` and trailing `/` so `././web/` style artifacts don't appear.
- Browser launch (`p`) pins stdio to `/dev/null` so it never scrambles raw-mode TUI.
- The author's response style preference is "caveman speak" (per the user's auto-memory entry); apply only to user-facing chat text, not to code, commit messages, file contents, or doc edits.

## What lives where

```
src/
├── main.rs            CLI entry (init-site / show-site / validate-site / export-html / tui)
├── model.rs           Site → Page → Node typed tree + Page::from_template / duplicate_from
├── storage.rs         JSON load/save round-trip + slug_locked / export_dir back-compat tests
├── validate.rs        validate_site + validate_site_with_root (missing-image)
├── renderer.rs        per-component render_* fns; handlebars templates inline
├── tui.rs             ~19k lines — App, Modal enum (16 variants), draw + event loop
└── tui/
    ├── cursor.rs      component → form-state mapping; apply_edit_form_to_component
    └── editform.rs    declarative FormEdit values for every migrated component

components/dd-*.md     component specs (humans + agents read these)
docs/superpowers/      design specs + implementation plans
Architecture.md        always start here for a high-level orientation
THEME_STRUCTURE_STANDARD.md   token schema
dd_staticsite_theme.yml       default theme values
```

## Build + test

```bash
cargo check     # typecheck only
cargo test -q   # 96 tests, all should pass
cargo build --release
cargo run -- tui /tmp/scratch.json   # interactive smoke
```

For interactive smoke testing, the user typically launches against a scratch path under `/tmp/`.

## Where to add new features

- New component → spec in `components/dd-*.md`, types in `src/model.rs` (with `#[serde(default)]` on optional fields), renderer in `src/renderer.rs`, FormEdit definition in `src/tui/editform.rs`, route in `src/tui/cursor.rs`.
- New modal → see "four-point plumbing" above.
- New CLI subcommand → `src/main.rs`. Mirror the existing pattern (clap derive + `with_context` for IO errors).

## Local state to never commit

`.gitignore`d on master:
- `site.json`, `site.json.backup` — author's working site
- `source/` — author's image source dir
- `web/` — export output (regenerates)
- `.kilo/` — alternative agent tooling

## When the user says "ship"

1. Smoke test passes → merge feature branch (regular fast-forward unless told otherwise).
2. Push the long-lived branch.
3. Annotated tag with a multi-line summary; bump `Cargo.toml` `version` to match.
4. Push the tag.
5. Don't open a PR unless the user asks — the bitbucket remote will print the URL automatically.

## When the user says "draft a plan"

Use the `superpowers:writing-plans` shape: TDD task list, one commit per task, exact code blocks (no placeholders), all the per-task details a fresh agent could execute in isolation. Save to `docs/superpowers/plans/YYYY-MM-DD-<topic>.md`. Commit the plan before starting on the feature branch.

## Anti-patterns to avoid

- Don't add features that aren't requested. The user prefers tight scope.
- Don't proactively run `cargo fmt` or `cargo fix` unless asked.
- Don't bypass git hooks (`--no-verify`) or skip pre-commit checks.
- Don't re-tag — once a tag is pushed, treat it as immutable.
- Don't introduce backwards-compatibility shims for code paths that have no old consumers.
- Don't write docstrings on every function. Default to no comments; add a one-liner only when the *why* is non-obvious.
