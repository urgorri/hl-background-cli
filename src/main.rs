use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use std::process;
use std::time::Instant;

use hl_bg::cli::args::CliArgs;
use hl_bg::{BackgroundGenerator, GeneratorConfig, TOTAL_TILES};

fn main() {
    let args = CliArgs::parse();

    if args.input.is_none() && (args.hd_image.is_none() || args.classic_image.is_none()) {
        eprintln!("Error: Please provide either a primary INPUT image or both --hd-image and --classic-image.");
        eprintln!("Run 'hl-bg --help' for usage instructions.");
        process::exit(1);
    }

    let start_time = Instant::now();

    println!("⚡ hl-bg: Half-Life Background Generator");
    if args.dry_run {
        println!("🔍 Running in DRY-RUN mode (no files will be written)");
    }

    let pb = ProgressBar::new(TOTAL_TILES as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}")
            .expect("Invalid progress bar template")
            .progress_chars("#>-"),
    );

    let verbose = args.verbose;

    let config = GeneratorConfig {
        input: args.input,
        output_dir: args.output_dir,
        fit: args.fit,
        bg_color: args.bg_color,
        hd_image: args.hd_image,
        classic_image: args.classic_image,
        include_layouts: !args.skip_layouts,
        dry_run: args.dry_run,
    };

    let generator = BackgroundGenerator::new(config);

    let result = generator.generate_with_progress(|filename, current, total| {
        pb.set_position(current as u64);
        if verbose {
            pb.set_message(format!("Wrote {}", filename));
        } else {
            pb.set_message(format!("{}/{}", current, total));
        }
    });

    match result {
        Ok(res) => {
            pb.finish_with_message("Done!");
            let duration = start_time.elapsed();
            println!("\n✅ Successfully generated:");
            println!("   - {} TGA background tiles", res.slices_written);
            if res.layouts_written > 0 {
                println!("   - {} layout descriptor files", res.layouts_written);
            }
            println!("   - Output directory: {}", res.output_dir.display());
            println!("   - Time taken: {:.2?}", duration);
        }
        Err(err) => {
            pb.abandon_with_message("Failed!");
            eprintln!("\n❌ Error: {}", err);
            process::exit(1);
        }
    }
}
