use std::path::PathBuf;
use thiserror::Error;

/// Error types for `hl-bg` operations.
#[derive(Debug, Error)]
pub enum HlBgError {
    #[error("I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to decode image from {path}: {source}")]
    ImageDecode {
        path: PathBuf,
        #[source]
        source: image::ImageError,
    },

    #[error("Invalid color format '{0}'. Expected hex string like '#RRGGBB' or '#RGB'")]
    InvalidColor(String),

    #[error("No input image specified. Provide either an input file or both --hd-image and --classic-image")]
    MissingInput,

    #[error("Image dimensions too small ({width}x{height}). Minimum required is 1x1")]
    InvalidDimensions { width: u32, height: u32 },
}

pub type Result<T> = std::result::Result<T, HlBgError>;
