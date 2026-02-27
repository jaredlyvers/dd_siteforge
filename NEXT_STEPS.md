# NEXT_STEPS

## Current State (Implemented)
- `dd-hero` editing flow is in place:
  - Enter to edit, Tab/Shift+Tab field traversal, Left/Right option cycling.
  - Secondary link support added.
  - `hero.copy` is multiline (3 rows), markdown/html export conversion.
- `dd-section` editing flow is aligned to row-scoped editing:
  - Enter edits selected row.
  - Space toggles collapse/expand.
  - Parent vs child field isolation is enforced.
- Implemented section components:
  - `dd-cta`
  - `dd-banner`
  - `dd-accordion` (with nested item collection and FAQ behavior)
  - `dd-alternating`
  - `dd-blockquote`
  - `dd-card`
  - `dd-filmstrip`
  - `dd-milestones`
  - `dd-modal` (derived `parent_modal_id` from title)
  - `dd-slider` (derived `parent_uid` from title, random `uid-######` fallback)
- Tree navigation supports nested item rows for collection components, with:
  - Space expand/collapse
  - A/X add/remove item
  - Enter to begin row-appropriate editing
- Insert flow:
  - `/` opens fuzzy component finder including newly added components.
- Save/load:
  - `s` opens save prompt.
  - Launch with file path to reopen saved site state.
- Architecture docs were updated to include latest components/rules.
- Current test baseline:
  - `cargo test -q` passes (`11 passed`).

## Quick Resume
```bash
cargo fmt
cargo test -q
cargo run -- site.json
```

## Next Session Checklist
1. Run focused in-app keyflow checks for newest components:
   - `dd-slider`
   - `dd-modal`
   - `dd-milestones`
2. Confirm generated HTML against each component `.md` one-by-one.
3. Add/extend TUI tests for slider and milestones edit traversal and A/X item actions.
4. Decide whether `site.json` and `web` deletion are intentional before committing.

## Notes
- Working tree currently has many uncommitted edits across:
  - `src/model.rs`
  - `src/renderer.rs`
  - `src/validate.rs`
  - `src/storage.rs`
  - `src/tui.rs`
  - `Architecture.md`
- `components/dd-slider.md` exists and is now aligned enough to build; implementation is in code.
