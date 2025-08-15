#!/bin/bash
# Start bkmr-lsp with detailed logging
export RUST_LOG=debug
exec ~/bin/bkmr-lsp 2>/tmp/bkmr-lsp-test.log