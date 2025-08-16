#!/usr/bin/env python3
"""
============================================================================
test_completion_behavior.py - Incremental Typing Behavior Analysis
============================================================================

Purpose:
    Analyzes whether bkmr-lsp implements server-side or client-side filtering
    by testing incremental typing patterns and monitoring bkmr query execution.

Test Method:
    Simulates typing sequence: ":" → ":g" → ":gh" and counts bkmr queries

Expected Behaviors:
    • Client-side filtering (problematic): 1 bkmr query total
      - Only initial trigger executes bkmr search
      - Subsequent typing filters cached results on client
      - May miss relevant completions
    
    • Server-side filtering (correct): 3+ bkmr queries  
      - Each keystroke triggers new bkmr search
      - Filters become progressively more specific
      - Shows all relevant completions

Usage:
    python3 scripts/test_completion_behavior.py <path-to-bkmr-lsp-binary>

Examples:
    python3 scripts/test_completion_behavior.py ~/bin/bkmr-lsp
    python3 scripts/test_completion_behavior.py ./target/debug/bkmr-lsp

Output:
    - Real-time monitoring of bkmr command executions
    - Analysis of filtering behavior (server vs client-side)
    - Filter progression showing query refinement
    - Clear pass/fail determination

Diagnostics:
    This test helps diagnose completion issues where not all expected
    results appear in LSP clients (particularly in Neovim/Vim).
============================================================================
"""

import json
import subprocess
import sys
import threading
import time
import re
from typing import List, Dict, Any


class BkmrQueryMonitor:
    """Monitors and analyzes bkmr command executions during LSP completion testing."""
    
    def __init__(self):
        self.bkmr_commands = []
        self.completion_responses = []
    
    def add_bkmr_command(self, command_line: str):
        """Extract and store bkmr command details"""
        # Parse bkmr command: ["search", "--json", "--interpolate", "--ntags-prefix", "_snip_", "--limit", "50", "metadata:gh*"]
        try:
            # Remove brackets and quotes, split by comma
            args_str = command_line.strip('[]')
            args = [arg.strip(' "') for arg in args_str.split('", "')]
            
            # Extract the search filter (last argument if it starts with metadata:)
            search_filter = None
            if args and args[-1].startswith('metadata:'):
                search_filter = args[-1]
            
            self.bkmr_commands.append({
                'full_command': command_line,
                'args': args,
                'search_filter': search_filter,
                'timestamp': time.time()
            })
        except Exception as e:
            # Fallback: just store the raw command
            self.bkmr_commands.append({
                'full_command': command_line,
                'args': [],
                'search_filter': None,
                'timestamp': time.time()
            })
    
    def add_completion_response(self, response: Dict[str, Any]):
        """Store completion response for analysis"""
        if 'result' in response:
            result = response['result']
            if isinstance(result, list):
                item_count = len(result)
                response_type = 'Array'
            elif isinstance(result, dict) and 'items' in result:
                item_count = len(result['items'])
                response_type = 'List'
                is_incomplete = result.get('isIncomplete', False)
            else:
                item_count = 0
                response_type = 'Unknown'
                is_incomplete = None
                
            self.completion_responses.append({
                'type': response_type,
                'item_count': item_count,
                'is_incomplete': is_incomplete if response_type == 'List' else None,
                'timestamp': time.time()
            })
    
    def analyze_results(self):
        """Analyze captured data to determine server vs client-side filtering behavior."""
        print(f"\n📊 BEHAVIOR ANALYSIS RESULTS:")
        print(f"   Total bkmr commands executed: {len(self.bkmr_commands)}")
        print(f"   Total completion responses: {len(self.completion_responses)}")
        
        if len(self.bkmr_commands) == 0:
            print("❌ No bkmr commands detected - something went wrong")
            return False
            
        print(f"\n🔍 BKMR COMMANDS:")
        for i, cmd in enumerate(self.bkmr_commands):
            filter_info = f" → Filter: {cmd['search_filter']}" if cmd['search_filter'] else " → No filter"
            print(f"   {i+1}. {cmd['full_command'][:80]}...{filter_info}")
        
        print(f"\n📋 COMPLETION RESPONSES:")
        for i, resp in enumerate(self.completion_responses):
            incomplete_info = f", incomplete={resp['is_incomplete']}" if resp['is_incomplete'] is not None else ""
            print(f"   {i+1}. Type: {resp['type']}, Items: {resp['item_count']}{incomplete_info}")
        
        # Determine behavior
        print(f"\n🎯 FILTERING BEHAVIOR DETERMINATION:")
        if len(self.bkmr_commands) == 1:
            print("   ❌ CLIENT-SIDE FILTERING DETECTED (Problematic)")
            print("   → Only 1 bkmr query executed for initial trigger")
            print("   → Subsequent keystrokes filter cached results client-side")
            print("   → This can cause missing completions in some LSP clients")
            return False
        elif len(self.bkmr_commands) >= 3:
            print("   ✅ SERVER-SIDE FILTERING DETECTED (Optimal)")
            print("   → Multiple bkmr queries executed, one per keystroke")
            print("   → Each query refines the search filter progressively")
            
            # Analyze filter progression
            filters = [cmd['search_filter'] for cmd in self.bkmr_commands if cmd['search_filter']]
            if len(filters) >= 2:
                print(f"   → Filter progression: {' → '.join(filters)}")
                if any('gh' in f for f in filters):
                    print("   → Filters becoming more specific as expected ✅")
                    print("   → This ensures comprehensive completion coverage")
                else:
                    print("   → Filter progression may not be working as expected ⚠️")
            return True
        else:
            print(f"   ⚠️  UNEXPECTED BEHAVIOR: {len(self.bkmr_commands)} queries detected")
            print(f"   → Expected either 1 (client-side) or 3+ (server-side) queries")
            return False


class LSPClient:
    def __init__(self, server_cmd: str, monitor: BkmrQueryMonitor):
        self.monitor = monitor
        
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
        
        # Start stderr monitoring thread
        self.stderr_thread = threading.Thread(target=self._read_stderr, daemon=True)
        self.stderr_thread.start()

    def _read_stderr(self):
        """Monitor stderr for bkmr command executions"""
        try:
            while True:
                line = self.process.stderr.readline()
                if not line:
                    break
                
                # Look for bkmr command executions
                if "Executing bkmr with args:" in line:
                    # Extract the args part: "Executing bkmr with args: ["search", "--json", ...]"
                    args_start = line.find('[')
                    if args_start != -1:
                        args_part = line[args_start:].strip()
                        self.monitor.add_bkmr_command(args_part)
                        print(f"[BKMR CMD] {args_part}")
                
                # Show other important logs
                if any(keyword in line for keyword in ["Successfully fetched", "Returning", "ERROR", "WARN"]):
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
            # Read Content-Length header
            header_line = self.process.stdout.readline()
            if not header_line or not header_line.startswith("Content-Length:"):
                return None
                
            content_length = int(header_line.split(":")[1].strip())
            
            # Read empty line
            empty_line = self.process.stdout.readline()
            
            # Read content
            content = self.process.stdout.read(content_length)
            response = json.loads(content)
            
            # Handle log messages by reading next response
            if response.get("method") == "window/logMessage":
                return self._read_response()  # Recursively read next message
                
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


def test_completion_behavior(server_cmd: str):
    """Test completion behavior using incremental typing sequence: ':' → ':g' → ':gh'"""
    
    monitor = BkmrQueryMonitor()
    client = LSPClient(server_cmd, monitor)
    
    try:
        print("🧪 Testing completion behavior with incremental typing...")
        print("   Test sequence: ':' → ':g' → ':gh'")
        print("   Monitoring: bkmr command executions and filtering behavior")
        print("   Goal: Determine if filtering is server-side or client-side")
        
        # Initialize LSP
        response = client.send_request("initialize", {
            "processId": None,
            "clientInfo": {"name": "test-incremental", "version": "0.1.0"},
            "capabilities": {
                "textDocument": {
                    "completion": {"completionItem": {"snippetSupport": True}}
                }
            },
            "workspaceFolders": None
        })
        
        if not response or "error" in response:
            print("❌ Initialize failed")
            return False
            
        client.send_notification("initialized", {})
        time.sleep(0.1)
        
        # Test sequence
        test_cases = [
            {"content": ":", "description": "Type ':'", "position": 1},
            {"content": ":g", "description": "Type 'g'", "position": 2},
            {"content": ":gh", "description": "Type 'h'", "position": 3},
        ]
        
        uri = "file:///tmp/test-incremental.txt"
        
        # Initial document open
        client.send_notification("textDocument/didOpen", {
            "textDocument": {
                "uri": uri,
                "languageId": "text",
                "version": 1,
                "text": ""
            }
        })
        
        for i, test_case in enumerate(test_cases):
            print(f"\n📝 Step {i+1}: {test_case['description']} → Document: '{test_case['content']}'")
            
            # Update document content
            client.send_notification("textDocument/didChange", {
                "textDocument": {
                    "uri": uri,
                    "version": i + 2
                },
                "contentChanges": [{
                    "text": test_case['content']
                }]
            })
            
            # Small delay to let document sync
            time.sleep(0.05)
            
            # Request completion
            response = client.send_request("textDocument/completion", {
                "textDocument": {"uri": uri},
                "position": {"line": 0, "character": test_case['position']},
                "context": {
                    "triggerKind": 1,  # Manual invocation
                    "triggerCharacter": None
                }
            })
            
            if response:
                monitor.add_completion_response(response)
                if 'result' in response:
                    result = response['result']
                    if isinstance(result, list):
                        print(f"   → Got {len(result)} completion items (Array response)")
                    elif isinstance(result, dict) and 'items' in result:
                        incomplete = result.get('isIncomplete', False)
                        print(f"   → Got {len(result['items'])} completion items (List response, incomplete={incomplete})")
                    else:
                        print(f"   → Got unexpected response format")
                else:
                    print(f"   → No completion results")
            else:
                print(f"   → No response received")
            
            # Delay between requests
            time.sleep(0.1)
        
        # Analyze results and determine filtering behavior
        success = monitor.analyze_results()
        
        # Cleanup
        client.send_request("shutdown", {})
        client.send_notification("exit", {})
        
        return success
        
    finally:
        client.close()


def main():
    if len(sys.argv) != 2:
        print("Usage: python3 scripts/test_completion_behavior.py <path-to-bkmr-lsp-binary>")
        print("")
        print("Examples:")
        print("  python3 scripts/test_completion_behavior.py ~/bin/bkmr-lsp")
        print("  python3 scripts/test_completion_behavior.py ./target/debug/bkmr-lsp")
        sys.exit(1)
        
    server_cmd = sys.argv[1]
    
    print("=" * 80)
    print("🔬 COMPLETION BEHAVIOR ANALYSIS")
    print("=" * 80)
    print("📋 Testing: Server-side vs Client-side filtering behavior")
    print("🎯 Method: Incremental typing with bkmr query monitoring")
    print("=" * 80)
    
    success = test_completion_behavior(server_cmd)
    
    print("\n" + "=" * 80)
    if success:
        print("✅ RESULT: Server-side filtering is working optimally")
        print("   🔄 Each keystroke triggers a new bkmr query with refined filters")
        print("   🎯 This ensures comprehensive completion coverage")
        print("   💡 LSP clients should receive all relevant completions")
    else:
        print("❌ RESULT: Client-side filtering detected (potentially problematic)")
        print("   🔄 Only initial trigger executes bkmr query")
        print("   📱 Subsequent typing filters cached results client-side")
        print("   ⚠️  This may cause missing completions in some LSP clients")
        print("   🐛 Particularly affects Neovim/Vim completion behavior")
    print("=" * 80)


if __name__ == "__main__":
    main()