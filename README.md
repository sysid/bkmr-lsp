# bkmr-lsp

Language Server Protocol (LSP) implementation for [bkmr](https://github.com/sysid/bkmr) snippet management.

## Overview

bkmr-lsp provides snippet completion for bkmr snippets in any LSP-compatible editor. Snippets are automatically
interpolated, delivering processed content rather than raw templates. Additionally it respects snippet tabstops, etc.

**Key Features:**
- **Language-aware filtering**: Snippets are filtered by file type (e.g., Rust files get only Rust snippets)
- **Universal snippets**: Language-agnostic snippets with natural Rust syntax that automatically adapt to target languages
- **Automatic interpolation**: Templates are processed using bkmr's `--interpolate` flag
- **Plain text snippets**: Snippets tagged with "plain" are treated as plain text without LSP snippet processing
- **Additional LSP commands**: Filepath comment insertion with automatic language detection

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

## CLI Options

```bash
# Disable bkmr template interpolation
bkmr-lsp --no-interpolation

# Show help and available options
bkmr-lsp --help

# Show version information
bkmr-lsp --version
```

### Plain Text Snippets

Snippets tagged with "plain" are treated as plain text, preventing LSP clients from interpreting snippet syntax like `$1`, `${2:default}`, etc.

Some content should be inserted literally without any LSP snippet processing:
- **Documentation templates**: Contains `${COMPANY}` or `${VERSION}` that should appear as literal text
- **Configuration files**: Raw templates with placeholder syntax
- **Shell scripts**: Variables like `$HOME` that shouldn't be treated as LSP placeholders

#### Usage

```bash
# Create a plain text snippet
bkmr add 'Config: ${DATABASE_URL}\nUser: ${USERNAME}' plain,_snip_ --title "Config Template"

# Regular snippet (with LSP processing)
bkmr add 'function ${1:name}() {\n    ${2:// implementation}\n}' javascript,_snip_ --title "JS Function"
```

### Template Interpolation

**Default behavior**: bkmr-lsp uses the `--interpolate` flag when calling the bkmr CLI, which processes template variables and functions before serving snippets to LSP clients.

#### Why?

bkmr templates support dynamic content generation through:
- **Variables**: `{{now}}`, `{{clipboard}}`, `{{file_stem}}`, etc.
- **Functions**: `{{date("+%Y-%m-%d")}}`, `{{path_relative()}}`, etc. 
- **Conditional logic**: `{{#if condition}}...{{/if}}`

**Example transformation:**
```bash
# Template stored in bkmr:
println!("Generated on {{now}} in {{file_stem}}");
// TODO: {{clipboard}}

# With interpolation (default):
println!("Generated on 2024-01-15 14:30:22 in main");
// TODO: copied text from clipboard

# Without interpolation (--no-interpolation):
println!("Generated on {{now}} in {{file_stem}}");
// TODO: {{clipboard}}
```

Use `--no-interpolation` when:
- You want to see the raw template syntax in completions  
- You prefer to handle template processing manually after snippet insertion
- Debugging template syntax or variables

## Configuration

### VS Code

Install an LSP extension and add to `settings.json`:

```json
{
  "languageServerExample.servers": {
    "bkmr-lsp": {
      "command": "bkmr-lsp",
      "args": [],
      "filetypes": ["rust", "javascript", "typescript", "python", "go", "java", "c", "cpp", "html", "css", "scss", "ruby", "php", "swift", "kotlin", "shell", "yaml", "json", "markdown", "xml", "vim"]
    }
  }
}
```

**To disable template interpolation:**
```json
{
  "languageServerExample.servers": {
    "bkmr-lsp": {
      "command": "bkmr-lsp",
      "args": ["--no-interpolation"],
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

**To disable template interpolation:**
```vim
if executable('bkmr-lsp')
  augroup LspBkmr
    autocmd!
    autocmd User lsp_setup call lsp#register_server({
      \ 'name': 'bkmr-lsp',
      \ 'cmd': {server_info->['bkmr-lsp', '--no-interpolation']},
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

**To disable template interpolation:**
```lua
require'lspconfig'.bkmr_lsp.setup{
  cmd = { "bkmr-lsp", "--no-interpolation" },
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

### Language-Aware Filtering

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
bkmr add 'export $HOME' _snip_,sh,plain --title export-home  # plain: do not interpret $HOME as snippet variable, keep literal
bkmr add '{{ "date -u +%Y-%m-%d %H:%M:%S" | shell }}' _snip_,universal --title date  # uses bkmr server-side interpolation
```

`bkmr` queries, generated by the LSP server for different languages:
```bash
# Shell file:
(tags:sh AND tags:"_snip_") OR (tags:universal AND tags:"_snip_")
# With word filter:
((tags:rust AND tags:"_snip_") OR (tags:universal AND tags:"_snip_")) AND metadata:hello*
```

### Universal Snippets

Universal snippets are written in Rust syntax and get automatically translated:

- **Python**: `// comment` becomes `# comment`
- **HTML**: `// comment` becomes `<!-- comment -->`
- **Indentation**: `    ` (4 spaces) becomes tabs for Go, 2 spaces for JavaScript, etc.
- **Block comments**: `/* comment */` adapts to target language syntax

See [UNIVERSAL_SNIPPETS.md](UNIVERSAL_SNIPPETS.md) for complete documentation.

### LSP Commands

The server provides LSP commands for additional functionality:

#### `bkmr.insertFilepathComment`
Insert the relative filepath as a comment at the beginning of the file.

**Example output:**
```rust
// src/backend.rs  <--- inserted at top of file
use tower_lsp::LanguageServer;
```

**Neovim Configuration:**

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
Most LSP clients can execute this command programmatically. For IntelliJ Platform IDEs, use the
[bkmr-intellij-plugin](../bkmr-intellij-plugin) which provides UI integration.


## Troubleshooting

### No Completions Appearing

1. Verify bkmr works: `bkmr search --json --interpolate 'tags:"_snip_"'`
2. Check bkmr version: `bkmr --version`
3. Test LSP server: `echo '{"jsonrpc":"2.0","method":"initialize","id":1,"params":{}}' | bkmr-lsp`

### LSP Placeholders Not Working

If LSP snippet navigation (`$1`, `${2:default}`) doesn't work:

**Problem**: Snippet might be tagged as "plain" or have malformed placeholder syntax
**Solutions**:
1. **Check if snippet is plain**: Plain text snippets (tagged with "plain") don't support LSP placeholders
2. **Verify placeholder syntax**: 
   - Simple tabstops: `$1`, `$2`, `$3`
   - Placeholders: `${1:default text}`, `${2:another default}`
   - Choices: `${1|option1,option2,option3|}`
3. **Remove plain tag**: If you want LSP processing, remove "plain" from snippet tags

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
  - Plain text snippets for literal content insertion (tag with "plain")
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
