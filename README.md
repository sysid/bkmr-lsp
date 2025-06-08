# bkmr-lsp

## Spelunking
```bash
# Fire up the server on one side, hexdump lets you see the exact bytes leaving the server.
./target/release/bkmr-lsp-poc | hexdump -C &
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
# bkmr-lsp
