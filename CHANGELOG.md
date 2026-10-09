# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-10-08

### Added
- Initial release of `hl-bg` CLI and `hl_bg` Rust library.
- Dedicated Truevision TGA 2.0 uncompressed BGR encoder with bottom-to-top scanlines.
- Support for Half-Life 25th Anniversary HD Ultrawide (3840×1600, 105 tiles) layout.
- Support for Classic 4:3 (800×600, 12 tiles) layout.
- Generation of `BackgroundLayout.txt`, `HD_BackgroundLayout.txt`, `BackgroundLoadingLayout.txt`, and `HD_BackgroundLoadingLayout.txt`.
- Multi-threaded tile generation with Rayon.
- Fit modes: `cover`, `contain` (with custom padding color), and `stretch`.
- Separate source image options for HD and Classic resolutions (`--hd-image`, `--classic-image`).
- GitHub Actions baseline for multi-platform CI and releases.
