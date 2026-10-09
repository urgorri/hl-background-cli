/// Tile coordinate descriptor defining the location, size, and filename of a TGA slice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TileDefinition {
    pub filename: &'static str,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

pub const HD_CANVAS_WIDTH: u32 = 3840;
pub const HD_CANVAS_HEIGHT: u32 = 1600;

pub const CLASSIC_CANVAS_WIDTH: u32 = 800;
pub const CLASSIC_CANVAS_HEIGHT: u32 = 600;

pub const TOTAL_HD_TILES: usize = 105;
pub const TOTAL_CLASSIC_TILES: usize = 12;
pub const TOTAL_TILES: usize = TOTAL_HD_TILES + TOTAL_CLASSIC_TILES; // 117

/// Generates the list of 105 tile definitions for the 21:9 HD layout.
pub fn get_hd_tiles() -> Vec<TileDefinition> {
    let mut tiles = Vec::with_capacity(TOTAL_HD_TILES);
    let cols = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o'];

    for row_idx in 1..=7 {
        let y = (row_idx - 1) * 256;
        let height = if row_idx == 7 { 64 } else { 256 };

        for (col_idx, &col_char) in cols.iter().enumerate() {
            let x = (col_idx as u32) * 256;
            let width = 256;

            // Box/leak static string for zero runtime allocation during file writes
            let filename = Box::leak(
                format!("21_9_{}_{}_loading.tga", row_idx, col_char).into_boxed_str(),
            );

            tiles.push(TileDefinition {
                filename,
                x,
                y,
                width,
                height,
            });
        }
    }

    tiles
}

/// Generates the list of 12 tile definitions for the 800x600 Classic 4:3 layout.
pub fn get_classic_tiles() -> Vec<TileDefinition> {
    let mut tiles = Vec::with_capacity(TOTAL_CLASSIC_TILES);
    let cols = ['a', 'b', 'c', 'd'];

    for row_idx in 1..=3 {
        let y = (row_idx - 1) * 256;
        let height = if row_idx == 3 { 88 } else { 256 };

        for (col_idx, &col_char) in cols.iter().enumerate() {
            let x = (col_idx as u32) * 256;
            let width = if col_char == 'd' { 32 } else { 256 };

            let filename = Box::leak(
                format!("800_{}_{}_loading.tga", row_idx, col_char).into_boxed_str(),
            );

            tiles.push(TileDefinition {
                filename,
                x,
                y,
                width,
                height,
            });
        }
    }

    tiles
}
