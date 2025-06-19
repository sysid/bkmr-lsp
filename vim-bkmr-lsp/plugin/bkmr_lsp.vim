" plugin/bkmr_lsp.vim
" vim-bkmr-lsp: Simple LSP integration
" Author: sysid
" Version: 3.2.0
" License: MIT

if exists('g:loaded_bkmr_lsp') || &compatible
  finish
endif
let g:loaded_bkmr_lsp = 1

" Check dependencies
if !exists('*lsp#register_server')
  echoerr 'vim-bkmr-lsp requires vim-lsp plugin. Please install prabirshrestha/vim-lsp'
  finish
endif

" Configuration
let g:bkmr_lsp_binary = get(g:, 'bkmr_lsp_binary', 'bkmr-lsp')
let g:bkmr_lsp_enabled = get(g:, 'bkmr_lsp_enabled', 1)

" Register the server and set omnifunc
if g:bkmr_lsp_enabled && executable(g:bkmr_lsp_binary)
  call lsp#register_server({
        \ 'name': 'bkmr-lsp',
        \ 'cmd': {server_info -> [g:bkmr_lsp_binary]},
        \ 'allowlist': ['*'],
        \ })

  " Ensure omnifunc is set for all buffers
  augroup bkmr_lsp_omnifunc
    autocmd!
    autocmd User lsp_buffer_enabled setlocal omnifunc=lsp#complete
  augroup END
endif

" Simple commands
command! BkmrLspStatus echo 'bkmr-lsp status: ' . lsp#get_server_status('bkmr-lsp')
command! BkmrLspTest call s:test_completion()

function! s:test_completion() abort
  echo 'Type ":ssl" then press <C-x><C-o> to test completion'
  echo 'Server status: ' . lsp#get_server_status('bkmr-lsp')
endfunction