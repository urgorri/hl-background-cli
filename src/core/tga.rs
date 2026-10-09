use std::io::{self, Write};
use image::RgbImage;

/// Standard Truevision TGA 2.0 footer (26 bytes).
/// 4 bytes extension offset + 4 bytes developer area offset + "TRUEVISION-XFILE.\0"
const TGA_2_0_FOOTER: [u8; 26] = [
    0x00, 0x00, 0x00, 0x00, // Extension area offset (0 = none)
    0x00, 0x00, 0x00, 0x00, // Developer area offset (0 = none)
    b'T', b'R', b'U', b'E', b'V', b'I', b'S', b'I', b'O', b'N', b'-', b'X', b'F', b'I', b'L', b'E', b'.',
    0x00, // Trailing null character
];

/// Encodes an RGB image buffer into an uncompressed Truevision TGA 2.0 format
/// matching the exact requirements of the Half-Life GoldSrc engine.
///
/// Specific engine requirements:
/// - Uncompressed True-Color (Image Type 2)
/// - 24 bits per pixel (BGR byte order)
/// - Bottom-to-top scanlines (origin at bottom-left, descriptor byte = 0x00)
/// - Standard 26-byte TGA 2.0 footer
pub fn encode_tga_24bit<W: Write>(writer: &mut W, image: &RgbImage) -> io::Result<()> {
    let width = image.width() as u16;
    let height = image.height() as u16;

    // 18-byte TGA Header
    let mut header = [0u8; 18];
    header[0] = 0; // ID length
    header[1] = 0; // Color map type (0 = none)
    header[2] = 2; // Image type 2 = Uncompressed true-color
    header[3..8].fill(0); // Color map specification

    header[8..10].copy_from_slice(&0u16.to_le_bytes()); // X-origin
    header[10..12].copy_from_slice(&0u16.to_le_bytes()); // Y-origin
    header[12..14].copy_from_slice(&width.to_le_bytes());
    header[14..16].copy_from_slice(&height.to_le_bytes());
    header[16] = 24; // Pixel depth (24 bits = 3 bytes per pixel)
    header[17] = 0x00; // Image descriptor: bit 5 is 0 -> bottom-to-top scanlines

    writer.write_all(&header)?;

    // Pixel Data: Scanlines stored bottom-to-top, BGR order
    let row_stride = (width as usize) * 3;
    let mut row_buffer = vec![0u8; row_stride];

    for y in (0..height).rev() {
        for x in 0..width {
            let pixel = image.get_pixel(x as u32, y as u32);
            let offset = (x as usize) * 3;
            // TGA stores in Blue, Green, Red order
            row_buffer[offset] = pixel[2];     // B
            row_buffer[offset + 1] = pixel[1]; // G
            row_buffer[offset + 2] = pixel[0]; // R
        }
        writer.write_all(&row_buffer)?;
    }

    // TGA 2.0 Footer
    writer.write_all(&TGA_2_0_FOOTER)?;

    Ok(())
}
