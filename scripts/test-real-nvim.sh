#!/bin/bash

echo "🧪 Testing real nvim completion behavior..."

# Start LSP server with debug logging in background
echo "Starting LSP server with debug logging..."
RUST_LOG=debug ~/bin/bkmr-lsp 2>/tmp/nvim-lsp-test.log &
LSP_PID=$!

# Give LSP server time to start
sleep 1

# Monitor logs in background
tail -f /tmp/nvim-lsp-test.log | grep -E "(Executing bkmr|Successfully fetched|Returning.*completion)" &
TAIL_PID=$!

echo "✅ LSP server started (PID: $LSP_PID)"
echo "✅ Log monitoring started (PID: $TAIL_PID)"
echo ""
echo "Now test manually:"
echo "1. Open nvim: nvim /tmp/test-completion.txt"
echo "2. Configure LSP: :lua vim.lsp.start({name='bkmr-lsp',cmd={'$HOME/bin/bkmr-lsp'},filetypes={'*'}})"
echo "3. Type ':' and trigger completion (Ctrl+X Ctrl+O or Ctrl+Space)"
echo "4. Type 'g' and trigger completion again"  
echo "5. Type 'h' and trigger completion again"
echo ""
echo "Watch this terminal for bkmr query logs!"
echo "Press Ctrl+C when done testing."

# Wait for user to finish testing
trap "kill $LSP_PID $TAIL_PID 2>/dev/null; exit" INT
wait