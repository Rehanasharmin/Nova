#!/bin/bash

set -e

GITHUB_REPO="Rehanasharmin/Nova"
INSTALL_DIR="$HOME/.local/bin"
CONFIG_DIR="$HOME/.config/nova"
VERSION_URL="https://api.github.com/repos/${GITHUB_REPO}/releases/latest"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

print_banner() {
    echo -e "${CYAN}"
    echo "   __   _     __         __  ___ "
    echo "  / | / /__  / /  ___   /  |/  /__  __  ___________ "
    echo " /  |/ / _ \\/ _ \\/ _ \\ / /|_/ / _ \\/ / / / ___/ "
    echo "/ /|  /  __/  __/  __// /  / /  __/ /_/ / /    "
    echo "/_/ |_/\\___/\\___/\\___//_/  /_/\\___/\\__,_/_/     ${NC}"
    echo ""
}

print_status() {
    echo -e "${BLUE}[${BOLD}*${NC}${BLUE}]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[${BOLD}✓${NC}${GREEN}]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[${BOLD}!${NC}${YELLOW}]${NC} $1"
}

print_error() {
    echo -e "${RED}[${BOLD}✗${NC}${RED}]${NC} $1"
}

spinner() {
    local pid=$1
    local delay=0.1
    local spinstr='|/-\'
    while kill -0 $pid 2>/dev/null; do
        printf "\r${BLUE}[${BOLD}*${NC}${BLUE}]${NC} $2 ${spinstr:0:1}"
        spinstr=${spinstr:1}${spinstr:0:1}
        sleep $delay
    done
    printf "\r${GREEN}[${BOLD}✓${NC}${GREEN}]${NC} $2\n"
}

detect_os() {
    case "$(uname -s)" in
        Linux*)     
            if [ -d "/data/data/com.termux" ]; then
                os="termux"
            else
                os="linux"
            fi
            ;;
        Darwin*)    os="macos";;
        *)          echo -e "${RED}Unsupported OS: $(uname -s)${NC}"; exit 1;;
    esac
}

detect_arch() {
    case "$(uname -m)" in
        x86_64)     arch="x86_64";;
        aarch64|arm64) 
            if [ "$os" = "termux" ]; then
                arch="aarch64"
            else
                arch="aarch64"
            fi
            ;;
        i386|i686)  arch="i686";;
        *)          echo -e "${RED}Unsupported architecture: $(uname -m)${NC}"; exit 1;;
    esac
}

detect_extension() {
    case "$os" in
        windows) ext=".exe";;
        *)       ext="";;
    esac
}

compile_from_source() {
    print_status "Checking for Rust..."
    
    if command -v cargo &> /dev/null; then
        print_status "Rust already installed"
    else
        print_status "Installing Rust..."
        if [ "$os" = "termux" ]; then
            pkg install -y rust clang 2>/dev/null || true
        elif [ "$os" = "linux" ]; then
            if command -v curl &> /dev/null; then
                curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y 2>/dev/null || true
                if [ -f "$HOME/.cargo/env" ]; then
                    source "$HOME/.cargo/env"
                fi
            fi
        elif [ "$os" = "macos" ]; then
            if command -v brew &> /dev/null; then
                brew install rustup 2>/dev/null || true
                rustup default stable 2>/dev/null || true
            fi
        fi
    fi
    
    if ! command -v cargo &> /dev/null; then
        print_error "Failed to install Rust. Please install manually: https://rustup.rs"
        exit 1
    fi
    
    if [ -f "$HOME/.cargo/env" ]; then
        source "$HOME/.cargo/env"
    fi
    
    print_status "Configuring cargo network settings..."
    mkdir -p "$HOME/.cargo"
    cat > "$HOME/.cargo/config.toml" << 'CARGOEOF'
[net]
retry = 5
git-fetch-with-cli = true

[http]
timeout = 300

[registries.crates-io]
protocol = "sparse"
CARGOEOF
    
    print_status "Installing build dependencies..."
    if [ "$os" = "termux" ]; then
        pkg install -y clang 2>/dev/null || true
    elif [ "$os" = "linux" ]; then
        if command -v apt-get &> /dev/null; then
            sudo apt-get install -y clang 2>/dev/null || true
        elif command -v dnf &> /dev/null; then
            sudo dnf install -y clang 2>/dev/null || true
        elif command -v pacman &> /dev/null; then
            sudo pacman -S --noconfirm clang 2>/dev/null || true
        fi
    elif [ "$os" = "macos" ]; then
        if command -v brew &> /dev/null; then
            brew install clang 2>/dev/null || true
        fi
    fi

    print_status "Cloning repository..."
    TEMP_DIR=$(mktemp -d)
    if ! git clone --depth 1 "https://github.com/${GITHUB_REPO}.git" "$TEMP_DIR/nova" 2>/dev/null; then
        print_error "Failed to clone repository"
        rm -rf "$TEMP_DIR"
        exit 1
    fi
    
    if [ -f "$HOME/.cargo/env" ]; then
        source "$HOME/.cargo/env"
    fi
    
    print_status "Building nova (this may take a few minutes)..."
    cd "$TEMP_DIR/nova"
    if ! cargo build --release 2>&1; then
        print_error "Build failed. Error output:"
        cargo build --release 2>&1 | tail -20
        rm -rf "$TEMP_DIR"
        exit 1
    fi

    if [ -z "$INSTALL_DIR" ]; then
        INSTALL_DIR="$HOME/.local/bin"
    fi
    mkdir -p "$INSTALL_DIR"

    if [ -f "$TEMP_DIR/nova/target/release/nova" ]; then
        cp "$TEMP_DIR/nova/target/release/nova" "$INSTALL_DIR/nova"
        chmod +x "$INSTALL_DIR/nova"
        print_success "Compiled and installed to $INSTALL_DIR/nova"
    else
        print_error "Build failed - binary not found"
        rm -rf "$TEMP_DIR"
        exit 1
    fi
    rm -rf "$TEMP_DIR"
}

print_banner
print_status "Detecting system..."

detect_os
detect_arch

if [ "$os" = "termux" ]; then
    INSTALL_DIR="$PREFIX/bin"
    CONFIG_DIR="$HOME/.config/nova"
fi

print_status "Operating system: $os"
print_status "Architecture: $arch"

print_status "Fetching latest release info..."
VERSION_DATA=$(curl -s "$VERSION_URL")

if [ $? -ne 0 ]; then
    print_error "Failed to fetch release info. Trying alternative..."
fi

DOWNLOAD_URL=""
if [ -n "$VERSION_DATA" ]; then
    DOWNLOAD_URL=$(echo "$VERSION_DATA" | grep -o "browser_download_url.*nova-${os}-${arch}[^\"]*" | head -1 | sed 's/browser_download_url.*"//' | tr -d '"')
fi

if [ "$os" = "termux" ]; then
    INSTALL_DIR="$PREFIX/bin"
    print_status "Termux detected - will compile from source..."
    compile_from_source
else
    if [ -n "$DOWNLOAD_URL" ]; then
        mkdir -p "$INSTALL_DIR"
        print_status "Downloading nova..."
        (
            curl -sL --progress-bar "$DOWNLOAD_URL" -o "$INSTALL_DIR/nova"
        ) &
        spinner $! "Downloading nova"
        chmod +x "$INSTALL_DIR/nova"
        if [ -f "$INSTALL_DIR/nova" ]; then
            print_success "Binary installed to $INSTALL_DIR/nova"
        else
            print_warning "Download failed, compiling from source..."
            compile_from_source
        fi
    else
        print_warning "No pre-built binary found, compiling from source..."
        INSTALL_DIR="$HOME/.local/bin"
        mkdir -p "$INSTALL_DIR"
        compile_from_source
    fi
fi

mkdir -p "$CONFIG_DIR"

if [ ! -f "$CONFIG_DIR/config.toml" ]; then
    print_status "Creating default configuration..."
    cat > "$CONFIG_DIR/config.toml" << 'EOF'
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
EOF
    print_success "Configuration created at $CONFIG_DIR/config.toml"
fi

SHELL_RC=""
if [ "$os" = "termux" ]; then
    if [ -f "$HOME/.bashrc" ]; then
        SHELL_RC="$HOME/.bashrc"
    elif [ -f "$HOME/.zshrc" ]; then
        SHELL_RC="$HOME/.zshrc"
    else
        SHELL_RC="$HOME/.bashrc"
        touch "$SHELL_RC"
    fi
elif [ -f "$HOME/.bashrc" ]; then
    SHELL_RC="$HOME/.bashrc"
elif [ -f "$HOME/.bash_profile" ]; then
    SHELL_RC="$HOME/.bash_profile"
elif [ -f "$HOME/.zshrc" ]; then
    SHELL_RC="$HOME/.zshrc"
fi

if [ -n "$SHELL_RC" ]; then
    if ! grep -q "$INSTALL_DIR" "$SHELL_RC"; then
        print_status "Adding to PATH in $SHELL_RC..."
        echo "" >> "$SHELL_RC"
        echo "# Nova Editor" >> "$SHELL_RC"
        echo "export PATH=\"$INSTALL_DIR:\$PATH\"" >> "$SHELL_RC"
        print_success "PATH updated. Run 'source $SHELL_RC' or restart your terminal."
    fi
fi

print_status "Installing shell completions..."
COMPLETION_DIR="$HOME/.local/share/zsh/site-functions"
mkdir -p "$COMPLETION_DIR"
if [ -f "$(dirname "$0")/completions/zsh" ]; then
    cp "$(dirname "$0")/completions/zsh" "$COMPLETION_DIR/_nova" 2>/dev/null || \
    curl -sL "https://raw.githubusercontent.com/${GITHUB_REPO}/main/completions/zsh" -o "$COMPLETION_DIR/_nova"
    print_success "Zsh completion installed"
fi

BASH_COMPLETION_DIR="$HOME/.local/bash_completion.d"
mkdir -p "$BASH_COMPLETION_DIR"
if [ -f "$(dirname "$0")/completions/bash" ]; then
    cp "$(dirname "$0")/completions/bash" "$BASH_COMPLETION_DIR/nova" 2>/dev/null || \
    curl -sL "https://raw.githubusercontent.com/${GITHUB_REPO}/main/completions/bash" -o "$BASH_COMPLETION_DIR/nova"
    print_success "Bash completion installed"
fi

if command -v nova &> /dev/null; then
    print_success "nova is now available in your PATH!"
else
    print_warning " nova not in current PATH. Run: export PATH=\"$INSTALL_DIR:\$PATH\""
fi

echo ""
echo -e "${GREEN}${BOLD}╔═══════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}${BOLD}║         Installation completed successfully!       ║${NC}"
echo -e "${GREEN}${BOLD}╚═══════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "  ${BOLD}Run nova:${NC}  nova"
echo ""
