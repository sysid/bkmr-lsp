#!/usr/bin/env python3
"""
============================================================================
lsp_debug_client.py - Enhanced LSP Protocol Debugging Client
============================================================================

Purpose:
    Professional LSP client for testing and debugging bkmr-lsp server with
    comprehensive error handling, detailed logging, and protocol validation.

Features:
    • Complete LSP protocol implementation
    • Detailed request/response logging with pretty-printing
    • Server stderr monitoring and filtering
    • Robust error handling and timeout management
    • Process lifecycle management
    • Real-time protocol debugging

Usage:
    python3 scripts/lsp_debug_client.py <path-to-bkmr-lsp-binary>

Examples:
    python3 scripts/lsp_debug_client.py ~/bin/bkmr-lsp
    python3 scripts/lsp_debug_client.py ./target/debug/bkmr-lsp
    python3 scripts/lsp_debug_client.py ./target/release/bkmr-lsp

Output:
    - Structured LSP message logging (requests and responses)
    - Server stderr output with filtering
    - Connection status and error diagnostics
    - Completion results analysis
    - Process management status

Use Cases:
    • LSP server development and debugging
    • Protocol compliance testing
    • Communication troubleshooting
    • Performance analysis
    • Integration testing
============================================================================
"""

import json
import subprocess
import sys
import threading
import time
from typing import Dict, Any, Optional


class LSPClient:
    """Enhanced LSP client with comprehensive debugging and error handling."""
    
    def __init__(self, server_cmd: str):
        print(f"🚀 Starting LSP server: {server_cmd}")

        self.process = subprocess.Popen(
            server_cmd,
            shell=True,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=0  # Unbuffered
        )
        self.request_id = 0

        # Start stderr reader thread
        self.stderr_thread = threading.Thread(target=self._read_stderr, daemon=True)
        self.stderr_thread.start()

        # Give server time to start
        time.sleep(0.5)

        # Validate server started successfully
        if self.process.poll() is not None:
            raise RuntimeError(f"❌ Server process exited immediately with code {self.process.returncode}")

    def _read_stderr(self):
        """Monitor and filter server stderr output in background thread."""
        try:
            for line in iter(self.process.stderr.readline, ''):
                if line:
                    # Filter important server messages
                    line = line.rstrip()
                    if any(keyword in line for keyword in ['ERROR', 'WARN', 'Successfully fetched', 'Executing bkmr']):
                        print(f"🔍 [SERVER] {line}")
                    elif 'DEBUG' in line and 'completion' in line.lower():
                        print(f"📊 [DEBUG] {line}")
        except Exception as e:
            # Silent failure for stderr monitoring
            pass

    def send_message(self, message: Dict[str, Any]) -> None:
        """Send a JSON-RPC message to the LSP server"""
        json_str = json.dumps(message)
        content = f"Content-Length: {len(json_str)}\r\n\r\n{json_str}"

        print(f"📤 >>> SENDING LSP MESSAGE:")
        print(f"    Content-Length: {len(json_str)}")
        print(f"    Method: {message.get('method', 'N/A')}")
        print(f"    ID: {message.get('id', 'N/A')}")
        print(json.dumps(message, indent=2))
        print()

        try:
            self.process.stdin.write(content)
            self.process.stdin.flush()
        except BrokenPipeError:
            raise RuntimeError("❌ Server stdin pipe broken - server may have crashed")

    def read_message(self, timeout: float = 5.0) -> Optional[Dict[str, Any]]:
        """Read a JSON-RPC message from the LSP server with timeout"""
        start_time = time.time()

        try:
            # Read Content-Length header with timeout
            while True:
                if time.time() - start_time > timeout:
                    print(f"⏰ TIMEOUT: No response after {timeout} seconds")
                    return None

                # Check if process died
                if self.process.poll() is not None:
                    print(f"❌ ERROR: Server process died with exit code {self.process.returncode}")
                    return None

                line = self.process.stdout.readline()
                if not line:
                    time.sleep(0.1)
                    continue

                print(f"[DEBUG] Read header line: {repr(line)}")

                if line.startswith("Content-Length:"):
                    content_length = int(line.split(":")[1].strip())
                    print(f"[DEBUG] Content length: {content_length}")
                    break

            # Skip empty line
            empty_line = self.process.stdout.readline()
            print(f"[DEBUG] Empty line: {repr(empty_line)}")

            # Read the JSON content
            content = self.process.stdout.read(content_length)
            print(f"[DEBUG] Raw content: {repr(content)}")

            message = json.loads(content)

            print(f"📥 <<< RECEIVED LSP MESSAGE:")
            print(f"    Content-Length: {content_length}")
            print(f"    Method: {message.get('method', 'N/A')}")
            print(f"    ID: {message.get('id', 'N/A')}")
            if 'error' in message:
                print(f"    ❌ ERROR: {message.get('error', {})}")
            print(json.dumps(message, indent=2))
            print()

            return message

        except json.JSONDecodeError as e:
            print(f"❌ JSON ERROR: Failed to decode server response: {e}")
            print(f"    Raw content: {repr(content)}")
            return None
        except Exception as e:
            print(f"❌ COMMUNICATION ERROR: {e}")
            return None

    def next_id(self) -> int:
        self.request_id += 1
        return self.request_id

    def initialize(self) -> Optional[Dict[str, Any]]:
        """Send initialize request"""
        message = {
            "jsonrpc": "2.0",
            "id": self.next_id(),
            "method": "initialize",
            "params": {
                "processId": None,
                "clientInfo": {
                    "name": "test-client",
                    "version": "0.1.0"
                },
                "capabilities": {
                    "textDocument": {
                        "completion": {
                            "completionItem": {
                                "snippetSupport": True
                            }
                        }
                    }
                },
                "workspaceFolders": None
            }
        }

        self.send_message(message)
        return self.read_message()

    def initialized(self) -> None:
        """Send initialized notification"""
        message = {
            "jsonrpc": "2.0",
            "method": "initialized",
            "params": {}
        }
        self.send_message(message)

    def completion(self, uri: str = "file:///tmp/test.txt", line: int = 0, character: int = 1) -> Optional[
        Dict[str, Any]]:
        """Request completion"""
        message = {
            "jsonrpc": "2.0",
            "id": self.next_id(),
            "method": "textDocument/completion",
            "params": {
                "textDocument": {
                    "uri": uri
                },
                "position": {
                    "line": line,
                    "character": character
                },
                "context": {
                    "triggerKind": 1  # Invoked
                }
            }
        }

        self.send_message(message)
        return self.read_message()

    def shutdown(self) -> Optional[Dict[str, Any]]:
        """Send shutdown request"""
        message = {
            "jsonrpc": "2.0",
            "id": self.next_id(),
            "method": "shutdown",
            "params": None
        }

        self.send_message(message)
        return self.read_message()

    def exit(self) -> None:
        """Send exit notification"""
        message = {
            "jsonrpc": "2.0",
            "method": "exit",
            "params": None
        }
        self.send_message(message)

    def close(self):
        """Close the LSP client"""
        if self.process:
            self.process.terminate()
            self.process.wait()


def test_lsp_server(server_path: str):
    """Execute comprehensive LSP server testing sequence."""
    print("=" * 80)
    print("🔧 Enhanced LSP Server Debug Session")
    print("=" * 80)
    print(f"💻 Server binary: {server_path}")
    print(f"📄 Protocol: Language Server Protocol (LSP) 3.17")
    print(f"🐛 Purpose: Debug and validate bkmr-lsp functionality")
    print("=" * 80)
    print()

    try:
        client = LSPClient(server_path)
    except RuntimeError as e:
        print(f"Failed to start server: {e}")
        return

    try:
        # Step 1: Initialize
        print("=== 1. INITIALIZE ===")
        init_response = client.initialize()
        if not init_response:
            print("❌ FAILED: No initialize response received")
            return
        elif 'error' in init_response:
            print(f"❌ FAILED: Initialize error: {init_response['error']}")
            return
        else:
            print("✅ SUCCESS: LSP server initialized")
            capabilities = init_response.get('result', {}).get('capabilities', {})
            if capabilities:
                print(f"    🛠️  Server capabilities: {list(capabilities.keys())}")

        # Step 2: Initialized notification
        print("\n=== 2. INITIALIZED NOTIFICATION ===")
        client.initialized()
        print("✅ Initialized notification sent")
        time.sleep(1)  # Give server time to process

        # Step 3: Test completion
        print("\n=== 3. COMPLETION REQUEST ===")
        completion_response = client.completion()
        if completion_response:
            if 'error' in completion_response:
                print(f"❌ COMPLETION ERROR: {completion_response['error']}")
            else:
                result = completion_response.get("result")
                if result:
                    item_count = len(result) if isinstance(result, list) else len(result.get('items', []))
                    print(f"✅ SUCCESS: Received {item_count} completion items")
                    
                    # Show first few items with details
                    items = result if isinstance(result, list) else result.get('items', [])
                    for i, item in enumerate(items[:3]):
                        label = item.get('label', 'No label')
                        kind = item.get('kind', 'Unknown')
                        detail = item.get('detail', '')
                        print(f"    {i + 1}. {label} (kind: {kind}) {detail}")
                    
                    if len(items) > 3:
                        print(f"    ... and {len(items) - 3} more items")
                else:
                    print("⚠️  Empty completion result")
        else:
            print("❌ FAILED: No completion response received")

        # Step 4: Shutdown
        print("\n=== 4. SHUTDOWN SEQUENCE ===")
        shutdown_response = client.shutdown()
        if shutdown_response:
            print("✅ Shutdown request acknowledged")
        else:
            print("⚠️  No shutdown response")
        
        client.exit()
        print("✅ Exit notification sent")
        
        print("\n" + "=" * 80)
        print("✅ LSP debug session completed successfully")
        print("=" * 80)

    except KeyboardInterrupt:
        print("\n❌ Test interrupted by user")
    except Exception as e:
        print(f"❌ Test failed with error: {e}")
        import traceback
        traceback.print_exc()
    finally:
        client.close()


def main():
    if len(sys.argv) != 2:
        print("Usage: python3 scripts/lsp_debug_client.py <path-to-bkmr-lsp-binary>")
        print("")
        print("Examples:")
        print("  python3 scripts/lsp_debug_client.py ~/bin/bkmr-lsp")
        print("  python3 scripts/lsp_debug_client.py ./target/debug/bkmr-lsp")
        print("  python3 scripts/lsp_debug_client.py ./target/release/bkmr-lsp")
        print("")
        print("Purpose: Debug LSP protocol communication with bkmr-lsp server")
        sys.exit(1)

    server_path = sys.argv[1]

    # Verify server binary exists
    import os
    if not os.path.exists(server_path):
        print(f"❌ ERROR: Server binary not found: {server_path}")
        print("")
        print("🔧 Build the project first:")
        print("  cargo build --release      # For release build")
        print("  cargo build                # For debug build")
        print("  make install-debug         # Build and install debug version")
        sys.exit(1)

    test_lsp_server(server_path)


if __name__ == "__main__":
    main()