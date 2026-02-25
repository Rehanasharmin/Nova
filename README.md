# Nova

A terminal-based text editor built with Rust and Ratatui.

## Installation

```bash
curl -sL https://raw.githubusercontent.com/YOUR_USERNAME/nova/main/install.sh | bash
```

## Usage

```bash
nova              # Open empty buffer
nova file.txt    # Open existing file or create new one
```

## Developer Setup

### Prerequisites
- Rust 1.70+
- Cargo

### Source Installation

```bash
git clone https://github.com/YOUR_USERNAME/nova.git
cd nova
cargo build --release
./target/release/nova
```

### Architecture

Nova uses a line-based buffer model with direct terminal rendering via Crossterm. The editor state is managed through an event loop that processes key events and updates the UI accordingly. Configuration is loaded from `~/.config/nova/config.toml` on startup.

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

## Configuration

Config file: `~/.config/nova/config.toml`

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

## Themes

- monokai_pro
- nord_frost
- dracula_vibrant
- gruvbox_soft
- one_dark
