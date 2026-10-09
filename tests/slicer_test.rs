use std::fs;
use image::{Rgb, RgbImage};
use tempfile::tempdir;

use hl_bg::{
    get_classic_tiles, get_hd_tiles, BackgroundGenerator, FitMode, GeneratorConfig,
    TOTAL_CLASSIC_TILES, TOTAL_HD_TILES, TOTAL_TILES,
};

#[test]
fn test_full_background_generation_pipeline() {
    let dir = tempdir().expect("Failed to create temporary directory");
    let input_path = dir.path().join("source.png");

    // Create a 1920x1080 dummy test image
    let img = RgbImage::from_pixel(1920, 1080, Rgb([100, 150, 200]));
    img.save(&input_path).expect("Failed to save test input image");

    let output_dir = dir.path().join("valve_resource");

    let config = GeneratorConfig {
        input: Some(input_path),
        output_dir: output_dir.clone(),
        fit: FitMode::Cover,
        include_layouts: true,
        ..Default::default()
    };

    let generator = BackgroundGenerator::new(config);
    let result = generator.generate().expect("Generation should succeed");

    assert_eq!(result.slices_written, TOTAL_TILES);
    assert_eq!(result.layouts_written, 4);

    let bg_dir = output_dir.join("background");
    assert!(bg_dir.exists(), "Background directory must exist");

    // Verify all 105 HD tiles exist
    for tile in get_hd_tiles() {
        let file_path = bg_dir.join(tile.filename);
        assert!(file_path.exists(), "Missing HD tile: {}", tile.filename);
        let meta = fs::metadata(&file_path).unwrap();
        assert!(meta.len() > 0, "Tile {} must not be empty", tile.filename);
    }

    // Verify all 12 Classic tiles exist
    for tile in get_classic_tiles() {
        let file_path = bg_dir.join(tile.filename);
        assert!(file_path.exists(), "Missing Classic tile: {}", tile.filename);
        let meta = fs::metadata(&file_path).unwrap();
        assert!(meta.len() > 0, "Tile {} must not be empty", tile.filename);
    }

    // Verify all 4 layout files exist in the parent folder
    let layout_files = [
        "BackgroundLayout.txt",
        "BackgroundLoadingLayout.txt",
        "HD_BackgroundLayout.txt",
        "HD_BackgroundLoadingLayout.txt",
    ];

    for name in layout_files {
        let file_path = output_dir.join(name);
        assert!(file_path.exists(), "Missing layout file: {}", name);
        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.starts_with("resolution\t"), "Layout file must have resolution header");
    }
}
