#!/usr/bin/env python3
"""
Simple test to check if each completion request triggers a new bkmr query
"""

import subprocess
import sys
import time
import threading

def run_test(server_binary):
    print("🧪 Testing if completion requests trigger new bkmr queries...")
    
    # Start LSP server with debug logging
    proc = subprocess.Popen([
        server_binary
    ], 
    stdin=subprocess.PIPE, 
    stdout=subprocess.PIPE, 
    stderr=subprocess.PIPE,
    text=True,
    env={"RUST_LOG": "debug"}
    )
    
    bkmr_command_count = 0
    
    def monitor_stderr():
        nonlocal bkmr_command_count
        while True:
            line = proc.stderr.readline()
            if not line:
                break
            if "Executing bkmr with args:" in line:
                bkmr_command_count += 1
                print(f"[BKMR QUERY #{bkmr_command_count}] {line.strip()}")
    
    # Start monitoring thread
    monitor_thread = threading.Thread(target=monitor_stderr, daemon=True)
    monitor_thread.start()
    
    # Send messages manually
    messages = [
        # Initialize
        '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}',
        # Initialized
        '{"jsonrpc":"2.0","method":"initialized","params":{}}',
        # Open document with ":"
        '{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///tmp/test.txt","languageId":"text","version":1,"text":":"}}}',
        # Request completion for ":"
        '{"jsonrpc":"2.0","id":2,"method":"textDocument/completion","params":{"textDocument":{"uri":"file:///tmp/test.txt"},"position":{"line":0,"character":1},"context":{"triggerKind":1}}}',
        # Update document to ":g"
        '{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":"file:///tmp/test.txt","version":2},"contentChanges":[{"text":":g"}]}}',
        # Request completion for ":g"
        '{"jsonrpc":"2.0","id":3,"method":"textDocument/completion","params":{"textDocument":{"uri":"file:///tmp/test.txt"},"position":{"line":0,"character":2},"context":{"triggerKind":1}}}',
        # Update document to ":gh"
        '{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":"file:///tmp/test.txt","version":3},"contentChanges":[{"text":":gh"}]}}',
        # Request completion for ":gh"
        '{"jsonrpc":"2.0","id":4,"method":"textDocument/completion","params":{"textDocument":{"uri":"file:///tmp/test.txt"},"position":{"line":0,"character":3},"context":{"triggerKind":1}}}',
        # Shutdown
        '{"jsonrpc":"2.0","id":5,"method":"shutdown","params":{}}',
        '{"jsonrpc":"2.0","method":"exit","params":{}}'
    ]
    
    for i, msg in enumerate(messages):
        content_length = len(msg)
        lsp_msg = f"Content-Length: {content_length}\r\n\r\n{msg}"
        
        print(f"\n📤 Sending message {i+1}...")
        try:
            proc.stdin.write(lsp_msg)
            proc.stdin.flush()
            time.sleep(0.3)  # Give time for processing
        except BrokenPipeError:
            print("❌ Server disconnected")
            break
    
    # Give time for final processing
    time.sleep(1)
    
    proc.terminate()
    proc.wait()
    
    print(f"\n📊 RESULTS:")
    print(f"   Total bkmr queries executed: {bkmr_command_count}")
    
    if bkmr_command_count == 1:
        print("   ❌ CLIENT-SIDE FILTERING: Only 1 query (initial trigger)")
        print("   → This explains missing gh-* completions in nvim")
        return False
    elif bkmr_command_count >= 3:
        print("   ✅ SERVER-SIDE FILTERING: Multiple queries per keystroke")
        print("   → Each completion request triggers new bkmr search")
        return True
    else:
        print(f"   ⚠️  UNCLEAR: {bkmr_command_count} queries (expected 1 or 3+)")
        return False

if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("Usage: python simple-incremental-test.py <lsp-binary-path>")
        sys.exit(1)
    
    success = run_test(sys.argv[1])
    print(f"\n{'✅ PASS' if success else '❌ FAIL'}")