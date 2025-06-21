# bkmr-lsp

Language Server Protocol (LSP) implementation for [bkmr](https://github.com/sysid/bkmr) snippet management.

## Overview

bkmr-lsp provides trigger-based snippet completion for bkmr snippets in any LSP-compatible editor. Type `:` followed by letters to get snippet completions. Snippets are automatically interpolated, delivering processed content rather than raw templates. Additionally, it provides LSP commands for filepath comment insertion with automatic language detection.

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

### Snippet Completion

Type `:` followed by letters to trigger snippet completion:

- **Examples**: `:hello`, `:js`, `:aws`
- **Trigger character**: `:` (colon)
- **Manual trigger**: `Ctrl+Space` when in snippet context
- **Navigation**: Use arrow keys, Tab or Enter to complete

### Template Interpolation

Snippets with templates are automatically processed:

```bash
# Snippet content: {{ "pwd" | shell }}
# Completion inserts: /Users/username/project
```

### Filtering

Use prefixes after `:` to filter completions:

- Type `:js` to show JavaScript snippets
- Type `:py` to show Python snippets  
- Type `:aws` to show AWS-related snippets
- Partial matches filter by snippet titles

### LSP Commands

The server provides LSP commands for additional functionality:

#### `bkmr.insertFilepathComment`
Insert the relative filepath as a comment at the beginning of the file.

**Features:**
- **Smart Comment Detection**: Automatically detects correct comment syntax for 20+ file types
- **Project-Relative Paths**: Generates relative paths from project root (searches for `Cargo.toml`, `package.json`, `.git`, etc.)
- **Language Support**: 
  - C-style (`//`): Rust, Java, JavaScript, TypeScript, C++, Go, Swift, Kotlin, Scala, Dart
  - Shell-style (`#`): Python, Shell, YAML, TOML, Ruby, Perl, R, Config files
  - HTML/XML (`<!-- -->`): HTML, XML, SVG
  - CSS (`/* */`): CSS, SCSS, Sass, Less
  - SQL (`--`): SQL files
  - And many more (Lua, Haskell, Lisp, VimScript, Batch, PowerShell, LaTeX, Fortran, MATLAB)

**Example:**
```rust
// src/backend.rs
use tower_lsp::LanguageServer;
```

**Usage in LSP Clients:**
Most LSP clients can execute this command programmatically. For IntelliJ Platform IDEs, use the [bkmr-intellij-plugin](../bkmr-intellij-plugin) which provides UI integration.


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
  - Text document completion with `:` trigger character
  - Template interpolation
  - Live snippet fetching
  - LSP commands for filepath comment insertion

## Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Submit a pull request

## Related Projects

- [bkmr](https://github.com/sysid/bkmr) - Command-line bookmark and snippet manager
- [vim-bkmr-lsp](https://github.com/sysid/vim-bkmr-lsp) - Vim plugin for bkmr-lsp