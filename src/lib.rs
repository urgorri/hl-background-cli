//! Convert any image into the TGA background tiles used by Half-Life (GoldSrc),
//! including the 25th Anniversary Update widescreen/ultrawide layout.
//!
//! A complete background set is made of 117 uncompressed TGA 2.0 tiles
//! (105 for the 3840x1600 HD grid and 12 for the 800x600 classic grid) plus
//! the layout descriptor `.txt` files.
//!
//! # Quick start
//!
//! ```no_run
//! use hl_background_cli::{generate, Config, FitMode};
//!
//! # fn main() -> Result<(), hl_background_cli::Error> {
//! let config = Config::new("artwork.png", "./valve/resource").with_fit(FitMode::Fill);
//! let report = generate(&config)?;
//! println!("{} tiles written", report.tiles_written);
//! # Ok(())
//! # }
//! ```
//!
//! # In-memory usage
//!
//! Tiles and layouts can also be produced without touching the filesystem:
//!
//! ```
//! use hl_background_cli::{encode_tile_region, fit_image, hd_tiles, FitMode, HD_HEIGHT, HD_WIDTH};
//! use hl_background_cli::image::{DynamicImage, Rgb, RgbImage};
//!
//! let src = DynamicImage::ImageRgb8(RgbImage::from_pixel(64, 64, Rgb([10, 20, 30])));
//! let canvas = fit_image(&src, HD_WIDTH, HD_HEIGHT, FitMode::Cover, [0, 0, 0]);
//! let tile = &hd_tiles()[0];
//! let mut bytes = Vec::new();
//! encode_tile_region(&mut bytes, &canvas, tile).unwrap();
//! assert_eq!(bytes.len(), 18 + 256 * 256 * 3 + 26);
//! ```

use std::fmt;
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView, Rgb, RgbImage};

/// Re-export of the `image` crate so downstream users never face version mismatches.
pub use image;

// -----------------------------------------------------------------------------
// Errors
// -----------------------------------------------------------------------------

/// Errors returned by this crate. Always `Send + Sync + 'static`.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// A filesystem or writer error.
    Io(io::Error),
    /// The input image could not be decoded.
    Image(image::ImageError),
    /// An unknown fit mode name was supplied.
    InvalidFitMode(String),
    /// An invalid hex color was supplied.
    InvalidColor(String),
    /// A tile has zero size, exceeds TGA limits or falls outside the canvas.
    InvalidTile(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "I/O error: {e}"),
            Error::Image(e) => write!(f, "image error: {e}"),
            Error::InvalidFitMode(s) => write!(
                f,
                "unknown fit mode '{s}'. Options: cover, fill (adaptive), contain, stretch"
            ),
            Error::InvalidColor(s) => {
                write!(f, "invalid color '{s}'. Expected '#RRGGBB' or '#RGB'")
            }
            Error::InvalidTile(s) => write!(f, "invalid tile: {s}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            Error::Image(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<image::ImageError> for Error {
    fn from(e: image::ImageError) -> Self {
        Error::Image(e)
    }
}

/// Convenience result alias.
pub type Result<T> = std::result::Result<T, Error>;

// -----------------------------------------------------------------------------
// Constants & Tile Layouts
// -----------------------------------------------------------------------------

/// Width of the HD (21:9) canvas.
pub const HD_WIDTH: u32 = 3840;
/// Height of the HD (21:9) canvas.
pub const HD_HEIGHT: u32 = 1600;
/// Width of the classic (4:3) canvas.
pub const CLASSIC_WIDTH: u32 = 800;
/// Height of the classic (4:3) canvas.
pub const CLASSIC_HEIGHT: u32 = 600;

/// A rectangular region of a canvas that becomes one TGA file.
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

/// Pixels converted per write call; keeps the scanline buffer on the stack.
const CHUNK_PIXELS: usize = 256;

/// Encodes a rectangular region directly from a canvas into an uncompressed TGA 2.0 file.
///
/// Scanlines are streamed bottom-to-top using a fixed stack buffer, so no heap
/// allocation happens for pixel data.
pub fn encode_tile_region<W: Write>(writer: &mut W, canvas: &RgbImage, tile: &Tile) -> Result<()> {
    if tile.width == 0 || tile.height == 0 {
        return Err(Error::InvalidTile(format!(
            "'{}' has zero size ({}x{})",
            tile.filename, tile.width, tile.height
        )));
    }
    if tile.width > u16::MAX as u32 || tile.height > u16::MAX as u32 {
        return Err(Error::InvalidTile(format!(
            "'{}' exceeds the TGA size limit of {} ({}x{})",
            tile.filename,
            u16::MAX,
            tile.width,
            tile.height
        )));
    }
    let in_bounds = matches!(
        (
            tile.x.checked_add(tile.width),
            tile.y.checked_add(tile.height)
        ),
        (Some(right), Some(bottom)) if right <= canvas.width() && bottom <= canvas.height()
    );
    if !in_bounds {
        return Err(Error::InvalidTile(format!(
            "region out of canvas bounds: tile [{}, {} + {}x{}] vs canvas {}x{}",
            tile.x,
            tile.y,
            tile.width,
            tile.height,
            canvas.width(),
            canvas.height()
        )));
    }

    let mut header = [0u8; 18];
    header[2] = 2; // Uncompressed True-Color
    header[12..14].copy_from_slice(&(tile.width as u16).to_le_bytes());
    header[14..16].copy_from_slice(&(tile.height as u16).to_le_bytes());
    header[16] = 24; // 24-bit
    header[17] = 0x00; // Bit 5 is 0 -> bottom-to-top scanlines
    writer.write_all(&header)?;

    let canvas_stride = (canvas.width() as usize) * 3;
    let raw = canvas.as_raw();
    let row_len = (tile.width as usize) * 3;
    let start_x = (tile.x as usize) * 3;
    let mut buf = [0u8; CHUNK_PIXELS * 3];

    for y in (0..tile.height).rev() {
        let row_start = (tile.y + y) as usize * canvas_stride + start_x;
        let row_src = &raw[row_start..row_start + row_len];

        for src_chunk in row_src.chunks(CHUNK_PIXELS * 3) {
            let out = &mut buf[..src_chunk.len()];
            let (src_px, _) = src_chunk.as_chunks::<3>();
            let (dst_px, _) = out.as_chunks_mut::<3>();
            for (src, dst) in src_px.iter().zip(dst_px.iter_mut()) {
                dst[0] = src[2]; // Blue
                dst[1] = src[1]; // Green
                dst[2] = src[0]; // Red
            }
            writer.write_all(out)?;
        }
    }

    writer.write_all(&TGA_FOOTER)?;
    Ok(())
}

/// Convenience function to encode an entire `RgbImage` to TGA 2.0.
pub fn encode_tga<W: Write>(writer: &mut W, img: &RgbImage) -> Result<()> {
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

/// Strategy used to fit the source image into a target canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FitMode {
    /// Scale to cover the canvas and crop the overflow (centered).
    #[default]
    Cover,
    /// Keep the whole image uncropped when it is narrower than the canvas and fill the
    /// side margins with a blurred, darkened extension of the artwork.
    Fill,
    /// Scale to fit inside the canvas, padding with a solid color.
    Contain,
    /// Stretch non-uniformly to the canvas size.
    Stretch,
}

impl std::str::FromStr for FitMode {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        match s.to_ascii_lowercase().as_str() {
            "cover" => Ok(FitMode::Cover),
            "fill" | "adaptive" => Ok(FitMode::Fill),
            "contain" => Ok(FitMode::Contain),
            "stretch" => Ok(FitMode::Stretch),
            _ => Err(Error::InvalidFitMode(s.to_string())),
        }
    }
}

/// Parses `#RRGGBB` or `#RGB` (the `#` is optional) into an RGB triple.
pub fn parse_hex_color(hex: &str) -> Result<[u8; 3]> {
    let err = || Error::InvalidColor(hex.to_string());
    let clean = hex.trim().trim_start_matches('#');
    if !clean.is_ascii() {
        return Err(err());
    }
    let parse = |s: &str| u8::from_str_radix(s, 16).map_err(|_| err());
    match clean.len() {
        6 => Ok([
            parse(&clean[0..2])?,
            parse(&clean[2..4])?,
            parse(&clean[4..6])?,
        ]),
        3 => {
            let d = |i: usize| parse(&clean[i..i + 1]).map(|v| v * 17);
            Ok([d(0)?, d(1)?, d(2)?])
        }
        _ => Err(err()),
    }
}

fn fit_image_cover(img: &DynamicImage, target_w: u32, target_h: u32) -> RgbImage {
    let (src_w, src_h) = img.dimensions();
    if src_w == 0 || src_h == 0 || target_w == 0 || target_h == 0 {
        return RgbImage::new(target_w, target_h);
    }

    let src_aspect = src_w as f64 / src_h as f64;
    let target_aspect = target_w as f64 / target_h as f64;

    let (crop_w, crop_h) = if src_aspect > target_aspect {
        let w = (src_h as f64 * target_aspect).round() as u32;
        (w.clamp(1, src_w), src_h)
    } else {
        let h = (src_w as f64 / target_aspect).round() as u32;
        (src_w, h.clamp(1, src_h))
    };

    let crop_x = (src_w.saturating_sub(crop_w)) / 2;
    let crop_y = (src_h.saturating_sub(crop_h)) / 2;

    img.crop_imm(crop_x, crop_y, crop_w, crop_h)
        .resize_exact(target_w, target_h, FilterType::Lanczos3)
        .to_rgb8()
}

/// Width in pixels of the soft transition between the sharp foreground and the ambient wings.
const FEATHER_PX: u32 = 24;

fn fit_image_fill(img: &DynamicImage, target_w: u32, target_h: u32) -> RgbImage {
    let (src_w, src_h) = img.dimensions();
    let src_aspect = src_w as f64 / src_h as f64;
    let target_aspect = target_w as f64 / target_h as f64;

    // Already as wide as the canvas: nothing to extend.
    if src_aspect >= target_aspect {
        return fit_image_cover(img, target_w, target_h);
    }

    let fg_w = ((target_h as f64 * src_aspect).round() as u32).clamp(1, target_w);
    let offset_x = (target_w - fg_w) / 2;
    let right_wing = target_w - offset_x - fg_w;

    // Ambient backdrop: downscale, blur, darken, upscale.
    let small_w = (target_w / 4).max(1);
    let small_h = (target_h / 4).max(1);
    let mut small_bg = fit_image_cover(img, small_w, small_h);
    small_bg = image::imageops::blur(&small_bg, 3.0);
    for p in small_bg.pixels_mut() {
        for c in p.0.iter_mut() {
            *c = (*c as f32 * 0.75) as u8;
        }
    }
    let mut canvas = DynamicImage::ImageRgb8(small_bg)
        .resize_exact(target_w, target_h, FilterType::Triangle)
        .to_rgb8();

    // Sharp foreground with feathered edges where it meets an ambient wing.
    let mut fg = img
        .resize_exact(fg_w, target_h, FilterType::Lanczos3)
        .to_rgb8();
    let feather = FEATHER_PX.min(fg_w / 2);
    for i in 0..feather {
        let alpha = (i + 1) as f32 / (feather + 1) as f32;
        for y in 0..target_h {
            if offset_x > 0 {
                blend_edge(&mut fg, &canvas, i, y, offset_x + i, alpha);
            }
            if right_wing > 0 {
                let local_x = fg_w - 1 - i;
                blend_edge(&mut fg, &canvas, local_x, y, offset_x + local_x, alpha);
            }
        }
    }

    image::imageops::overlay(&mut canvas, &fg, offset_x as i64, 0);
    canvas
}

fn blend_edge(fg: &mut RgbImage, bg: &RgbImage, fx: u32, y: u32, bx: u32, alpha: f32) {
    let f = fg.get_pixel(fx, y).0;
    let b = bg.get_pixel(bx, y).0;
    let mut out = [0u8; 3];
    for k in 0..3 {
        out[k] = (b[k] as f32 + (f[k] as f32 - b[k] as f32) * alpha).round() as u8;
    }
    fg.put_pixel(fx, y, Rgb(out));
}

/// Fits `img` into a canvas of exactly `target_w` x `target_h` pixels.
///
/// `bg_color` is only used by [`FitMode::Contain`].
pub fn fit_image(
    img: &DynamicImage,
    target_w: u32,
    target_h: u32,
    mode: FitMode,
    bg_color: [u8; 3],
) -> RgbImage {
    let (src_w, src_h) = img.dimensions();
    if src_w == 0 || src_h == 0 || target_w == 0 || target_h == 0 {
        return RgbImage::new(target_w, target_h);
    }

    match mode {
        FitMode::Stretch => img
            .resize_exact(target_w, target_h, FilterType::Lanczos3)
            .to_rgb8(),
        FitMode::Cover => fit_image_cover(img, target_w, target_h),
        FitMode::Fill => fit_image_fill(img, target_w, target_h),
        FitMode::Contain => {
            let mut canvas = RgbImage::from_pixel(target_w, target_h, Rgb(bg_color));
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
// Layout descriptors
// -----------------------------------------------------------------------------

/// Which resolution the standard `BackgroundLayout.txt` / `BackgroundLoadingLayout.txt`
/// files point to. The `HD_*` files always point to the HD grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LayoutMode {
    /// Standard files use the 3840x1600 grid, so mods without `hd_background "1"` also
    /// load HD backgrounds.
    #[default]
    Hd,
    /// Standard files use the classic 800x600 grid (stock GoldSrc behavior).
    Classic,
}

/// Layout file text for the 3840x1600 grid.
pub fn hd_layout_text() -> String {
    let mut txt = format!("resolution\t{HD_WIDTH}\t{HD_HEIGHT}\n\n");
    for t in hd_tiles() {
        txt.push_str(&format!(
            "resource/background/{}\tfit\t{}\t{}\n",
            t.filename, t.x, t.y
        ));
    }
    txt
}

/// Layout file text for the 800x600 grid.
pub fn classic_layout_text() -> String {
    let mut txt = format!("resolution\t{CLASSIC_WIDTH}\t{CLASSIC_HEIGHT}\n\n");
    let tiles = classic_tiles();
    for (i, t) in tiles.iter().enumerate() {
        txt.push_str(&format!(
            "resource/background/{}\t\tfit\t\t{}\t{}\n",
            t.filename, t.x, t.y
        ));
        if (i + 1) % 4 == 0 && i + 1 < tiles.len() {
            txt.push('\n');
        }
    }
    txt
}

/// Returns `(file name, contents)` for the four layout descriptors of the given mode.
pub fn layout_files(mode: LayoutMode) -> [(&'static str, String); 4] {
    let hd = hd_layout_text();
    let standard = match mode {
        LayoutMode::Hd => hd.clone(),
        LayoutMode::Classic => classic_layout_text(),
    };
    [
        ("BackgroundLayout.txt", standard.clone()),
        ("BackgroundLoadingLayout.txt", standard),
        ("HD_BackgroundLayout.txt", hd.clone()),
        ("HD_BackgroundLoadingLayout.txt", hd),
    ]
}

// -----------------------------------------------------------------------------
// liblist.gam handling
// -----------------------------------------------------------------------------

/// Result of trying to enable `hd_background "1"` in `liblist.gam`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum LiblistStatus {
    /// Updating `liblist.gam` was not requested.
    Skipped,
    /// No `liblist.gam` was found near the output directory.
    NotFound,
    /// `hd_background "1"` was already present.
    AlreadyEnabled(PathBuf),
    /// The file was modified.
    Updated(PathBuf),
}

/// Returns the updated `liblist.gam` text, or `None` if `hd_background "1"` is already set.
pub fn enable_hd_background(content: &str) -> Option<String> {
    let eol = if content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut found = false;
    let mut changed = false;
    let mut lines: Vec<String> = Vec::new();

    for line in content.lines() {
        let code = line.split("//").next().unwrap_or("").trim();
        let mut tokens = code.split_whitespace();
        if tokens.next() == Some("hd_background") {
            found = true;
            let value = tokens.next().map(|v| v.trim_matches('"'));
            if value != Some("1") {
                lines.push("hd_background \"1\"".to_string());
                changed = true;
                continue;
            }
        }
        lines.push(line.to_string());
    }

    if !found {
        lines.push("hd_background \"1\"".to_string());
        changed = true;
    }
    changed.then(|| {
        let mut out = lines.join(eol);
        out.push_str(eol);
        out
    })
}

fn update_liblist(start_dir: &Path) -> Result<LiblistStatus> {
    let mut current = start_dir;
    for _ in 0..3 {
        let candidate = current.join("liblist.gam");
        if candidate.is_file() {
            let content = fs::read_to_string(&candidate)?;
            return match enable_hd_background(&content) {
                Some(updated) => {
                    fs::write(&candidate, updated)?;
                    Ok(LiblistStatus::Updated(candidate))
                }
                None => Ok(LiblistStatus::AlreadyEnabled(candidate)),
            };
        }
        match current.parent() {
            Some(p) if !p.as_os_str().is_empty() => current = p,
            _ => break,
        }
    }
    Ok(LiblistStatus::NotFound)
}

// -----------------------------------------------------------------------------
// Generator API
// -----------------------------------------------------------------------------

/// Generation settings. Use [`Config::new`] plus the `with_*` methods.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Config {
    /// Source image path (ignored by [`generate_from_image`]).
    pub input: PathBuf,
    /// Output directory. `background/` is appended unless it already ends with it.
    pub output_dir: PathBuf,
    pub fit: FitMode,
    /// Padding color for [`FitMode::Contain`].
    pub bg_color: [u8; 3],
    /// Write the four layout descriptor files.
    pub write_layouts: bool,
    pub layout_mode: LayoutMode,
    /// Enable `hd_background "1"` in a nearby `liblist.gam` (requires `write_layouts`).
    /// Off by default because it modifies files outside the output directory.
    pub update_liblist: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            input: PathBuf::new(),
            output_dir: PathBuf::from("./output"),
            fit: FitMode::Cover,
            bg_color: [0, 0, 0],
            write_layouts: true,
            layout_mode: LayoutMode::Hd,
            update_liblist: false,
        }
    }
}

impl Config {
    pub fn new(input: impl Into<PathBuf>, output_dir: impl Into<PathBuf>) -> Self {
        Self {
            input: input.into(),
            output_dir: output_dir.into(),
            ..Self::default()
        }
    }
    pub fn with_fit(mut self, fit: FitMode) -> Self {
        self.fit = fit;
        self
    }
    pub fn with_bg_color(mut self, color: [u8; 3]) -> Self {
        self.bg_color = color;
        self
    }
    pub fn with_layouts(mut self, write: bool) -> Self {
        self.write_layouts = write;
        self
    }
    pub fn with_layout_mode(mut self, mode: LayoutMode) -> Self {
        self.layout_mode = mode;
        self
    }
    pub fn with_update_liblist(mut self, update: bool) -> Self {
        self.update_liblist = update;
        self
    }
}

/// Summary of a [`generate`] run.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct GenerateReport {
    pub tiles_written: usize,
    pub layouts_written: bool,
    pub liblist: LiblistStatus,
}

/// Opens `config.input` and writes the full background set. See [`generate_from_image`].
pub fn generate(config: &Config) -> Result<GenerateReport> {
    let img = image::open(&config.input)?;
    generate_from_image(&img, config)
}

/// Slices an already decoded image into the 117 background TGAs and layout files.
pub fn generate_from_image(img: &DynamicImage, config: &Config) -> Result<GenerateReport> {
    let output_is_bg = config.output_dir.ends_with("background");
    let bg_dir = if output_is_bg {
        config.output_dir.clone()
    } else {
        config.output_dir.join("background")
    };
    fs::create_dir_all(&bg_dir)?;

    let mut tiles_written = 0;
    for (w, h, tiles) in [
        (HD_WIDTH, HD_HEIGHT, hd_tiles()),
        (CLASSIC_WIDTH, CLASSIC_HEIGHT, classic_tiles()),
    ] {
        let canvas = fit_image(img, w, h, config.fit, config.bg_color);
        for tile in &tiles {
            let mut writer = BufWriter::new(File::create(bg_dir.join(&tile.filename))?);
            encode_tile_region(&mut writer, &canvas, tile)?;
            writer.flush()?;
            tiles_written += 1;
        }
    }

    let mut liblist = LiblistStatus::Skipped;
    if config.write_layouts {
        let layout_dir = if output_is_bg {
            config
                .output_dir
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or_else(|| Path::new("."))
        } else {
            &config.output_dir
        };
        fs::create_dir_all(layout_dir)?;
        for (name, text) in layout_files(config.layout_mode) {
            fs::write(layout_dir.join(name), text)?;
        }
        if config.update_liblist {
            liblist = update_liblist(layout_dir)?;
        }
    }

    Ok(GenerateReport {
        tiles_written,
        layouts_written: config.write_layouts,
        liblist,
    })
}
