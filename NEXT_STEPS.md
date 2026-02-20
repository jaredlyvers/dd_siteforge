# Next Steps

## Current State
- Added ASCII section map in TUI details pane, including component names per column.
- Active column is marked with `*` in the ASCII map.
- Added TUI theming support with Catppuccin-style palette values.
- Theme lookup order:
  1. `./theme.yml`
  2. `./.theme.yml`
  3. `~/.config/ldnddev/dd_staticbuilder/.theme.yml`
- Added local `theme.yml` with Catppuccin Mocha defaults.
- Adjusted TUI layout so left `Nodes` pane is 30% width and right `Details` pane is 70%.
- Replaced manual theme parsing with `serde_yaml`.

## Quick Resume Commands
```bash
cargo test
cargo run -- tui
```

## Suggested Follow-ups
1. Add explicit documentation for theme keys/format in `README` or `Architecture.md`.
2. Add unit tests for theme loading precedence and invalid theme handling.
3. Consider making panel split ratio configurable in theme file (e.g. `layout.nodes_width_percent`).
4. Improve ASCII map truncation for long component lists (optional scrolling or paging).
5. Surface active-column context in status bar when adding/removing components.

## Example Theme File
```yaml
colors:
  base: "#1e1e2e"
  mantle: "#181825"
  crust: "#11111b"
  text: "#cdd6f4"
  subtext0: "#a6adc8"
  surface0: "#313244"
  overlay0: "#6c7086"
  lavender: "#b4befe"
  blue: "#89b4fa"
```
