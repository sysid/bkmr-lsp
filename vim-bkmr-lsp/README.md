# vim-bkmr-lsp

A Vim plugin that integrates [bkmr](https://github.com/sysid/bkmr) snippet manager with Vim through Language Server Protocol (LSP).

## Features

- 🚀 **Auto-completion** of bkmr snippets
- 🔍 **Interactive search** and filtering
- ⚡ **Execute snippet actions** directly from Vim
- 🎯 **Smart trigger characters** for context-aware completion
- 🔧 **Highly configurable** with sensible defaults
- 📚 **Comprehensive documentation** and help system

## Requirements

- Vim 8.0+ or Neovim 0.4+
- [vim-lsp](https://github.com/prabirshrestha/vim-lsp) plugin
- [bkmr-lsp](https://github.com/sysid/bkmr-lsp) binary in PATH
- [bkmr](https://github.com/sysid/bkmr) command-line tool

## Installation

### Using vim-plug

```vim
Plug 'prabirshrestha/vim-lsp'
Plug 'mattn/vim-lsp-settings'  " Optional but recommended
Plug 'yourusername/vim-bkmr-lsp'
```

### Using Vundle

```vim
Plugin 'prabirshrestha/vim-lsp'
Plugin 'mattn/vim-lsp-settings'
Plugin 'yourusername/vim-bkmr-lsp'
```

### Manual Installation

1. Clone this repository to your Vim plugin directory:
   ```bash
   git clone https://github.com/yourusername/vim-bkmr-lsp.git ~/.vim/pack/plugins/start/vim-bkmr-lsp
   ```

2. Ensure dependencies are installed and `bkmr-lsp` is in your PATH

## Quick Start

1. **Install the plugin** using your preferred method
2. **Build and install bkmr-lsp**:
   ```bash
   git clone https://github.com/sysid/bkmr-lsp
   cd bkmr-lsp
   cargo build --release
   cp target/release/bkmr-lsp ~/bin/  # or any directory in PATH
   ```
3. **Add some snippets to bkmr**:
   ```bash
   bkmr add "console.log('Hello World!');" javascript,test --type snip --title "JS Hello"
   ```
4. **Open Vim and test**:
   ```vim
   :BkmrLspStatus  " Check plugin status
   :BkmrLspInfo    " Show available commands
   ```

## Usage

### Key Mappings (Default)

| Key | Action | Description |
|-----|--------|-------------|
| `<leader>bs` | Complete | Trigger snippet completion |
| `<leader>bo` | Open | Open snippet by ID |
| `<leader>br` | Refresh | Refresh snippet cache |
| `<leader>bf` | Search | Interactive snippet search |

### Commands

| Command | Description |
|---------|-------------|
| `:BkmrLspStatus` | Show plugin and server status |
| `:BkmrLspRefresh` | Refresh snippet cache |
| `:BkmrLspOpen [id]` | Open snippet by ID |
| `:BkmrLspSearch` | Interactive snippet search |
| `:BkmrLspComplete` | Trigger completion manually |
| `:BkmrLspInfo` | Show plugin information |

### Auto-completion

The plugin automatically triggers completion when you type trigger characters (`.` and `:` by default):

```
Type: hello.
      ^
      Completion popup appears automatically
```

You can also trigger completion manually with `<C-x><C-o>` or `<leader>bs`.

## Configuration

### Basic Configuration

```vim
" Disable auto-completion
let g:bkmr_lsp_auto_complete = 0

" Change trigger characters
let g:bkmr_lsp_trigger_chars = ['.', ':', '@', '#']

" Increase max completions
let g:bkmr_lsp_max_completions = 100

" Custom binary path
let g:bkmr_lsp_binary = '/usr/local/bin/bkmr-lsp'
```

### Custom Key Mappings

```vim
" Customize all mappings
let g:bkmr_lsp_mappings = {
      \ 'complete': '<C-Space>',
      \ 'open': '<leader>so',
      \ 'refresh': '<leader>sr',
      \ 'search': '<leader>ss'
      \ }

" Disable specific mappings
let g:bkmr_lsp_mappings = {
      \ 'open': '',  " Disable open mapping
      \ 'search': '<leader>find'  " Custom search mapping
      \ }
```

### Advanced Configuration

```vim
" Complete configuration example
let g:bkmr_lsp_enabled = 1
let g:bkmr_lsp_binary = 'bkmr-lsp'
let g:bkmr_lsp_auto_complete = 1
let g:bkmr_lsp_trigger_chars = ['.', ':']
let g:bkmr_lsp_max_completions = 50
let g:bkmr_lsp_mappings = {
      \ 'complete': '<leader>bs',
      \ 'open': '<leader>bo',
      \ 'refresh': '<leader>br',
      \ 'search': '<leader>bf'
      \ }
```
