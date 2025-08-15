# Development Guide

## Server Output and Logging

The LSP server output goes to different locations depending on how you run it:

### Default LSP Server Output

**stderr**: The server logs to stderr by default (see `main.rs:77`)
```rust
.with_writer(std::io::stderr)
```

### Development Logging

**~/bkmr-lsp.log**: When running with `RUST_LOG` environment variable, output typically gets redirected here. The Makefile shows this pattern:

```bash
make log-lsp    # Tails ~/bkmr-lsp.log with JSON formatting
```

### Manual Logging Setup

To capture server output during development:

```bash
# Redirect stderr to a log file
RUST_LOG=debug bkmr-lsp 2>~/bkmr-lsp.log

# Or use the make target to watch logs
make log-lsp    # Tails ~/bkmr-lsp.log and formats JSON output
```

### LSP Client Integration

When run by an LSP client (VS Code, Vim, IntelliJ), the server output typically goes to:
- The client's LSP logs (varies by editor)
- For IntelliJ plugin development: `make log-plugin` shows filtered completion logs

### Quick Check

To see if the server is producing output:
```bash
ls -la ~/bkmr-lsp.log    # Check if log file exists
tail -f ~/bkmr-lsp.log   # Watch live output
```

The `make init` command clears this log file as part of development setup.