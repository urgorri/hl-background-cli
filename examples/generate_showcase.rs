//! Generates pre-rendered visual showcase assets for the GitHub Pages documentation (`docs/assets/`).
//!
//! Run with:
//! ```bash
//! cargo run --example generate_showcase --release
//! ```

use std::fs;
use std::path::Path;

use hl_background_cli::image::imageops::FilterType;
use hl_background_cli::image::{DynamicImage, Rgb, RgbImage};
use hl_background_cli::{
    classic_tiles, encode_tile_region, fit_image, hd_tiles, FitMode, Tile, CLASSIC_HEIGHT,
    CLASSIC_WIDTH, HD_HEIGHT, HD_WIDTH,
};

fn clamp_u8(v: f32) -> u8 {
    v.round().clamp(0.0, 255.0) as u8
}

fn mix(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    let t = t.clamp(0.0, 1.0);
    [
        clamp_u8(a[0] as f32 * (1.0 - t) + b[0] as f32 * t),
        clamp_u8(a[1] as f32 * (1.0 - t) + b[1] as f32 * t),
        clamp_u8(a[2] as f32 * (1.0 - t) + b[2] as f32 * t),
    ]
}

/// Signed distance to a line segment `a` -> `b`.
fn sd_segment(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let pax = px - ax;
    let pay = py - ay;
    let bax = bx - ax;
    let bay = by - ay;
    let h = ((pax * bax + pay * bay) / (bax * bax + bay * bay)).clamp(0.0, 1.0);
    let dx = pax - bax * h;
    let dy = pay - bay * h;
    (dx * dx + dy * dy).sqrt()
}

/// Renders a stylized Black Mesa / Half-Life inspired 16:9 widescreen scene (`1920x1080`).
/// Includes:
/// - Warm amber/orange reactor glow on the left & center, cool cyan/slate facility tones on the right
/// - Perspective floor grid & mesa canyon silhouettes so cropping/stretching/wings are unmistakable
/// - Central circular containment ring with an iconic geometric Lambda glyph
/// - Crisp corner calibration brackets so visitors can immediately see if corners are cropped or preserved
fn build_scene_16_9(width: u32, height: u32) -> RgbImage {
    let mut img = RgbImage::new(width, height);
    let wf = width as f32;
    let hf = height as f32;
    let cx = wf * 0.5;
    let cy = hf * 0.46;
    let horizon = hf * 0.62;

    for y in 0..height {
        let yf = y as f32;
        for x in 0..width {
            let xf = x as f32;
            let u = xf / wf;

            // Base sky / facility backdrop
            let top_col = mix([12, 16, 26], [8, 28, 42], u);
            let mid_col = mix([42, 24, 18], [16, 48, 68], u);
            let horizon_glow = mix([245, 130, 32], [14, 165, 233], (u * 0.85).clamp(0.0, 1.0));

            let mut col = if yf < horizon {
                let t = (yf / horizon).powf(1.2);
                let base = mix(top_col, mid_col, t);
                let glow_t = ((yf / horizon).powf(3.0)) * 0.65;
                mix(base, horizon_glow, glow_t)
            } else {
                // Perspective floor plane
                let floor_t = ((yf - horizon) / (hf - horizon)).clamp(0.0, 1.0);
                let mut floor = mix([26, 20, 22], [10, 12, 16], floor_t.powf(0.7));

                // Perspective grid lines
                let depth = 1.0 / (floor_t + 0.06);
                let persp_x = (u - 0.5) * depth * 3.2;
                let gx = (persp_x.fract().abs() - 0.5).abs();
                let gz = ((depth * 1.4).fract().abs() - 0.5).abs();
                let line_intensity = ((1.0 - (gx * 28.0).clamp(0.0, 1.0))
                    .max(1.0 - (gz * 22.0).clamp(0.0, 1.0)))
                    * (1.0 - floor_t * 0.65);
                let grid_col = if u < 0.55 {
                    [245, 158, 11]
                } else {
                    [6, 182, 212]
                };
                floor = mix(floor, grid_col, line_intensity * 0.55);
                floor
            };

            // Mesa silhouette ridges near horizon
            let ridge1 = horizon
                - 38.0
                - 26.0 * (u * 9.5).sin()
                - 14.0 * (u * 23.0 + 0.7).cos()
                - 45.0 * (1.0 - ((u - 0.22) * 3.2).powi(2)).clamp(0.0, 1.0);
            if yf >= ridge1 && yf < horizon {
                col = mix(col, [14, 15, 20], 0.82);
            }

            // Radial portal glow around center
            let dx = xf - cx;
            let dy = yf - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            let ring_r = hf * 0.23;

            let radial_glow = (1.0 - (dist / (ring_r * 1.9)).clamp(0.0, 1.0)).powi(2);
            col = mix(col, [245, 158, 11], radial_glow * 0.42);

            // Outer containment ring
            let ring_dist = (dist - ring_r).abs();
            if ring_dist < 7.0 {
                let aa = 1.0 - (ring_dist / 7.0).clamp(0.0, 1.0);
                col = mix(col, [245, 158, 11], aa * 0.92);
            }
            // Inner dark disc
            if dist < ring_r - 7.0 {
                col = mix(col, [12, 13, 17], 0.78);
            }

            // Lambda symbol inside the ring (normalized to ring radius)
            let lx = dx / ring_r;
            let ly = dy / ring_r;
            let d1 = sd_segment(lx, ly, -0.28, -0.44, -0.06, -0.44); // top serif
            let d2 = sd_segment(lx, ly, -0.06, -0.44, 0.34, 0.44); // main diagonal
            let d3 = sd_segment(lx, ly, 0.06, -0.02, -0.32, 0.44); // left leg
            let d4 = sd_segment(lx, ly, 0.22, 0.44, 0.42, 0.44); // bottom-right serif
            let d_lambda = d1.min(d2).min(d3).min(d4);
            let stroke = 0.085;
            if d_lambda < stroke {
                let edge = ((stroke - d_lambda) / 0.02).clamp(0.0, 1.0);
                col = mix(col, [255, 176, 36], edge);
            }

            // Corner calibration brackets (36px inset, 64px arm length, 5px thick)
            // Makes it visually obvious whether corners are cropped (cover) or preserved (fill/contain)
            let inset = 28;
            let arm = 72;
            let thick = 6;
            let in_left = x >= inset && x < inset + arm;
            let in_right = x >= width - inset - arm && x < width - inset;
            let in_top = y >= inset && y < inset + arm;
            let in_bot = y >= height - inset - arm && y < height - inset;

            let on_v_left = x >= inset && x < inset + thick;
            let on_v_right = x >= width - inset - thick && x < width - inset;
            let on_h_top = y >= inset && y < inset + thick;
            let on_h_bot = y >= height - inset - thick && y < height - inset;

            let corner_bracket = (in_left && in_top && (on_v_left || on_h_top))
                || (in_right && in_top && (on_v_right || on_h_top))
                || (in_left && in_bot && (on_v_left || on_h_bot))
                || (in_right && in_bot && (on_v_right || on_h_bot));

            if corner_bracket {
                col = [16, 185, 129]; // Emerald calibration marker
            }

            // Subtle outer 2px border
            if x < 2 || x >= width - 2 || y < 2 || y >= height - 2 {
                col = mix(col, [56, 189, 248], 0.65);
            }

            img.put_pixel(x, y, Rgb(col));
        }
    }

    img
}

/// Overlay a clean tile grid blueprint on top of an existing canvas.
fn render_grid_blueprint(canvas: &RgbImage, tiles: &[Tile], accent: [u8; 3]) -> RgbImage {
    let mut out = canvas.clone();
    let w = out.width();
    let h = out.height();

    for t in tiles {
        let right = (t.x + t.width).min(w);
        let bottom = (t.y + t.height).min(h);

        // Draw top and left borders of each tile
        for x in t.x..right {
            for dy in 0..2 {
                if t.y + dy < h {
                    let p = out.get_pixel(x, t.y + dy).0;
                    out.put_pixel(x, t.y + dy, Rgb(mix(p, accent, 0.75)));
                }
            }
            if bottom > 0 && bottom == h {
                let p = out.get_pixel(x, bottom - 1).0;
                out.put_pixel(x, bottom - 1, Rgb(mix(p, accent, 0.85)));
            }
        }

        for y in t.y..bottom {
            for dx in 0..2 {
                if t.x + dx < w {
                    let p = out.get_pixel(t.x + dx, y).0;
                    out.put_pixel(t.x + dx, y, Rgb(mix(p, accent, 0.75)));
                }
            }
            if right > 0 && right == w {
                let p = out.get_pixel(right - 1, y).0;
                out.put_pixel(right - 1, y, Rgb(mix(p, accent, 0.85)));
            }
        }

        // Highlight irregular edge tiles (row 7 on HD, row 3 / col d on Classic)
        let is_edge_strip = t.width < 256 || t.height < 256;
        if is_edge_strip {
            for y in t.y..bottom {
                for x in t.x..right {
                    let p = out.get_pixel(x, y).0;
                    out.put_pixel(x, y, Rgb(mix(p, [245, 158, 11], 0.22)));
                }
            }
        }
    }

    out
}

fn save_downscaled(img: &RgbImage, max_w: u32, path: &Path) {
    let (w, h) = img.dimensions();
    if w <= max_w {
        img.save(path).expect("failed to save image");
    } else {
        let new_h = ((h as f64 * max_w as f64) / w as f64).round() as u32;
        let resized =
            DynamicImage::ImageRgb8(img.clone()).resize_exact(max_w, new_h, FilterType::Lanczos3);
        resized.to_rgb8().save(path).expect("failed to save image");
    }
}

fn save_tile_preview(canvas: &RgbImage, tile: &Tile, path: &Path) {
    let sub =
        DynamicImage::ImageRgb8(canvas.clone()).crop_imm(tile.x, tile.y, tile.width, tile.height);
    sub.to_rgb8()
        .save(path)
        .expect("failed to save tile preview");

    // Also verify byte-exact TGA encoding in memory
    let mut tga_bytes = Vec::new();
    encode_tile_region(&mut tga_bytes, canvas, tile).expect("TGA encode failed");
    assert_eq!(
        tga_bytes.len(),
        18 + (tile.width as usize * tile.height as usize * 3) + 26
    );
}

fn main() {
    let assets_dir = Path::new("docs/assets");
    fs::create_dir_all(assets_dir).expect("failed to create docs/assets");

    println!("1. Loading or building 16:9 source scene...");
    let sample_candidates = [
        Path::new("res/sample.png"),
        Path::new("sample.png"),
        Path::new("docs/assets/sample.png"),
        Path::new(r"C:\Users\BEELINK\Documents\Untitled.png"),
    ];

    let dyn_src = if let Some(found) = sample_candidates.iter().find(|p| p.exists()) {
        println!("Loading sample image from: {}", found.display());
        let img = hl_background_cli::image::open(found).expect("failed to open sample image");
        let rgb = img.to_rgb8();
        save_downscaled(&rgb, 960, &assets_dir.join("source_16_9.png"));
        DynamicImage::ImageRgb8(rgb)
    } else {
        println!("Generating synthetic 16:9 source scene (1920x1080)...");
        let src_16_9 = build_scene_16_9(1920, 1080);
        save_downscaled(&src_16_9, 960, &assets_dir.join("source_16_9.png"));
        DynamicImage::ImageRgb8(src_16_9)
    };

    let bg_color = [16, 24, 36]; // Dark slate blue for contain mode (#101824)

    let modes = [
        ("cover", FitMode::Cover),
        ("fill", FitMode::Fill),
        ("contain", FitMode::Contain),
        ("stretch", FitMode::Stretch),
    ];

    let hd_tile_list = hd_tiles();
    let classic_tile_list = classic_tiles();

    for (name, mode) in modes {
        println!("2. Rendering mode '{name}' for HD (3840x1600) and Classic (800x600)...");
        let hd_canvas = fit_image(&dyn_src, HD_WIDTH, HD_HEIGHT, mode, bg_color);
        save_downscaled(&hd_canvas, 960, &assets_dir.join(format!("hd_{name}.png")));

        let classic_canvas = fit_image(&dyn_src, CLASSIC_WIDTH, CLASSIC_HEIGHT, mode, bg_color);
        save_downscaled(
            &classic_canvas,
            600,
            &assets_dir.join(format!("classic_{name}.png")),
        );

        if name == "fill" {
            println!("3. Rendering tile grid blueprints and sample tile slices...");
            let hd_grid = render_grid_blueprint(&hd_canvas, &hd_tile_list, [14, 165, 233]);
            save_downscaled(&hd_grid, 960, &assets_dir.join("hd_fill_grid.png"));

            let classic_grid =
                render_grid_blueprint(&classic_canvas, &classic_tile_list, [16, 185, 129]);
            save_downscaled(
                &classic_grid,
                600,
                &assets_dir.join("classic_fill_grid.png"),
            );

            // Save representative individual tiles from the Fill HD & Classic canvases
            // 1) Left ambient wing tile (row 4, col a -> index 3*15 + 0 = 45)
            save_tile_preview(
                &hd_canvas,
                &hd_tile_list[45],
                &assets_dir.join("tile_21_9_4_a.png"),
            );
            // 2) Wing-to-foreground feathered transition tile (row 4, col b -> index 46)
            save_tile_preview(
                &hd_canvas,
                &hd_tile_list[46],
                &assets_dir.join("tile_21_9_4_b.png"),
            );
            // 3) Center Lambda focal tile (row 4, col h -> index 3*15 + 7 = 52)
            save_tile_preview(
                &hd_canvas,
                &hd_tile_list[52],
                &assets_dir.join("tile_21_9_4_h.png"),
            );
            // 4) Bottom row 256x64 strip tile (row 7, col h -> index 6*15 + 7 = 97)
            save_tile_preview(
                &hd_canvas,
                &hd_tile_list[97],
                &assets_dir.join("tile_21_9_7_h.png"),
            );
            // 5) Classic right-bottom corner 32x88 tile (row 3, col d -> index 11)
            save_tile_preview(
                &classic_canvas,
                &classic_tile_list[11],
                &assets_dir.join("tile_800_3_d.png"),
            );
        }
    }

    println!(
        "Showcase assets successfully generated in {}",
        assets_dir.display()
    );
}
