# Updating tutorial screenshots

The HTML tutorial at `docs/tutorial/index.html` loads PNGs from `docs/tutorial/images/`. Filenames are a contract — keep them, or update every `src=` in the HTML.

| File | What it must show | How the capture script gets there |
|---|---|---|
| `01-shell.png` | Main TUI: header, Regions (Site/Header/Footer), Pages, Layout, Details, footer starting `F1:Help  F2:Theme` | Idle after `tui` |
| `02-help.png` | F1 Help overlay | `F1` |
| `03-theme.png` | F2 Theme overlay, canonical token names | `Esc`, `F2` |
| `04-edit-form.png` | FormEdit on `dd-hero` (`Ctrl+S` in the modal footer) | `Esc`, `3`, `Enter` |
| `05-insert.png` | `/` insert picker (page kinds, no header-only) | `Esc`, `/` |
| `06-details.png` | Details focused (`4`); footer includes `j/k:Scroll` | `Esc`, `4` |
| `07-regions.png` | Regions focused, **Site** selected; Details shows site settings summary | `1`, `j` |
| `08-site-settings.png` | FormEdit titled `Site settings` | `Enter` |

## Automatic recapture (Hyprland)

From the repo root:

```bash
cargo build --release
./docs/tutorial/capture.sh
```

Needs `foot`, `grim`, `wtype`, `hyprctl`, and `python` on `$PATH`. The script:

1. Builds a throwaway site at `/tmp/dd_siteforge_tutorial/` (`init-site --name tutorial`) if missing.
2. Writes a local `dd_siteforge_theme.yml` there with a **fixed** header quote so screenshots do not rotate.
3. Opens a 132×42 `foot` window (`app-id` `dd-siteforge-tutorial`) running `target/release/dd_siteforge tui site.json`.
4. Focuses that window, sends the key sequence above, and `grim`s the window rect into `docs/tutorial/images/`.

Override the scratch site with `DD_SITEFORGE_TUTORIAL_SITE=/path ./docs/tutorial/capture.sh`. Slow a sluggish compositor with `SLEEP_AFTER_KEY=0.8`.

Inspect every PNG before committing. The compositor tiles the window; a tall tile is expected.

## Manual recapture

Use any 132×42 (or similar) terminal:

```bash
cd /tmp/dd_siteforge_tutorial   # or a fresh init-site
# copy the theme file the script writes, or accept a random header quote
dd_siteforge tui site.json
```

Follow the key column in the table. Screenshot the **terminal window only** (no desktop bar). Save under the same filenames.

## After recapture

```bash
git add docs/tutorial/images/*.png
git commit -m "docs: refresh tutorial screenshots"
```

Do not commit `/tmp/dd_siteforge_tutorial/` or `site.json`.
