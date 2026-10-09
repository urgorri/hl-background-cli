use std::fs::{self, File};
use std::io::BufWriter;
use std::path::Path;

use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView, Rgb, RgbImage};

#[cfg(feature = "parallel")]
use rayon::prelude::*;

use crate::core::constants::{
    get_classic_tiles, get_hd_tiles, TileDefinition, CLASSIC_CANVAS_HEIGHT, CLASSIC_CANVAS_WIDTH,
    HD_CANVAS_HEIGHT, HD_CANVAS_WIDTH,
};
use crate::core::tga::encode_tga_24bit;
use crate::error::{HlBgError, Result};

/// Image fitting strategy when resizing input image to target canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FitMode {
    /// Scale to fill the entire canvas while preserving aspect ratio, cropping center.
    #[default]
    Cover,
    /// Scale to fit within the canvas while preserving aspect ratio, letterboxing with background color.
    Contain,
    /// Stretch non-uniformly to exact canvas dimensions.
    Stretch,
}

impl std::str::FromStr for FitMode {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "cover" => Ok(FitMode::Cover),
            "contain" => Ok(FitMode::Contain),
            "stretch" => Ok(FitMode::Stretch),
            other => Err(format!(
                "Invalid fit mode '{}'. Options: cover, contain, stretch",
                other
            )),
        }
    }
}

/// Parses a hex color string (e.g., "#000000" or "#FFFFFF") into an RGB pixel.
pub fn parse_hex_color(hex: &str) -> Result<Rgb<u8>> {
    let clean = hex.trim_start_matches('#');
    if clean.len() == 6 {
        let r = u8::from_str_radix(&clean[0..2], 16)
            .map_err(|_| HlBgError::InvalidColor(hex.to_string()))?;
        let g = u8::from_str_radix(&clean[2..4], 16)
            .map_err(|_| HlBgError::InvalidColor(hex.to_string()))?;
        let b = u8::from_str_radix(&clean[4..6], 16)
            .map_err(|_| HlBgError::InvalidColor(hex.to_string()))?;
        Ok(Rgb([r, g, b]))
    } else if clean.len() == 3 {
        let r = u8::from_str_radix(&clean[0..1].repeat(2), 16)
            .map_err(|_| HlBgError::InvalidColor(hex.to_string()))?;
        let g = u8::from_str_radix(&clean[1..2].repeat(2), 16)
            .map_err(|_| HlBgError::InvalidColor(hex.to_string()))?;
        let b = u8::from_str_radix(&clean[2..3].repeat(2), 16)
            .map_err(|_| HlBgError::InvalidColor(hex.to_string()))?;
        Ok(Rgb([r, g, b]))
    } else {
        Err(HlBgError::InvalidColor(hex.to_string()))
    }
}

/// Prepares and fits a source image onto a canvas of exact target dimensions.
pub fn fit_to_canvas(
    img: &DynamicImage,
    target_width: u32,
    target_height: u32,
    mode: FitMode,
    bg_color: Rgb<u8>,
) -> RgbImage {
    let (src_w, src_h) = img.dimensions();

    match mode {
        FitMode::Stretch => {
            img.resize_exact(target_width, target_height, FilterType::Lanczos3)
                .to_rgb8()
        }
        FitMode::Cover => {
            let src_aspect = src_w as f64 / src_h as f64;
            let target_aspect = target_width as f64 / target_height as f64;

            let (crop_w, crop_h) = if src_aspect > target_aspect {
                // Source is wider: crop horizontal sides
                let new_w = (src_h as f64 * target_aspect).round() as u32;
                (new_w, src_h)
            } else {
                // Source is taller: crop vertical top/bottom
                let new_h = (src_w as f64 / target_aspect).round() as u32;
                (src_w, new_h)
            };

            let crop_x = (src_w.saturating_sub(crop_w)) / 2;
            let crop_y = (src_h.saturating_sub(crop_h)) / 2;

            let cropped = img.crop_imm(crop_x, crop_y, crop_w, crop_h);
            cropped
                .resize_exact(target_width, target_height, FilterType::Lanczos3)
                .to_rgb8()
        }
        FitMode::Contain => {
            let mut canvas = RgbImage::from_pixel(target_width, target_height, bg_color);
            let scaled = img.resize(target_width, target_height, FilterType::Lanczos3);
            let (scaled_w, scaled_h) = scaled.dimensions();

            let offset_x = (target_width.saturating_sub(scaled_w)) / 2;
            let offset_y = (target_height.saturating_sub(scaled_h)) / 2;

            image::imageops::overlay(&mut canvas, &scaled.to_rgb8(), offset_x as i64, offset_y as i64);
            canvas
        }
    }
}

/// Slices a canvas image into the specified tiles and writes them to the output directory.
pub fn slice_and_write_tiles<F>(
    canvas: &RgbImage,
    tiles: &[TileDefinition],
    output_dir: &Path,
    dry_run: bool,
    progress_callback: &F,
) -> Result<usize>
where
    F: Fn(&str, usize, usize) + Sync + Send,
{
    if !dry_run {
        fs::create_dir_all(output_dir).map_err(|e| HlBgError::Io {
            path: output_dir.to_path_buf(),
            source: e,
        })?;
    }

    let total = tiles.len();

    #[cfg(feature = "parallel")]
    let process_tile = |(idx, tile): (usize, &TileDefinition)| -> Result<()> {
        let sub = image::imageops::crop_imm(canvas, tile.x, tile.y, tile.width, tile.height).to_image();
        let file_path = output_dir.join(tile.filename);

        if !dry_run {
            let file = File::create(&file_path).map_err(|e| HlBgError::Io {
                path: file_path.clone(),
                source: e,
            })?;
            let mut writer = BufWriter::new(file);
            encode_tga_24bit(&mut writer, &sub).map_err(|e| HlBgError::Io {
                path: file_path.clone(),
                source: e,
            })?;
        }

        progress_callback(tile.filename, idx + 1, total);
        Ok(())
    };

    #[cfg(feature = "parallel")]
    {
        tiles
            .par_iter()
            .enumerate()
            .map(process_tile)
            .collect::<Result<Vec<()>>>()?;
    }

    #[cfg(not(feature = "parallel"))]
    {
        for (idx, tile) in tiles.iter().enumerate() {
            let sub = image::imageops::crop_imm(canvas, tile.x, tile.y, tile.width, tile.height).to_image();
            let file_path = output_dir.join(tile.filename);

            if !dry_run {
                let file = File::create(&file_path).map_err(|e| HlBgError::Io {
                    path: file_path.clone(),
                    source: e,
                })?;
                let mut writer = BufWriter::new(file);
                encode_tga_24bit(&mut writer, &sub).map_err(|e| HlBgError::Io {
                    path: file_path.clone(),
                    source: e,
                })?;
            }

            progress_callback(tile.filename, idx + 1, total);
        }
    }

    Ok(tiles.len())
}

/// Slices the HD Ultrawide canvas (3840x1600) into 105 tiles.
pub fn slice_hd_canvas<F>(
    canvas: &RgbImage,
    output_dir: &Path,
    dry_run: bool,
    callback: &F,
) -> Result<usize>
where
    F: Fn(&str, usize, usize) + Sync + Send,
{
    let tiles = get_hd_tiles();
    slice_and_write_tiles(canvas, &tiles, output_dir, dry_run, callback)
}

/// Slices the Classic 4:3 canvas (800x600) into 12 tiles.
pub fn slice_classic_canvas<F>(
    canvas: &RgbImage,
    output_dir: &Path,
    dry_run: bool,
    callback: &F,
) -> Result<usize>
where
    F: Fn(&str, usize, usize) + Sync + Send,
{
    let tiles = get_classic_tiles();
    slice_and_write_tiles(canvas, &tiles, output_dir, dry_run, callback)
}
