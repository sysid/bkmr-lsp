# vim-bkmr-lsp

A minimal Vim plugin that integrates [bkmr](https://github.com/sysid/bkmr) snippets via LSP.

## Features

- Auto-completion of bkmr snippets
- Prefix-based filtering
- Execute snippet actions

## Requirements

- Vim 8.0+ or Neovim 0.4+
- [vim-lsp](https://github.com/prabirshrestha/vim-lsp) plugin
- `bkmr-lsp` binary in PATH
- `bkmr` command-line tool

## Installation

### Using vim-plug

```vim
Plug 'prabirshrestha/vim-lsp'
Plug 'yourusername/vim-bkmr-lsp'
```

The plugin works out of the box with default settings.

## Usage

### Default Key Mappings

| Mode | Key | Action |
|------|-----|--------|
| Insert | `<C-Space>` | Trigger completion |
| Normal | `<leader>bc` | Trigger completion |
| Normal | `<leader>bo` | Open snippet by ID |

### Commands

- `:BkmrLspStatus` - Show plugin status
- `:BkmrLspOpen [id]` - Open snippet by ID

### Custom Mappings

Use `<Plug>` mappings for customization:

```vim
" Custom completion mapping
imap <C-k> <Plug>(bkmr-lsp-complete)
nmap <leader>s <Plug>(bkmr-lsp-complete)

" Custom open mapping  
nmap <leader>so <Plug>(bkmr-lsp-open)
```

## Configuration

Minimal configuration options:

```vim
" Custom binary path (default: 'bkmr-lsp')
let g:bkmr_lsp_binary = '/usr/local/bin/bkmr-lsp'

" Disable plugin (default: 1)
let g:bkmr_lsp_enabled = 0
```

## Building bkmr-lsp

```bash
git clone https://github.com/sysid/bkmr-lsp
cd bkmr-lsp
cargo build --release
cp target/release/bkmr-lsp ~/bin/  # or any directory in PATH
```

## Troubleshooting

Check status:
```vim
:BkmrLspStatus
:LspStatus
```

The plugin automatically:
- Registers the LSP server when vim-lsp loads
- Sets up completion when a buffer is opened
- Uses sensible defaults for all settings