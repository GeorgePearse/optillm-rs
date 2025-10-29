# Installation

## Prerequisites

- **Rust 1.70+** - [Install Rust](https://www.rust-lang.org/tools/install)
- **Cargo** - Comes with Rust installation
- A modern C compiler (for some dependencies)

## Setup Steps

### 1. Clone the Repository

```bash
git clone https://github.com/coohom/optillm-rs.git
cd optillm-rs
```

### 2. Verify Installation

```bash
cargo --version  # Verify Rust is installed
rustc --version # Check Rust version (1.70+)
```

## Using as a Dependency

Add to your `Cargo.toml`:

```toml
[dependencies]
optillm-core = { path = "path/to/optillm-rs/crates/core" }
optillm-mars = { path = "path/to/optillm-rs/crates/mars" }
```

Or if published to crates.io:

```toml
[dependencies]
optillm-core = "0.1"
optillm-mars = "0.1"
```

## Building

### Build All Crates

```bash
cargo build --release
```

### Build Specific Crate

```bash
cargo build -p optillm-mars --release
cargo build -p optillm-core --release
```

### Development Build (Faster)

```bash
cargo build        # Debug build, faster compilation
cargo build --dev  # Explicit development profile
```

## Verification

Check that everything builds correctly:

```bash
cargo check --all
```

Run tests to verify functionality:

```bash
cargo test --all
```

## Troubleshooting

### Rust Version Issues

If you see errors about Rust version, update Rust:

```bash
rustup update
rustc --version  # Should be 1.70+
```

### Dependency Resolution

If you encounter dependency conflicts:

```bash
cargo update
cargo clean
cargo build
```

### Platform-Specific Issues

#### macOS with Apple Silicon
Some dependencies may need compilation from source. Install Xcode Command Line Tools:

```bash
xcode-select --install
```

#### Linux
Ensure you have the necessary build tools:

```bash
# Ubuntu/Debian
sudo apt-get install build-essential

# Fedora
sudo dnf install gcc gcc-c++ make
```

#### Windows
Visual Studio Build Tools or MinGW are required. The Rust installer usually handles this.

## Next Steps

- [Quick Start Guide](quick-start.md) - Get your first optimizer running
- [Examples](examples.md) - See practical code examples
- [Architecture Overview](../architecture/overview.md) - Understand the design

## Docker Setup (Optional)

For containerized development:

```bash
docker build -t optillm-rs .
docker run -it optillm-rs bash
```

## IDE Setup

### VS Code

1. Install [Rust Analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
2. Install [CodeLLDB](https://marketplace.visualstudio.com/items?itemName=vadimcn.vscode-lldb) for debugging
3. Create `.vscode/settings.json`:

```json
{
  "[rust]": {
    "editor.defaultFormatter": "rust-lang.rust-analyzer",
    "editor.formatOnSave": true
  }
}
```

### IntelliJ IDEA / CLion

1. Install the official [Rust plugin](https://www.jetbrains.com/help/idea/rust-support.html)
2. Configure Rust toolchain in IDE settings
3. Enable code inspections

## System Requirements

| Component | Requirement |
|-----------|-------------|
| Rust | 1.70+ |
| Cargo | Latest stable |
| RAM | 2GB minimum (4GB+ recommended) |
| Disk | 2GB for build artifacts |
| OS | Linux, macOS, Windows |

## Getting Help

- [GitHub Issues](https://github.com/coohom/optillm-rs/issues) - Report bugs or ask questions
- [Rust Community](https://www.rust-lang.org/community/) - General Rust help
- Check [FAQ](../faq.md) for common questions
