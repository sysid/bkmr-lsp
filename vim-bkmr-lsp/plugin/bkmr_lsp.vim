" plugin/bkmr_lsp.vim
" vim-bkmr-lsp: Simplified and fixed version
" Author: sysid
" Version: 3.0.0
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
let g:bkmr_lsp_auto_complete = get(g:, 'bkmr_lsp_auto_complete', 1)
let g:bkmr_lsp_max_completions = get(g:, 'bkmr_lsp_max_completions', 50)

" Track registration state
let s:server_registered = 0

" Register the bkmr-lsp server
function! s:register_server() abort
  if s:server_registered
    return
  endif

  " Check if already registered
  try
    let l:info = lsp#get_server_info('bkmr-lsp')
    if !empty(l:info)
      let s:server_registered = 1
      return
    endif
  catch
  endtry

  call lsp#register_server({
        \ 'name': 'bkmr-lsp',
        \ 'cmd': {server_info -> [g:bkmr_lsp_binary]},
        \ 'allowlist': ['*'],
        \ 'capabilities': {
        \   'completionProvider': {
        \     'resolveProvider': v:false,
        \   },
        \   'executeCommandProvider': {
        \     'commands': ['bkmr.open', 'bkmr.refresh']
        \   }
        \ },
        \ 'config': {
        \   'bkmr': {
        \     'maxCompletions': g:bkmr_lsp_max_completions
        \   }
        \ },
        \ 'initialization_options': {},
        \ 'workspace_config': {},
        \ })

  let s:server_registered = 1
endfunction

" Initialize the plugin after functions are defined
if g:bkmr_lsp_enabled && executable(g:bkmr_lsp_binary)
  call s:register_server()

  augroup bkmr_lsp_setup
    autocmd!
    " Setup buffer when LSP is ready for any buffer
    autocmd User lsp_buffer_enabled call s:setup_current_buffer()
    " Also setup on BufEnter to catch files opened directly
    autocmd BufEnter * call s:maybe_setup_buffer()
    " Setup on InsertEnter to guarantee completion works
    autocmd InsertEnter * call s:ensure_completion_ready()
  augroup END
endif

" Check if buffer qualifies for bkmr-lsp
function! s:is_valid_buffer() abort
  return &buftype ==# '' && expand('%') !=# '' && &filetype !=# 'help'
endfunction

" Setup current buffer immediately
function! s:setup_current_buffer() abort
  if !s:is_valid_buffer() || exists('b:bkmr_lsp_setup_done')
    return
  endif

  call s:setup_buffer()
endfunction

" Maybe setup buffer (for BufEnter)
function! s:maybe_setup_buffer() abort
  if !s:is_valid_buffer() || exists('b:bkmr_lsp_setup_done') || !s:server_registered
    return
  endif

  " Small delay to let other LSP servers settle
  call timer_start(50, {-> s:setup_buffer_if_ready()})
endfunction

" Ensure completion is ready when entering insert mode
function! s:ensure_completion_ready() abort
  if !s:is_valid_buffer()
    return
  endif

  " If not set up yet, do it now with minimal delay
  if !exists('b:bkmr_lsp_setup_done')
    call s:setup_buffer_if_ready()
  endif

  " Ensure server is running and responsive
  if lsp#get_server_status('bkmr-lsp') !=# 'running'
    " Retry setup after short delay
    call timer_start(100, {-> s:setup_buffer_if_ready()})
  endif
endfunction

" Setup buffer if server is ready
function! s:setup_buffer_if_ready() abort
  if !s:is_valid_buffer() || exists('b:bkmr_lsp_setup_done')
    return
  endif

  let l:status = lsp#get_server_status('bkmr-lsp')
  if l:status ==# 'running'
    call s:setup_buffer()
  elseif l:status !=# 'not running'
    " Server is starting, wait a bit more
    call timer_start(200, {-> s:setup_buffer_if_ready()})
  endif
endfunction

" Setup buffer-local settings
function! s:setup_buffer() abort
  " Final check
  if exists('b:bkmr_lsp_setup_done') || !s:is_valid_buffer()
    return
  endif

  " Setup completion functions
  if &omnifunc ==# '' || &omnifunc ==# 'syntaxcomplete#Complete'
    setlocal omnifunc=lsp#complete
  elseif &omnifunc !=# 'lsp#complete'
    " Preserve existing omnifunc
    call s:setup_multi_omnifunc()
  endif

  if &completefunc ==# ''
    setlocal completefunc=lsp#complete
  endif

  " Define plug mappings
  inoremap <buffer> <Plug>(bkmr-lsp-complete) <C-x><C-o>
  nnoremap <buffer> <Plug>(bkmr-lsp-complete) :call <SID>trigger_completion()<CR>
  nnoremap <buffer> <Plug>(bkmr-lsp-open) :call <SID>open_snippet()<CR>

  " Default mappings only if not already mapped
  if !hasmapto('<Plug>(bkmr-lsp-complete)', 'i')
    imap <buffer> <C-Space> <Plug>(bkmr-lsp-complete)
  endif
  if !hasmapto('<Plug>(bkmr-lsp-complete)', 'n')
    nmap <buffer> <leader>bc <Plug>(bkmr-lsp-complete)
  endif
  if !hasmapto('<Plug>(bkmr-lsp-open)', 'n')
    nmap <buffer> <leader>bo <Plug>(bkmr-lsp-open)
  endif

  " Setup auto-completion
  if g:bkmr_lsp_auto_complete
    augroup bkmr_lsp_buffer
      autocmd! * <buffer>
      autocmd TextChangedI <buffer> call s:auto_complete()
    augroup END
  endif

  " Mark buffer as set up
  let b:bkmr_lsp_setup_done = 1
endfunction

" Setup multiple omnifunc support
function! s:setup_multi_omnifunc() abort
  let l:original_omnifunc = &omnifunc
  let b:bkmr_original_omnifunc = l:original_omnifunc

  function! BkmrMultiOmnifunc(findstart, base) abort
    if a:findstart
      return lsp#complete(a:findstart, a:base)
    else
      let l:lsp_results = lsp#complete(0, a:base)
      if exists('b:bkmr_original_omnifunc') && b:bkmr_original_omnifunc !=# ''
        try
          let l:original_results = call(b:bkmr_original_omnifunc, [0, a:base])
          if type(l:original_results) == type([])
            return l:lsp_results + l:original_results
          endif
        catch
        endtry
      endif
      return l:lsp_results
    endif
  endfunction

  setlocal omnifunc=BkmrMultiOmnifunc
endfunction

" Trigger completion from normal mode
function! s:trigger_completion() abort
  if lsp#get_server_status('bkmr-lsp') !=# 'running'
    echo 'bkmr-lsp server not running'
    return
  endif

  if mode() ==# 'i'
    return "\<C-x>\<C-o>"
  else
    startinsert
    call feedkeys("\<C-x>\<C-o>", 'n')
  endif
endfunction

" Auto-complete when typing
function! s:auto_complete() abort
  if mode() !=# 'i' || pumvisible() || lsp#get_server_status('bkmr-lsp') !=# 'running'
    return
  endif

  let l:col = col('.') - 1
  let l:line = getline('.')

  " Don't trigger on whitespace or at start of line
  if l:col == 0 || l:line[l:col - 1] =~# '\s'
    return
  endif

  " Find word start
  let l:word_start = match(l:line[0:l:col-1], '\k*$')
  if l:word_start == -1
    return
  endif

  let l:current_word = l:line[l:word_start:l:col-1]

  " Trigger completion for words >= 2 characters
  if len(l:current_word) >= 2
    call feedkeys("\<C-x>\<C-o>", 'n')
  endif
endfunction

" Open snippet by ID
function! s:open_snippet() abort
  if lsp#get_server_status('bkmr-lsp') !=# 'running'
    echo 'bkmr-lsp server not running'
    return
  endif

  let l:id = input('Snippet ID: ')
  if empty(l:id) || l:id !~# '^\d\+$'
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
command! BkmrLspReset call s:reset_buffer()
command! -nargs=? BkmrLspOpen call s:open_snippet_cmd(<q-args>)

" Reset buffer setup
function! s:reset_buffer() abort
  unlet! b:bkmr_lsp_setup_done
  unlet! b:bkmr_original_omnifunc
  call s:setup_buffer()
  echo 'Buffer reset and setup complete'
endfunction

" Show status
function! s:show_status() abort
  echo 'bkmr-lsp Status:'
  echo '  Binary: ' . g:bkmr_lsp_binary . (executable(g:bkmr_lsp_binary) ? ' ✓' : ' ✗')
  echo '  Enabled: ' . (g:bkmr_lsp_enabled ? 'Yes' : 'No')
  echo '  Server registered: ' . (s:server_registered ? 'Yes' : 'No')
  echo '  Server status: ' . lsp#get_server_status('bkmr-lsp')
  echo '  Buffer setup: ' . (exists('b:bkmr_lsp_setup_done') ? 'Done' : 'Not done')
  echo '  Current filetype: ' . &filetype
  echo '  Omnifunc: ' . &omnifunc
  echo '  Auto-complete: ' . (g:bkmr_lsp_auto_complete ? 'Enabled' : 'Disabled')
  echo '  bkmr available: ' . (executable('bkmr') ? 'Yes' : 'No')

  try
    let l:servers = lsp#get_server_names()
    echo '  All LSP servers: ' . join(l:servers, ', ')
  catch
    echo '  All LSP servers: Unable to retrieve'
  endtry
endfunction

" Open snippet command wrapper
function! s:open_snippet_cmd(id) abort
  if empty(a:id)
    call s:open_snippet()
  else
    if lsp#get_server_status('bkmr-lsp') !=# 'running'
      echo 'bkmr-lsp server not running'
      return
    endif

    call lsp#send_request('bkmr-lsp', {
          \ 'method': 'workspace/executeCommand',
          \ 'params': {
          \   'command': 'bkmr.open',
          \   'arguments': [str2nr(a:id)]
          \ },
          \ })
  endif
endfunction
