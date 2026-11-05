use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView, Rgb, RgbImage};

// -----------------------------------------------------------------------------
// Constants & Tile Layouts
// -----------------------------------------------------------------------------

pub const HD_WIDTH: u32 = 3840;
pub const HD_HEIGHT: u32 = 1600;
pub const CLASSIC_WIDTH: u32 = 800;
pub const CLASSIC_HEIGHT: u32 = 600;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tile {
    pub filename: String,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Returns the 105 tile coordinates for the 3840x1600 (21:9) layout.
pub fn hd_tiles() -> Vec<Tile> {
    let mut tiles = Vec::with_capacity(105);
    let cols = [
        'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o',
    ];

    for row in 1..=7 {
        let y = (row - 1) * 256;
        let height = if row == 7 { 64 } else { 256 };

        for (col_idx, &col_char) in cols.iter().enumerate() {
            let x = (col_idx as u32) * 256;
            tiles.push(Tile {
                filename: format!("21_9_{}_{}_loading.tga", row, col_char),
                x,
                y,
                width: 256,
                height,
            });
        }
    }
    tiles
}

/// Returns the 12 tile coordinates for the 800x600 (4:3) layout.
pub fn classic_tiles() -> Vec<Tile> {
    let mut tiles = Vec::with_capacity(12);
    let cols = ['a', 'b', 'c', 'd'];

    for row in 1..=3 {
        let y = (row - 1) * 256;
        let height = if row == 3 { 88 } else { 256 };

        for (col_idx, &col_char) in cols.iter().enumerate() {
            let x = (col_idx as u32) * 256;
            let width = if col_char == 'd' { 32 } else { 256 };
            tiles.push(Tile {
                filename: format!("800_{}_{}_loading.tga", row, col_char),
                x,
                y,
                width,
                height,
            });
        }
    }
    tiles
}

// -----------------------------------------------------------------------------
// TGA 2.0 Encoder (Uncompressed True-Color, Bottom-to-Top BGR)
// -----------------------------------------------------------------------------

const TGA_FOOTER: [u8; 26] = [
    0, 0, 0, 0, // Extension offset
    0, 0, 0, 0, // Developer offset
    b'T', b'R', b'U', b'E', b'V', b'I', b'S', b'I', b'O', b'N', b'-', b'X', b'F', b'I', b'L', b'E',
    b'.', 0, // Null byte
];

/// Encodes a rectangular region directly from a canvas into an uncompressed TGA 2.0 file.
/// Zero heap allocation for pixel data buffers — streams directly from canvas to disk.
pub fn encode_tile_region<W: Write>(
    writer: &mut W,
    canvas: &RgbImage,
    tile: &Tile,
) -> io::Result<()> {
    if tile.x + tile.width > canvas.width() || tile.y + tile.height > canvas.height() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "Tile region out of canvas bounds: tile [{}, {} + {}x{}] vs canvas {}x{}",
                tile.x,
                tile.y,
                tile.width,
                tile.height,
                canvas.width(),
                canvas.height()
            ),
        ));
    }

    let width = tile.width as u16;
    let height = tile.height as u16;

    let mut header = [0u8; 18];
    header[2] = 2; // Uncompressed True-Color
    header[12..14].copy_from_slice(&width.to_le_bytes());
    header[14..16].copy_from_slice(&height.to_le_bytes());
    header[16] = 24; // 24-bit RGB
    header[17] = 0x00; // Bit 5 is 0 -> bottom-to-top scanlines

    writer.write_all(&header)?;

    // Stream scanlines bottom-to-top directly from the canvas buffer
    let canvas_stride = (canvas.width() as usize) * 3;
    let raw = canvas.as_raw();
    let row_len = (width as usize) * 3;
    let mut row_buf = vec![0u8; row_len];

    for y in (0..tile.height).rev() {
        let canvas_y = (tile.y + y) as usize;
        let start_x = (tile.x as usize) * 3;
        let row_start = canvas_y * canvas_stride + start_x;
        let row_src = &raw[row_start..row_start + row_len];

        let (src_chunks, _) = row_src.as_chunks::<3>();
        let (dst_chunks, _) = row_buf.as_chunks_mut::<3>();
        for (src, dst) in src_chunks.iter().zip(dst_chunks.iter_mut()) {
            dst[0] = src[2]; // Blue
            dst[1] = src[1]; // Green
            dst[2] = src[0]; // Red
        }
        writer.write_all(&row_buf)?;
    }

    writer.write_all(&TGA_FOOTER)?;
    Ok(())
}

/// Convenience function to encode an entire RgbImage to TGA 2.0.
pub fn encode_tga<W: Write>(writer: &mut W, img: &RgbImage) -> io::Result<()> {
    let tile = Tile {
        filename: String::new(),
        x: 0,
        y: 0,
        width: img.width(),
        height: img.height(),
    };
    encode_tile_region(writer, img, &tile)
}

// -----------------------------------------------------------------------------
// Image Sizing & Fitting
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FitMode {
    #[default]
    Cover,
    Contain,
    Stretch,
}

impl std::str::FromStr for FitMode {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "cover" => Ok(FitMode::Cover),
            "contain" => Ok(FitMode::Contain),
            "stretch" => Ok(FitMode::Stretch),
            _ => Err(format!(
                "Unknown fit mode '{}'. Options: cover, contain, stretch",
                s
            )),
        }
    }
}

pub fn fit_image(img: &DynamicImage, target_w: u32, target_h: u32, mode: FitMode) -> RgbImage {
    let (src_w, src_h) = img.dimensions();
    if src_w == 0 || src_h == 0 || target_w == 0 || target_h == 0 {
        return RgbImage::new(target_w, target_h);
    }

    match mode {
        FitMode::Stretch => img
            .resize_exact(target_w, target_h, FilterType::Lanczos3)
            .to_rgb8(),
        FitMode::Cover => {
            let src_aspect = src_w as f64 / src_h as f64;
            let target_aspect = target_w as f64 / target_h as f64;

            let (crop_w, crop_h) = if src_aspect > target_aspect {
                let w = (src_h as f64 * target_aspect).round() as u32;
                (w.min(src_w), src_h)
            } else {
                let h = (src_w as f64 / target_aspect).round() as u32;
                (src_w, h.min(src_h))
            };

            let crop_x = (src_w.saturating_sub(crop_w)) / 2;
            let crop_y = (src_h.saturating_sub(crop_h)) / 2;

            img.crop_imm(crop_x, crop_y, crop_w, crop_h)
                .resize_exact(target_w, target_h, FilterType::Lanczos3)
                .to_rgb8()
        }
        FitMode::Contain => {
            let mut canvas = RgbImage::from_pixel(target_w, target_h, Rgb([0, 0, 0]));
            let scaled = img.resize(target_w, target_h, FilterType::Lanczos3);
            let (scaled_w, scaled_h) = scaled.dimensions();
            let offset_x = (target_w.saturating_sub(scaled_w)) / 2;
            let offset_y = (target_h.saturating_sub(scaled_h)) / 2;
            image::imageops::overlay(
                &mut canvas,
                &scaled.to_rgb8(),
                offset_x as i64,
                offset_y as i64,
            );
            canvas
        }
    }
}

// -----------------------------------------------------------------------------
// Generator API
// -----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Config {
    pub input: PathBuf,
    pub output_dir: PathBuf,
    pub fit: FitMode,
    pub write_layouts: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            input: PathBuf::new(),
            output_dir: PathBuf::from("./output"),
            fit: FitMode::Cover,
            write_layouts: true,
        }
    }
}

/// Slices an input image into the 117 Half-Life background TGAs and layout files.
/// Streamlined: directly extracts tile scanlines without intermediate image allocations.
pub fn generate(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let img = image::open(&config.input)?;

    let bg_dir = if config.output_dir.ends_with("background") {
        config.output_dir.clone()
    } else {
        config.output_dir.join("background")
    };
    fs::create_dir_all(&bg_dir)?;

    // 1. Process HD 21:9 Canvas (3840x1600 -> 105 slices streamed directly)
    let hd_canvas = fit_image(&img, HD_WIDTH, HD_HEIGHT, config.fit);
    for tile in hd_tiles() {
        let file = File::create(bg_dir.join(&tile.filename))?;
        let mut writer = BufWriter::new(file);
        encode_tile_region(&mut writer, &hd_canvas, &tile)?;
    }

    // 2. Process Classic 4:3 Canvas (800x600 -> 12 slices streamed directly)
    let classic_canvas = fit_image(&img, CLASSIC_WIDTH, CLASSIC_HEIGHT, config.fit);
    for tile in classic_tiles() {
        let file = File::create(bg_dir.join(&tile.filename))?;
        let mut writer = BufWriter::new(file);
        encode_tile_region(&mut writer, &classic_canvas, &tile)?;
    }

    // 3. Write layout descriptor files
    if config.write_layouts {
        let layout_dir = if config.output_dir.ends_with("background") {
            config
                .output_dir
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or_else(|| Path::new("."))
        } else {
            &config.output_dir
        };

        write_layout_files(layout_dir)?;
    }

    Ok(())
}

fn write_layout_files(dir: &Path) -> io::Result<()> {
    if !dir.as_os_str().is_empty() && dir != Path::new(".") {
        fs::create_dir_all(dir)?;
    }

    let mut classic_txt = String::from("resolution\t800\t600\n\n");
    let c_tiles = classic_tiles();
    for (i, t) in c_tiles.iter().enumerate() {
        classic_txt.push_str(&format!(
            "resource/background/{}\t\tfit\t\t{}\t{}\n",
            t.filename, t.x, t.y
        ));
        if (i + 1) % 4 == 0 && i + 1 < c_tiles.len() {
            classic_txt.push('\n');
        }
    }

    let mut hd_txt = String::from("resolution\t3840\t1600\n\n");
    for t in hd_tiles() {
        hd_txt.push_str(&format!(
            "resource/background/{}\tfit\t{}\t{}\n",
            t.filename, t.x, t.y
        ));
    }

    fs::write(dir.join("BackgroundLayout.txt"), &classic_txt)?;
    fs::write(dir.join("BackgroundLoadingLayout.txt"), &classic_txt)?;
    fs::write(dir.join("HD_BackgroundLayout.txt"), &hd_txt)?;
    fs::write(dir.join("HD_BackgroundLoadingLayout.txt"), &hd_txt)?;
    Ok(())
}
