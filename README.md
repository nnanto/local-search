# Local Search Engine

A fast, local search engine built in Rust with vector embeddings and SQLite storage.

## Features

- 🔍 Full-Text + Semantic Search using embeddings generated and stored locally
- 📁 Local file indexing and search
- 🗄️ SQLite-based storage
- 📚 Both library and CLI interfaces

# Installation Guide

## Quick Install

### Linux/macOS
```bash
curl -sSL https://raw.githubusercontent.com/nnanto/localsearch/main/scripts/install.sh | bash
```

### Windows (PowerShell)
```powershell
irm https://raw.githubusercontent.com/nnanto/localsearch/main/scripts/install.ps1 | iex
```

## Manual Installation

### Pre-built Binaries

Download the appropriate binary for your platform from the [latest release](https://github.com/nnanto/localsearch/releases/latest):

#### Linux (x86_64)
```bash
curl -L https://github.com/nnanto/localsearch/releases/latest/download/localsearch-linux-x86_64.tar.gz | tar xz
sudo mv localsearch /usr/local/bin/
```

#### macOS (Intel)
```bash
curl -L https://github.com/nnanto/localsearch/releases/latest/download/localsearch-macos-x86_64.tar.gz | tar xz
sudo mv localsearch /usr/local/bin/
```

#### macOS (Apple Silicon)
```bash
curl -L https://github.com/nnanto/localsearch/releases/latest/download/localsearch-macos-aarch64.tar.gz | tar xz
sudo mv localsearch /usr/local/bin/
```

#### Windows
1. Download [localsearch-windows-x86_64.zip](https://github.com/nnanto/localsearch/releases/latest/download/localsearch-windows-x86_64.zip)
2. Extract the ZIP file
3. Add the extracted directory to your PATH environment variable

### From Source

If you have Rust installed, you can build from source:

```bash
cargo install --git https://github.com/nnanto/localsearch --features cli
```

Or clone and build:

```bash
git clone https://github.com/nnanto/localsearch.git
cd localsearch
cargo build --release --features cli
sudo cp target/release/localsearch /usr/local/bin/
```

## Verify Installation

After installation, verify that the tool is working:

```bash
localsearch --help
```

You should see the help output for the localsearch CLI tool.

## Updating

To update to the latest version, simply re-run the installation command. The installer will replace the existing binary with the latest version.

## Uninstallation

### Linux/macOS
```bash
sudo rm /usr/local/bin/localsearch
```

### Windows
Remove the installation directory and update your PATH environment variable to remove the localsearch directory.

## Troubleshooting

### Permission Issues
If you get permission errors on Linux/macOS, make sure you're running the installation with appropriate permissions (using `sudo` when needed).

### Path Issues
If the `localsearch` command is not found after installation, make sure the installation directory is in your PATH:

- **Linux/macOS**: `/usr/local/bin` should be in your PATH
- **Windows**: The installation directory should be added to your PATH environment variable

### Antivirus False Positives
Some antivirus software may flag the binary as suspicious. This is a common issue with Rust binaries. You may need to add an exception for the localsearch binary.

# Usage

## CLI Usage

```bash
# Index documents
localsearch index /path/to/documents

# Search for content
localsearch search "your query here"
```

## Library Usage

```rust
use localsearch::{SqliteLocalSearchEngine, LocalEmbedder, DocumentIndexer, LocalSearch, SearchType, DocumentRequest};

fn main() -> anyhow::Result<()> {
    // Create embedder and search engine
    let embedder = LocalEmbedder::new_with_default_model()?;
    let mut engine = SqliteLocalSearchEngine::new("search.db", Some(embedder))?;

    // Index a document
    engine.insert_document(DocumentRequest {
        path: "some/unique/path".to_string(),
        content: "This is example content".to_string(),
        metadata: None,
    })?;

    // Search
    let results = engine.search("example", SearchType::Hybrid, Some(10))?;
    Ok(())
}
```

## Development

```bash
# Clone the repository
git clone https://github.com/nnanto/localsearch.git
cd localsearch

# Run tests
cargo test

# Run CLI with features
cargo run --features cli -- search "query"
```

## License

MIT License - see [LICENSE](LICENSE) file for details.