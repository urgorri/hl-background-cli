use std::path::PathBuf;
use clap::Parser;

use crate::core::slicer::FitMode;

#[derive(Parser, Debug)]
#[command(
    name = "hl-bg",
    author = "Half-Life Community",
    version,
    about = "Ultra-lightweight CLI to convert images into Half-Life 25th Anniversary background TGA slices",
    long_about = "Converts input images into the exact 117 uncompressed Truevision TGA 2.0 slices\n\
                  required for Half-Life's HD Ultrawide (3840x1600) and Classic 4:3 (800x600) background layouts."
)]
pub struct CliArgs {
    /// Path to input image (PNG, JPEG, WebP, BMP, TIFF)
    #[arg(value_name = "INPUT")]
    pub input: Option<PathBuf>,

    /// Destination directory for output files
    #[arg(short = 'o', long = "output-dir", default_value = "./output")]
    pub output_dir: PathBuf,

    /// Scaling strategy: 'cover', 'contain', or 'stretch'
    #[arg(long = "fit", default_value = "cover")]
    pub fit: FitMode,

    /// Background padding color in hex for contain mode (e.g. #000000)
    #[arg(long = "bg-color", default_value = "#000000")]
    pub bg_color: String,

    /// Separate source image for the 21:9 HD Ultrawide layout (3840x1600)
    #[arg(long = "hd-image", value_name = "PATH")]
    pub hd_image: Option<PathBuf>,

    /// Separate source image for the Classic 4:3 layout (800x600)
    #[arg(long = "classic-image", value_name = "PATH")]
    pub classic_image: Option<PathBuf>,

    /// Do not generate the four *.txt layout descriptor files
    #[arg(long = "skip-layouts", default_value_t = false)]
    pub skip_layouts: bool,

    /// Simulate generation without writing files to disk
    #[arg(long = "dry-run", default_value_t = false)]
    pub dry_run: bool,

    /// Enable verbose log messages
    #[arg(short = 'v', long = "verbose", default_value_t = false)]
    pub verbose: bool,
}
