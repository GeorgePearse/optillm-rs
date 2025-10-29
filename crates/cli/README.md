# OptimLLM CLI

A command-line interface for integrating OptimLLM optimization strategies into any LLM-powered system.

## Quick Start

### Installation

#### Using Cargo
```bash
cargo install --path .
# or
cargo binstall optillm-cli
```

#### Using Pre-built Binaries
Download from [GitHub Releases](https://github.com/GeorgePearse/optillm-rs/releases):

```bash
# Linux
wget https://github.com/GeorgePearse/optillm-rs/releases/download/v0.1.0/optillm-linux-x86_64
chmod +x optillm-linux-x86_64
./optillm-linux-x86_64 --help

# macOS
wget https://github.com/GeorgePearse/optillm-rs/releases/download/v0.1.0/optillm-macos-x86_64
chmod +x optillm-macos-x86_64
./optillm-macos-x86_64 --help

# Windows
# Download optillm-windows-x86_64.exe
optillm-windows-x86_64.exe --help
```

## Usage

### View Available Strategies
```bash
optillm strategies
```

### Run a Strategy

#### AutoThink (Complexity-Aware Reasoning)
```bash
optillm autothink \
  --query "What is the capital of France?" \
  --system "You are a helpful assistant" \
  --simple-threshold 0.25 \
  --complex-threshold 0.40
```

#### Deep Thinking (Difficulty-Based Token Allocation)
```bash
optillm deep-thinking \
  --query "Solve this complex algorithm problem" \
  --system "You are an expert programmer" \
  --min-tokens 256 \
  --max-tokens 2048 \
  --iterations 3
```

#### Entropy Decoding (Diversity Control)
```bash
optillm entropy-decoding \
  --query "Generate creative ideas for a product" \
  --system "You are a product designer" \
  --target-entropy 0.8 \
  --num-samples 5
```

#### CoT Decoding (Structured Reasoning)
```bash
optillm cot-decoding \
  --query "Prove that the sum of angles in a triangle is 180 degrees" \
  --system "You are a mathematics tutor" \
  --steps 4 \
  --verify
```

#### R* Algorithm (Enhanced Tree Search)
```bash
optillm r-star \
  --query "Find the optimal solution to this problem" \
  --system "You are an expert problem solver" \
  --simulations 10 \
  --exploration 1.414 \
  --candidates 3
```

#### MARS (Multi-Agent Reasoning)
```bash
optillm mars \
  --query "Analyze this situation from multiple perspectives" \
  --system "You are an expert analyst" \
  --num-agents 3
```

## Integration with Your Coding CLI

### Rust Integration

Add to your `Cargo.toml`:

```toml
[dependencies]
optillm-core = { path = "../optillm-rs/crates/core" }
optillm-mars = { path = "../optillm-rs/crates/mars" }
```

Use in your code:

```rust
use optillm_mars::strategies::AutoThinkAggregator;
use optillm_core::ModelClient;

// Implement ModelClient for your LLM provider
// Then use strategies directly in your code
```

### Shell Integration

Pipe the CLI output to your system:

```bash
# Execute and capture output
RESULT=$(optillm autothink --query "$QUERY" --system "$SYSTEM_PROMPT")

# Use in your CLI workflow
echo "Optimized response: $RESULT"
```

### Python Integration

```python
import subprocess
import json

def run_optillm_strategy(strategy, query, **kwargs):
    """Run an OptimLLM strategy from Python"""
    cmd = ["optillm", strategy, "--query", query]

    for key, value in kwargs.items():
        cmd.append(f"--{key}")
        cmd.append(str(value))

    result = subprocess.run(cmd, capture_output=True, text=True)
    return result.stdout

# Usage
output = run_optillm_strategy(
    "autothink",
    "What is the capital of France?",
    system="You are helpful"
)
print(output)
```

## Global Options

```bash
# Verbose logging (can be repeated: -vv, -vvv for more detail)
optillm -v <COMMAND>

# Disable colored output
optillm --no-color <COMMAND>

# Use config file
optillm --config ~/.optillm/config.json <COMMAND>

# Set config file via environment variable
export OPTILLM_CONFIG=~/.optillm/config.json
optillm <COMMAND>
```

## Configuration File

Create `~/.optillm/config.json`:

```json
{
  "api_key": "your-api-key",
  "model": "gpt-4",
  "api_base": "https://api.openai.com/v1",
  "system_prompt": "You are a helpful assistant",
  "timeout": 60
}
```

## Building from Source

```bash
# Clone the repository
git clone https://github.com/GeorgePearse/optillm-rs.git
cd optillm-rs

# Build the CLI
cargo build --release -p optillm-cli

# Run the binary
./target/release/optillm --help

# Install system-wide
cargo install --path crates/cli
```

## Strategies Overview

| Strategy | Best For | Usage |
|----------|----------|-------|
| **AutoThink** | Adaptive complexity-based reasoning | Simple to complex queries |
| **Deep Thinking** | Allocating tokens based on difficulty | Variable-complexity problems |
| **Entropy Decoding** | Controlling diversity in outputs | Creative tasks, brainstorming |
| **CoT Decoding** | Structured step-by-step reasoning | Math, logic, proofs |
| **R* Algorithm** | Intelligent solution space exploration | Complex optimization problems |
| **MARS** | Multi-perspective analysis | Analysis, planning, verification |

## Performance

Typical performance on modern systems:
- Binary size: ~3.6 MB (release build)
- Startup time: <100ms
- Memory usage: Minimal (depends on strategy)

## Extending the CLI

To add a new strategy command:

1. Create a new module in `src/commands/`:
   ```rust
   // src/commands/my_strategy.rs
   pub async fn run(query: String, system: String) -> CliResult<()> {
       // Implementation
       Ok(())
   }
   ```

2. Add to `src/commands/mod.rs`:
   ```rust
   pub mod my_strategy;
   ```

3. Add variant to `Commands` enum in `src/main.rs`:
   ```rust
   /// Run MyStrategy
   MyStrategy {
       #[arg(short, long)]
       query: String,
       // ... more args
   },
   ```

4. Add handler in the match statement

## Troubleshooting

### Binary not found
Ensure the installation directory is in your `PATH`:
```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

### Permission denied
Make the binary executable:
```bash
chmod +x optillm
```

### API key issues
Set environment variables:
```bash
export OPTILLM_API_KEY="your-key"
export OPTILLM_MODEL="gpt-4"
```

## Contributing

Contributions welcome! See [CONTRIBUTING.md](../../CONTRIBUTING.md)

## License

MIT

## Resources

- [OptimLLM Documentation](../../docs/)
- [GitHub Repository](https://github.com/GeorgePearse/optillm-rs)
- [Crates.io Package](https://crates.io/crates/optillm-cli)
