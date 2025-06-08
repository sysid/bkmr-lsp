# bkmr-lsp

Language Server Protocol (LSP) implementation for [bkmr](https://github.com/sysid/bkmr) snippet management.

## Overview

bkmr-lsp provides code completion for bkmr snippets in any LSP-compatible editor. Snippets are automatically interpolated, delivering processed content rather than raw templates.

## Requirements

- **bkmr**: Version 4.24.0 or later
- **LSP Client**: Any LSP-compatible editor (VS Code, Vim/Neovim, Emacs, etc.)

## Installation

### From Source

```bash
git clone https://github.com/sysid/bkmr-lsp
cd bkmr-lsp
cargo build --release
cp target/release/bkmr-lsp /usr/local/bin/
```

### Prerequisites

Ensure bkmr (>= 4.24.0) is installed and contains snippets:

```bash
# Install bkmr if not present
cargo install bkmr

# Verify version
bkmr --version  # Must be >= 4.24.0

# Add test snippet
bkmr add "console.log('Hello World');" javascript,test --type snip --title "JS Hello"
```

## Configuration

### VS Code

Install an LSP extension and add to `settings.json`:

```json
{
  "languageServerExample.servers": {
    "bkmr-lsp": {
      "command": "bkmr-lsp",
      "filetypes": ["*"]
    }
  }
}
```

### Vim/Neovim with vim-lsp

```vim
if executable('bkmr-lsp')
  augroup LspBkmr
    autocmd!
    autocmd User lsp_setup call lsp#register_server({
      \ 'name': 'bkmr-lsp',
      \ 'cmd': {server_info->['bkmr-lsp']},
      \ 'allowlist': ['*'],
      \ })
  augroup END
endif
```

### Neovim with nvim-lspconfig

```lua
require'lspconfig'.bkmr_lsp.setup{
  cmd = { "bkmr-lsp" },
  filetypes = { "*" },
}
```

### Emacs with lsp-mode

```elisp
(with-eval-after-load 'lsp-mode
  (add-to-list 'lsp-language-id-configuration '(".*" . "text"))
  (lsp-register-client
   (make-lsp-client :new-connection (lsp-stdio-connection "bkmr-lsp")
                    :major-modes '(text-mode)
                    :server-id 'bkmr-lsp)))
```

## Usage

### Code Completion

Start typing in any file and trigger completion:

- **VS Code**: `Ctrl+Space`
- **Vim**: `Ctrl+X Ctrl+O` or `Ctrl+N`
- **Neovim**: Automatic with completion plugins
- **Emacs**: `M-x completion-at-point`

### Template Interpolation

Snippets with templates are automatically processed:

```bash
# Snippet content: {{ "pwd" | shell }}
# Completion inserts: /Users/username/project
```

### Filtering

Use prefixes to filter completions:

- Type `js` to show JavaScript snippets
- Type `py` to show Python snippets
- Type partial titles to narrow results

## Commands

The LSP server supports these commands:

- `bkmr.refresh`: Refresh snippet cache (no-op, snippets are fetched live)
- `bkmr.open`: Open snippet by ID


## Troubleshooting

### No Completions Appearing

1. Verify bkmr works: `bkmr search -t _snip_`
2. Check bkmr version: `bkmr --version`
3. Test LSP server: `echo '{"jsonrpc":"2.0","method":"initialize","id":1,"params":{}}' | bkmr-lsp`

### Raw Templates in Completions

Update bkmr to version 4.24.0 or later:

```bash
cargo install bkmr --force
```

### LSP Server Not Starting

1. Verify binary is in PATH: `which bkmr-lsp`
2. Check editor LSP configuration
3. Review editor LSP logs for errors

## Development

### Building

```bash
cargo build
```

### Testing

```bash
cargo test
```

### Logging

Set `RUST_LOG=debug` for detailed logging:

```bash
RUST_LOG=debug bkmr-lsp 2>lsp.log
```

## Protocol Support

- **LSP Version**: 3.17
- **Features**: 
  - Text document completion
  - Command execution
  - Workspace commands

## Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Submit a pull request

## Related Projects

- [bkmr](https://github.com/sysid/bkmr) - Command-line bookmark and snippet manager
- [vim-bkmr-lsp](https://github.com/sysid/vim-bkmr-lsp) - Vim plugin for bkmr-lsp