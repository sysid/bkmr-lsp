# bkmr-lsp

Language Server Protocol (LSP) implementation for [bkmr](https://github.com/sysid/bkmr) snippet management.

## Overview

bkmr-lsp provides trigger-based snippet completion for bkmr snippets in any LSP-compatible editor. Type `:` followed by letters to get snippet completions. Snippets are automatically interpolated, delivering processed content rather than raw templates. 

**Key Features:**
- **Language-aware filtering**: Snippets are filtered by file type (e.g., Rust files get only Rust snippets)
- **Automatic interpolation**: Templates are processed using bkmr's `--interpolate` flag
- **LSP commands**: Filepath comment insertion with automatic language detection

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

### Language-Based Filtering

The LSP server automatically filters snippets based on the file type (language ID) provided by your editor:

**Automatic filtering:**
- **Rust files** (`.rs`): Only shows snippets tagged with `rust`
- **Python files** (`.py`): Only shows snippets tagged with `python`
- **JavaScript files** (`.js`): Only shows snippets tagged with `javascript`
- **And more**: Supports all LSP language identifiers

**Setting up language-specific snippets:**
```bash
# Tag snippets with language identifiers
bkmr add -t rust -t _snip_ 'fn main() { println!("Hello"); }' 'Rust main function'
bkmr add -t python -t _snip_ 'if __name__ == "__main__":' 'Python main guard'  
bkmr add -t javascript -t _snip_ 'console.log("Hello");' 'JS console log'
```

### Text-Based Filtering

Use prefixes after `:` for additional text-based filtering:

- Type `:aws` to show AWS-related snippets
- Type `:config` to show configuration snippets
- Partial matches filter by snippet titles and content

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

1. Verify bkmr works: `bkmr search --ntags-prefix _snip_`
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

### Testing Language Filtering

A test script is provided to demonstrate filetype extraction:

```bash
# Build and install LSP server
make install-debug

# Run filetype extraction test
./scripts/test_filetype_extraction.py
```

This script opens different file types and shows how the server extracts and uses the `language_id` for filtering.

### Logging

The LSP server automatically adjusts log levels based on the execution context:

- **LSP mode** (when run by LSP clients): Defaults to ERROR level to avoid noise in client logs
- **Terminal mode** (when run manually): Defaults to WARN level for development

**Manual log level control:**

```bash
# Enable debug logging (will appear as ERRORs in LSP client logs)
RUST_LOG=debug bkmr-lsp

# Completely disable logging
BKMR_LSP_NO_LOG=1 bkmr-lsp

# Log to file for debugging
RUST_LOG=debug bkmr-lsp 2>/tmp/bkmr-lsp.log
```

**Debug log entries for language filtering:**
```
Document opened: file:///example.rs (language: rust)
Document language ID: Some("rust")
Using language filter: rust
```

**Note:** LSP clients (like Neovim) treat all stderr output as errors, so debug messages will appear under ERROR in client logs. This is normal LSP behavior.

## Implementation Details

### Language Filtering Architecture

The LSP server implements language-aware filtering through:

1. **Language ID Capture**: When a document is opened via `textDocument/didOpen`, the server captures and caches the `language_id` field from the `TextDocumentItem`

2. **Filtering Logic**: During completion requests, the server adds the cached language ID as a tag filter to the bkmr search command:
   ```bash
   # Without language filtering:
   bkmr search --json --interpolate --ntags-prefix _snip_ --limit 50 metadata:query*
   
   # With language filtering (e.g., for Rust files):
   bkmr search --json --interpolate --ntags-prefix _snip_ --limit 50 -t rust metadata:query*
   ```

3. **Cache Management**: Language IDs are stored per document URI and cleaned up when documents are closed

### Common Language Identifiers

LSP clients typically provide these language identifiers:
- `rust` - Rust files (.rs)
- `python` - Python files (.py)
- `javascript` - JavaScript files (.js)
- `typescript` - TypeScript files (.ts)
- `java` - Java files (.java)
- `c` - C files (.c)
- `cpp` - C++ files (.cpp, .cc, .cxx)
- `go` - Go files (.go)
- `shell` - Shell scripts (.sh, .bash)
- And many more...

## Protocol Support

- **LSP Version**: 3.17
- **Features**: 
  - Text document completion with manual triggering
  - Language-aware snippet filtering using `textDocument/didOpen` language ID
  - Template interpolation via bkmr `--interpolate` flag
  - Live snippet fetching with bkmr CLI integration
  - LSP commands for filepath comment insertion with language detection

## Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Submit a pull request

## Related Projects

- [bkmr](https://github.com/sysid/bkmr) - Command-line bookmark and snippet manager
- [vim-bkmr-lsp](https://github.com/sysid/vim-bkmr-lsp) - Vim plugin for bkmr-lsp