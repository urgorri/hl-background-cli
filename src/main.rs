use std::path::PathBuf;
use std::process;
use std::time::Instant;

use clap::Parser;
use hl_bg::{generate, Config, FitMode};

#[derive(Parser, Debug)]
#[command(
    name = "hl-bg",
    version,
    about = "Converts an image into the exact 117 TGA slices for Half-Life background screens"
)]
struct Args {
    /// Path to input image (PNG, JPEG, WebP, BMP)
    #[arg(value_name = "IMAGE")]
    input: PathBuf,

    /// Destination directory (e.g. valve/resource)
    #[arg(short = 'o', long = "output-dir", default_value = "./output")]
    output_dir: PathBuf,

    /// Sizing mode: cover, contain, stretch
    #[arg(long = "fit", default_value = "cover")]
    fit: FitMode,

    /// Skip writing the 4 layout .txt files
    #[arg(long = "skip-layouts", default_value_t = false)]
    skip_layouts: bool,
}

fn main() {
    let args = Args::parse();
    let start = Instant::now();

    println!("⚡ hl-bg: Converting {}...", args.input.display());

    let config = Config {
        input: args.input,
        output_dir: args.output_dir.clone(),
        fit: args.fit,
        write_layouts: !args.skip_layouts,
    };

    match generate(&config) {
        Ok(()) => {
            println!(
                "✅ Done in {:.2?}! 117 TGA files written to {}",
                start.elapsed(),
                args.output_dir.display()
            );
        }
        Err(err) => {
            eprintln!("❌ Error: {}", err);
            process::exit(1);
        }
    }
}
