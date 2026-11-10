# Technical Blueprint & Architecture: Half-Life Background Generator (`hl-background-cli`)

## 1. Executive Summary

`hl-background-cli` is an ultra-lightweight, high-performance CLI tool and library written in **Rust**. It converts standard image formats (PNG, JPEG, WebP, BMP) into the exact 117 uncompressed Truevision TGA 2.0 slices and layout descriptors required by the **Half-Life GoldSrc engine** (specifically supporting the modern **25th Anniversary Update** widescreen and ultrawide layouts alongside the classic 4:3 fallback).

The project is designed to be:
- **Zero-Boilerplate**: Minimalist, readable codebase free of unnecessary bureaucracy.
- **Ultra-Lightweight**: Target compiled binary size < 1 MB using release size optimizations and `argh`.
- **Zero-Copy / Zero-Allocation Streaming**: Reads tile regions directly from the in-memory canvas into buffered disk writers without creating 117 intermediate image buffers.
- **Dual Interface**: A clean CLI tool and a reusable Rust crate (`hl_background_cli`) that any developer can easily import.

---

## 2. Reverse-Engineered Half-Life Specifications

Analysis of `valve/resource/background` and engine layout files from the official Half-Life 25th Anniversary installation reveals two distinct layout tiers:

```
resource/
├── background/
│   ├── 800_1_a_loading.tga  ...  800_3_d_loading.tga   (12 tiles, 4:3 legacy fallback)
│   └── 21_9_1_a_loading.tga ...  21_9_7_o_loading.tga  (105 tiles, 21:9 ultrawide HD)
├── BackgroundLayout.txt
├── BackgroundLoadingLayout.txt
├── HD_BackgroundLayout.txt
└── HD_BackgroundLoadingLayout.txt
```

Total files per complete background set: **117 TGA files** + 4 layout definition text files.

### 2.1. Layout Grid Breakdowns

#### A. HD Ultrawide Set (`21_9_*_loading.tga`)
* **Target Resolution:** `3840 x 1600` (24:10 / 21:9 aspect ratio).
* **Grid:** 15 columns (`a` through `o`) $\times$ 7 rows (`1` through `7`) = **105 slices**.
* **Column Widths:** Columns `a` to `o` are all `256 px` ($15 \times 256 = 3840$).
* **Row Heights:**
  * Rows `1` to `6`: `256 px` ($6 \times 256 = 1536$).
  * Row `7`: `64 px` ($1536 + 64 = 1600$).
* **Tile Dimensions:**
  * Rows 1–6: `256 x 256 px`
  * Row 7: `256 x 64 px`

#### B. Classic 4:3 Fallback Set (`800_*_loading.tga`)
* **Target Resolution:** `800 x 600` (4:3 aspect ratio).
* **Grid:** 4 columns (`a`, `b`, `c`, `d`) $\times$ 3 rows (`1`, `2`, `3`) = **12 slices**.
* **Column Widths:**
  * Columns `a`, `b`, `c`: `256 px`.
  * Column `d`: `32 px` ($3 \times 256 + 32 = 800$).
* **Row Heights:**
  * Rows `1`, `2`: `256 px`.
  * Row `3`: `88 px` ($2 \times 256 + 88 = 600$).
* **Tile Dimensions:**
  * Rows 1–2, Cols a–c: `256 x 256 px`
  * Rows 1–2, Col d: `32 x 256 px`
  * Row 3, Cols a–c: `256 x 88 px`
  * Row 3, Col d: `32 x 88 px`

### 2.2. Binary TGA Specification

Every generated tile must strictly adhere to the Truevision TGA 2.0 specification:

| Field | Size | Expected Value | Notes |
| :--- | :--- | :--- | :--- |
| **ID Length** | 1 byte | `0x00` | No image ID field |
| **Color Map Type** | 1 byte | `0x00` | No palette |
| **Image Type** | 1 byte | `0x02` | Uncompressed true-color image |
| **Color Map Spec** | 5 bytes | `0x00` $\times$ 5 | Empty |
| **X-Origin / Y-Origin** | 4 bytes | `0x00, 0x00, 0x00, 0x00` | (0, 0) |
| **Width / Height** | 4 bytes | Little-endian `u16` | Exact tile dimensions |
| **Pixel Depth (BPP)** | 1 byte | `0x18` (24-bit) | 24-bit RGB |
| **Image Descriptor** | 1 byte | `0x00` | **Bit 5 is 0: Bottom-to-top scanlines** |
| **Pixel Data** | Variable | BGR byte order | Row $H-1$ down to Row 0 |
| **TGA 2.0 Footer** | 26 bytes | `0x00...0x00TRUEVISION-XFILE.\0` | Extension/developer offsets = 0 |

> [!IMPORTANT]
> The GoldSrc VGUI TGA loader expects scanlines oriented **bottom-to-top** (origin at bottom-left). Top-to-bottom TGAs will appear upside down or inverted in-game. Run-Length Encoded (RLE) TGAs are also prone to engine rendering bugs and are avoided.

---

## 3. Technology Stack & Dependency Justifications

To achieve maximum performance and avoid unnecessary code duplication, the project uses a lean, targeted set of dependencies:

### 3.1. What Existing Libraries Handle
* **Image Decoding & Resampling:** `image` crate (v0.25) with `default-features = false, features = ["png", "jpeg", "webp", "bmp"]`.
  - Avoids writing custom decompressors for PNG (deflate), JPEG (DCT), and WebP.
  - Disabling default features avoids compiling bulky unused formats (HDR, EXR, AVIF, TIFF, QOI, PNM), saving compile time and binary size.
  - Leverages high-quality `Lanczos3` filtering for image scaling.
* **CLI Argument Parsing:** `argh` (v0.1), optional behind the default `cli` feature so library users do not compile it.
  - Replaces bulky CLI frameworks (`clap`) with Google's code-size-optimized derive macro parser.
  - Compiles in ~1 second and adds less than 40 KB to the binary.

### 3.2. Why We Use a Dedicated TGA Encoder
We evaluated `image::codecs::tga::TgaEncoder` from the `image` crate, but deliberately chose our own 35-line encoder because:
1. `image`'s `TgaEncoder` defaults to RLE compression (`ImageType::RunTrueColor = 10`), violating Half-Life's requirement for Type 2 uncompressed images.
2. It writes top-to-bottom scanlines by default, inverting the image in GoldSrc.
3. It does not append the 26-byte Truevision 2.0 footer (`TRUEVISION-XFILE.\0`).
Our custom encoder in [`src/lib.rs`](../src/lib.rs) guarantees byte-for-byte engine compliance with zero external crate overhead.

### 3.3. Zero-Allocation Streaming Architecture
Instead of cropping 117 sub-images with `.to_image()` (which allocates and clones 117 individual heap buffers), `hl-background-cli` implements `encode_tile_region`:
- Directly references the parent canvas already in RAM.
- Streams scanlines line by line from the canvas into the buffered disk writer.
- Uses a fixed stack buffer for scanline conversion, so no heap allocation happens for pixel data.

---

## 4. Repository Layout

```
.
├── .github/workflows/
│   ├── ci.yml                   # fmt, clippy, tests on 3 OSes, MSRV build
│   └── release.yml              # tag-triggered binaries + optional crates.io publish
├── docs/PLAN.md                 # This document
├── src/
│   ├── lib.rs                   # Library: tiles, TGA encoder, fit modes, layouts, generator
│   └── main.rs                  # CLI entrypoint (feature `cli`)
├── tests/integration_test.rs    # Byte-level, fit-mode, layout and liblist tests
├── Cargo.toml / Cargo.lock
├── LICENSE
└── README.md
```

---

## 5. Behavior Notes

### 5.1. Fit modes

| Mode | Behavior |
| :--- | :--- |
| `cover` | Center-crop to the canvas aspect ratio, then resize (Lanczos3). |
| `fill` | If the source is at least as wide as the canvas: same as `cover`. Otherwise the whole image is scaled to full canvas height (no crop), centered, and the remaining side margins are filled with a downscaled, blurred, 25% darkened copy of the artwork. A 24 px feather blends the foreground edges into the wings. On the 3840×1600 canvas a 16:9 source occupies the central 2844×1600 region. |
| `contain` | Letterbox inside the canvas using a configurable solid color. |
| `stretch` | Non-uniform resize to the canvas. |

### 5.2. Layout files

`BackgroundLayout.txt`, `BackgroundLoadingLayout.txt`, `HD_BackgroundLayout.txt` and `HD_BackgroundLoadingLayout.txt` are written. By default (`LayoutMode::Hd`) all four reference the 3840×1600 grid so mods lacking `hd_background "1"` still load HD tiles. `LayoutMode::Classic` (`--classic-layout`) makes the two standard files reference the 800×600 grid instead.

### 5.3. `liblist.gam`

`generate` only edits `liblist.gam` when `Config::update_liblist` is set (the CLI sets it unless `--no-liblist` is passed). The search starts at the layout directory and walks up at most two parents. Lines are parsed ignoring `//` comments; an existing `hd_background` with a value other than `1` is replaced, otherwise the setting is appended using the file's line-ending style. I/O errors are returned instead of swallowed.

---

## 6. API & CLI Reference

See the README for the CLI option table and library examples. Key public items: `Config` (builder, `#[non_exhaustive]`), `generate`, `generate_from_image`, `GenerateReport`, `FitMode`, `LayoutMode`, `LiblistStatus`, `Error`, `fit_image`, `encode_tile_region`, `encode_tga`, `hd_tiles`, `classic_tiles`, `layout_files`, `enable_hd_background`, `parse_hex_color`.

---

## 7. Verification & Testing Strategy

Tests in [`tests/integration_test.rs`](../tests/integration_test.rs) and the rustdoc examples cover:

1. **TGA compliance:** 18-byte header (type 2, 24-bit, descriptor `0x00`), BGR bottom-to-top pixel order, 26-byte footer, tiles wider than the stack chunk.
2. **Validation:** zero-size, oversize (> u16) and out-of-bounds/overflowing tiles return `Error::InvalidTile`.
3. **Fit modes:** exact output dimensions for every mode and canvas, `contain` color, `fill` wing extension and no-crop behavior for ultrawide sources.
4. **End-to-end:** 105 + 12 tiles, four layout files in both layout modes, `background/` directory handling, `--skip-layouts`.
5. **`liblist.gam`:** parsing edge cases and opt-in behavior.