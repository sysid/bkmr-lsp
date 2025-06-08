#!/bin/bash

# Function to send LSP message with proper headers
send_lsp_message() {
    local message="$1"
    local length=${#message}
    printf "Content-Length: %d\r\n\r\n%s" "$length" "$message"
}

# Start the LSP server
./target/release/bkmr-lsp &
SERVER_PID=$!

# Give it a moment to start
sleep 1

# Test 1: Initialize
echo "=== Testing Initialize ==="
send_lsp_message '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{"textDocument":{"completion":{"completionItem":{"snippetSupport":true}}}}}}' | socat - EXEC:"cat"

echo -e "\n=== Testing Completion ==="
# Test 2: Request completion
send_lsp_message '{"jsonrpc":"2.0","id":2,"method":"textDocument/completion","params":{"textDocument":{"uri":"file:///tmp/test.txt"},"position":{"line":0,"character":1}}}' | socat - EXEC:"cat"

# Clean up
kill $SERVER_PID 2>/dev/null