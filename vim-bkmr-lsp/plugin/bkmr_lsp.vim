" plugin/bkmr_lsp.vim
" vim-bkmr-lsp: Simple LSP integration
" Author: sysid
" Version: 3.2.0
" License: MIT

if exists('g:loaded_bkmr_lsp') || &compatible
  finish
endif
let g:loaded_bkmr_lsp = 1

if g:bkmr_lsp_enabled
  augroup bkmr_lsp_omnifunc
    autocmd!
    autocmd User lsp_buffer_enabled setlocal omnifunc=lsp#complete
  augroup END
endif