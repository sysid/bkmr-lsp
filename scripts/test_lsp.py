#!/usr/bin/env python3
"""
Enhanced LSP client for testing bkmr-lsp server with better error handling and debugging.
"""

import json
import subprocess
import sys
import threading
import time
from typing import Dict, Any, Optional


class LSPClient:
    def __init__(self, server_cmd: str):
        print(f"Starting server: {server_cmd}")

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

        # Check if process is still running
        if self.process.poll() is not None:
            raise RuntimeError(f"Server process exited immediately with code {self.process.returncode}")

    def _read_stderr(self):
        """Read stderr output from server in background"""
        try:
            for line in iter(self.process.stderr.readline, ''):
                if line:
                    print(f"[SERVER STDERR] {line.rstrip()}")
        except:
            pass

    def send_message(self, message: Dict[str, Any]) -> None:
        """Send a JSON-RPC message to the LSP server"""
        json_str = json.dumps(message)
        content = f"Content-Length: {len(json_str)}\r\n\r\n{json_str}"

        print(f">>> SENDING:")
        print(f"Content-Length: {len(json_str)}")
        print(json.dumps(message, indent=2))
        print()

        try:
            self.process.stdin.write(content)
            self.process.stdin.flush()
        except BrokenPipeError:
            raise RuntimeError("Server stdin pipe broken - server may have crashed")

    def read_message(self, timeout: float = 5.0) -> Optional[Dict[str, Any]]:
        """Read a JSON-RPC message from the LSP server with timeout"""
        start_time = time.time()

        try:
            # Read Content-Length header with timeout
            while True:
                if time.time() - start_time > timeout:
                    print(f"TIMEOUT: No response after {timeout} seconds")
                    return None

                # Check if process died
                if self.process.poll() is not None:
                    print(f"ERROR: Server process died with exit code {self.process.returncode}")
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

            print(f"<<< RECEIVED:")
            print(f"Content-Length: {content_length}")
            print(json.dumps(message, indent=2))
            print()

            return message

        except json.JSONDecodeError as e:
            print(f"ERROR: Failed to decode JSON: {e}")
            print(f"Raw content was: {repr(content)}")
            return None
        except Exception as e:
            print(f"ERROR: Exception reading message: {e}")
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
    """Test the LSP server with a sequence of operations"""
    print("=== Starting Enhanced LSP Server Test ===")
    print(f"Server command: {server_path}")
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
            print("❌ Failed to get initialize response")
            return
        else:
            print("✅ Initialize successful")

        # Step 2: Initialized notification
        print("=== 2. INITIALIZED ===")
        client.initialized()
        time.sleep(1)  # Give server time to process and cache

        # Step 3: Test completion
        print("=== 3. COMPLETION REQUEST ===")
        completion_response = client.completion()
        if completion_response:
            result = completion_response.get("result")
            if result:
                print(f"✅ Got {len(result)} completion items")
                for i, item in enumerate(result[:3]):  # Show first 3 items
                    print(f"  {i + 1}. {item.get('label', 'No label')} - {item.get('kind', 'No kind')}")
            else:
                print("⚠️  No completion results")
        else:
            print("❌ No completion response")

        # Step 4: Shutdown
        print("=== 4. SHUTDOWN ===")
        shutdown_response = client.shutdown()
        if shutdown_response:
            print("✅ Shutdown successful")
        client.exit()

        print("=== Test completed ===")

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
        print("Usage: python test_lsp_debug.py <path-to-bkmr-lsp-binary>")
        print("Example: python test_lsp_debug.py ./target/release/bkmr-lsp")
        sys.exit(1)

    server_path = sys.argv[1]

    # Verify server binary exists
    import os
    if not os.path.exists(server_path):
        print(f"❌ Server binary not found: {server_path}")
        print("Make sure to build the project first: cargo build --release")
        sys.exit(1)

    test_lsp_server(server_path)


if __name__ == "__main__":
    main()