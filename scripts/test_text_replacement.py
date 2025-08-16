#!/usr/bin/env python3
"""
============================================================================
test_text_replacement.py - LSP Text Replacement Verification
============================================================================

Purpose:
    Verifies that LSP completion items use proper TextEdit for replacing
    query words instead of appending at cursor position.

Test Case:
    - Opens document with query word (e.g., "md")  
    - Requests completion at end of query
    - Verifies completion items have TextEdit with correct range
    - Confirms range covers entire query word for replacement

Expected Behavior:
    ✅ CompletionItem has textEdit field with proper Range
    ✅ Range covers the entire query word (start to end position)
    ✅ new_text contains the interpolated snippet content
    ❌ Should NOT use insertText for appending

Usage:
    python3 scripts/test_text_replacement.py <path-to-bkmr-lsp-binary>

Example:
    python3 scripts/test_text_replacement.py ~/bin/bkmr-lsp
============================================================================
"""

import json
import subprocess
import sys
import threading
import time

class SimpleTextEditTest:
    def __init__(self, server_cmd):
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
        
        # Start stderr reader
        self.stderr_thread = threading.Thread(target=self._read_stderr, daemon=True)
        self.stderr_thread.start()
        time.sleep(0.5)

    def _read_stderr(self):
        try:
            for line in iter(self.process.stderr.readline, ''):
                if line:
                    print(f"[SERVER] {line.rstrip()}")
        except:
            pass

    def send_message(self, message):
        content = json.dumps(message)
        lsp_message = f"Content-Length: {len(content)}\r\n\r\n{content}"
        self.process.stdin.write(lsp_message)
        self.process.stdin.flush()

    def read_response(self):
        try:
            while True:
                header_line = self.process.stdout.readline()
                if not header_line or not header_line.startswith("Content-Length:"):
                    continue
                content_length = int(header_line.split(":")[1].strip())
                self.process.stdout.readline()  # empty line
                content = self.process.stdout.read(content_length)
                response = json.loads(content)
                if response.get("method") == "window/logMessage":
                    continue
                return response
        except:
            return None

    def test_text_edit_completion(self):
        print("🧪 Testing TextEdit completion replacement...")
        
        # Initialize
        self.request_id += 1
        init_msg = {
            "jsonrpc": "2.0",
            "id": self.request_id,
            "method": "initialize",
            "params": {
                "processId": None,
                "capabilities": {
                    "textDocument": {
                        "completion": {"completionItem": {"snippetSupport": True}}
                    }
                }
            }
        }
        self.send_message(init_msg)
        response = self.read_response()
        if not response:
            print("❌ Initialize failed")
            return False
        print("✅ Initialized")

        # Send initialized notification
        self.send_message({"jsonrpc": "2.0", "method": "initialized", "params": {}})
        time.sleep(0.1)

        # Open document with query text
        uri = "file:///tmp/test-textedit.txt"
        content = "md"  # This should trigger completion for "md"
        
        self.send_message({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "markdown",
                    "version": 1,
                    "text": content
                }
            }
        })
        time.sleep(0.1)

        # Request completion at position after "md" 
        self.request_id += 1
        completion_msg = {
            "jsonrpc": "2.0",
            "id": self.request_id,
            "method": "textDocument/completion",
            "params": {
                "textDocument": {"uri": uri},
                "position": {"line": 0, "character": 2},  # After "md"
                "context": {"triggerKind": 1}
            }
        }
        self.send_message(completion_msg)
        
        response = self.read_response()
        if not response or 'result' not in response:
            print("❌ No completion response")
            return False

        result = response['result']
        items = result if isinstance(result, list) else result.get('items', [])
        
        if not items:
            print("❌ No completion items returned")
            return False
            
        print(f"✅ Got {len(items)} completion items")
        
        # Check first item for TextEdit
        first_item = items[0]
        print(f"First item label: {first_item.get('label', 'N/A')}")
        
        if 'textEdit' in first_item:
            text_edit = first_item['textEdit']
            if isinstance(text_edit, dict) and 'range' in text_edit:
                range_info = text_edit['range']
                new_text = text_edit.get('newText', '')
                print(f"✅ Found TextEdit:")
                print(f"   Range: {range_info}")
                print(f"   New text preview: {new_text[:50]}...")
                
                # Verify range replaces the query word
                start = range_info['start']
                end = range_info['end']
                if start['line'] == 0 and start['character'] == 0 and end['character'] == 2:
                    print("✅ Range correctly covers 'md' word (0-2)")
                    return True
                else:
                    print(f"❌ Range incorrect: expected 0-2, got {start['character']}-{end['character']}")
                    return False
            else:
                print("❌ TextEdit format unexpected")
                print(f"TextEdit: {text_edit}")
                return False
        else:
            print("❌ No textEdit field found")
            if 'insertText' in first_item:
                print(f"   Found insertText instead: {first_item['insertText'][:50]}...")
            print(f"   Item keys: {list(first_item.keys())}")
            return False

    def close(self):
        self.process.terminate()
        self.process.wait()

def main():
    if len(sys.argv) != 2:
        print("Usage: python3 test_text_edit.py <lsp-server-path>")
        sys.exit(1)
    
    test = SimpleTextEditTest(sys.argv[1])
    try:
        success = test.test_text_edit_completion()
        print(f"\n{'✅ PASS' if success else '❌ FAIL'}: TextEdit replacement test")
        return 0 if success else 1
    finally:
        test.close()

if __name__ == "__main__":
    sys.exit(main())