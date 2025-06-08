#!/bin/bash
# install.sh - Installation script for vim-bkmr-lsp plugin

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_step() {
    echo -e "${BLUE}[STEP]${NC} $1"
}

# Check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Detect Vim type and configuration
detect_vim() {
    if command_exists nvim; then
        VIM_TYPE="neovim"
        VIM_CMD="nvim"
        if [ -d "$HOME/.config/nvim" ]; then
            VIM_CONFIG_DIR="$HOME/.config/nvim"
        else
            VIM_CONFIG_DIR="$HOME/.vim"
        fi
    elif command_exists vim; then
        VIM_TYPE="vim"
        VIM_CMD="vim"
        VIM_CONFIG_DIR="$HOME/.vim"
    else
        log_error "Neither vim nor neovim found"
        exit 1
    fi

    log_info "Detected $VIM_TYPE at $VIM_CONFIG_DIR"
}

# Check prerequisites
check_prerequisites() {
    log_step "Checking prerequisites..."

    # Check Vim
    detect_vim

    # Check Rust/Cargo
    if ! command_exists cargo; then
        log_error "Cargo (Rust) not found. Please install Rust first:"
        echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        exit 1
    fi

    # Check bkmr
    if ! command_exists bkmr; then
        log_warn "bkmr not found. Installing..."
        cargo install bkmr
    fi

    # Check git
    if ! command_exists git; then
        log_error "Git not found. Please install git first."
        exit 1
    fi

    log_info "Prerequisites check passed"
}

# Build bkmr-lsp
build_bkmr_lsp() {
    log_step "Building bkmr-lsp..."

    # Check if we're in the project directory or need to clone
    if [ ! -d "bkmr-lsp" ]; then
        if [ -f "bkmr-lsp/Cargo.toml" ]; then
            # We're in the parent directory
            cd bkmr-lsp
        else
            log_error "bkmr-lsp source not found. Please run from project root or provide --clone option"
            exit 1
        fi
    else
        cd bkmr-lsp
    fi

    # Build the project
    if ! cargo build --release; then
        log_error "Failed to build bkmr-lsp"
        exit 1
    fi

    # Go back to original directory
    cd ..

    log_info "bkmr-lsp built successfully"
}

# Install bkmr-lsp binary
install_binary() {
    log_step "Installing bkmr-lsp binary..."

    # Determine install location
    if [ -d "$HOME/bin" ] && [[ ":$PATH:" == *":$HOME/bin:"* ]]; then
        INSTALL_DIR="$HOME/bin"
    elif [ -d "$HOME/.local/bin" ]; then
        INSTALL_DIR="$HOME/.local/bin"
        # Add to PATH if not already there
        if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
            log_warn "$HOME/.local/bin not in PATH. Consider adding it to your shell profile:"
            echo '  export PATH="$HOME/.local/bin:$PATH"'
        fi
    else
        mkdir -p "$HOME/.local/bin"
        INSTALL_DIR="$HOME/.local/bin"
        log_warn "Created $HOME/.local/bin. Add it to your PATH:"
        echo '  export PATH="$HOME/.local/bin:$PATH"'
    fi

    # Copy binary
    if [ -f "bkmr-lsp/target/release/bkmr-lsp" ]; then
        cp bkmr-lsp/target/release/bkmr-lsp "$INSTALL_DIR/"
        chmod +x "$INSTALL_DIR/bkmr-lsp"
        log_info "bkmr-lsp installed to $INSTALL_DIR"
    else
        log_error "bkmr-lsp binary not found. Build may have failed."
        exit 1
    fi
}

# Install vim plugin
install_plugin() {
    log_step "Installing vim-bkmr-lsp plugin..."

    # Detect plugin manager
    PLUGIN_MANAGER=""

    if [ -f "$VIM_CONFIG_DIR/autoload/plug.vim" ] || [ -f "$HOME/.vim/autoload/plug.vim" ]; then
        PLUGIN_MANAGER="vim-plug"
    elif [ -d "$VIM_CONFIG_DIR/bundle/Vundle.vim" ] || [ -d "$HOME/.vim/bundle/Vundle.vim" ]; then
        PLUGIN_MANAGER="vundle"
    elif [ -d "$VIM_CONFIG_DIR/pack" ] || [ -d "$HOME/.vim/pack" ]; then
        PLUGIN_MANAGER="native"
    fi

    case "$PLUGIN_MANAGER" in
        "vim-plug")
            install_with_plug
            ;;
        "vundle")
            install_with_vundle
            ;;
        "native"|"")
            install_manually
            ;;
    esac
}

# Install with vim-plug
install_with_plug() {
    log_info "Detected vim-plug plugin manager"

    local vimrc
    if [ "$VIM_TYPE" = "neovim" ]; then
        vimrc="$VIM_CONFIG_DIR/init.vim"
    else
        vimrc="$HOME/.vimrc"
    fi

    # Check if plugins are already configured
    if [ -f "$vimrc" ] && grep -q "vim-bkmr-lsp" "$vimrc"; then
        log_info "Plugin already configured in $vimrc"
        return
    fi

    # Add plugin configuration
    cat >> "$vimrc" << 'EOF'

" vim-bkmr-lsp plugin configuration
Plug 'prabirshrestha/vim-lsp'
Plug 'mattn/vim-lsp-settings'
" Add your vim-bkmr-lsp plugin line here, e.g.:
" Plug 'yourusername/vim-bkmr-lsp'

EOF

    log_info "Added plugin configuration to $vimrc"
    log_warn "Please add your vim-bkmr-lsp plugin line and run :PlugInstall in Vim"
}

# Install with Vundle
install_with_vundle() {
    log_info "Detected Vundle plugin manager"

    local vimrc
    if [ "$VIM_TYPE" = "neovim" ]; then
        vimrc="$VIM_CONFIG_DIR/init.vim"
    else
        vimrc="$HOME/.vimrc"
    fi

    # Add plugin configuration
    if [ -f "$vimrc" ] && ! grep -q "vim-bkmr-lsp" "$vimrc"; then
        # Find the Vundle plugin section and add our plugins
        sed -i '/call vundle#begin/a\
Plugin '\''prabirshrestha/vim-lsp'\''\
Plugin '\''mattn/vim-lsp-settings'\''\
" Plugin '\''yourusername/vim-bkmr-lsp'\''' "$vimrc"

        log_info "Added plugin configuration to $vimrc"
        log_warn "Please uncomment and update the vim-bkmr-lsp plugin line, then run :PluginInstall in Vim"
    fi
}

# Manual installation
install_manually() {
    log_info "Installing plugin manually using Vim 8 packages"

    # Create plugin directory structure
    local plugin_dir="$VIM_CONFIG_DIR/pack/plugins/start"
    mkdir -p "$plugin_dir"

    # Clone or copy required plugins
    cd "$plugin_dir"

    # Install vim-lsp if not present
    if [ ! -d "vim-lsp" ]; then
        log_info "Installing vim-lsp..."
        git clone https://github.com/prabirshrestha/vim-lsp.git
    fi

    # Install vim-lsp-settings if not present
    if [ ! -d "vim-lsp-settings" ]; then
        log_info "Installing vim-lsp-settings..."
        git clone https://github.com/mattn/vim-lsp-settings.git
    fi

    # Copy our plugin files
    if [ ! -d "vim-bkmr-lsp" ]; then
        mkdir -p vim-bkmr-lsp
        log_info "Creating vim-bkmr-lsp plugin directory"
    fi

    # Copy plugin files from current directory
    local source_dir
    if [ -f "../plugin/bkmr_lsp.vim" ]; then
        source_dir=".."
    elif [ -f "plugin/bkmr_lsp.vim" ]; then
        source_dir="."
    else
        log_error "Plugin source files not found"
        exit 1
    fi

    cp -r "$source_dir"/{plugin,autoload,doc} vim-bkmr-lsp/ 2>/dev/null || true

    log_info "Plugin installed manually to $plugin_dir"
    cd - > /dev/null
}

# Add test snippets
add_test_snippets() {
    log_step "Adding test snippets..."

    # Check if test snippets already exist
    if bkmr search -t test,vim-bkmr-lsp --json 2>/dev/null | grep -q "vim-bkmr-lsp"; then
        log_info "Test snippets already exist"
        return
    fi

    # Add test snippets
    bkmr add "console.log('Hello from vim-bkmr-lsp!');" javascript,test,vim-bkmr-lsp --type snip --title "JS Hello (vim)"
    bkmr add "println!('Hello from vim-bkmr-lsp!');" rust,test,vim-bkmr-lsp --type snip --title "Rust Hello (vim)"
    bkmr add "print('Hello from vim-bkmr-lsp!')" python,test,vim-bkmr-lsp --type snip --title "Python Hello (vim)"
    bkmr add "# Test snippet for vim-bkmr-lsp" test,vim-bkmr-lsp --type snip --title "Comment Test"

    log_info "Test snippets added successfully"
}

# Test installation
test_installation() {
    log_step "Testing installation..."

    # Test binary
    if ! command_exists bkmr-lsp; then
        log_error "bkmr-lsp not found in PATH"
        return 1
    fi

    # Test binary responds
    if echo '{"jsonrpc":"2.0","method":"initialize","id":1,"params":{}}' | timeout 5 bkmr-lsp >/dev/null 2>&1; then
        log_info "bkmr-lsp binary responds correctly"
    else
        log_warn "bkmr-lsp binary may not be working correctly"
    fi

    # Test bkmr has snippets
    local snippet_count
    snippet_count=$(bkmr search -t _snip_ --json 2>/dev/null | jq '. | length' 2>/dev/null || echo "0")

    if [ "$snippet_count" -gt 0 ]; then
        log_info "Found $snippet_count snippets in bkmr database"
    else
        log_warn "No snippets found in bkmr database"
    fi

    log_info "Installation test completed"
}

# Show usage instructions
show_instructions() {
    cat << EOF

${GREEN}vim-bkmr-lsp installation completed!${NC}

${YELLOW}Next steps:${NC}

1. ${BLUE}Restart Vim/Neovim${NC}

2. ${BLUE}Check plugin status:${NC}
   :BkmrLspStatus

3. ${BLUE}Test completion:${NC}
   - Open any file
   - Type a trigger character: .  or  :
   - Or use manual completion: <C-x><C-o>
   - Or use key mapping: <leader>bs

4. ${BLUE}Key mappings:${NC}
   <leader>bs  - Trigger completion
   <leader>bo  - Open snippet by ID
   <leader>br  - Refresh snippets
   <leader>bf  - Search snippets

5. ${BLUE}Commands:${NC}
   :BkmrLspInfo      - Show plugin info
   :BkmrLspStatus    - Check status
   :BkmrLspRefresh   - Refresh cache

${YELLOW}Troubleshooting:${NC}
- If plugin doesn't load: :echo exists('g:loaded_bkmr_lsp')
- Check LSP status: :LspStatus
- View logs: :LspLog
- Plugin help: :help bkmr-lsp

${YELLOW}Configuration:${NC}
Add to your vimrc for customization:
  let g:bkmr_lsp_auto_complete = 1
  let g:bkmr_lsp_trigger_chars = ['.', ':']
  let g:bkmr_lsp_mappings = {'complete': '<C-Space>'}

EOF
}

# Main function
main() {
    local CLONE_REPO=""

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --clone)
                CLONE_REPO="$2"
                shift 2
                ;;
            -h|--help)
                echo "Usage: $0 [--clone REPO_URL]"
                echo "  --clone REPO_URL  Clone the repository first"
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                exit 1
                ;;
        esac
    done

    log_info "Starting vim-bkmr-lsp installation..."

    # Clone repository if requested
    if [ -n "$CLONE_REPO" ]; then
        log_step "Cloning repository..."
        git clone "$CLONE_REPO" vim-bkmr-lsp-repo
        cd vim-bkmr-lsp-repo
    fi

    check_prerequisites
    build_bkmr_lsp
    install_binary
    install_plugin
    add_test_snippets
    test_installation
    show_instructions

    log_info "Installation completed successfully!"
}

# Run main function
main "$@"