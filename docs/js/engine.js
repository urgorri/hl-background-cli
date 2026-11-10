/**
 * hl-background-cli Interactive Documentation Engine
 * Mirrors the exact geometry, tile grids, TGA 2.0 byte sizing, layout descriptors,
 * CLI command generation, and Rust library code snippets from src/lib.rs.
 */
window.HlBgEngine = {
  HD_WIDTH: 3840,
  HD_HEIGHT: 1600,
  CLASSIC_WIDTH: 800,
  CLASSIC_HEIGHT: 600,
  FEATHER_PX: 24,

  // Generates the 105 tile descriptors for the 3840x1600 (21:9) HD grid
  hdTiles() {
    const tiles = [];
    const cols = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o'];
    for (let row = 1; row <= 7; row++) {
      const y = (row - 1) * 256;
      const height = row === 7 ? 64 : 256;
      for (let colIdx = 0; colIdx < cols.length; colIdx++) {
        const colChar = cols[colIdx];
        const x = colIdx * 256;
        const width = 256;
        const pixelBytes = width * height * 3;
        const totalBytes = 18 + pixelBytes + 26;
        tiles.push({
          index: tiles.length,
          grid: 'hd',
          row,
          col: colChar,
          colIdx: colIdx + 1,
          filename: `21_9_${row}_${colChar}_loading.tga`,
          x,
          y,
          width,
          height,
          pixelBytes,
          totalBytes,
          isEdgeStrip: height !== 256,
          leftPct: (x / 3840) * 100,
          topPct: (y / 1600) * 100,
          widthPct: (width / 3840) * 100,
          heightPct: (height / 1600) * 100,
          layoutLine: `resource/background/21_9_${row}_${colChar}_loading.tga\tfit\t${x}\t${y}`
        });
      }
    }
    return tiles;
  },

  // Generates the 12 tile descriptors for the 800x600 (4:3) Classic grid
  classicTiles() {
    const tiles = [];
    const cols = ['a', 'b', 'c', 'd'];
    for (let row = 1; row <= 3; row++) {
      const y = (row - 1) * 256;
      const height = row === 3 ? 88 : 256;
      for (let colIdx = 0; colIdx < cols.length; colIdx++) {
        const colChar = cols[colIdx];
        const x = colIdx * 256;
        const width = colChar === 'd' ? 32 : 256;
        const pixelBytes = width * height * 3;
        const totalBytes = 18 + pixelBytes + 26;
        tiles.push({
          index: tiles.length,
          grid: 'classic',
          row,
          col: colChar,
          colIdx: colIdx + 1,
          filename: `800_${row}_${colChar}_loading.tga`,
          x,
          y,
          width,
          height,
          pixelBytes,
          totalBytes,
          isEdgeStrip: width !== 256 || height !== 256,
          leftPct: (x / 800) * 100,
          topPct: (y / 600) * 100,
          widthPct: (width / 800) * 100,
          heightPct: (height / 600) * 100,
          layoutLine: `resource/background/800_${row}_${colChar}_loading.tga\t\tfit\t\t${x}\t${y}`
        });
      }
    }
    return tiles;
  },

  // Parses #RRGGBB or #RGB into [r, g, b]
  parseHexColor(hex) {
    const clean = String(hex || '').trim().replace(/^#/, '');
    if (/^[0-9a-fA-F]{6}$/.test(clean)) {
      return [
        parseInt(clean.substring(0, 2), 16),
        parseInt(clean.substring(2, 4), 16),
        parseInt(clean.substring(4, 6), 16)
      ];
    }
    if (/^[0-9a-fA-F]{3}$/.test(clean)) {
      return [
        parseInt(clean[0], 16) * 17,
        parseInt(clean[1], 16) * 17,
        parseInt(clean[2], 16) * 17
      ];
    }
    return [0, 0, 0];
  },

  // Computes exact fit geometry for a source image on a target canvas (mirrors fit_image in src/lib.rs)
  calcCanvasGeometry(srcW, srcH, targetW, targetH, fitMode) {
    const w = Math.max(1, parseInt(srcW) || 1920);
    const h = Math.max(1, parseInt(srcH) || 1080);
    const srcAspect = w / h;
    const targetAspect = targetW / targetH;

    if (fitMode === 'stretch') {
      return {
        summary: `Non-uniform Lanczos3 resize (${w}x${h} -> ${targetW}x${targetH})`,
        detail: '0 px cropped | Aspect distortion: ' + ((targetAspect / srcAspect - 1) * 100).toFixed(1) + '%'
      };
    }

    if (fitMode === 'cover') {
      if (srcAspect > targetAspect) {
        const cropW = Math.min(w, Math.max(1, Math.round(h * targetAspect)));
        const cropX = Math.floor((w - cropW) / 2);
        return {
          summary: `Center-crop ${cropW}x${h} (offset X=${cropX}px) -> ${targetW}x${targetH}`,
          detail: `Crops ${w - cropW}px horizontally (${(((w - cropW) / w) * 100).toFixed(1)}% of width)`
        };
      } else {
        const cropH = Math.min(h, Math.max(1, Math.round(w / targetAspect)));
        const cropY = Math.floor((h - cropH) / 2);
        return {
          summary: `Center-crop ${w}x${cropH} (offset Y=${cropY}px) -> ${targetW}x${targetH}`,
          detail: `Crops ${h - cropH}px vertically (${(((h - cropH) / h) * 100).toFixed(1)}% of height)`
        };
      }
    }

    if (fitMode === 'fill') {
      if (srcAspect >= targetAspect) {
        const cropW = Math.min(w, Math.max(1, Math.round(h * targetAspect)));
        return {
          summary: `Source wider than canvas: behaves like Cover (crop ${cropW}x${h})`,
          detail: `No ambient side wings needed on ${targetW}x${targetH}`
        };
      }
      const fgW = Math.min(targetW, Math.max(1, Math.round(targetH * srcAspect)));
      const leftWing = Math.floor((targetW - fgW) / 2);
      const rightWing = targetW - leftWing - fgW;
      const feather = Math.min(this.FEATHER_PX, Math.floor(fgW / 2));
      return {
        summary: `Foreground ${fgW}x${targetH} centered | Wings: ${leftWing}px L / ${rightWing}px R`,
        detail: `Uncropped artwork + blurred 75% darkened backdrop (${feather}px edge feather)`
      };
    }

    // contain
    let scaledW = targetW;
    let scaledH = Math.round(targetW / srcAspect);
    if (scaledH > targetH) {
      scaledH = targetH;
      scaledW = Math.round(targetH * srcAspect);
    }
    const padX = Math.floor((targetW - scaledW) / 2);
    const padY = Math.floor((targetH - scaledH) / 2);
    return {
      summary: `Scaled ${scaledW}x${scaledH} centered inside ${targetW}x${targetH}`,
      detail: padX > 0 ? `Pillarbox bars: ${padX}px left & right` : (padY > 0 ? `Letterbox bars: ${padY}px top & bottom` : 'Exact aspect match (0px bars)')
    };
  },

  // Generates CLI command string
  generateCli(opts) {
    let cmd = `hl-background-cli "${opts.inputPath || 'wallpaper.png'}"`;
    const out = (opts.outputDir || './output').trim();
    if (out !== './output') {
      cmd += ` -o "${out}"`;
    }
    if (opts.fit !== 'cover') {
      cmd += ` --fit ${opts.fit}`;
    }
    if (opts.fit === 'contain' && opts.bgColor && opts.bgColor.toLowerCase() !== '#000000') {
      cmd += ` --bg-color "${opts.bgColor}"`;
    }
    if (opts.skipLayouts) {
      cmd += ' --skip-layouts';
    }
    if (opts.classicLayout) {
      cmd += ' --classic-layout';
    }
    if (opts.noLiblist) {
      cmd += ' --no-liblist';
    }
    if (opts.quiet) {
      cmd += ' -q';
    }
    return cmd;
  },

  // Generates Rust crate snippet
  generateRustSdk(opts) {
    const fitVariant = {
      cover: 'FitMode::Cover',
      fill: 'FitMode::Fill',
      contain: 'FitMode::Contain',
      stretch: 'FitMode::Stretch'
    }[opts.fit] || 'FitMode::Cover';

    const rgb = this.parseHexColor(opts.bgColor);
    const lines = [
      'use hl_background_cli::{generate, Config, FitMode, LayoutMode};',
      '',
      'fn main() -> Result<(), hl_background_cli::Error> {',
      `    let config = Config::new("${opts.inputPath || 'wallpaper.png'}", "${opts.outputDir || './output'}")`,
      `        .with_fit(${fitVariant})`
    ];

    if (opts.fit === 'contain' || (rgb[0] !== 0 || rgb[1] !== 0 || rgb[2] !== 0)) {
      lines.push(`        .with_bg_color([${rgb[0]}, ${rgb[1]}, ${rgb[2]}])`);
    }
    if (opts.skipLayouts) {
      lines.push('        .with_layouts(false)');
    } else if (opts.classicLayout) {
      lines.push('        .with_layout_mode(LayoutMode::Classic)');
    }
    if (!opts.noLiblist && !opts.skipLayouts) {
      lines.push('        .with_update_liblist(true)');
    }

    lines[lines.length - 1] += ';';
    lines.push('');
    lines.push('    let report = generate(&config)?;');
    lines.push('    println!("{} TGA tiles written", report.tiles_written);');
    lines.push('    Ok(())');
    lines.push('}');
    return lines.join('\n');
  },

  // Generates preview of the layout descriptor text
  generateLayoutPreview(opts) {
    if (opts.skipLayouts) {
      return '// --skip-layouts is enabled: no .txt layout descriptors will be written.\n// Only the 117 TGA slices in background/ will be generated.';
    }
    if (opts.classicLayout) {
      const classic = this.classicTiles();
      let txt = `// File: BackgroundLayout.txt & BackgroundLoadingLayout.txt (Classic 800x600 mode)\n// Note: HD_BackgroundLayout.txt still points to the 3840x1600 grid.\nresolution\t800\t600\n\n`;
      classic.forEach((t, idx) => {
        txt += `${t.layoutLine}\n`;
        if ((idx + 1) % 4 === 0 && idx + 1 < classic.length) txt += '\n';
      });
      return txt.trimEnd();
    }

    const hd = this.hdTiles();
    let txt = `// File: BackgroundLayout.txt & HD_BackgroundLayout.txt (Default HD 3840x1600 mode)\nresolution\t3840\t1600\n\n`;
    for (let i = 0; i < 8; i++) {
      txt += `${hd[i].layoutLine}\n`;
    }
    txt += `// ... (95 intermediate tile entries) ...\n`;
    txt += `${hd[103].layoutLine}\n`;
    txt += `${hd[104].layoutLine}`;
    return txt;
  }
};
