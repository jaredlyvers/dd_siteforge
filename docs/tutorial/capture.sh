#!/usr/bin/env bash
# Recapture tutorial screenshots from a live TUI session on Hyprland.
#
# Requires: foot, grim, wtype, hyprctl, python, a release binary.
# See SCREENSHOTS.md for the filename contract and a manual fallback.
#
# Usage (from repo root):
#   ./docs/tutorial/capture.sh
#
# Writes PNGs into docs/tutorial/images/. Does not commit.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
IMG="$ROOT/docs/tutorial/images"
BIN="$ROOT/target/release/dd_siteforge"
WORK="${DD_SITEFORGE_TUTORIAL_SITE:-/tmp/dd_siteforge_tutorial}"
SITE="$WORK/site.json"
APP_ID="dd-siteforge-tutorial"
SLEEP_AFTER_KEY="${SLEEP_AFTER_KEY:-0.5}"
FOOT_PID=""

mkdir -p "$IMG"

die() { echo "capture.sh: $*" >&2; exit 1; }

for cmd in foot grim wtype hyprctl python cargo; do
    command -v "$cmd" >/dev/null || die "missing $cmd"
done

if [ ! -x "$BIN" ]; then
    echo "Building release binary…"
    (cd "$ROOT" && cargo build --release)
fi

if [ ! -f "$SITE" ]; then
    mkdir -p "$WORK"
    "$BIN" init-site "$SITE" --name tutorial
fi

# Deterministic header quote so screenshots don't rotate between runs.
cat > "$WORK/dd_siteforge_theme.yml" <<'EOF'
version: 1
header_quotes:
  - "Drafts are just commits that lost their nerve."
colors:
  base_background: "#0F1114"
  body_background: "#2A2D31"
  modal_background: "#1C1E21"
  text_primary: "#F5F6F7"
  text_secondary: "#9EA3AA"
  text_labels: "#FFAF46"
  text_active_focus: "#64B4F5"
  modal_labels: "#64B4F5"
  modal_text: "#F5F6F7"
  modal_header: "#64B4F5"
  selected_background: "#0F1114"
  border_default: "#F5F6F7"
  border_active: "#64B4F5"
  scrollbar: "#FFA087"
  scrollbar_hover: "#64B4F5"
  input_border_default: "#F5F6F7"
  input_border_focus: "#64B4F5"
  input_text_default: "#F5F6F7"
  input_text_focus: "#64B4F5"
  cursor: "#64B4F5"
  success: "#82e0aa"
  warning: "#f5c469"
  error: "#e57373"
  info: "#5dade2"
  folders: "#64B4F5"
  files: "#FFAF46"
  links: "#FFA087"
EOF

window_pid() {
    hyprctl -j clients | python -c "
import json, sys
want = int(sys.argv[1])
for c in json.load(sys.stdin):
    if c.get('pid') == want or c.get('class') == sys.argv[2]:
        print(c.get('pid') or '')
        raise SystemExit(0)
raise SystemExit(1)
" "${1:-0}" "$APP_ID"
}

window_geom() {
    hyprctl -j clients | python -c "
import json, sys
pid = int(sys.argv[1])
cls = sys.argv[2]
for c in json.load(sys.stdin):
    if c.get('pid') == pid or c.get('class') == cls:
        x, y = c['at']
        w, h = c['size']
        print(f'{x},{y} {w}x{h}')
        raise SystemExit(0)
raise SystemExit(1)
" "$FOOT_PID" "$APP_ID"
}

focus_win() {
    hyprctl eval "(function()
      for _, w in ipairs(hl.get_windows()) do
        if w.pid == ${FOOT_PID} then
          hl.dsp.focus({ window = w })
          hl.dsp.window.bring_to_top({ window = w })
          return 1
        end
      end
      return 0
    end)()" >/dev/null
    sleep 0.15
}

shot() {
    local name="$1"
    sleep "$SLEEP_AFTER_KEY"
    focus_win
    grim -g "$(window_geom)" "$IMG/$name"
    echo "wrote $name ($(window_geom))"
}

cleanup() {
    if [ -n "${FOOT_PID}" ] && kill -0 "$FOOT_PID" 2>/dev/null; then
        kill "$FOOT_PID" 2>/dev/null || true
        wait "$FOOT_PID" 2>/dev/null || true
    fi
}
trap cleanup EXIT

# Close a leftover capture window from a previous run.
hyprctl -j clients | python -c "
import json, sys, os, signal
for c in json.load(sys.stdin):
    if c.get('class') == 'dd-siteforge-tutorial' and c.get('pid'):
        try:
            os.kill(int(c['pid']), signal.SIGTERM)
        except OSError:
            pass
" || true
sleep 0.2

foot -a "$APP_ID" -T "$APP_ID" -W 132x42 -D "$WORK" "$BIN" tui "$SITE" \
    >/tmp/dd-siteforge-tutorial-foot.log 2>&1 &
FOOT_PID=$!

ok=0
for _ in $(seq 1 40); do
    if window_geom >/dev/null 2>&1; then
        ok=1
        break
    fi
    sleep 0.15
done
[ "$ok" = 1 ] || die "timed out waiting for $APP_ID (see /tmp/dd-siteforge-tutorial-foot.log)"

focus_win
sleep 0.7
shot 01-shell.png

wtype -k F1
shot 02-help.png
wtype -k Escape
sleep "$SLEEP_AFTER_KEY"

wtype -k F2
shot 03-theme.png
wtype -k Escape
sleep "$SLEEP_AFTER_KEY"

wtype 3
sleep 0.15
wtype -k Return
shot 04-edit-form.png
wtype -k Escape
sleep "$SLEEP_AFTER_KEY"

wtype /
shot 05-insert.png
wtype -k Escape
sleep "$SLEEP_AFTER_KEY"

wtype 4
shot 06-details.png

# Key 1 focuses Regions but leaves selected_region on Page until j/k.
wtype 1
sleep 0.15
wtype j
shot 07-regions.png

wtype -k Return
shot 08-site-settings.png
wtype -k Escape
sleep "$SLEEP_AFTER_KEY"

wtype -M ctrl -k q -m ctrl
sleep 0.25
wtype y
sleep 0.35

echo "Done. PNGs in $IMG"
