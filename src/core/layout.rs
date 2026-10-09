use crate::core::constants::{get_classic_tiles, get_hd_tiles};

/// Generates the contents of `BackgroundLayout.txt` and `BackgroundLoadingLayout.txt`.
pub fn generate_classic_layout_content() -> String {
    let mut out = String::from("resolution\t800\t600\n\n");
    let tiles = get_classic_tiles();

    for (idx, tile) in tiles.iter().enumerate() {
        out.push_str(&format!(
            "resource/background/{}\t\tfit\t\t{}\t{}\n",
            tile.filename, tile.x, tile.y
        ));
        // Add empty line between rows (every 4 items)
        if (idx + 1) % 4 == 0 && idx + 1 < tiles.len() {
            out.push('\n');
        }
    }

    out
}

/// Generates the contents of `HD_BackgroundLayout.txt` and `HD_BackgroundLoadingLayout.txt`.
pub fn generate_hd_layout_content() -> String {
    let mut out = String::from("resolution\t3840\t1600\n\n");
    let tiles = get_hd_tiles();

    for tile in &tiles {
        out.push_str(&format!(
            "resource/background/{}\tfit\t{}\t{}\n",
            tile.filename, tile.x, tile.y
        ));
    }

    out
}
