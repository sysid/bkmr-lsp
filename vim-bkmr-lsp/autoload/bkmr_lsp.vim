" autoload/bkmr_lsp.vim
" Minimal autoload functions for vim-bkmr-lsp

" Check if server is running
function! bkmr_lsp#is_server_running() abort
  return lsp#get_server_status('bkmr-lsp') ==# 'running'
endfunction

" Get server info
function! bkmr_lsp#get_server_info() abort
  return lsp#get_server_info('bkmr-lsp')
endfunction

" Public API for other plugins
function! bkmr_lsp#open_snippet(id) abort
  if !bkmr_lsp#is_server_running()
    echoerr 'bkmr-lsp server not running'
    return
  endif

  call lsp#send_request('bkmr-lsp', {
        \ 'method': 'workspace/executeCommand',
        \ 'params': {
        \   'command': 'bkmr.open',
        \   'arguments': [a:id]
        \ },
        \ })
endfunction