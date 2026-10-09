# hl-bg

[![CI](https://github.com/username/hl-bg/actions/workflows/ci.yml/badge.svg)](https://github.com/username/hl-bg/actions)
[![Crates.io](https://img.shields.io/crates/v/hl-bg.svg)](https://crates.io/crates/hl-bg)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

An ultra-lightweight CLI and Rust library to convert any image into the exact TGA background slices required by the **Half-Life GoldSrc engine** (including the modern **25th Anniversary Update** widescreen and 21:9 ultrawide layouts).

---

## Features

- 🎮 **Exact Valve Compatibility**: Generates the exact 117 uncompressed Truevision TGA 2.0 files with bottom-to-top scanlines and standard footers expected by the GoldSrc engine.
- 📐 **Dual Layout Generation**:
  - **HD Ultrawide (3840×1600)**: 105 slices ($7 \times 15$ grid) for modern 21:9 and widescreen displays.
  - **Classic 4:3 Fallback (800×600)**: 12 slices ($3 \times 4$ grid) for legacy resolutions.
- 📄 **Layout Descriptors**: Optionally outputs `BackgroundLayout.txt`, `HD_BackgroundLayout.txt`, `BackgroundLoadingLayout.txt`, and `HD_BackgroundLoadingLayout.txt`.
- 🖼️ **Wide Format Support**: Reads PNG, JPEG, WebP, BMP, and TIFF via pure-Rust decoders.
- ✂️ **Smart Fitting**: Supports `cover` (aspect ratio crop to center), `contain` (letterbox with background color), and `stretch`.
- ⚡ **Multi-threaded & Blazing Fast**: Parallel tile encoding using `rayon`.
- 📦 **Dual Interface**: Use as a standalone CLI tool or import into your own Rust projects as a crate.

---

## Installation

### CLI

Using `cargo`:

```bash
cargo install hl-bg
```

Or download a pre-built static binary (< 3 MB) from the [Releases](https://github.com/username/hl-bg/releases) page.

### As a Library

Add to your `Cargo.toml`:

```toml
[dependencies]
hl-bg = "0.1.0"
```

---

## CLI Usage

### Basic Conversion

Generate all 117 slices and layout files into your Half-Life directory:

```bash
hl-bg my-wallpaper.png -o "C:/Program Files (x86)/Steam/steamapps/common/Half-Life/valve/resource"
```

### Options

```
Usage: hl-bg [OPTIONS] [INPUT]

Arguments:
  [INPUT]  Path to input image (PNG, JPEG, WebP, BMP, TIFF)

Options:
  -o, --output-dir <DIR>      Output directory [default: ./output]
      --fit <FIT>             Fitting strategy: cover, contain, stretch [default: cover]
      --bg-color <HEX>        Background padding color for contain mode (e.g. #000000) [default: #000000]
      --hd-image <PATH>       Separate source image for HD ultrawide layout (3840x1600)
      --classic-image <PATH>  Separate source image for Classic 4:3 layout (800x600)
      --skip-layouts          Do not generate *.txt layout files
      --dry-run               Simulate generation without writing files to disk
  -v, --verbose               Enable verbose logging
  -h, --help                  Print help
  -V, --version               Print version
```

### Examples

**Letterbox / Contain mode with custom background color:**
```bash
hl-bg banner.jpg --fit contain --bg-color "#1A1A1A" -o ./valve/resource
```

**Using dedicated images for widescreen and 4:3:**
```bash
hl-bg --hd widescreen.png --classic retro.png -o ./valve/resource
```

---

## Library Usage

```rust
use hl_bg::{BackgroundGenerator, FitMode, GeneratorConfig};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = GeneratorConfig {
        input: Some(PathBuf::from("wallpaper.png")),
        output_dir: PathBuf::from("./valve/resource"),
        fit: FitMode::Cover,
        include_layouts: true,
        ..Default::default()
    };

    let result = BackgroundGenerator::new(config).generate()?;
    println!("Generated {} TGA tiles successfully!", result.slices_written);
    Ok(())
}
```

---

## How It Works

Half-Life's 25th Anniversary Update tiles the background across two resolution grids using uncompressed TGA (Type 2, 24-bit RGB, bottom-to-top scanlines):

1. **HD Layout (21:9)**:
   - Resolution: `3840 x 1600`
   - Slices: 15 columns (`a` to `o`) $\times$ 7 rows (`1` to `7`)
   - Naming: `21_9_<row>_<col>_loading.tga`
2. **Classic Layout (4:3)**:
   - Resolution: `800 x 600`
   - Slices: 4 columns (`a` to `d`) $\times$ 3 rows (`1` to `3`)
   - Naming: `800_<row>_<col>_loading.tga`

`hl-bg` slices the input image to exact pixel boundaries, formats raw pixels in BGR order, and writes the required Truevision TGA 2.0 binary headers and footers.

---

## License

Dual-licensed under either:
- MIT License ([LICENSE-MIT](LICENSE-MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
