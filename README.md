# dd_siteforge

Terminal-UI CMS for authoring framework-native static pages. Single Rust binary: edit a typed site tree, export HTML, host anywhere static.

**Tutorial (setup, TUI walkthrough, screenshots):** open [`docs/tutorial/index.html`](docs/tutorial/index.html).

## Install

```bash
./install.sh
```

Builds release, installs `$HOME/.local/bin/dd_siteforge`, and writes the default theme to `$HOME/.config/ldnddev/dd_siteforge_theme.yml` only when that file is missing. Override with `PREFIX`, `BIN_DIR`, or `CONFIG_DIR`.

```bash
cargo install --path .            # ~/.cargo/bin
cargo build --release             # ./target/release/dd_siteforge
./install.sh uninstall
```

## Quick start

```bash
dd_siteforge init-site site.json --name my-site
npm install && npx grunt build
dd_siteforge tui site.json
```

In the TUI: `F1` help, `Shift+E` export, `p` preview, `Ctrl+Q` quit. Put images in `./source/images/`.

## Tests

```bash
cargo test -q
```

## License

MIT License.
