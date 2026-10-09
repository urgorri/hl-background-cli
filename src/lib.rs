pub mod core;
pub mod error;

#[cfg(feature = "cli")]
pub mod cli;

use std::fs;
use std::path::{Path, PathBuf};

use image::DynamicImage;

pub use crate::core::constants::{
    get_classic_tiles, get_hd_tiles, TileDefinition, CLASSIC_CANVAS_HEIGHT, CLASSIC_CANVAS_WIDTH,
    HD_CANVAS_HEIGHT, HD_CANVAS_WIDTH, TOTAL_CLASSIC_TILES, TOTAL_HD_TILES, TOTAL_TILES,
};
pub use crate::core::layout::{generate_classic_layout_content, generate_hd_layout_content};
pub use crate::core::slicer::{
    fit_to_canvas, parse_hex_color, slice_classic_canvas, slice_hd_canvas, FitMode,
};
pub use crate::core::tga::encode_tga_24bit;
pub use crate::error::{HlBgError, Result};

/// Configuration options for the background generator.
#[derive(Debug, Clone)]
pub struct GeneratorConfig {
    /// Single input image to use for both layouts.
    pub input: Option<PathBuf>,

    /// Destination directory for output files.
    pub output_dir: PathBuf,

    /// Sizing/fitting strategy.
    pub fit: FitMode,

    /// Hex background padding color for `FitMode::Contain`.
    pub bg_color: String,

    /// Separate source image for the HD 21:9 layout. Overrides `input` for HD.
    pub hd_image: Option<PathBuf>,

    /// Separate source image for the Classic 4:3 layout. Overrides `input` for Classic.
    pub classic_image: Option<PathBuf>,

    /// Whether to generate the four *.txt layout descriptor files.
    pub include_layouts: bool,

    /// Simulate generation without writing files to disk.
    pub dry_run: bool,
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        Self {
            input: None,
            output_dir: PathBuf::from("./output"),
            fit: FitMode::Cover,
            bg_color: String::from("#000000"),
            hd_image: None,
            classic_image: None,
            include_layouts: true,
            dry_run: false,
        }
    }
}

/// Results returned after successful background generation.
#[derive(Debug, Clone)]
pub struct GeneratorResult {
    /// Total number of TGA slices generated (normally 117).
    pub slices_written: usize,

    /// Number of layout descriptor files written (0 or 4).
    pub layouts_written: usize,

    /// Destination directory.
    pub output_dir: PathBuf,
}

/// Orchestrates image processing and tile generation.
pub struct BackgroundGenerator {
    config: GeneratorConfig,
}

impl BackgroundGenerator {
    pub fn new(config: GeneratorConfig) -> Self {
        Self { config }
    }

    /// Executes generation using a no-op progress callback.
    pub fn generate(&self) -> Result<GeneratorResult> {
        self.generate_with_progress(|_, _, _| {})
    }

    /// Executes generation with a custom callback invoked for each completed tile.
    pub fn generate_with_progress<F>(&self, callback: F) -> Result<GeneratorResult>
    where
        F: Fn(&str, usize, usize) + Sync + Send,
    {
        // 1. Resolve source images
        let hd_source_path = self
            .config
            .hd_image
            .as_ref()
            .or(self.config.input.as_ref())
            .ok_or(HlBgError::MissingInput)?;

        let classic_source_path = self
            .config
            .classic_image
            .as_ref()
            .or(self.config.input.as_ref())
            .ok_or(HlBgError::MissingInput)?;

        let bg_color = parse_hex_color(&self.config.bg_color)?;

        // 2. Decode source images
        let hd_img = image::open(hd_source_path).map_err(|e| HlBgError::ImageDecode {
            path: hd_source_path.clone(),
            source: e,
        })?;

        let classic_img = if hd_source_path == classic_source_path {
            hd_img.clone()
        } else {
            image::open(classic_source_path).map_err(|e| HlBgError::ImageDecode {
                path: classic_source_path.clone(),
                source: e,
            })?
        };

        // 3. Fit to target canvases
        let hd_canvas = fit_to_canvas(
            &hd_img,
            HD_CANVAS_WIDTH,
            HD_CANVAS_HEIGHT,
            self.config.fit,
            bg_color,
        );

        let classic_canvas = fit_to_canvas(
            &classic_img,
            CLASSIC_CANVAS_WIDTH,
            CLASSIC_CANVAS_HEIGHT,
            self.config.fit,
            bg_color,
        );

        // 4. Output directory setup
        let bg_dir = if self.config.output_dir.ends_with("background") {
            self.config.output_dir.clone()
        } else {
            self.config.output_dir.join("background")
        };

        if !self.config.dry_run {
            fs::create_dir_all(&bg_dir).map_err(|e| HlBgError::Io {
                path: bg_dir.clone(),
                source: e,
            })?;
        }

        // 5. Slice and write tiles
        let hd_count = slice_hd_canvas(&hd_canvas, &bg_dir, self.config.dry_run, &callback)?;
        let classic_count =
            slice_classic_canvas(&classic_canvas, &bg_dir, self.config.dry_run, &callback)?;
        let total_slices = hd_count + classic_count;

        // 6. Write layout descriptor files if requested
        let mut layouts_written = 0;
        if self.config.include_layouts {
            let layout_parent_dir = if self.config.output_dir.ends_with("background") {
                self.config.output_dir.parent().unwrap_or(&self.config.output_dir)
            } else {
                &self.config.output_dir
            };

            let classic_content = generate_classic_layout_content();
            let hd_content = generate_hd_layout_content();

            let layout_files = [
                ("BackgroundLayout.txt", &classic_content),
                ("BackgroundLoadingLayout.txt", &classic_content),
                ("HD_BackgroundLayout.txt", &hd_content),
                ("HD_BackgroundLoadingLayout.txt", &hd_content),
            ];

            for (filename, content) in &layout_files {
                let target_file = layout_parent_dir.join(filename);
                if !self.config.dry_run {
                    fs::write(&target_file, content).map_err(|e| HlBgError::Io {
                        path: target_file,
                        source: e,
                    })?;
                }
                layouts_written += 1;
            }
        }

        Ok(GeneratorResult {
            slices_written: total_slices,
            layouts_written,
            output_dir: self.config.output_dir.clone(),
        })
    }
}
