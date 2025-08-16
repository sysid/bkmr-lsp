#!/bin/bash
# ============================================================================
# integration_test.sh - Complete LSP Server Integration Testing Pipeline
# ============================================================================
# 
# Purpose: Comprehensive testing of bkmr-lsp server functionality including:
#   - Project building and compilation
#   - Unit and integration test execution  
#   - bkmr CLI availability and snippet validation
#   - LSP protocol communication testing
#   - Python and raw LSP message testing capabilities
#
# Usage:
#   ./scripts/integration_test.sh [test_mode]
#
# Test Modes:
#   raw    - Test with raw LSP protocol messages
#   python - Test with Python LSP client (default)
#   both   - Run both raw and Python tests
#
# Examples:
#   ./scripts/integration_test.sh         # Python client test (default)
#   ./scripts/integration_test.sh raw     # Raw LSP message test 
#   ./scripts/integration_test.sh both    # Both test methods
#
# Prerequisites:
#   - Rust toolchain installed
#   - bkmr CLI tool installed with snippets tagged as '_snip_'
#   - jq command for JSON processing (optional)
#   - Python 3 for Python client testing
# ============================================================================

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to send LSP message with proper headers
send_lsp_message() {
    local message="$1"
    local length=${#message}
    printf "Content-Length: %d\r\n\r\n%s" "$length" "$message"
}

# Function to test LSP server manually with raw messages
test_lsp_raw() {
    echo -e "${YELLOW}=== Testing LSP Server with Raw Messages ===${NC}"

    # Start the LSP server in background
    echo "Starting bkmr-lsp server..."
    ./target/release/bkmr-lsp &
    SERVER_PID=$!

    # Give it a moment to start
    sleep 1

    echo -e "${GREEN}Server started with PID: $SERVER_PID${NC}"

    # Create a named pipe for two-way communication
    PIPE_IN=$(mktemp -u)
    PIPE_OUT=$(mktemp -u)
    mkfifo "$PIPE_IN" "$PIPE_OUT"

    # Start server with pipes
    ./target/release/bkmr-lsp < "$PIPE_IN" > "$PIPE_OUT" &
    SERVER_PID=$!

    # Test messages
    echo -e "${YELLOW}Sending initialize message...${NC}"
    {
        send_lsp_message '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{"textDocument":{"completion":{"completionItem":{"snippetSupport":true}}}}}}'

        sleep 1

        echo -e "${YELLOW}Sending initialized notification...${NC}"
        send_lsp_message '{"jsonrpc":"2.0","method":"initialized","params":{}}'

        sleep 1

        echo -e "${YELLOW}Sending completion request...${NC}"
        send_lsp_message '{"jsonrpc":"2.0","id":2,"method":"textDocument/completion","params":{"textDocument":{"uri":"file:///tmp/test.txt"},"position":{"line":0,"character":1}}}'

        sleep 1

        echo -e "${YELLOW}Sending shutdown...${NC}"
        send_lsp_message '{"jsonrpc":"2.0","id":3,"method":"shutdown","params":null}'

        sleep 1

        send_lsp_message '{"jsonrpc":"2.0","method":"exit","params":null}'
    } > "$PIPE_IN" &

    # Read responses
    timeout 5 cat "$PIPE_OUT" | head -20

    # Cleanup
    kill $SERVER_PID 2>/dev/null || true
    rm -f "$PIPE_IN" "$PIPE_OUT"

    echo -e "${GREEN}Raw test completed${NC}"
}

# Function to test with Python script if available
test_lsp_python() {
    if ! command -v python3 &> /dev/null; then
        echo -e "${YELLOW}Python3 not available, skipping Python test${NC}"
        return
    fi

    echo -e "${YELLOW}=== Testing LSP Server with Python Client ===${NC}"

    if [ -f "test_lsp.py" ]; then
        python3 test_lsp.py ./target/release/bkmr-lsp
    else
        echo -e "${RED}test_lsp.py not found${NC}"
    fi
}

# Function to check if bkmr is available and has snippets
check_bkmr() {
    echo -e "${YELLOW}=== Checking bkmr availability ===${NC}"

    if ! command -v bkmr &> /dev/null; then
        echo -e "${RED}bkmr command not found! Please install bkmr first.${NC}"
        echo "You can install it with: cargo install bkmr"
        return 1
    fi

    echo -e "${GREEN}bkmr found${NC}"

    # Check if we have any snippets
    echo -e "${YELLOW}Checking for bkmr snippets...${NC}"
    SNIPPET_COUNT=$(bkmr search --json --ntags-prefix _snip_ 2>/dev/null | jq '. | length' 2>/dev/null || echo "0")

    if [ "$SNIPPET_COUNT" -eq 0 ]; then
        echo -e "${YELLOW}\u26a0\ufe0f  No snippets found. Adding a test snippet...${NC}"
        bkmr add "console.log('Hello from bkmr-lsp!');" test,javascript --type snip --title "LSP Test Snippet" || {
            echo -e "${RED}\u274c Failed to add test snippet${NC}"
            return 1
        }
        echo -e "${GREEN}\u2713 Test snippet added successfully${NC}"
    else
        echo -e "${GREEN}\u2713 Found $SNIPPET_COUNT snippets with _snip_ tag${NC}"
    fi

    # Show first few snippets
    echo -e "${BLUE}Sample snippets:${NC}"
    bkmr search --json --ntags-prefix _snip_ --limit 3 | jq -r '.[] | "  \u2022 \(.title)"' 2>/dev/null || {
        bkmr search --ntags-prefix _snip_ --limit 3 | head -5 | sed 's/^/  \u2022 /'
    }
}

# Function to build the project
build_project() {
    echo -e "${YELLOW}=== Building bkmr-lsp ===${NC}"

    if ! cargo build --release; then
        echo -e "${RED}Build failed!${NC}"
        exit 1
    fi

    echo -e "${GREEN}Build successful${NC}"
}

# Function to run tests
run_tests() {
    echo -e "${YELLOW}=== Running Tests ===${NC}"

    if ! cargo test; then
        echo -e "${RED}Tests failed!${NC}"
        exit 1
    fi

    echo -e "${GREEN}Tests passed${NC}"
}

# Display usage information
show_usage() {
    echo -e "${BLUE}Usage: $0 [test_mode]${NC}"
    echo ""
    echo "Test Modes:"
    echo "  raw    - Test with raw LSP protocol messages"
    echo "  python - Test with Python LSP client (default)"
    echo "  both   - Run both raw and Python tests"
    echo ""
    echo "Examples:"
    echo "  $0         # Python client test (default)"
    echo "  $0 raw     # Raw LSP message test"
    echo "  $0 both    # Both test methods"
}

# Main function
main() {
    echo -e "${GREEN}============================================================================${NC}"
    echo -e "${GREEN}bkmr-lsp Integration Testing Pipeline${NC}"
    echo -e "${GREEN}============================================================================${NC}"
    echo ""

    # Check if we're in the right directory
    if [ ! -f "Cargo.toml" ]; then
        echo -e "${RED}❌ Error: Cargo.toml not found.${NC}"
        echo -e "${YELLOW}Please run this script from the bkmr-lsp project directory.${NC}"
        exit 1
    fi

    # Build the project
    build_project

    # Run unit tests
    run_tests

    # Check bkmr availability
    check_bkmr || {
        echo -e "${RED}bkmr check failed. Please fix bkmr setup before testing LSP server.${NC}"
        exit 1
    }

    # Test the LSP server
    case "${1:-python}" in
        "raw")
            test_lsp_raw
            ;;
        "python")
            test_lsp_python
            ;;
        "both")
            test_lsp_python
            echo
            test_lsp_raw
            ;;
        "help"|"--help"|"-h")
            show_usage
            exit 0
            ;;
        *)
            echo -e "${RED}❌ Unknown test mode: $1${NC}"
            echo ""
            show_usage
            exit 1
            ;;
    esac

    echo -e "${GREEN}============================================================================${NC}"
    echo -e "${GREEN}✅ All integration tests completed successfully${NC}"
    echo -e "${GREEN}============================================================================${NC}"
}

# Run main function
main "$@"