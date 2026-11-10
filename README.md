# hl-background-cli

[![CI](https://github.com/urgorri/hl-background-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/urgorri/hl-background-cli/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/hl-background-cli.svg)](https://crates.io/crates/hl-background-cli)
[![docs.rs](https://img.shields.io/docsrs/hl-background-cli)](https://docs.rs/hl-background-cli)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Ultra-lightweight CLI and Rust library that converts any image into the exact 117 TGA background tiles required by **Half-Life** (GoldSrc), including the **25th Anniversary Update** widescreen/ultrawide layout.

## Why?

Half-Life's engine does not load a single background image. It splits backgrounds across two resolution grids using raw uncompressed TGA (bottom-to-top scanlines):

- **HD Ultrawide (3840×1600)**: 105 tiles (7 × 15 grid)
- **Classic 4:3 (800×600)**: 12 tiles (3 × 4 grid)

`hl-background-cli` handles resizing, cropping, slicing, layout files and TGA binary formatting in a fraction of a second, streaming tiles to disk without per-tile image buffers.

## Installation

```bash
# From crates.io
cargo install hl-background-cli

# Prebuilt binary (needs cargo-binstall)
cargo binstall hl-background-cli
```

Prebuilt binaries for Windows, Linux and macOS are also attached to every [GitHub release](https://github.com/urgorri/hl-background-cli/releases).

## CLI Usage

```bash
# Convert an image directly into your Half-Life directory
hl-background-cli wallpaper.png -o "C:/Program Files (x86)/Steam/steamapps/common/Half-Life/valve/resource"

# Choose a fitting mode
hl-background-cli photo.jpg --fit fill -o ./output

# Pad with a custom color in contain mode
hl-background-cli logo.png --fit contain --bg-color "#101820"
```

| Option | Description |
| :--- | :--- |
| `-o, --output-dir <DIR>` | Destination directory (default `./output`). `background/` is appended unless the path already ends with it. |
| `--fit <MODE>` | `cover` (default), `fill`, `contain`, `stretch`. |
| `--bg-color <HEX>` | Padding color for `contain` (`#RRGGBB` or `#RGB`, default `#000000`). |
| `--skip-layouts` | Do not write the 4 layout `.txt` files. |
| `--classic-layout` | Make `BackgroundLayout.txt` / `BackgroundLoadingLayout.txt` point to the 800×600 grid instead of HD. |
| `--no-liblist` | Do not enable `hd_background "1"` in a nearby `liblist.gam` (see below). |
| `-q, --quiet` | Print only errors. |
| `-V, --version` | Print the version. |

### Fit modes

| Mode | Behavior |
| :--- | :--- |
| `cover` | Scale to cover the canvas and crop the overflow (centered). |
| `fill` | Keep the whole image when it is narrower than the canvas and extend the sides with a blurred, darkened ambient copy (feathered edges). Images already as wide as the canvas behave like `cover`. |
| `contain` | Scale to fit inside the canvas and pad with `--bg-color`. |
| `stretch` | Stretch non-uniformly to the canvas size. |

### Layout files and `liblist.gam`

By default `BackgroundLayout.txt` and `BackgroundLoadingLayout.txt` point to the HD grid, like the `HD_*` files, so mods load HD backgrounds even without `hd_background "1"`. Use `--classic-layout` for stock behavior.

When layouts are written, the CLI also enables `hd_background "1"` in the first `liblist.gam` found in the layout directory or up to two parent directories (an existing `hd_background "0"` is replaced, comments are ignored). Pass `--no-liblist` to leave it untouched. The library never touches `liblist.gam` unless you opt in.

## Library Usage

```toml
[dependencies]
hl-background-cli = { version = "0.1", default-features = false }
```

Disabling default features removes the CLI-only `argh` dependency.

```rust
use hl_background_cli::{generate, Config, FitMode};

fn main() -> Result<(), hl_background_cli::Error> {
    let config = Config::new("my-image.png", "./valve/resource").with_fit(FitMode::Fill);
    let report = generate(&config)?;
    println!("{} tiles written", report.tiles_written);
    Ok(())
}
```

Other entry points: `generate_from_image` (already decoded `DynamicImage`), `fit_image`, `encode_tile_region`, `hd_tiles` / `classic_tiles`, and in-memory layout builders (`layout_files`, `hd_layout_text`, `classic_layout_text`). The `image` crate is re-exported as `hl_background_cli::image`. Errors implement `std::error::Error + Send + Sync + 'static`.

Minimum supported Rust version: **1.88**.

## Development

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

See [docs/PLAN.md](docs/PLAN.md) for the engine specification and architecture notes.

## License

MIT
