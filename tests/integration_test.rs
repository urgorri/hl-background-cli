use image::{DynamicImage, Rgb, RgbImage};
use tempfile::tempdir;

use hl_background_cli::{
    classic_layout_text, classic_tiles, enable_hd_background, encode_tga, encode_tile_region,
    fit_image, generate, generate_from_image, hd_layout_text, hd_tiles, parse_hex_color, Config,
    Error, FitMode, LayoutMode, LiblistStatus, Tile, HD_HEIGHT, HD_WIDTH,
};

fn tile(x: u32, y: u32, width: u32, height: u32) -> Tile {
    Tile {
        filename: "t.tga".to_string(),
        x,
        y,
        width,
        height,
    }
}

#[test]
fn test_tga_binary_spec() {
    let mut img = RgbImage::new(256, 256);
    img.put_pixel(0, 255, Rgb([0, 0, 255])); // Blue at bottom-left

    let mut buf = Vec::new();
    encode_tga(&mut buf, &img).unwrap();

    assert_eq!(buf.len(), 18 + (256 * 256 * 3) + 26);
    assert_eq!(buf[2], 2, "Uncompressed True-Color");
    assert_eq!(u16::from_le_bytes([buf[12], buf[13]]), 256, "Width");
    assert_eq!(u16::from_le_bytes([buf[14], buf[15]]), 256, "Height");
    assert_eq!(buf[16], 24, "24-bit");
    assert_eq!(buf[17], 0x00, "Bottom-to-top scanlines");
    assert_eq!(buf[18], 255, "BGR Blue channel of bottom-left pixel");
    assert_eq!(
        &buf[buf.len() - 26 + 8..buf.len() - 1],
        b"TRUEVISION-XFILE."
    );
}

#[test]
fn test_tile_region_streaming() {
    let mut canvas = RgbImage::new(512, 512);
    canvas.put_pixel(256, 511, Rgb([0, 255, 0]));

    let mut buf = Vec::new();
    encode_tile_region(&mut buf, &canvas, &tile(256, 256, 256, 256)).unwrap();
    assert_eq!(&buf[18..21], &[0, 255, 0]);
}

#[test]
fn test_wide_tile_spanning_multiple_chunks() {
    let mut canvas = RgbImage::new(700, 2);
    canvas.put_pixel(699, 1, Rgb([1, 2, 3]));

    let mut buf = Vec::new();
    encode_tile_region(&mut buf, &canvas, &tile(0, 0, 700, 2)).unwrap();
    assert_eq!(buf.len(), 18 + 700 * 2 * 3 + 26);
    // Bottom row first; last pixel of that row is (699, 1) in BGR order.
    let off = 18 + 699 * 3;
    assert_eq!(&buf[off..off + 3], &[3, 2, 1]);
}

#[test]
fn test_tile_validation() {
    let canvas = RgbImage::new(100, 100);
    let mut buf = Vec::new();

    for t in [
        tile(50, 50, 60, 50),
        tile(u32::MAX, 0, 10, 10),
        tile(0, 0, 0, 10),
        tile(0, 0, 10, 0),
    ] {
        let res = encode_tile_region(&mut buf, &canvas, &t);
        assert!(matches!(res, Err(Error::InvalidTile(_))), "{t:?}");
    }

    let big = RgbImage::new(70_000, 1);
    let res = encode_tile_region(&mut buf, &big, &tile(0, 0, 70_000, 1));
    assert!(matches!(res, Err(Error::InvalidTile(_))));
}

#[test]
fn test_generate_all_files() {
    let dir = tempdir().unwrap();
    let img_path = dir.path().join("test.png");
    RgbImage::from_pixel(1920, 1080, Rgb([50, 100, 150]))
        .save(&img_path)
        .unwrap();

    let out_dir = dir.path().join("output");
    let report = generate(&Config::new(img_path, &out_dir)).unwrap();
    assert_eq!(report.tiles_written, 117);
    assert_eq!(report.liblist, LiblistStatus::Skipped);

    let bg_dir = out_dir.join("background");
    assert_eq!(hd_tiles().len(), 105);
    assert_eq!(classic_tiles().len(), 12);
    for t in hd_tiles().iter().chain(classic_tiles().iter()) {
        assert!(bg_dir.join(&t.filename).exists(), "Missing {}", t.filename);
    }

    for name in [
        "BackgroundLayout.txt",
        "BackgroundLoadingLayout.txt",
        "HD_BackgroundLayout.txt",
        "HD_BackgroundLoadingLayout.txt",
    ] {
        let text = std::fs::read_to_string(out_dir.join(name)).unwrap();
        assert!(text.starts_with("resolution\t3840\t1600"), "{name}");
    }
    assert!(!out_dir.join("Legacy_BackgroundLayout.txt").exists());
}

#[test]
fn test_classic_layout_mode() {
    let dir = tempdir().unwrap();
    let img = DynamicImage::ImageRgb8(RgbImage::from_pixel(64, 64, Rgb([1, 2, 3])));
    let config = Config::new("unused.png", dir.path()).with_layout_mode(LayoutMode::Classic);
    generate_from_image(&img, &config).unwrap();

    let std_layout = std::fs::read_to_string(dir.path().join("BackgroundLayout.txt")).unwrap();
    let hd_layout = std::fs::read_to_string(dir.path().join("HD_BackgroundLayout.txt")).unwrap();
    assert_eq!(std_layout, classic_layout_text());
    assert_eq!(hd_layout, hd_layout_text());
}

#[test]
fn test_generate_skip_layouts_and_background_dir() {
    let dir = tempdir().unwrap();
    let img_path = dir.path().join("test_skip.png");
    RgbImage::from_pixel(400, 300, Rgb([10, 20, 30]))
        .save(&img_path)
        .unwrap();

    let target_bg = dir.path().join("my_mod").join("background");
    let config = Config::new(img_path, &target_bg).with_layouts(false);
    generate(&config).unwrap();

    assert!(target_bg.join("21_9_1_a_loading.tga").exists());
    assert!(target_bg.join("800_1_a_loading.tga").exists());
    assert!(!dir
        .path()
        .join("my_mod")
        .join("BackgroundLayout.txt")
        .exists());
    assert!(!target_bg.join("BackgroundLayout.txt").exists());
}

#[test]
fn test_missing_input_is_image_or_io_error() {
    let dir = tempdir().unwrap();
    let res = generate(&Config::new(dir.path().join("nope.png"), dir.path()));
    assert!(matches!(res, Err(Error::Image(_)) | Err(Error::Io(_))));
}

#[test]
fn test_error_is_send_sync() {
    fn assert_traits<T: Send + Sync + 'static + std::error::Error>() {}
    assert_traits::<Error>();
}

#[test]
fn test_fit_modes_dimensions() {
    let img = DynamicImage::ImageRgb8(RgbImage::from_pixel(100, 200, Rgb([255, 0, 0])));
    for mode in [
        FitMode::Stretch,
        FitMode::Cover,
        FitMode::Contain,
        FitMode::Fill,
    ] {
        for (w, h) in [(300, 300), (800, 600), (HD_WIDTH, HD_HEIGHT)] {
            let out = fit_image(&img, w, h, mode, [0, 0, 0]);
            assert_eq!((out.width(), out.height()), (w, h), "{mode:?} {w}x{h}");
        }
    }
}

#[test]
fn test_contain_uses_bg_color() {
    let img = DynamicImage::ImageRgb8(RgbImage::from_pixel(10, 10, Rgb([255, 255, 255])));
    let out = fit_image(&img, 100, 50, FitMode::Contain, [9, 8, 7]);
    assert_eq!(out.get_pixel(0, 0).0, [9, 8, 7]);
    assert_eq!(out.get_pixel(50, 25).0, [255, 255, 255]);
}

#[test]
fn test_fill_keeps_full_image_and_extends_wings() {
    // 16:9 source on the 21:9 canvas: center is the source, wings are darkened ambient.
    let img = DynamicImage::ImageRgb8(RgbImage::from_pixel(1600, 900, Rgb([200, 200, 200])));
    let out = fit_image(&img, HD_WIDTH, HD_HEIGHT, FitMode::Fill, [0, 0, 0]);
    assert_eq!(
        out.get_pixel(HD_WIDTH / 2, HD_HEIGHT / 2).0,
        [200, 200, 200]
    );
    let wing = out.get_pixel(5, HD_HEIGHT / 2).0;
    assert!(wing[0] < 200 && wing[0] > 100, "wing = {wing:?}");
}

#[test]
fn test_fill_does_not_crop_ultrawide_source() {
    // A source already as wide as the canvas must behave like cover.
    let mut src = RgbImage::from_pixel(3840, 1600, Rgb([0, 0, 0]));
    src.put_pixel(0, 800, Rgb([255, 255, 255]));
    let img = DynamicImage::ImageRgb8(src);
    let fill = fit_image(&img, HD_WIDTH, HD_HEIGHT, FitMode::Fill, [0, 0, 0]);
    let cover = fit_image(&img, HD_WIDTH, HD_HEIGHT, FitMode::Cover, [0, 0, 0]);
    assert_eq!(fill, cover);
}

#[test]
fn test_fill_on_classic_canvas_extends_square_source() {
    let img = DynamicImage::ImageRgb8(RgbImage::from_pixel(600, 600, Rgb([240, 240, 240])));
    let out = fit_image(&img, 800, 600, FitMode::Fill, [0, 0, 0]);
    assert_eq!(out.get_pixel(400, 300).0, [240, 240, 240]);
    assert!(out.get_pixel(2, 300).0[0] < 240);
}

#[test]
fn test_fit_mode_from_str() {
    use std::str::FromStr;
    assert_eq!(FitMode::from_str("cover").unwrap(), FitMode::Cover);
    assert_eq!(FitMode::from_str("COVER").unwrap(), FitMode::Cover);
    assert_eq!(FitMode::from_str("fill").unwrap(), FitMode::Fill);
    assert_eq!(FitMode::from_str("adaptive").unwrap(), FitMode::Fill);
    assert_eq!(FitMode::from_str("contain").unwrap(), FitMode::Contain);
    assert_eq!(FitMode::from_str("stretch").unwrap(), FitMode::Stretch);
    assert!(matches!(
        FitMode::from_str("invalid"),
        Err(Error::InvalidFitMode(_))
    ));
}

#[test]
fn test_parse_hex_color() {
    assert_eq!(parse_hex_color("#FF8000").unwrap(), [255, 128, 0]);
    assert_eq!(parse_hex_color("fff").unwrap(), [255, 255, 255]);
    assert_eq!(parse_hex_color("#0a0").unwrap(), [0, 170, 0]);
    for bad in ["", "#12", "#GGGGGG", "#1234567", "é12345"] {
        assert!(parse_hex_color(bad).is_err(), "{bad}");
    }
}

#[test]
fn test_enable_hd_background_parsing() {
    assert_eq!(
        enable_hd_background("game \"X\"\r\n").unwrap(),
        "game \"X\"\r\nhd_background \"1\"\r\n"
    );
    assert_eq!(
        enable_hd_background("game \"X\"\nhd_background \"0\"\nfoo \"1\"\n").unwrap(),
        "game \"X\"\nhd_background \"1\"\nfoo \"1\"\n"
    );
    // A commented-out line does not count as set.
    assert!(enable_hd_background("// hd_background \"1\"\n")
        .unwrap()
        .ends_with("hd_background \"1\"\n"));
    assert!(enable_hd_background("hd_background \"1\"\n").is_none());
    assert!(enable_hd_background("hd_background 1 // on\n").is_none());
}

#[test]
fn test_liblist_updated_only_when_requested() {
    let dir = tempdir().unwrap();
    let mod_dir = dir.path().join("my_mod");
    let resource_dir = mod_dir.join("resource");
    std::fs::create_dir_all(&resource_dir).unwrap();
    let liblist = mod_dir.join("liblist.gam");
    let original = "game \"Test Mod\"\r\n";
    std::fs::write(&liblist, original).unwrap();

    let img = DynamicImage::ImageRgb8(RgbImage::from_pixel(200, 200, Rgb([0, 255, 0])));

    let config = Config::new("unused.png", &resource_dir);
    let report = generate_from_image(&img, &config).unwrap();
    assert_eq!(report.liblist, LiblistStatus::Skipped);
    assert_eq!(std::fs::read_to_string(&liblist).unwrap(), original);

    let report = generate_from_image(&img, &config.clone().with_update_liblist(true)).unwrap();
    assert_eq!(report.liblist, LiblistStatus::Updated(liblist.clone()));
    assert!(std::fs::read_to_string(&liblist)
        .unwrap()
        .contains("hd_background \"1\""));

    let report = generate_from_image(&img, &config.with_update_liblist(true)).unwrap();
    assert_eq!(report.liblist, LiblistStatus::AlreadyEnabled(liblist));
}
