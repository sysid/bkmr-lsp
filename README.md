# bkmr-lsp
Why this is safe:

No unsafe Rust; command output is parsed with serde_json so injection is impossible.

Only the snippet body is echoed back to the editor; no shell execution.

The skeleton follows the official tower-lsp README example
and mirrors how simple-completion-language-server does word/snippet completion

```bash
RUST_LOG=debug ./target/release/bkmr-lsp 2>lsp.log &
```

## Spelunking
```bash
# Fire up the server on one side, hexdump lets you see the exact bytes leaving the server.
./target/release/bkmr_lsp-poc | hexdump -C &
SERVER_PID=$!

# Initialise the session
# You should receive something like (header + JSON) – mind the ASCII 0d / 0a pairs
send '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' \
  | socat - "EXEC:cat >&0"

  # Ask for completionssend '{
  "jsonrpc":"2.0","id":2,
  "method":"textDocument/completion",
  "params":{
    "textDocument":{"uri":"file:///tmp/foo"},
    "position":{"line":0,"character":1},
    "context":{"triggerKind":1}
  }}' \
  | socat - "EXEC:cat >&0"
```
You’ll get back a `CompletionList` whose `insertText` fields are the first five snippets pulled from bkmr.
Feel free to change the server code, rebuild, and replay the same frames to see what changes—nothing beats visualising raw packets for LSP literacy.

### netcat
```bash
# Terminal 1: Start LSP server 
./target/release/bkmr-lsp

# Terminal 2: Send messages
echo 'Content-Length: 131

{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{"textDocument":{"completion":{"completionItem":{"snippetSupport":true}}}}}}' | nc localhost 3000
```
