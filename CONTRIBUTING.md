# Contributing to boba

Thank you for your interest in contributing to boba. This document covers the
basics of getting started, building the project, and submitting changes.

## Getting Started

boba is organized as a Cargo workspace with three crates:

| Crate | Path | Purpose |
|---|---|---|
| `boba` | `crates/boba` | Umbrella crate that re-exports core, widgets, ratatui, and crossterm |
| `boba-core` | `crates/boba-core` | Runtime, `Model` and `Component` traits, `Command`, `Subscription`, `TestProgram` |
| `boba-widgets` | `crates/boba-widgets` | Reusable widget components (text input, list, table, spinner, etc.) |

## Development Setup

You need a stable Rust toolchain (MSRV 1.75). Then:

```bash
# Clone the repository
git clone https://github.com/TODO/boba.git
cd boba

# Build the entire workspace
cargo build

# Run all tests
cargo test --workspace

# Run clippy lints
cargo clippy --workspace --all-targets -- -D warnings

# Check formatting
cargo fmt --all -- --check
```

## Running Examples

Examples live in the top-level `examples/` directory and are compiled through
the `boba` umbrella crate:

```bash
cargo run --example counter
cargo run --example input_form
cargo run --example async_http
cargo run --example full_app
cargo run --example file_browser
```

## Code Style

- Follow the patterns already established in the codebase.
- Run `cargo fmt` before committing.
- Ensure `cargo clippy --workspace --all-targets -- -D warnings` passes with no warnings.
- Write doc comments for all public items.
- Add tests for new functionality. Use `TestProgram` for model-level tests where possible.

## Adding a New Widget

1. Create a new module in `crates/boba-widgets/src/`.
2. Implement the `Component` trait from `boba-core`.
3. Re-export the module from `crates/boba-widgets/src/lib.rs`.
4. Add an example or extend an existing one to demonstrate the widget.
5. Add a row to the widget table in `README.md`.

## Pull Request Process

1. Fork the repository and create a feature branch from `main`.
2. Make your changes in small, focused commits.
3. Ensure all tests pass (`cargo test --workspace`).
4. Ensure clippy and fmt checks pass.
5. Open a pull request against `main` with a clear description of the change and its motivation.
6. Address any review feedback.

## Reporting Issues

Open an issue on GitHub. Include:

- A clear description of the problem or feature request.
- Steps to reproduce (for bugs).
- Your Rust version (`rustc --version`) and operating system.

## License

By contributing to boba, you agree that your contributions will be licensed
under the MIT license.
