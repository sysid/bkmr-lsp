#!/usr/bin/env python3
"""
Simple test script to demonstrate filetype extraction from LSP client.

This script sends textDocument/didOpen with different language_id values
and verifies the server extracts and logs the filetype information.

tail /tmp/lsp.log | grep -E '(Document opened|language|Using language filter)'
"""

import json
import subprocess
import sys
import time


def send_lsp_message(process, message):
    """Send a JSON-RPC message to the LSP server."""
    json_msg = json.dumps(message)
    content_length = len(json_msg.encode('utf-8'))
    header = f"Content-Length: {content_length}\r\n\r\n"
    full_message = header + json_msg

    print(f"→ {json_msg}")
    process.stdin.write(full_message.encode('utf-8'))
    process.stdin.flush()


def read_lsp_response(process):
    """Read a JSON-RPC response from the LSP server."""
    # Read header
    header_lines = []
    while True:
        line = process.stdout.readline().decode('utf-8')
        if line == '\r\n':
            break
        header_lines.append(line.strip())

    # Parse content length
    content_length = 0
    for line in header_lines:
        if line.startswith('Content-Length:'):
            content_length = int(line.split(':')[1].strip())
            break

    if content_length == 0:
        return None

    # Read content
    content = process.stdout.read(content_length).decode('utf-8')
    print(f"← {content}")

    try:
        return json.loads(content)
    except json.JSONDecodeError:
        return None


def test_filetype_extraction():
    """Test that the server extracts filetype from textDocument/didOpen."""

    # Start LSP server with debug logging
    print("Starting bkmr-lsp with debug logging...")
    print("Check /tmp/lsp.log for detailed server logs\n")

    try:
        process = subprocess.Popen(
            ['bkmr-lsp'],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env={'RUST_LOG': 'debug', 'PATH': 'target/debug'},
            text=False
        )
    except FileNotFoundError:
        print("Error: bkmr-lsp not found. Run 'make install-debug' first.")
        return False

    # Redirect stderr to log file
    with open('/tmp/lsp.log', 'w') as log_file:
        stderr_process = subprocess.Popen(
            ['tee', '/tmp/lsp.log'],
            stdin=process.stderr,
            stdout=log_file
        )

    try:
        # Initialize
        print("1. Initialize LSP server")
        init_msg = {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "processId": None,
                "capabilities": {}
            }
        }
        send_lsp_message(process, init_msg)
        read_lsp_response(process)

        # Send initialized
        print("\n2. Send initialized notification")
        initialized_msg = {
            "jsonrpc": "2.0",
            "method": "initialized",
            "params": {}
        }
        send_lsp_message(process, initialized_msg)
        time.sleep(0.1)  # Let server process

        # Test different file types
        test_files = [
            ("rust", "file:///test/example.rs", "fn main() {\n    println!(\"Hello\");\n}"),
            ("python", "file:///test/example.py", "#!/usr/bin/env python3\nprint('Hello')"),
            ("javascript", "file:///test/example.js", "console.log('Hello');"),
            ("go", "file:///test/example.go", "package main\n\nfunc main() {\n    println(\"Hello\")\n}"),
            ("c", "file:///test/example.c", "#include <stdio.h>\n\nint main() {\n    printf(\"Hello\\n\");\n}")
        ]

        for i, (language_id, uri, content) in enumerate(test_files, 3):
            print(f"\n{i}. Open {language_id} file")

            did_open_msg = {
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": {
                    "textDocument": {
                        "uri": uri,
                        "languageId": language_id,
                        "version": 1,
                        "text": content
                    }
                }
            }
            send_lsp_message(process, did_open_msg)
            time.sleep(0.1)  # Let server process

            # Test completion to trigger filetype usage
            print(f"   Request completion for {language_id}")
            completion_msg = {
                "jsonrpc": "2.0",
                "id": i,
                "method": "textDocument/completion",
                "params": {
                    "textDocument": {"uri": uri},
                    "position": {"line": 0, "character": 5},
                    "context": {"triggerKind": 1}
                }
            }
            send_lsp_message(process, completion_msg)
            response = read_lsp_response(process)

            # Close document
            did_close_msg = {
                "jsonrpc": "2.0",
                "method": "textDocument/didClose",
                "params": {
                    "textDocument": {"uri": uri}
                }
            }
            send_lsp_message(process, did_close_msg)
            time.sleep(0.1)

        # Shutdown
        print(f"\n{len(test_files) + 3}. Shutdown server")
        shutdown_msg = {
            "jsonrpc": "2.0",
            "id": 99,
            "method": "shutdown",
            "params": {}
        }
        send_lsp_message(process, shutdown_msg)
        read_lsp_response(process)

        exit_msg = {
            "jsonrpc": "2.0",
            "method": "exit",
            "params": {}
        }
        send_lsp_message(process, exit_msg)

        return True

    finally:
        try:
            process.terminate()
            stderr_process.terminate()
            process.wait(timeout=2)
            stderr_process.wait(timeout=2)
        except:
            process.kill()
            stderr_process.kill()


def main():
    print("=== bkmr-lsp Filetype Extraction Test ===\n")

    if test_filetype_extraction():
        print("\n✓ Test completed!")
        print("\nCheck the server logs for filetype extraction:")
        print("  tail /tmp/lsp.log | grep -E '(Document opened|language|Using language filter)'")
        print("\nYou should see log entries like:")
        print("  Document opened: file:///test/example.rs (language: rust)")
        print("  Document language ID: Some(\"rust\")")
        print("  Using language filter: rust")
        return 0
    else:
        print("\n✗ Test failed")
        return 1


if __name__ == '__main__':
    sys.exit(main())
