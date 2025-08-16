# bkmr-lsp

Language Server Protocol (LSP) implementation for [bkmr](https://github.com/sysid/bkmr) snippet management.

## Overview

bkmr-lsp provides manual snippet completion for bkmr snippets in any LSP-compatible editor. Use Ctrl+Space (or your editor's completion trigger) to access snippets based on the current word context. Snippets are automatically interpolated, delivering processed content rather than raw templates.

**Key Features:**
- **Manual completion**: Triggered via Ctrl+Space with word-based filtering for improved performance
- **Language-aware filtering**: Snippets are filtered by file type (e.g., Rust files get only Rust snippets)
- **Universal snippets**: Language-agnostic snippets with natural Rust syntax that automatically adapt to target languages
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

Use manual completion to access snippets based on the current word under your cursor:

- **Manual trigger**: `Ctrl+Space` (or your editor's completion hotkey)
- **Word-based filtering**: Type a word, then trigger completion to find matching snippets
- **Examples**: Type `hello` then Ctrl+Space, type `aws` then Ctrl+Space
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
- **Rust files** (`.rs`): Shows snippets tagged with `rust` AND `universal`
- **Python files** (`.py`): Shows snippets tagged with `python` AND `universal`
- **JavaScript files** (`.js`): Shows snippets tagged with `javascript` AND `universal`
- **And more**: Supports all LSP language identifiers

**Setting up language-specific snippets:**
```bash
# Tag snippets with language identifiers
bkmr add -t rust -t _snip_ 'fn main() { println!("Hello"); }' 'Rust main function'
bkmr add -t python -t _snip_ 'if __name__ == "__main__":' 'Python main guard'  
bkmr add -t javascript -t _snip_ 'console.log("Hello");' 'JS console log'
```

### Universal Snippets

Universal snippets work across all languages by using natural Rust syntax that gets automatically translated:

**Creating universal snippets:**
```bash
# Use natural Rust syntax with 'universal' tag
bkmr add -t universal -t _snip_ '// Function: {{ function_name }}
// TODO: implement
    return {{ value }};' 'Function template'
```

**Automatic translation:**
- **Python**: `// comment` becomes `# comment`
- **HTML**: `// comment` becomes `<!-- comment -->`
- **Indentation**: `    ` (4 spaces) becomes tabs for Go, 2 spaces for JavaScript, etc.
- **Block comments**: `/* comment */` adapts to target language syntax

See [UNIVERSAL_SNIPPETS.md](UNIVERSAL_SNIPPETS.md) for complete documentation.

### Word-Based Filtering

The completion system uses the current word under your cursor for intelligent filtering:

- Type `aws` then Ctrl+Space to show AWS-related snippets
- Type `config` then Ctrl+Space to show configuration snippets
- Partial matches filter by snippet titles and content
- Empty word shows all available snippets for the current language

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

1. Verify bkmr works: `bkmr search --json --interpolate 'tags:"_snip_"'`
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

## Architecture

bkmr-lsp follows Clean Architecture principles with clear separation of concerns:

- **Domain Layer**: Pure business models for snippets, languages, and completion
- **Repository Layer**: Data access abstraction with Repository pattern for bkmr CLI integration
- **Service Layer**: Business logic orchestration with dependency injection
- **Infrastructure Layer**: LSP protocol implementation with clean separation

For detailed architecture documentation, see [DEVELOPMENT.md](DEVELOPMENT.md).

## Development

### Building

```bash
cargo build --release
```

### Testing

The project includes comprehensive testing with 84 tests covering:
- Unit tests for domain logic and services
- Integration tests with real bkmr CLI execution
- LSP protocol tests with actual server instances
- Mock repositories for isolated testing

```bash
# Run all tests
cargo test

# Run specific test categories
cargo test test_backend              # Unit tests
cargo test test_lsp_integration      # LSP protocol tests
cargo test integration_test          # bkmr CLI integration
```

### Development Scripts

The project includes several development and testing scripts:

```bash
# Build and install LSP server for development
make install-debug

# Language filtering demonstration
./scripts/demo_language_filtering.py

# Completion behavior testing
./scripts/test_completion_behavior.py

# Text replacement testing
./scripts/test_text_replacement.py

# Integration testing
./scripts/integration_test.sh
```

These scripts demonstrate various features including language detection, completion behavior, and universal snippet translation.

### Testing Universal Snippets

The project includes comprehensive tests for universal snippet functionality:

```bash
# Run all tests including universal snippet translation
cargo test

# Test only universal snippet features
cargo test universal
cargo test fts_query
cargo test rust_pattern
```

**Testing with real data:**
```bash
# Add a universal snippet for testing
bkmr add -t universal -t _snip_ '// Header: {{ title }}
/* 
Author: {{ author }}
*/
fn example() {
    // TODO: implement
}' 'Universal function header'

# Test FTS query manually
bkmr search --json --interpolate '(tags:python AND tags:"_snip_") OR (tags:universal AND tags:"_snip_")'
```

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

2. **FTS Query Building**: During completion requests, the server builds FTS (Full Text Search) queries that combine language-specific and universal snippets:
   ```bash
   # For Rust files:
   bkmr search --json --interpolate --limit 50 '(tags:rust AND tags:"_snip_") OR (tags:universal AND tags:"_snip_")'
   
   # For Python files:
   bkmr search --json --interpolate --limit 50 '(tags:python AND tags:"_snip_") OR (tags:universal AND tags:"_snip_")'
   
   # With word-based filtering:
   bkmr search --json --interpolate --limit 50 '((tags:rust AND tags:"_snip_") OR (tags:universal AND tags:"_snip_")) AND metadata:hello*'
   ```

3. **Universal Snippet Processing**: Snippets tagged with "universal" are automatically translated from Rust syntax to the target language using regex-based pattern matching

4. **Cache Management**: Language IDs are stored per document URI and cleaned up when documents are closed

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
  - Manual completion triggered via Ctrl+Space with word-based filtering
  - Language-aware snippet filtering using `textDocument/didOpen` language ID
  - Universal snippets with natural Rust syntax translation
  - Template interpolation via bkmr `--interpolate` flag
  - FTS-based queries for optimal snippet retrieval
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