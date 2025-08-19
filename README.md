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

### Standard Installation

Install directly from crates.io using cargo:

```bash
cargo install bkmr-lsp
```

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
      "filetypes": ["rust", "javascript", "typescript", "python", "go", "java", "c", "cpp", "html", "css", "scss", "ruby", "php", "swift", "kotlin", "shell", "yaml", "json", "markdown", "xml", "vim"]
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
      \ 'allowlist': ['rust', 'javascript', 'typescript', 'python', 'go', 'java', 'c', 'cpp', 'html', 'css', 'scss', 'ruby', 'php', 'swift', 'kotlin', 'shell', 'yaml', 'json', 'markdown', 'xml', 'vim'],
      \ })
  augroup END
endif
```

### Neovim with nvim-lspconfig

Basic setup:
```lua
require'lspconfig'.bkmr_lsp.setup{
  cmd = { "bkmr-lsp" },
  filetypes = { "rust", "javascript", "typescript", "python", "go", "java", "c", "cpp", "html", "css", "scss", "ruby", "php", "swift", "kotlin", "shell", "yaml", "json", "markdown", "xml", "vim" },
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

Snippets with templates are automatically processed ([server-side interpolation](https://github.com/sysid/bkmr/blob/main/docs/interpolation.md#template-interpolation-in-bkmr)):

```bash
# Snippet content: {{ "pwd" | shell }}
# Completion inserts: /Users/username/project
```

### Language-Aware Filtering

The LSP server automatically filters snippets based on the file type (language ID) provided by your editor. This ensures you only see relevant snippets for your current context.

**How it works:**
1. **Language Detection**: When you open a file, the LSP client sends the language ID (e.g., `rust`, `python`, `javascript`)
2. **Smart Filtering**: Shows snippets tagged with your current language PLUS universal snippets
3. **FTS Query**: Builds optimized full-text search queries for bkmr

**Supported Languages:**
| Language | File Extensions | LSP Language ID |
|----------|----------------|-----------------|
| Rust | `.rs` | `rust` |
| Python | `.py` | `python` |
| JavaScript | `.js` | `javascript` |
| TypeScript | `.ts`, `.tsx` | `typescript` |
| Go | `.go` | `go` |
| Java | `.java` | `java` |
| C/C++ | `.c`, `.cpp`, `.cc` | `c`, `cpp` |
| Shell | `.sh`, `.bash` | `shell`, `sh` |
| YAML | `.yaml`, `.yml` | `yaml` |
| JSON | `.json` | `json` |
| Markdown | `.md` | `markdown` |
| And many more... | | |

**Setting up language-specific snippets:**
```bash
# Tag snippets with language identifiers
bkmr add -t rust -t _snip_ 'fn main() { println!("Hello"); }' 'Rust main function'
bkmr add -t python -t _snip_ 'if __name__ == "__main__":' 'Python main guard'  
bkmr add -t javascript -t _snip_ 'console.log("Hello");' 'JS console log'
bkmr add -t yaml -t _snip_ 'version: "3.8"' 'Docker Compose version'
```

**Query Examples:**
```bash
# What the LSP server generates for different languages:
# Rust file: (tags:rust AND tags:"_snip_") OR (tags:universal AND tags:"_snip_")
# Python file: (tags:python AND tags:"_snip_") OR (tags:universal AND tags:"_snip_")
# With word filter: ((tags:rust AND tags:"_snip_") OR (tags:universal AND tags:"_snip_")) AND metadata:hello*
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

**Example output:**
```rust
// src/backend.rs  <---
use tower_lsp::LanguageServer;
```

**Neovim Configuration with Custom Commands:**

```lua
if vim.fn.executable('bkmr-lsp') == 1 then
    local lspconfig = require('lspconfig')
    local configs = require('lspconfig.configs')

    -- Register bkmr_lsp if not already registered
    if not configs.bkmr_lsp then
        configs.bkmr_lsp = {
            default_config = {
                cmd = { 'bkmr-lsp' },
                filetypes = { 'markdown', 'text', 'lua', 'python', 'rust', 'javascript', 'typescript', 'sh', 'bash', 'yaml', 'toml', 'json' },
                root_dir = function(fname)
                    return lspconfig.util.find_git_ancestor(fname) or vim.fn.getcwd()
                end,
                settings = {},
            },
        }
    end

    lspconfig.bkmr_lsp.setup({
        capabilities = require('cmp_nvim_lsp').default_capabilities(),
        settings = {
            bkmr = {
                enableIncrementalCompletion = false
            }
        },
        on_attach = function(client, bufnr)
            -- Create bkmr-lsp custom commands
            vim.api.nvim_create_user_command('BkmrInsertPath', function()
                -- Use the modern LSP API and correct argument format
                vim.lsp.buf_request(0, 'workspace/executeCommand', {
                    command = "bkmr.insertFilepathComment",
                    arguments = {
                        vim.uri_from_bufnr(0)  -- Pass URI as string, not object
                    }
                }, function(err, result)
                    if err then
                        vim.notify("Error executing bkmr command: " .. tostring(err), vim.log.levels.ERROR)
                    elseif result then
                        -- The server returns a WorkspaceEdit that should be applied
                        vim.lsp.util.apply_workspace_edit(result, client.offset_encoding)
                    end
                end)
            end, { desc = "Insert filepath comment via bkmr-lsp" })
            
            -- Additional bkmr-lsp commands can be added here
        end
    })
end
```

**Usage in Other LSP Clients:**
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

**Build and Development:**
```bash
# Quick development cycle
make all-fast                    # Debug build + install (symlinked)
make build-fast                  # Debug build only
make install-debug               # Install debug version to ~/bin

# Release builds
make build                       # Release build with optimizations
make install                     # Install release version

# Code quality
make format                      # Format code with cargo fmt
make lint                        # Run clippy with fixes
make test                        # Run all tests
```

**Demo and Testing Scripts:**
```bash
# Language filtering demonstration
./scripts/demo_language_filtering.py

# Completion behavior testing
./scripts/test_completion_behavior.py

# Text replacement testing
./scripts/test_text_replacement.py

# Integration testing
./scripts/integration_test.sh

# LSP protocol testing
./scripts/test_lsp.py
./scripts/test_lsp.sh
```

**Development Logging:**
```bash
# View LSP server logs during development
make log-lsp                     # Tail LSP server logs (JSON formatted)
make log-plugin                  # Tail IntelliJ plugin logs (filtered)

# Clear logs and reset development environment
make init                        # Clear logs and reset
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

### Architecture Overview

The LSP server implements efficient language-aware filtering and universal snippet processing:

**Core Components:**
1. **Language ID Capture**: Captures and caches language IDs from `textDocument/didOpen` events
2. **FTS Query Building**: Builds optimized Full Text Search queries combining language-specific and universal snippets
3. **Universal Translation**: Automatically translates Rust syntax patterns to target languages using regex-based processing
4. **Cache Management**: Maintains document language state and cleans up on document close

**Processing Flow:**
```
Document Open → Language ID Cache → Completion Request → FTS Query Build → 
bkmr CLI Call → Universal Translation → LSP Response
```

**Example FTS Queries:**
```bash
# Language-specific with universal fallback
(tags:python AND tags:"_snip_") OR (tags:universal AND tags:"_snip_")

# With word-based filtering
((tags:rust AND tags:"_snip_") OR (tags:universal AND tags:"_snip_")) AND metadata:config*
```

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