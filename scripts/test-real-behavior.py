#!/usr/bin/env python3
"""
Test the REAL nvim completion behavior by simulating what nvim actually does
"""

import json
import subprocess
import sys
import threading
import time
import os

class NvimLSPSimulator:
    def __init__(self, server_cmd):
        # Start server with debug logging
        env = os.environ.copy()
        env['RUST_LOG'] = 'debug'
        
        self.server = subprocess.Popen(
            server_cmd,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE, 
            stderr=subprocess.PIPE,
            text=True,
            bufsize=0,
            env=env
        )
        
        self.request_id = 0
        self.completion_requests = []
        self.bkmr_queries = []
        
        # Monitor stderr for bkmr queries
        self.stderr_thread = threading.Thread(target=self._monitor_stderr, daemon=True)
        self.stderr_thread.start()
        
    def _monitor_stderr(self):
        while True:
            line = self.server.stderr.readline()
            if not line:
                break
            
            if "Executing bkmr with args:" in line:
                self.bkmr_queries.append(line.strip())
                print(f"🔍 BKMR QUERY #{len(self.bkmr_queries)}: {line.strip()}")
            elif "Successfully fetched" in line:
                print(f"✅ {line.strip()}")
    
    def send_message(self, method, params, expect_response=True):
        if expect_response:
            self.request_id += 1
            message = {
                "jsonrpc": "2.0",
                "id": self.request_id,
                "method": method,
                "params": params
            }
        else:
            message = {
                "jsonrpc": "2.0",
                "method": method, 
                "params": params
            }
        
        content = json.dumps(message)
        lsp_message = f"Content-Length: {len(content)}\r\n\r\n{content}"
        
        print(f"📤 Sending: {method}")
        self.server.stdin.write(lsp_message)
        self.server.stdin.flush()
        
        if expect_response:
            return self._read_response()
        
        time.sleep(0.1)  # Give server time to process
    
    def _read_response(self):
        # Read Content-Length
        header = self.server.stdout.readline()
        if not header.startswith("Content-Length:"):
            return None
            
        length = int(header.split(":")[1].strip())
        
        # Read empty line
        self.server.stdout.readline()
        
        # Read content
        content = self.server.stdout.read(length)
        
        try:
            response = json.loads(content)
            # Skip log messages
            if response.get("method") == "window/logMessage":
                return self._read_response()
            return response
        except:
            return None
    
    def test_completion_sequence(self):
        """Test the exact sequence: ':' -> ':g' -> ':gh' as nvim would"""
        
        print("🧪 Testing REAL nvim completion behavior...")
        print("=" * 60)
        
        # Initialize
        init_response = self.send_message("initialize", {
            "processId": None,
            "clientInfo": {"name": "nvim", "version": "0.11.3"},
            "capabilities": {
                "textDocument": {
                    "completion": {
                        "completionItem": {"snippetSupport": True}
                    }
                }
            }
        })
        
        if not init_response or "error" in init_response:
            print("❌ Initialize failed")
            return
            
        self.send_message("initialized", {}, expect_response=False)
        
        # Open document
        uri = "file:///tmp/test-nvim.txt"
        self.send_message("textDocument/didOpen", {
            "textDocument": {
                "uri": uri,
                "languageId": "text",
                "version": 1,
                "text": ""
            }
        }, expect_response=False)
        
        # Test sequence
        test_cases = [
            {"content": ":", "pos": 1, "desc": "After typing ':'"},
            {"content": ":g", "pos": 2, "desc": "After typing 'g' (now ':g')"},
            {"content": ":gh", "pos": 3, "desc": "After typing 'h' (now ':gh')"}
        ]
        
        for i, test in enumerate(test_cases):
            print(f"\n📝 STEP {i+1}: {test['desc']}")
            print(f"   Document content: '{test['content']}'")
            
            # Update document
            self.send_message("textDocument/didChange", {
                "textDocument": {"uri": uri, "version": i + 2},
                "contentChanges": [{"text": test['content']}]
            }, expect_response=False)
            
            # THIS IS THE KEY: Test both trigger kinds that nvim might use
            
            # First: Manual completion (Ctrl+Space)
            print(f"   🎯 Testing manual completion...")
            response = self.send_message("textDocument/completion", {
                "textDocument": {"uri": uri},
                "position": {"line": 0, "character": test['pos']},
                "context": {
                    "triggerKind": 1,  # Invoked (manual)
                    "triggerCharacter": None
                }
            })
            
            if response and 'result' in response:
                result = response['result']
                if isinstance(result, list):
                    print(f"   📋 Got {len(result)} items (Array)")
                elif isinstance(result, dict) and 'items' in result:
                    incomplete = result.get('isIncomplete', False)
                    print(f"   📋 Got {len(result['items'])} items (List, incomplete={incomplete})")
            
            time.sleep(0.2)
        
        print(f"\n📊 FINAL RESULTS:")
        print(f"   Completion requests sent: {len(test_cases)}")
        print(f"   Total bkmr queries executed: {len(self.bkmr_queries)}")
        
        if len(self.bkmr_queries) == 1:
            print("   ❌ CLIENT-SIDE FILTERING: Only 1 bkmr query")
            print("   → nvim is caching results and filtering client-side")
        elif len(self.bkmr_queries) >= 3:
            print("   ✅ SERVER-SIDE FILTERING: Multiple bkmr queries") 
            print("   → Each completion request triggers new bkmr search")
        else:
            print(f"   ⚠️  UNCLEAR: {len(self.bkmr_queries)} bkmr queries")
        
        # Show query progression
        if len(self.bkmr_queries) > 1:
            print(f"\n🔍 Query progression:")
            for i, query in enumerate(self.bkmr_queries):
                if 'metadata:' in query:
                    filter_part = query.split('metadata:')[1].split('"')[0] if 'metadata:' in query else 'none'
                    print(f"   {i+1}. Filter: {filter_part}")
                else:
                    print(f"   {i+1}. No filter (broad search)")
    
    def cleanup(self):
        self.server.terminate()

if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("Usage: python test-real-behavior.py <lsp-server-path>")
        sys.exit(1)
    
    simulator = NvimLSPSimulator(sys.argv[1])
    
    try:
        simulator.test_completion_sequence()
    finally:
        simulator.cleanup()