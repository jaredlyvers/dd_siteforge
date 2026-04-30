#!/usr/bin/env bash
# install.sh — build + install dd_siteforge, or remove an existing install.
#
# Usage:
#   ./install.sh                # build (release) and install
#   ./install.sh install        # same as above
#   ./install.sh uninstall      # remove the binary + theme + config dir if empty
#   ./install.sh --help         # this help
#
# Override defaults via env vars:
#   PREFIX=$HOME/.local                       # binary lives at $PREFIX/bin/dd_siteforge
#   CONFIG_DIR=$HOME/.config/ldnddev          # theme lives here
#
# Re-run safe: existing themes are left alone on install; the binary is overwritten.

set -euo pipefail

# ---- config -----------------------------------------------------------------
PREFIX="${PREFIX:-$HOME/.local}"
BIN_DIR="${BIN_DIR:-$PREFIX/bin}"
CONFIG_DIR="${CONFIG_DIR:-${XDG_CONFIG_HOME:-$HOME/.config}/ldnddev}"
BIN_NAME="dd_siteforge"
THEME_FILE="dd_siteforge_theme.yml"

# ---- pretty -----------------------------------------------------------------
if [ -t 1 ]; then
    cyan()   { printf '\033[36m%s\033[0m\n' "$*"; }
    green()  { printf '\033[32m%s\033[0m\n' "$*"; }
    yellow() { printf '\033[33m%s\033[0m\n' "$*"; }
    red()    { printf '\033[31m%s\033[0m\n' "$*" >&2; }
else
    cyan()   { printf '%s\n' "$*"; }
    green()  { printf '%s\n' "$*"; }
    yellow() { printf '%s\n' "$*"; }
    red()    { printf '%s\n' "$*" >&2; }
fi

require() {
    command -v "$1" >/dev/null 2>&1 || { red "Required command not found: $1"; exit 1; }
}

usage() {
    sed -n '2,11p' "$0" | sed 's/^# \{0,1\}//'
}

# ---- subcommands ------------------------------------------------------------
do_install() {
    require cargo
    require install

    repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    cd "$repo_root"

    [ -f Cargo.toml ] || { red "No Cargo.toml in $repo_root — run install.sh from the repo root."; exit 1; }
    [ -f "$THEME_FILE" ] || { red "Missing $THEME_FILE in $repo_root."; exit 1; }

    cyan "Building $BIN_NAME (release)…"
    cargo build --release

    src_bin="target/release/$BIN_NAME"
    [ -x "$src_bin" ] || { red "Build did not produce $src_bin"; exit 1; }

    mkdir -p "$BIN_DIR"
    install -m 0755 "$src_bin" "$BIN_DIR/$BIN_NAME"
    green "Installed $BIN_NAME → $BIN_DIR/$BIN_NAME"

    mkdir -p "$CONFIG_DIR"
    theme_dst="$CONFIG_DIR/$THEME_FILE"
    if [ -f "$theme_dst" ]; then
        yellow "Theme already exists at $theme_dst — leaving it alone."
    else
        install -m 0644 "$THEME_FILE" "$theme_dst"
        green "Installed default theme → $theme_dst"
    fi

    case ":$PATH:" in
        *":$BIN_DIR:"*) ;;
        *)
            yellow ""
            yellow "Note: $BIN_DIR is not on \$PATH. Add it to your shell rc:"
            yellow "    export PATH=\"$BIN_DIR:\$PATH\""
            ;;
    esac

    green ""
    green "Done. Try:  $BIN_NAME --help"
}

do_uninstall() {
    bin_dst="$BIN_DIR/$BIN_NAME"
    theme_dst="$CONFIG_DIR/$THEME_FILE"
    removed_any=0

    if [ -f "$bin_dst" ] || [ -L "$bin_dst" ]; then
        rm -f "$bin_dst"
        green "Removed $bin_dst"
        removed_any=1
    else
        yellow "No binary at $bin_dst"
    fi

    if [ -f "$theme_dst" ] || [ -L "$theme_dst" ]; then
        rm -f "$theme_dst"
        green "Removed $theme_dst"
        removed_any=1
    else
        yellow "No theme at $theme_dst"
    fi

    # Remove config dir only if empty (don't clobber other tools' files)
    if [ -d "$CONFIG_DIR" ] && [ -z "$(ls -A "$CONFIG_DIR")" ]; then
        rmdir "$CONFIG_DIR"
        green "Removed empty $CONFIG_DIR"
    fi

    if [ "$removed_any" -eq 0 ]; then
        yellow "Nothing to uninstall."
    else
        green ""
        green "Uninstall complete."
    fi
}

# ---- dispatch ---------------------------------------------------------------
cmd="${1:-install}"
case "$cmd" in
    install)              do_install ;;
    uninstall|remove)     do_uninstall ;;
    -h|--help|help)       usage ;;
    *) red "Unknown command: $cmd"; usage; exit 1 ;;
esac
