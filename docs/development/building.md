# Building optillm-rs

Guide for building the optillm-rs project from source.

## Prerequisites

- Rust 1.75+ ([install](https://rustup.rs/))
- Cargo
- Git

## Installation

```bash
# Clone the repository
git clone https://github.com/coohom/optillm-rs.git
cd optillm-rs

# Install Rust (if needed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Building

```bash
# Build all crates in release mode
cargo build --release

# Build specific crate
cargo build --release -p optillm-mars

# Build for development (faster, unoptimized)
cargo build

# Check code without building (faster)
cargo check --all
```

## Running Tests

```bash
# Run all tests
cargo test --all

# Run tests for specific crate
cargo test -p optillm-core

# Run with output
cargo test -- --nocapture

# Run single test
cargo test test_name --
```

## Features

```bash
# Build with all features
cargo build --all-features

# Build with specific features
cargo build --features "feature1,feature2"
```

## Documentation

```bash
# Generate and open documentation
cargo doc --all --open

# Generate without opening
cargo doc --all
```

## Workspace Structure

```
optillm-rs/
├── crates/
│   ├── core/          # Core traits and types
│   └── mars/          # MARS implementation
├── Cargo.toml         # Workspace configuration
└── Cargo.lock         # Locked dependencies
```

## Dependency Management

```bash
# Update dependencies
cargo update

# Check for outdated dependencies
cargo outdated

# Verify dependency tree
cargo tree
```

## Troubleshooting

### Clean Build

```bash
# Remove build artifacts
cargo clean

# Rebuild from scratch
cargo build --release
```

### Compiler Errors

```bash
# Update Rust
rustup update

# Check Rust version
rustc --version
```

### Dependency Issues

```bash
# Fetch latest dependencies
cargo update

# Lock to specific version
# Edit Cargo.toml version constraints
```

## Performance

```bash
# Optimize for performance
cargo build --release -C opt-level=3

# Profile build time
cargo build -Z timings
```

## Platform-Specific

### macOS

```bash
# Ensure Xcode Command Line Tools installed
xcode-select --install

cargo build --release
```

### Linux

```bash
# Install build tools (Ubuntu/Debian)
sudo apt-get install build-essential

cargo build --release
```

### Windows

```bash
# Use Visual Studio Build Tools
# Then build normally
cargo build --release
```

See [Testing](testing.md) for detailed testing instructions.
