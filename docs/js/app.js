/**
 * Alpine.js Reactive Component for hl-background-cli Documentation & Interactive Playground
 */
document.addEventListener('alpine:init', () => {
  Alpine.data('hlBgApp', () => ({
    // Playground Controls State
    inputPath: 'wallpaper.png',
    outputDir: './valve/resource',
    fit: 'fill',
    bgColor: '#101824',
    skipLayouts: false,
    classicLayout: false,
    noLiblist: false,
    quiet: false,

    // Geometry Calculator State
    srcWidth: 1920,
    srcHeight: 1080,

    // Editor Tabs
    leftTab: 'config',
    outputTab: 'cli',

    // Interactive Tile Slicer Inspector State
    inspectorGrid: 'hd',
    showGridOverlay: true,
    hdTilesList: [],
    classicTilesList: [],
    selectedTile: null,

    // Calculated Outputs
    cliCommand: '',
    rustSnippet: '',
    layoutPreview: '',
    configJson: {},
    hdGeo: {},
    classicGeo: {},

    init() {
      if (window.HlBgEngine) {
        this.hdTilesList = window.HlBgEngine.hdTiles();
        this.classicTilesList = window.HlBgEngine.classicTiles();
        // Default selected tile: center Lambda tile (row 4, col h -> index 52)
        this.selectedTile = this.hdTilesList[52];
      }
      this.update();
    },

    setInspectorGrid(grid) {
      this.inspectorGrid = grid;
      if (grid === 'hd') {
        this.selectedTile = this.hdTilesList[52] || this.hdTilesList[0];
      } else {
        this.selectedTile = this.classicTilesList[5] || this.classicTilesList[0];
      }
    },

    selectTile(tile) {
      this.selectedTile = tile;
    },

    selectPresetResolution(w, h) {
      this.srcWidth = w;
      this.srcHeight = h;
      this.update();
    },

    update() {
      if (!window.HlBgEngine) return;

      this.cliCommand = window.HlBgEngine.generateCli(this);
      this.rustSnippet = window.HlBgEngine.generateRustSdk(this);
      this.layoutPreview = window.HlBgEngine.generateLayoutPreview(this);

      this.hdGeo = window.HlBgEngine.calcCanvasGeometry(
        this.srcWidth,
        this.srcHeight,
        3840,
        1600,
        this.fit
      );
      this.classicGeo = window.HlBgEngine.calcCanvasGeometry(
        this.srcWidth,
        this.srcHeight,
        800,
        600,
        this.fit
      );

      const rgb = window.HlBgEngine.parseHexColor(this.bgColor);
      const outClean = (this.outputDir || './output').replace(/[\\/]+$/, '');
      const bgDir = outClean.endsWith('background') ? outClean : `${outClean}/background`;

      this.configJson = {
        input: this.inputPath || 'wallpaper.png',
        tiles_dir: bgDir,
        fit_mode: this.fit,
        bg_color_rgb: rgb,
        write_layouts: !this.skipLayouts,
        layout_mode: this.classicLayout ? 'Classic (800x600)' : 'Hd (3840x1600)',
        update_liblist: !this.noLiblist && !this.skipLayouts,
        output_summary: {
          hd_tiles: 105,
          classic_tiles: 12,
          total_tga_files: 117,
          layout_txt_files: this.skipLayouts ? 0 : 4,
          total_disk_bytes: 19860708
        }
      };
    }
  }));
});
