# Project Setup Guide

## Initial Setup Commands

```bash
# Initialize Rust project
cargo init --name gumol-viz-engine

# Initialize git repository
git init
git add .
git commit -m "Initial commit"

# Create project structure
mkdir -p src/{core,io,rendering,systems,camera,interaction,ui,export,utils}
mkdir -p assets/{shaders,fonts}
mkdir -p examples

# Create initial Cargo.toml with dependencies
```

## Cargo.toml Configuration

```toml
[package]
name = "gumol-viz-engine"
version = "0.1.0"
edition = "2021"
description = "A Rust-based visualization engine for Molecular Dynamics simulations"
authors = ["Your Name <you@example.com>"]
license = "MIT OR Apache-2.0"
repository = "https://github.com/yourusername/gumol-viz-engine"

[dependencies]
# Bevy game engine
bevy = { version = "0.15", features = ["dynamic_linking"] }

# UI overlay
bevy_egui = "0.29"

# 3D object picking
bevy_mod_picking = "0.21"

# Camera controls
bevy_panorbit_camera = "0.20"

# Parallel processing
rayon = "1.10"

# Linear algebra
nalgebra = "0.33"

# Fast hashing
rustc-hash = "2.0"

# Memory-mapped files
memmap2 = "0.9"

# Parser combinators
nom = "7.1"

# Binary file parsing
byteorder = "1.5"

# Zero-copy casting
bytemuck = "1.18"

# XML parsing (for mmCIF)
quick-xml = "0.36"

# Hash maps (faster than std)
ahash = "0.8"

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

# Error handling
thiserror = "1.0"
anyhow = "1.0"

# Serde for serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Video encoding (optional, feature-gated)
# ffmpeg-next = { version = "7.1", optional = true }

[dev-dependencies]
criterion = "0.5"      # Benchmarking
proptest = "1.5"       # Property-based testing

[features]
default = ["video"]
video = ["ffmpeg-next"]
vr = ["bevy/openxr"]

# Optimize for development (faster compile times)
[profile.dev]
opt-level = 1

# Optimize for dependencies in dev builds
[profile.dev.package."*"]
opt-level = 3

# Optimize for release
[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
strip = true

[[bin]]
name = "gumol-viz"
path = "src/main.rs"

[[example]]
name = "basic_load"
path = "examples/basic_load.rs"

[[example]]
name = "timeline_demo"
path = "examples/timeline_demo.rs"

[[example]]
name = "interactive_selection"
path = "examples/interactive_selection.rs"

[[bench]]
name = "parsing"
harness = false
```

## Development Environment Setup

### Prerequisites

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add nightly toolchain (for some Bevy features)
rustup install nightly
rustup default stable

# Install useful tools
cargo install cargo-watch      # Watch for changes and rebuild
cargo install cargo-edit        # Cargo command for adding dependencies
cargo install cargo-expand      # Expand macros
cargo install cargo-audit       # Security audit
cargo install cargo-outdated    # Check for outdated dependencies
cargo install cargo-binstall    # Binary installer
cargo install flamegraph        # Profiling

# Install system dependencies (Ubuntu/Debian)
sudo apt update
sudo apt install build-essential libssl-dev pkg-config cmake

# Install FFmpeg (for video export)
sudo apt install ffmpeg

# For VR support (optional)
sudo apt install libopenxr-loader-dev
```

### VS Code Setup

```json
// .vscode/settings.json
{
  "rust-analyzer.cargo.loadOutDirsFromCheck": true,
  "rust-analyzer.cargo.features": "all",
  "rust-analyzer.cargo.runBuildScripts": true,
  "rust-analyzer.linkedProjects": ["Cargo.toml"],
  "rust-analyzer.checkOnSave.command": "clippy",
  "rust-analyzer.completion.addCallArgumentSnippets": true,
  "rust-analyzer.completion.addCallParenthesis": true,
  "rust-analyzer.inlayHints.enable": true,
  "files.associations": {
    "*.rs": "rust"
  },
  "editor.formatOnSave": true,
  "rust-analyzer.rustfmt.overrideCommand": ["rustfmt", "+nightly"]
}
```

### VS Code Extensions

```json
// .vscode/extensions.json
{
  "recommendations": [
    "rust-lang.rust-analyzer",
    "tamasfe.even-better-toml",
    "serayuzgur.crates",
    "usernamehw.errorlens",
    "vadimcn.vscode-lldb"
  ]
}
```

## Quick Start

After setting up the project:

```bash
# Build the project
cargo build --release

# Run the example
cargo run --example basic_load

# Watch for changes and rebuild
cargo watch -x run

# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run

# Build documentation
cargo doc --open

# Format code
cargo fmt

# Run linter
cargo clippy -- -D warnings

# Check for security vulnerabilities
cargo audit
```

## Project Structure After Setup

```
gumol-viz-engine/
├── .vscode/
│   ├── settings.json
│   └── extensions.json
├── assets/
│   ├── shaders/
│   └── fonts/
├── examples/
├── src/
│   ├── core/
│   │   └── mod.rs
│   ├── io/
│   │   └── mod.rs
│   ├── rendering/
│   │   └── mod.rs
│   ├── systems/
│   │   └── mod.rs
│   ├── camera/
│   │   └── mod.rs
│   ├── interaction/
│   │   └── mod.rs
│   ├── ui/
│   │   └── mod.rs
│   ├── export/
│   │   └── mod.rs
│   ├── utils/
│   │   └── mod.rs
│   ├── main.rs
│   └── lib.rs
├── benches/
├── tests/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── DEVELOPMENT_PLAN.md
├── ARCHITECTURE.md
└── .gitignore
```

## .gitignore

```gitignore
# Rust
/target/
**/*.rs.bk
*.pdb
Cargo.lock

# IDE
.vscode/
.idea/
*.swp
*.swo
*~

# OS
.DS_Store
Thumbs.db

# Build artifacts
*.o
*.so
*.dylib

# Testing
*.profraw
*.profdata

# Logs
*.log

# User-specific files
.env
.env.local

# Assets (optional)
# assets/*
# !assets/.gitkeep
```

## Initial Commit Strategy

```bash
# Commit 1: Project setup
git add Cargo.toml .gitignore
git commit -m "chore: initialize Cargo project"

# Commit 2: Directory structure
git add src/ assets/ examples/ tests/ benches/
git commit -m "chore: create project directory structure"

# Commit 3: VS Code config
git add .vscode/
git commit -m "chore: add VS Code configuration"

# Commit 4: Documentation
git add DEVELOPMENT_PLAN.md ARCHITECTURE.md README.md
git commit -m "docs: add planning and architecture documentation"
```

## Development Workflow

```bash
# Feature branch workflow
git checkout -b feature/xyz-parser
# ... work on feature ...
git add .
git commit -m "feat: implement XYZ file parser"
git push -u origin feature/xyz-parser
# Create pull request

# Run checks before pushing
cargo fmt
cargo clippy -- -D warnings
cargo test
cargo build --release
```

## Performance Profiling

```bash
# Flamegraph profiling
cargo flamegraph --example basic_load

# Criterion benchmarks
cargo bench

# Time compilation
cargo build --timings
```

## Continuous Integration (GitHub Actions)

```yaml
# .github/workflows/ci.yml
name: CI

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rs/toolchain@v1
        with:
          profile: minimal
          toolchain: stable
          components: rustfmt, clippy
      - name: Install dependencies
        run: sudo apt-get install libssl-dev pkg-config cmake ffmpeg
      - name: Check formatting
        run: cargo fmt -- --check
      - name: Run clippy
        run: cargo clippy -- -D warnings
      - name: Run tests
        run: cargo test --all-features
      - name: Build release
        run: cargo build --release --all-features
```

## Next Steps After Setup

1. ✅ Initialize the project structure
2. ✅ Set up dependencies in Cargo.toml
3. ✅ Configure development environment
4. ✅ Create initial module files with stubs
5. ⏭️ Implement XYZ parser (Phase 1)
6. ⏭️ Implement basic atom rendering
7. ⏭️ Add camera controls
8. ⏭️ Implement PDB parser
9. ⏭️ Build timeline system
10. ⏭️ Add interaction and selection
