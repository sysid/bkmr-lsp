#!/usr/bin/env python3
"""
Test LSP completion and count actual results returned

python3 scripts/test-gh-expect-15.py ~/bin/bkmr-lsp
"""

import json
import subprocess
import sys
import threading
import time


class LSPClient:
    def __init__(self, server_cmd: str):
        self.process = subprocess.Popen(
            server_cmd,
            shell=True,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=0
        )
        self.request_id = 0

        # Start stderr reader thread
        self.stderr_thread = threading.Thread(target=self._read_stderr, daemon=True)
        self.stderr_thread.start()

    def _read_stderr(self):
        """Read stderr in background thread"""
        try:
            while True:
                line = self.process.stderr.readline()
                if not line:
                    break
                # Only show important logs
                if "Successfully fetched" in line or "Returning" in line or "ERROR" in line or "WARN" in line:
                    print(f"[SERVER] {line.strip()}")
        except Exception as e:
            pass

    def send_request(self, method: str, params: dict, request_id: int = None) -> dict:
        if request_id is None:
            self.request_id += 1
            request_id = self.request_id

        message = {
            "jsonrpc": "2.0",
            "id": request_id,
            "method": method,
            "params": params
        }

        return self._send_message(message)

    def send_notification(self, method: str, params: dict):
        message = {
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        }

        self._send_message(message, expect_response=False)

    def _send_message(self, message: dict, expect_response: bool = True):
        content = json.dumps(message)
        lsp_message = f"Content-Length: {len(content)}\r\n\r\n{content}"

        try:
            self.process.stdin.write(lsp_message)
            self.process.stdin.flush()
        except BrokenPipeError:
            return None

        if expect_response:
            return self._read_response()
        return None

    def _read_response(self):
        try:
            # Skip log messages and find completion response
            while True:
                # Read Content-Length header
                header_line = self.process.stdout.readline()
                if not header_line:
                    return None

                if not header_line.startswith("Content-Length:"):
                    continue

                content_length = int(header_line.split(":")[1].strip())

                # Read empty line
                empty_line = self.process.stdout.readline()

                # Read content
                content = self.process.stdout.read(content_length)
                response = json.loads(content)

                # Skip log messages, return actual response
                if response.get("method") == "window/logMessage":
                    continue

                return response

        except Exception as e:
            return None

    def close(self):
        try:
            self.process.stdin.close()
            self.process.terminate()
            self.process.wait(timeout=2)
        except:
            self.process.kill()


def main():
    if len(sys.argv) != 2:
        print("Usage: python test-gh-expect-15.py <path-to-bkmr-lsp-binary>")
        sys.exit(1)

    server_cmd = sys.argv[1]
    client = LSPClient(server_cmd)

    try:
        print("Testing LSP completion results count...")

        # Initialize
        response = client.send_request("initialize", {
            "processId": None,
            "clientInfo": {"name": "test-client", "version": "0.1.0"},
            "capabilities": {
                "textDocument": {
                    "completion": {"completionItem": {"snippetSupport": True}}
                }
            },
            "workspaceFolders": None
        })

        if not response or "error" in response:
            print("❌ Initialize failed")
            return

        client.send_notification("initialized", {})
        time.sleep(0.1)  # Give server time to initialize

        # Send document with ":gh"
        client.send_notification("textDocument/didOpen", {
            "textDocument": {
                "uri": "file:///tmp/test.txt",
                "languageId": "text",
                "version": 1,
                "text": ":gh"
            }
        })

        # Request completion
        response = client.send_request("textDocument/completion", {
            "textDocument": {"uri": "file:///tmp/test.txt"},
            "position": {"line": 0, "character": 3},  # After ":gh"
            "context": {
                "triggerKind": 1,  # Manual invocation
                "triggerCharacter": None
            }
        })

        if response and "result" in response:
            result = response["result"]
            
            # Handle both Array and List response formats
            if isinstance(result, list):
                items = result
                response_type = "Array"
            elif isinstance(result, dict) and "items" in result:
                items = result["items"]
                response_type = f"List (incomplete={result.get('isIncomplete', False)})"
            else:
                items = []
                response_type = "Unknown"
            
            print(f"\n✅ LSP Server returned {len(items)} completion items ({response_type}):")

            for i, item in enumerate(items):
                label = item.get('label', 'No label')
                filter_text = item.get('filterText', 'No filter')
                sort_text = item.get('sortText', 'No sort')
                print(f"  {i+1:2d}. {label} (filter:{filter_text}, sort:{sort_text})")

            print(f"\n📊 Expected: 15 items")
            print(f"📊 Actual:   {len(items)} items")

            if len(items) != 15:
                print(f"❌ MISMATCH: LSP server is not returning all 15 items!")
            else:
                print(f"✅ SUCCESS: All items returned by LSP server")

        else:
            print("❌ No completion results from LSP server")

        # Cleanup
        client.send_request("shutdown", {})
        client.send_notification("exit", {})

    finally:
        client.close()


if __name__ == "__main__":
    main()
