#!/bin/bash

set -e

GITHUB_REPO="YOUR_USERNAME/nova"
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

print_banner
print_status "Detecting system..."

detect_os
detect_arch

print_status "Operating system: $os"
print_status "Architecture: $arch"

print_status "Fetching latest release info..."
VERSION_DATA=$(curl -s "$VERSION_URL")

if [ $? -ne 0 ]; then
    print_error "Failed to fetch release info. Check your internet connection."
    exit 1
fi

DOWNLOAD_URL=$(echo "$VERSION_DATA" | grep -o "browser_download_url.*nova-${os}-${arch}[^\"]*" | head -1 | sed 's/browser_download_url.*"//' | tr -d '"')

if [ "$os" = "termux" ]; then
    INSTALL_DIR="$PREFIX/bin"
fi

mkdir -p "$INSTALL_DIR"

if [ -z "$DOWNLOAD_URL" ]; then
    print_warning "No pre-built binary for $os-$arch"
    echo ""
    read -p "Compile from source instead? [y/N] " -n 1 -r
    echo ""
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        print_status "Installing build dependencies..."
        if [ "$os" = "termux" ]; then
            pkg install -y rust clang make
        else
            if command -v apt &> /dev/null; then
                sudo apt install -y rustc cargo clang
            elif command -v pacman &> /dev/null; then
                sudo pacman -S --noconfirm rust clang
            elif command -v dnf &> /dev/null; then
                sudo dnf install -y rust clang
            fi
        fi
        print_status "Building nova (this may take a few minutes)..."
        TEMP_DIR=$(mktemp -d)
        git clone --depth 1 "https://github.com/${GITHUB_REPO}.git" "$TEMP_DIR/nova" 2>/dev/null || \
        curl -sL "https://github.com/${GITHUB_REPO}/archive/refs/heads/main.zip" -o "/tmp/nova.zip" && \
        unzip -q "/tmp/nova.zip" -d /tmp && TEMP_DIR="/tmp/nova-main"
        cd "$TEMP_DIR"
        cargo build --release 2>/dev/null
        if [ -f "$TEMP_DIR/target/release/nova" ]; then
            cp "$TEMP_DIR/target/release/nova" "$INSTALL_DIR/nova"
            chmod +x "$INSTALL_DIR/nova"
            print_success "Binary installed to $INSTALL_DIR/nova"
        else
            print_error "Build failed"
            exit 1
        fi
        rm -rf "$TEMP_DIR" "/tmp/nova.zip" 2>/dev/null
    else
        print_error "Installation cancelled"
        exit 0
    fi
else
    print_status "Downloading: $FILENAME"
    (
        curl -sL --progress-bar "$DOWNLOAD_URL" -o "$INSTALL_DIR/nova"
        chmod +x "$INSTALL_DIR/nova"
    ) &
    spinner $! "Installing nova"

    if [ -f "$INSTALL_DIR/nova" ]; then
        print_success "Binary installed to $INSTALL_DIR/nova"
    else
        print_error "Download failed"
        exit 1
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
    SHELL_RC="$HOME/.bashrc"
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
