" plugin/bkmr_lsp.vim
" vim-bkmr-lsp: Simplified Vim plugin for bkmr snippet manager via LSP
" Author: sysid
" Version: 2.0.0
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

" Configuration with sensible defaults
let g:bkmr_lsp_binary = get(g:, 'bkmr_lsp_binary', 'bkmr-lsp')
let g:bkmr_lsp_enabled = get(g:, 'bkmr_lsp_enabled', 1)

" Register LSP server automatically if enabled and binary exists
if g:bkmr_lsp_enabled && executable(g:bkmr_lsp_binary)
  augroup bkmr_lsp_setup
    autocmd!
    autocmd User lsp_setup call s:register_server()
    autocmd User lsp_buffer_enabled call s:setup_buffer()
  augroup END
endif

" Register the bkmr-lsp server
function! s:register_server() abort
  call lsp#register_server({
        \ 'name': 'bkmr-lsp',
        \ 'cmd': {server_info -> [g:bkmr_lsp_binary]},
        \ 'allowlist': ['*'],
        \ 'capabilities': {
        \   'completionProvider': {
        \     'resolveProvider': v:false,
        \   },
        \ },
        \ })
endfunction

" Setup buffer-local settings
function! s:setup_buffer() abort
  if lsp#get_server_status('bkmr-lsp') !=# 'running'
    return
  endif

  " Enable completion
  setlocal omnifunc=lsp#complete
  setlocal completefunc=lsp#complete

  " Define plug mappings
  inoremap <buffer> <Plug>(bkmr-lsp-complete) <C-x><C-o>
  nnoremap <buffer> <Plug>(bkmr-lsp-complete) :call <SID>trigger_completion()<CR>
  nnoremap <buffer> <Plug>(bkmr-lsp-open) :call <SID>open_snippet()<CR>

  " Default mappings (can be overridden by user)
  if !hasmapto('<Plug>(bkmr-lsp-complete)', 'i')
    imap <buffer> <C-Space> <Plug>(bkmr-lsp-complete)
  endif
  if !hasmapto('<Plug>(bkmr-lsp-complete)', 'n')
    nmap <buffer> <leader>bc <Plug>(bkmr-lsp-complete)
  endif
  if !hasmapto('<Plug>(bkmr-lsp-open)', 'n')
    nmap <buffer> <leader>bo <Plug>(bkmr-lsp-open)
  endif

  " Setup auto-completion on text changes
  augroup bkmr_lsp_buffer
    autocmd! * <buffer>
    autocmd TextChangedI <buffer> call s:auto_complete()
  augroup END
endfunction

" Trigger completion from normal mode
function! s:trigger_completion() abort
  if mode() ==# 'i'
    return "\<C-x>\<C-o>"
  else
    startinsert
    call feedkeys("\<C-x>\<C-o>", 'n')
  endif
endfunction

" Auto-complete when typing
function! s:auto_complete() abort
  " Only trigger if we're in insert mode and no menu is already visible
  if mode() !=# 'i' || pumvisible()
    return
  endif

  let l:col = col('.') - 1
  let l:line = getline('.')

  " Don't trigger on empty line or just whitespace
  if l:col == 0 || l:line[l:col - 1] =~# '\s'
    return
  endif

  " Get the current word being typed
  let l:word_start = match(l:line[0:l:col-1], '\k*$')
  if l:word_start == -1
    return
  endif

  let l:current_word = l:line[l:word_start:l:col-1]

  " Only trigger if we have at least 2 characters
  if len(l:current_word) >= 2
    call timer_start(100, {-> s:delayed_complete()})
  endif
endfunction

" Delayed completion to avoid too frequent triggers
function! s:delayed_complete() abort
  if mode() ==# 'i' && !pumvisible()
    call feedkeys("\<C-x>\<C-o>", 'n')
  endif
endfunction

" Open snippet by ID
function! s:open_snippet() abort
  let l:id = input('Snippet ID: ')
  if empty(l:id) || !match(l:id, '^\d\+$')
    return
  endif

  call lsp#send_request('bkmr-lsp', {
        \ 'method': 'workspace/executeCommand',
        \ 'params': {
        \   'command': 'bkmr.open',
        \   'arguments': [str2nr(l:id)]
        \ },
        \ })
endfunction

" Commands
command! BkmrLspStatus call s:show_status()
command! -nargs=? BkmrLspOpen call s:open_snippet_cmd(<q-args>)

" Show status
function! s:show_status() abort
  echo 'bkmr-lsp Status:'
  echo '  Binary: ' . g:bkmr_lsp_binary . (executable(g:bkmr_lsp_binary) ? ' ✓' : ' ✗')
  echo '  Server: ' . lsp#get_server_status('bkmr-lsp')
  echo '  bkmr: ' . (executable('bkmr') ? 'available' : 'missing')
endfunction

" Open snippet command wrapper
function! s:open_snippet_cmd(id) abort
  if empty(a:id)
    call s:open_snippet()
  else
    call lsp#send_request('bkmr-lsp', {
          \ 'method': 'workspace/executeCommand',
          \ 'params': {
          \   'command': 'bkmr.open',
          \   'arguments': [str2nr(a:id)]
          \ },
          \ })
  endif
endfunction