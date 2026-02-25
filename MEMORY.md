# Nova Project Documentation

## Overview
Nova is a terminal-based text editor built with Rust and Ratatui.

## Repository
- **URL**: https://github.com/Rehanasharmin/Nova
- **Token used**: Classic token (ghp_)

## Project Structure
```
Nova/
├── src/
│   ├── main.rs           # Main editor code
│   ├── buffer/           # Text buffer management
│   │   ├── buffer.rs
│   │   └── mod.rs
│   ├── config/           # Settings
│   │   ├── settings.rs
│   │   └── mod.rs
│   ├── syntax/           # Syntax highlighting
│   │   ├── highlight.rs
│   │   └── mod.rs
│   └── ui/               # UI components
│       ├── mod.rs
│       ├── theme.rs
│       └── widgets.rs
├── completions/          # Shell completions
│   ├── bash
│   └── zsh
├── Cargo.toml            # Project config
├── Cargo.lock            # Dependencies lock
├── README.md             # Documentation
├── install.sh            # Installation script
├── uninstall.sh          # Uninstall script
└── target/release/nova  # Binary (x86_64 Linux)
```

## Installation

### Quick Install
```bash
curl -sL https://raw.githubusercontent.com/Rehanasharmin/Nova/main/install.sh | bash
```

### Manual
```bash
git clone https://github.com/Rehanasharmin/Nova
cd Nova
./target/release/nova file.txt
```

## Features
- File open/create: `nova filename.txt`
- Multiple themes (monokai_pro, nord_frost, dracula_vibrant, gruvbox_soft, one_dark)
- Line numbers
- Cursor blinking
- Search (Ctrl+F)
- Undo/Redo (Ctrl+Z/Y)
- Save (Ctrl+S), Quit (Ctrl+Q)
- Toggle line numbers (Ctrl+B)
- Cycle themes (Ctrl+T)

## Keybindings
| Key | Action |
|-----|--------|
| Ctrl+S | Save |
| Ctrl+O | Open file |
| Ctrl+F | Search |
| Ctrl+Z | Undo |
| Ctrl+Y | Redo |
| Ctrl+T | Cycle theme |
| Ctrl+B | Toggle line numbers |
| Ctrl+Q | Quit |

## Dependencies
- ratatui = "0.30"
- crossterm = "0.28"
- serde = "1"
- toml = "0.8"
- dirs = "5"

## Configuration
Location: `~/.config/nova/config.toml`

```toml
tab_size = 4
use_spaces = true
show_line_numbers = true
highlight_current_line = true
word_wrap = false
auto_save = false
theme = "monokai_pro"
show_tabs = true
show_status_bar = true
show_help = true
mouse_support = true
```

## Platform Support
- Linux (x86_64, aarch64)
- macOS (x86_64, aarch64)
- Termux/Android (ARM64)
- Falls back to source compilation if no binary available

## Install Script Features
1. Detects OS (Linux, macOS, Termux)
2. Detects architecture (x86_64, aarch64, i686)
3. Downloads binary from GitHub raw URL
4. Falls back to source compilation if download fails
5. Creates config file
6. Adds to PATH
7. Installs shell completions

## Build from Source
```bash
cargo build --release
./target/release/nova
```

## Issues & Fixes
1. Binary was built for x86_64 - ARM64 users need to compile from source or get ARM64 binary
2. Install script now downloads from raw GitHub URL as fallback
3. Token: Initially used fine-grained token (github_pat_) - switched to classic token (ghp_)
