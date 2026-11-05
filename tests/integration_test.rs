use image::{Rgb, RgbImage};
use tempfile::tempdir;

use hl_background_cli::{
    classic_tiles, encode_tga, encode_tile_region, generate, hd_tiles, Config, FitMode, Tile,
};

#[test]
fn test_tga_binary_spec() {
    let width = 256;
    let height = 256;
    let mut img = RgbImage::new(width, height);
    img.put_pixel(0, 255, Rgb([0, 0, 255])); // Blue at bottom-left

    let mut buf = Vec::new();
    encode_tga(&mut buf, &img).unwrap();

    // Size: 18 (header) + 256*256*3 (pixels) + 26 (footer)
    assert_eq!(buf.len(), 18 + (256 * 256 * 3) + 26);

    // Header checks
    assert_eq!(buf[2], 2, "Uncompressed True-Color");
    assert_eq!(u16::from_le_bytes([buf[12], buf[13]]), 256, "Width");
    assert_eq!(u16::from_le_bytes([buf[14], buf[15]]), 256, "Height");
    assert_eq!(buf[16], 24, "24-bit RGB");
    assert_eq!(buf[17], 0x00, "Bottom-to-top scanlines");

    // Scanline bottom-to-top check (first pixel byte in file is bottom-left Blue)
    assert_eq!(buf[18], 255, "BGR Blue channel");

    // Footer check
    let footer = &buf[buf.len() - 26..];
    assert_eq!(&footer[8..25], b"TRUEVISION-XFILE.");
}

#[test]
fn test_tile_region_streaming() {
    let mut canvas = RgbImage::new(512, 512);
    // Put a green pixel at tile (256, 256)'s bottom-left (x=256, y=256+255 = 511)
    canvas.put_pixel(256, 511, Rgb([0, 255, 0]));

    let tile = Tile {
        filename: "test_tile.tga".to_string(),
        x: 256,
        y: 256,
        width: 256,
        height: 256,
    };

    let mut buf = Vec::new();
    encode_tile_region(&mut buf, &canvas, &tile).unwrap();

    // Verify first pixel byte is Green in BGR order: [B=0, G=255, R=0]
    assert_eq!(buf[18], 0);
    assert_eq!(buf[19], 255);
    assert_eq!(buf[20], 0);
}

#[test]
fn test_generate_all_files() {
    let dir = tempdir().unwrap();
    let img_path = dir.path().join("test.png");

    let img = RgbImage::from_pixel(1920, 1080, Rgb([50, 100, 150]));
    img.save(&img_path).unwrap();

    let out_dir = dir.path().join("output");
    let config = Config {
        input: img_path,
        output_dir: out_dir.clone(),
        fit: FitMode::Cover,
        write_layouts: true,
    };

    generate(&config).unwrap();

    let bg_dir = out_dir.join("background");
    assert!(bg_dir.exists());

    // 105 HD tiles + 12 Classic tiles = 117
    let hd = hd_tiles();
    assert_eq!(hd.len(), 105);
    for t in &hd {
        assert!(bg_dir.join(&t.filename).exists(), "Missing {}", t.filename);
    }

    let classic = classic_tiles();
    assert_eq!(classic.len(), 12);
    for t in &classic {
        assert!(bg_dir.join(&t.filename).exists(), "Missing {}", t.filename);
    }

    // 4 layout files
    assert!(out_dir.join("BackgroundLayout.txt").exists());
    assert!(out_dir.join("HD_BackgroundLayout.txt").exists());
    assert!(out_dir.join("BackgroundLoadingLayout.txt").exists());
    assert!(out_dir.join("HD_BackgroundLoadingLayout.txt").exists());
}

#[test]
fn test_tile_region_bounds_check() {
    let canvas = RgbImage::new(100, 100);
    let tile_out_of_bounds = Tile {
        filename: "overflow.tga".to_string(),
        x: 50,
        y: 50,
        width: 60, // 50 + 60 = 110 > 100
        height: 50,
    };

    let mut buf = Vec::new();
    let res = encode_tile_region(&mut buf, &canvas, &tile_out_of_bounds);
    assert!(res.is_err());
    assert_eq!(res.unwrap_err().kind(), std::io::ErrorKind::InvalidInput);
}

#[test]
fn test_fit_modes() {
    use hl_background_cli::fit_image;
    use image::DynamicImage;

    let img = DynamicImage::ImageRgb8(RgbImage::from_pixel(100, 200, Rgb([255, 0, 0])));

    let stretch = fit_image(&img, 300, 300, FitMode::Stretch);
    assert_eq!(stretch.width(), 300);
    assert_eq!(stretch.height(), 300);

    let cover = fit_image(&img, 300, 300, FitMode::Cover);
    assert_eq!(cover.width(), 300);
    assert_eq!(cover.height(), 300);

    let contain = fit_image(&img, 300, 300, FitMode::Contain);
    assert_eq!(contain.width(), 300);
    assert_eq!(contain.height(), 300);
}

#[test]
fn test_fit_mode_from_str() {
    use std::str::FromStr;

    assert_eq!(FitMode::from_str("cover").unwrap(), FitMode::Cover);
    assert_eq!(FitMode::from_str("COVER").unwrap(), FitMode::Cover);
    assert_eq!(FitMode::from_str("contain").unwrap(), FitMode::Contain);
    assert_eq!(FitMode::from_str("stretch").unwrap(), FitMode::Stretch);
    assert!(FitMode::from_str("invalid").is_err());
}

#[test]
fn test_generate_skip_layouts_and_background_dir() {
    let dir = tempdir().unwrap();
    let img_path = dir.path().join("test_skip.png");

    let img = RgbImage::from_pixel(400, 300, Rgb([10, 20, 30]));
    img.save(&img_path).unwrap();

    // Test with output directory already named "background" and write_layouts = false
    let target_bg = dir.path().join("my_mod").join("background");
    let config = Config {
        input: img_path,
        output_dir: target_bg.clone(),
        fit: FitMode::Cover,
        write_layouts: false,
    };

    generate(&config).unwrap();

    // Verify 117 tiles were written directly into target_bg
    assert!(target_bg.join("21_9_1_a_loading.tga").exists());
    assert!(target_bg.join("800_1_a_loading.tga").exists());

    // Verify NO layout files were generated
    assert!(!dir
        .path()
        .join("my_mod")
        .join("BackgroundLayout.txt")
        .exists());
    assert!(!target_bg.join("BackgroundLayout.txt").exists());
}
