use std::path::PathBuf;
use std::process;
use std::str::FromStr;
use std::time::Instant;

use argh::FromArgs;
use hl_background_cli::{generate, Config, FitMode};

#[derive(FromArgs, Debug)]
/// Ultra-lightweight tool to convert images into Half-Life background TGA slices.
struct Args {
    /// path to input image (PNG, JPEG, WebP, BMP)
    #[argh(positional)]
    input: PathBuf,

    /// destination directory [default: ./output]
    #[argh(option, short = 'o', default = "String::from(\"./output\")")]
    output_dir: String,

    /// sizing mode: cover, fill (adaptive 16:9), contain, stretch [default: cover]
    #[argh(option, default = "String::from(\"cover\")")]
    fit: String,

    /// skip writing the 4 layout .txt files
    #[argh(switch)]
    skip_layouts: bool,
}

fn main() {
    let args: Args = argh::from_env();
    let start = Instant::now();

    let fit_mode = match FitMode::from_str(&args.fit) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("❌ Error: {}", e);
            process::exit(1);
        }
    };

    println!(
        "⚡ hl-background-cli: Converting {}...",
        args.input.display()
    );

    let config = Config {
        input: args.input,
        output_dir: PathBuf::from(args.output_dir),
        fit: fit_mode,
        write_layouts: !args.skip_layouts,
    };

    match generate(&config) {
        Ok(()) => {
            println!(
                "✅ Done in {:.2?}! 117 TGA files written to {}",
                start.elapsed(),
                config.output_dir.display()
            );
        }
        Err(err) => {
            eprintln!("❌ Error: {}", err);
            process::exit(1);
        }
    }
}
