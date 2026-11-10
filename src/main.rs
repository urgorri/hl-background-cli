use std::path::PathBuf;
use std::process;
use std::str::FromStr;
use std::time::Instant;

use argh::FromArgs;
use hl_background_cli::{generate, parse_hex_color, Config, FitMode, LayoutMode, LiblistStatus};

#[derive(FromArgs, Debug)]
/// Ultra-lightweight tool to convert images into Half-Life background TGA slices.
struct Args {
    /// path to input image (PNG, JPEG, WebP, BMP)
    #[argh(positional)]
    input: Option<PathBuf>,

    /// destination directory [default: ./output]
    #[argh(option, short = 'o', default = "PathBuf::from(\"./output\")")]
    output_dir: PathBuf,

    /// sizing mode: cover, fill (adaptive), contain, stretch [default: cover]
    #[argh(option, default = "String::from(\"cover\")")]
    fit: String,

    /// padding color for contain mode, as #RRGGBB or #RGB [default: #000000]
    #[argh(option, default = "String::from(\"#000000\")")]
    bg_color: String,

    /// skip writing the 4 layout .txt files
    #[argh(switch)]
    skip_layouts: bool,

    /// point BackgroundLayout.txt and BackgroundLoadingLayout.txt to the classic
    /// 800x600 grid instead of the HD grid
    #[argh(switch)]
    classic_layout: bool,

    /// do not enable hd_background "1" in a nearby liblist.gam
    #[argh(switch)]
    no_liblist: bool,

    /// suppress all output except errors
    #[argh(switch, short = 'q')]
    quiet: bool,

    /// print version and exit
    #[argh(switch, short = 'V')]
    version: bool,
}

fn fail(message: impl std::fmt::Display) -> ! {
    eprintln!("Error: {message}");
    process::exit(1);
}

fn main() {
    let args: Args = argh::from_env();

    if args.version {
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        return;
    }

    let input = args
        .input
        .unwrap_or_else(|| fail("missing input image. Run with --help for usage"));
    let fit = FitMode::from_str(&args.fit).unwrap_or_else(|e| fail(e));
    let bg_color = parse_hex_color(&args.bg_color).unwrap_or_else(|e| fail(e));

    if !args.quiet {
        println!("Converting {}...", input.display());
    }
    let start = Instant::now();

    let config = Config::new(input, args.output_dir)
        .with_fit(fit)
        .with_bg_color(bg_color)
        .with_layouts(!args.skip_layouts)
        .with_layout_mode(if args.classic_layout {
            LayoutMode::Classic
        } else {
            LayoutMode::Hd
        })
        .with_update_liblist(!args.no_liblist);

    let report = generate(&config).unwrap_or_else(|e| fail(e));

    if !args.quiet {
        println!(
            "Done in {:.2?}: {} TGA files written to {}",
            start.elapsed(),
            report.tiles_written,
            config.output_dir.display()
        );
        match &report.liblist {
            LiblistStatus::Updated(p) => {
                println!("Enabled hd_background \"1\" in {}", p.display())
            }
            LiblistStatus::AlreadyEnabled(p) => {
                println!("hd_background \"1\" already set in {}", p.display())
            }
            _ => {}
        }
    }
}
