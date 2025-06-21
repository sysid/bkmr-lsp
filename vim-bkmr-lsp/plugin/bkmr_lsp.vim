" plugin/bkmr_lsp.vim
" vim-bkmr-lsp: Simple LSP integration
" Author: sysid
" Version: 3.2.0
" License: MIT

if exists('g:loaded_bkmr_lsp') || &compatible
  finish
endif
let g:loaded_bkmr_lsp = 1

"---------------------------------------------------------------------
" Settings -----------------------------------------------------------
"---------------------------------------------------------------------
" Enable the plugin unless the user explicitly disables it
if !exists('g:bkmr_lsp_enabled')
  let g:bkmr_lsp_enabled = 1
endif

"---------------------------------------------------------------------
" Omnifunc / Completefunc hook ---------------------------------------
"---------------------------------------------------------------------
if g:bkmr_lsp_enabled
  augroup bkmr_lsp_omnifunc
    autocmd!

    " ── vim-lsp ─────────────────────────────────────────────────────
    if exists('*lsp#complete')
      autocmd User lsp_buffer_enabled setlocal omnifunc=lsp#complete
    endif

    " ── yegappan/lsp (Vim9 LSP) ─────────────────────────────────────
    if exists('*lsp#CompleteFunc')
      autocmd User LspSetup setlocal completefunc=lsp#CompleteFunc
    endif

  augroup END
endif
