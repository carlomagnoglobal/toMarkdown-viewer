# toMarkdown-viewer

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/Rust-1.88+-ce422b?logo=rust)](https://www.rust-lang.org/)
[![Node.js](https://img.shields.io/badge/Node.js-18+-339933?logo=node.js)](https://nodejs.org/)

Desktop Markdown vault viewer for [toMarkdownMCP](https://github.com/carlomagnoglobal/toMarkdownMCP).

A cross-platform Tauri application that provides a beautiful, feature-rich viewer for exploring Markdown vaults with advanced features including:

- **Multi-tab interface** with keyboard shortcuts and navigation
- **Recycle bin** for safe file deletion with configurable retention
- **Clipboard operations** with multiple encoding formats (Base64, Hex, Markdown)
- **Image zoom** with pan capability (1x to 15x magnification)
- **Syntax highlighting** for code files
- **Vault indexing** with co-occurrence analysis and word graphs
- **Dark/Light/Sepia themes** with full keyboard navigation
- **Advanced file operations** including search, preview, and recent files

## Installation

### macOS
```bash
# Apple Silicon
wget https://github.com/carlomagnoglobal/toMarkdown-viewer/releases/download/gui-v0.6.1/toMarkdown.Viewer_0.6.1_aarch64.dmg
open toMarkdown.Viewer_0.6.1_aarch64.dmg

# Intel
wget https://github.com/carlomagnoglobal/toMarkdown-viewer/releases/download/gui-v0.6.1/toMarkdown.Viewer_0.6.1_x64.dmg
open toMarkdown.Viewer_0.6.1_x64.dmg
```
The app is unsigned, so macOS Gatekeeper will block it on first launch. Right-click
the app in Finder and choose **Open** to bypass this once.

### Windows
Download and run the installer from the
[latest release](https://github.com/carlomagnoglobal/toMarkdown-viewer/releases/latest):
`toMarkdown.Viewer_0.6.1_x64-setup.exe` (or the `.msi`).

### Linux
Download from the
[latest release](https://github.com/carlomagnoglobal/toMarkdown-viewer/releases/latest):
`.deb`, `.AppImage`, or `.rpm`, matching your distribution.

## Building from Source

### Requirements
- Rust 1.88+ (via [rustup](https://rustup.rs/))
- Node.js 18+
- Tauri CLI: `npm install -g @tauri-apps/cli@next`

### Build
```bash
git clone https://github.com/carlomagnoglobal/toMarkdown-viewer.git
cd toMarkdown-viewer

# Install dependencies
npm install

# Build desktop app
npm run tauri build
```

Binary will be in `src-tauri/target/release/bundle/`.

## Development

### Run in dev mode
```bash
npm run tauri dev
```

### Run tests
```bash
cargo test -p to_markdown_gui
```

### Generate release bundle
```bash
npm run tauri build --release
```

## Version Compatibility

| Viewer | toMarkdownMCP |
|--------|---------------|
| 0.6.1  | ≥0.1.0        |
| 0.6.0  | ≥0.1.0        |
| 0.5.1  | ≥0.1.0        |
| 0.5.0  | ≥0.1.0        |

## Architecture

- **Backend**: Rust (Tauri 2) with SQLite vault indexing
- **Frontend**: TypeScript + HTML/CSS with vanilla JS
- **Core Dependencies**: syntect, pulldown-cmark, tauri, rusqlite

## License

MIT — See [LICENSE](./LICENSE) file.

## Contributing

Contributions welcome! Please open issues for bugs or feature requests.

## Related Projects

- [toMarkdownMCP](https://github.com/carlomagnoglobal/toMarkdownMCP) — Markdown conversion toolkit (CLI + MCP server)
- [toMarkdownMCP on crates.io](https://crates.io/crates/to_markdown_mcp)
