
#!/bin/bash
# debug_lsp.sh - Debug script to test bkmr-lsp server

set -e

echo "=== bkmr-lsp Debug Script ==="
echo

# Check if binary exists
if [ ! -f "./target/release/bkmr-lsp" ]; then
    echo "ERROR: bkmr-lsp binary not found. Building..."
    cargo build --release
fi

# Test 1: Check if binary is executable
echo "1. Testing binary execution..."
if ./target/release/bkmr-lsp --help 2>/dev/null; then
    echo "   ✓ Binary executes successfully"
else
    echo "   ✗ Binary execution failed"
    echo "   Trying to see what happens:"
    ./target/release/bkmr-lsp --help || echo "   Failed with exit code $?"
fi

# Test 2: Check bkmr availability
echo
echo "2. Testing bkmr availability..."
if command -v bkmr >/dev/null 2>&1; then
    echo "   ✓ bkmr command found"

    # Test bkmr search
    echo "   Testing bkmr search..."
    if bkmr search -t _snip_ --json --limit 5 >/dev/null 2>&1; then
        echo "   ✓ bkmr search works"

        # Show snippet count
        local count=$(bkmr search -t _snip_ --json 2>/dev/null | jq '. | length' 2>/dev/null || echo "?")
        echo "   Found $count snippets"
    else
        echo "   ✗ bkmr search failed"
        echo "   Error output:"
        bkmr search -t _snip_ --json --limit 5 2>&1 | head -5
    fi
else
    echo "   ✗ bkmr command not found"
fi

# Test 3: Test LSP server startup
echo
echo "3. Testing LSP server startup..."

# Create a simple initialize message
cat > /tmp/lsp_init.json << 'EOF'
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{"textDocument":{"completion":{"completionItem":{"snippetSupport":true}}}}}}
EOF

echo "   Sending initialize message..."

# Start server and send message with timeout
if timeout 5s bash -c '
    echo "Content-Length: $(wc -c < /tmp/lsp_init.json)"
    echo
    cat /tmp/lsp_init.json
' | RUST_LOG=debug ./target/release/bkmr-lsp 2>/tmp/lsp_debug.log &

SERVER_PID=$!
sleep 2

if kill -0 $SERVER_PID 2>/dev/null; then
    echo "   ✓ Server started successfully (PID: $SERVER_PID)"
    kill $SERVER_PID 2>/dev/null || true
    wait $SERVER_PID 2>/dev/null || true
else
    echo "   ✗ Server failed to start or crashed"
fi

# Show debug output
if [ -f /tmp/lsp_debug.log ]; then
    echo
    echo "4. Server debug output:"
    echo "   --- Debug Log ---"
    cat /tmp/lsp_debug.log
    echo "   --- End Log ---"
fi

# Test 4: Simple LSP message test
echo
echo "5. Testing simple LSP communication..."

# Create test script
cat > /tmp/test_lsp_simple.py << 'EOF'
#!/usr/bin/env python3
import subprocess
import json
import sys
import time

def send_lsp_message(proc, message):
    json_str = json.dumps(message)
    content = f"Content-Length: {len(json_str)}\r\n\r\n{json_str}"
    proc.stdin.write(content.encode())
    proc.stdin.flush()

def main():
    try:
        # Start server
        proc = subprocess.Popen(
            ['./target/release/bkmr-lsp'],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE
        )

        # Send initialize
        init_msg = {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "capabilities": {
                    "textDocument": {
                        "completion": {
                            "completionItem": {"snippetSupport": True}
                        }
                    }
                }
            }
        }

        send_lsp_message(proc, init_msg)

        # Wait a bit and check if process is still alive
        time.sleep(1)

        if proc.poll() is None:
            print("   ✓ Server is responsive")

            # Try to read response
            try:
                # Read with timeout
                proc.stdout.settimeout(2)
                response = proc.stdout.read(1024)
                if response:
                    print(f"   ✓ Got response: {len(response)} bytes")
                else:
                    print("   - No response received")
            except:
                print("   - Timeout reading response")

            proc.terminate()
            proc.wait(timeout=2)
        else:
            print(f"   ✗ Server exited with code: {proc.returncode}")
            stderr_output = proc.stderr.read().decode()
            if stderr_output:
                print(f"   Error: {stderr_output}")

    except Exception as e:
        print(f"   ✗ Error: {e}")

if __name__ == "__main__":
    main()
EOF

if command -v python3 >/dev/null 2>&1; then
    python3 /tmp/test_lsp_simple.py
else
    echo "   - Python3 not available, skipping LSP communication test"
fi

# Cleanup
rm -f /tmp/lsp_init.json /tmp/lsp_debug.log /tmp/test_lsp_simple.py

echo
echo "=== Debug completed ==="
echo "If server is crashing, check the debug log above"
echo "If bkmr is not working, install/configure bkmr first"
