#!/bin/bash

set -e

INSTALL_DIR="$HOME/.local/bin"
CONFIG_DIR="$HOME/.config/nova"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

print_warning() {
    echo -e "${YELLOW}[${BOLD}!${NC}${YELLOW}]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[${BOLD}✓${NC}${GREEN}]${NC} $1"
}

print_error() {
    echo -e "${RED}[${BOLD}✗${NC}${RED}]${NC} $1"
}

echo -e "${CYAN}"
echo "   __   _     __         __  ___ "
echo "  / | / /__  / /  ___   /  |/  /__  __  ___________ "
echo " /  |/ / _ \\/ _ \\/ _ \\ / /|_/ / _ \\/ / / / ___/ "
echo "/ /|  /  __/  __/  __// /  / /  __/ /_/ / /    "
echo "/_/ |_/\\___/\\___/\\___//_/  /_/\\___/\\__,_/_/     ${NC}"
echo -e "${BOLD}              Uninstall${NC}"
echo ""

if [ ! -f "$INSTALL_DIR/nova" ]; then
    print_warning "Nova is not installed."
    exit 0
fi

echo -e "${YELLOW}This will remove:${NC}"
echo "  - Binary: $INSTALL_DIR/nova"
echo "  - Config: $CONFIG_DIR/"
echo ""

read -p "Continue? [y/N] " -n 1 -r
echo ""

if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Cancelled."
    exit 0
fi

rm -f "$INSTALL_DIR/nova"
print_success "Binary removed"

rm -rf "$CONFIG_DIR"
print_success "Config removed"

for rc in "$HOME/.bashrc" "$HOME/.bash_profile" "$HOME/.zshrc"; do
    if [ -f "$rc" ]; then
        sed -i '/# Nova Editor/,/export PATH=.*\.local\/bin:\$PATH/d' "$rc"
    fi
done
print_success "PATH entries removed"

echo ""
print_success "Nova uninstalled successfully!"
