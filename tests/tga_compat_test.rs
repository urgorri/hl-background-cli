use std::io::Cursor;
use image::{Rgb, RgbImage};
use hl_bg::core::tga::encode_tga_24bit;

#[test]
fn test_tga_header_and_footer_binary_spec() {
    let width = 256;
    let height = 256;
    let mut img = RgbImage::new(width, height);

    // Put a distinct pixel at top-left (0, 0) and bottom-left (0, 255)
    img.put_pixel(0, 0, Rgb([255, 0, 0])); // Red at top
    img.put_pixel(0, 255, Rgb([0, 0, 255])); // Blue at bottom

    let mut buf = Vec::new();
    encode_tga_24bit(&mut buf, &img).expect("Failed to encode TGA");

    let expected_len = 18 + (width as usize * height as usize * 3) + 26;
    assert_eq!(buf.len(), expected_len, "File size mismatch");

    // Validate 18-byte Header
    assert_eq!(buf[0], 0, "ID length should be 0");
    assert_eq!(buf[1], 0, "Color map type should be 0");
    assert_eq!(buf[2], 2, "Image type should be 2 (uncompressed true-color)");
    assert_eq!(&buf[3..8], &[0, 0, 0, 0, 0], "Color map spec should be empty");
    assert_eq!(&buf[8..10], &[0, 0], "X-origin should be 0");
    assert_eq!(&buf[10..12], &[0, 0], "Y-origin should be 0");
    assert_eq!(u16::from_le_bytes([buf[12], buf[13]]), 256, "Width must be 256");
    assert_eq!(u16::from_le_bytes([buf[14], buf[15]]), 256, "Height must be 256");
    assert_eq!(buf[16], 24, "Pixel depth must be 24-bit");
    assert_eq!(buf[17], 0x00, "Descriptor must be 0x00 (bottom-to-top scanlines)");

    // Validate Bottom-to-Top Scanline Order:
    // First pixel written in the file (offset 18) must correspond to bottom-left (0, 255)
    // TGA BGR order: Blue is [255, 0, 0] in BGR
    assert_eq!(buf[18], 255, "First byte of first row should be Blue channel");
    assert_eq!(buf[19], 0, "Second byte should be Green");
    assert_eq!(buf[20], 0, "Third byte should be Red");

    // Validate 26-byte Footer
    let footer_start = buf.len() - 26;
    let footer = &buf[footer_start..];
    assert_eq!(&footer[0..8], &[0, 0, 0, 0, 0, 0, 0, 0], "Extension & Developer offsets");
    assert_eq!(&footer[8..25], b"TRUEVISION-XFILE.", "Signature mismatch");
    assert_eq!(footer[25], 0x00, "Trailing null byte");
}
