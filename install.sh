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

print_banner
print_status "Detecting system..."

detect_os
detect_arch

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

# If no release binary, download from repo
if [ -z "$DOWNLOAD_URL" ]; then
    DOWNLOAD_URL="https://raw.githubusercontent.com/${GITHUB_REPO}/main/target/release/nova"
    FILENAME="nova"
else
    FILENAME=$(basename "$DOWNLOAD_URL")
fi

if [ "$os" = "termux" ]; then
    INSTALL_DIR="$PREFIX/bin"
fi

mkdir -p "$INSTALL_DIR"

print_status "Downloading nova..."

if [[ "$DOWNLOAD_URL" == *"raw.githubusercontent.com"* ]]; then
    curl -sL "$DOWNLOAD_URL" -o "$INSTALL_DIR/nova"
else
    (
        curl -sL --progress-bar "$DOWNLOAD_URL" -o "$INSTALL_DIR/nova"
    ) &
    spinner $! "Downloading nova"
fi

chmod +x "$INSTALL_DIR/nova"

if [ -f "$INSTALL_DIR/nova" ]; then
    print_success "Binary installed to $INSTALL_DIR/nova"
else
    print_error "Download failed"
    print_warning "Trying to compile from source..."
    if [ "$os" = "termux" ]; then
        pkg install -y rust clang 2>/dev/null || true
    fi
    TEMP_DIR=$(mktemp -d)
    git clone --depth 1 "https://github.com/${GITHUB_REPO}.git" "$TEMP_DIR/nova" 2>/dev/null
    cd "$TEMP_DIR/nova"
    cargo build --release 2>/dev/null
    if [ -f "$TEMP_DIR/nova/target/release/nova" ]; then
        cp "$TEMP_DIR/nova/target/release/nova" "$INSTALL_DIR/nova"
        chmod +x "$INSTALL_DIR/nova"
        print_success "Compiled and installed!"
    else
        print_error "Build failed"
        exit 1
    fi
    rm -rf "$TEMP_DIR"
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
