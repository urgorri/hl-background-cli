# hl-bg

Ultra-lightweight CLI and Rust library to convert any image into the exact 117 TGA background tiles required by **Half-Life** (including the **25th Anniversary Update** widescreen/ultrawide layout).

## Why?

Half-Life's engine doesn't load a single background image. Instead, it splits backgrounds across two resolution grids using raw uncompressed TGA (bottom-to-top scanlines):
- **HD Ultrawide (3840×1600)**: 105 tiles ($7 \times 15$ grid)
- **Classic 4:3 (800×600)**: 12 tiles ($3 \times 4$ grid)

`hl-bg` handles resizing, cropping, slicing, and TGA binary formatting automatically in a fraction of a second.

## CLI Usage

```bash
# Convert an image directly into your Half-Life directory
hl-bg wallpaper.png -o "C:/Program Files (x86)/Steam/steamapps/common/Half-Life/valve/resource"

# Available fitting modes: cover (default), contain, stretch
hl-bg photo.jpg --fit contain -o ./output

# Skip writing the 4 layout .txt files
hl-bg photo.jpg --skip-layouts
```

## Library Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
hl-bg = { git = "https://github.com/username/hl-bg" }
```

```rust
use hl_bg::{generate, Config, FitMode};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config {
        input: PathBuf::from("my-image.png"),
        output_dir: PathBuf::from("./valve/resource"),
        fit: FitMode::Cover,
        write_layouts: true,
    };

    generate(&config)?;
    Ok(())
}
```

## License

MIT
