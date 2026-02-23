# NEXT_STEPS

## Current State (Implemented)
- Nodes panel is now a tree/accordion:
  - Section rows can expand/collapse.
  - Expanded sections show columns and their components.
- Details panel now renders page-level ASCII previews:
  - `dd-hero` is rendered as full-width ASCII.
  - `dd-section` columns/components are rendered responsively to panel width.
- Editing is modal-based.
- Enter key behavior:
  - On section rows: expand/collapse.
  - On hero/column/component rows: opens edit modal.
- Component add workflow:
  - `/` opens component picker modal.
  - Picker supports fuzzy search (`card`, `dd_card`, etc.).
  - `Enter` inserts selected component into active section column.
- Save workflow:
  - `s` opens Save modal.
  - User enters path/filename and confirms with `Enter`.
- Edit UX:
  - Cursor is visible in active editable fields (edit/save/picker modals).
  - `Tab` / `Shift+Tab` cycles between editable component fields.
- Section ID and modifiers:
  - Sections always get default IDs (`section-N`) and remain editable.
  - Section modifier class options are surfaced in details/modal.
- Theme:
  - `theme.yml` updated with app-specific palette and key mapping comments.

## Quick Resume
```bash
cargo check
cargo run -- tui
```

## Suggested Next Tasks
1. Add mouse click support inside modals (field focus + picker selection).
2. Add dedicated section modifier selector modal (instead of only cycle keys).
3. Add “Save As existing file overwrite confirmation” modal.
4. Add tests for:
   - section id normalization on load
   - fuzzy picker ranking
   - tab/shift-tab field traversal behavior
5. Add README usage section with updated keybindings and modal flows.

## Notes
- Active branch includes substantial `src/tui.rs` changes.
- Before merging, run a manual TUI pass on narrow and wide terminal sizes.
