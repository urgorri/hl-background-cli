# Contributing to hl-bg

Thank you for your interest in contributing to `hl-bg`!

## Code Guidelines

- Keep dependencies minimal and pure Rust where possible.
- Ensure all TGA output adheres strictly to the Truevision TGA 2.0 uncompressed format with bottom-to-top scanlines.
- Format code before committing:
  ```bash
  cargo fmt --all
  ```
- Run linter and tests:
  ```bash
  cargo clippy --all-targets --all-features -- -D warnings
  cargo test --all-targets --all-features
  ```

## Submitting Pull Requests

1. Fork the repository and create your branch from `main`.
2. Ensure new features or bug fixes have corresponding unit or integration tests.
3. Open a pull request using the provided PR template.
