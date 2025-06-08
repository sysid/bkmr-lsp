" autoload/bkmr_lsp.vim
" Core functionality for vim-bkmr-lsp plugin

let s:cached_snippets = []
let s:last_completion_pos = [0, 0]

" Register the bkmr-lsp server with vim-lsp
function! bkmr_lsp#register_server() abort
  if !executable(g:bkmr_lsp_binary)
    echohl ErrorMsg
    echo 'bkmr-lsp binary not found: ' . g:bkmr_lsp_binary
    echohl None
    return
  endif

  call lsp#register_server({
        \ 'name': 'bkmr-lsp',
        \ 'cmd': {server_info -> [g:bkmr_lsp_binary]},
        \ 'allowlist': ['*'],
        \ 'capabilities': {
        \   'completionProvider': {
        \     'triggerCharacters': g:bkmr_lsp_trigger_chars,
        \     'resolveProvider': v:false,
        \   },
        \   'executeCommandProvider': {
        \     'commands': ['bkmr.open', 'bkmr.refresh']
        \   }
        \ },
        \ 'initialization_options': {
        \   'maxCompletions': g:bkmr_lsp_max_completions
        \ }
        \ })

  echo 'bkmr-lsp server registered successfully'
endfunction

" Setup buffer-local key mappings
function! bkmr_lsp#setup_buffer_mappings() abort
  " Check if bkmr-lsp server info exists (returns dict if exists, 0 if not)
  let l:server_info = lsp#get_server_info('bkmr-lsp')
  if empty(l:server_info)
    return
  endif

  " Set up omnifunc for completion
  setlocal omnifunc=lsp#complete
  setlocal signcolumn=yes

  " Apply user-configured mappings for normal mode
  for [action, mapping] in items(g:bkmr_lsp_mappings)
    if !empty(mapping)
      if action ==# 'complete'
        " Special handling for completion - works in both modes
        execute printf('nmap <buffer> %s :call bkmr_lsp#trigger_completion()<CR>', mapping)
        execute printf('imap <buffer> %s <C-x><C-o>', mapping)
      else
        execute printf('nmap <buffer> %s :call bkmr_lsp#%s()<CR>', mapping, action)
      endif
    endif
  endfor

  " Additional insert mode mappings for convenience
  imap <buffer> <C-Space> <C-x><C-o>
  imap <buffer> <C-@> <C-x><C-o>
endfunction

" Trigger completion manually
function! bkmr_lsp#trigger_completion() abort
  " Check if LSP is available and force setup if needed
  if !bkmr_lsp#is_server_running()
    echohl ErrorMsg
    echo 'bkmr-lsp server not running. Check :LspStatus'
    echohl None
    return
  endif

  " Ensure omnifunc is set
  if empty(&omnifunc)
    setlocal omnifunc=lsp#complete
    " Only call setup if we have server info
    let l:server_info = lsp#get_server_info('bkmr-lsp')
    if !empty(l:server_info)
      call bkmr_lsp#setup_buffer_mappings()
    endif
  endif

  if mode() ==# 'i'
    return "\<C-x>\<C-o>"
  else
    startinsert
    call feedkeys("\<C-x>\<C-o>", 'n')
  endif
endfunction

" Auto-trigger completion on specific characters
function! bkmr_lsp#auto_trigger_completion() abort
  if !g:bkmr_lsp_auto_complete
    return
  endif

  let l:col = col('.') - 1
  let l:line = getline('.')

  if l:col == 0
    return
  endif

  let l:char = l:line[l:col - 1]
  let l:pos = [line('.'), l:col]

  " Avoid triggering too frequently
  if s:last_completion_pos == l:pos
    return
  endif

  " Check if character is in trigger list
  if index(g:bkmr_lsp_trigger_chars, l:char) >= 0
    let s:last_completion_pos = l:pos
    call timer_start(100, {-> s:delayed_completion()})
  endif
endfunction

" Delayed completion trigger
function! s:delayed_completion() abort
  if mode() ==# 'i' && pumvisible() == 0
    call feedkeys("\<C-x>\<C-o>", 'n')
  endif
endfunction

" Open snippet by ID
function! bkmr_lsp#open_snippet(...) abort
  let l:id = a:0 > 0 && !empty(a:1) ? a:1 : input('Enter snippet ID: ')

  if empty(l:id)
    return
  endif

  " Validate ID is numeric
  if !match(l:id, '^\d\+$')
    echohl ErrorMsg
    echo 'Invalid snippet ID: ' . l:id
    echohl None
    return
  endif

  call lsp#send_request('bkmr-lsp', {
        \ 'method': 'workspace/executeCommand',
        \ 'params': {
        \   'command': 'bkmr.open',
        \   'arguments': [str2nr(l:id)]
        \ },
        \ 'on_notification': function('s:handle_command_response')
        \ })
endfunction

" Refresh snippets cache
function! bkmr_lsp#refresh_snippets() abort
  call lsp#send_request('bkmr-lsp', {
        \ 'method': 'workspace/executeCommand',
        \ 'params': {
        \   'command': 'bkmr.refresh',
        \   'arguments': []
        \ },
        \ 'on_notification': function('s:handle_refresh_response')
        \ })
endfunction

" Search snippets interactively
function! bkmr_lsp#search_snippets() abort
  " Get current snippets via completion
  let l:save_pos = getpos('.')

  " Trigger completion to get snippet list
  call bkmr_lsp#trigger_completion()

  " Show instructions
  echo 'Use completion menu to browse snippets. Press <Esc> to cancel.'
endfunction

" Show plugin status
function! bkmr_lsp#show_status() abort
  echo '=== bkmr-lsp Status ==='

  " Check binary
  if executable(g:bkmr_lsp_binary)
    echo 'Binary: ' . g:bkmr_lsp_binary . ' ✓'
  else
    echohl ErrorMsg
    echo 'Binary: ' . g:bkmr_lsp_binary . ' ✗ (not found)'
    echohl None
  endif

  " Check LSP server status
  let l:server_status = lsp#get_server_status('bkmr-lsp')
  echo 'LSP Server: ' . l:server_status

  " Check bkmr command
  if executable('bkmr')
    echo 'bkmr command: available ✓'

    " Get snippet count
    try
      let l:result = system('bkmr search -t _snip_ --json | jq ". | length" 2>/dev/null')
      let l:count = str2nr(trim(l:result))
      echo 'Snippets available: ' . l:count
    catch
      echo 'Snippets: unable to count'
    endtry
  else
    echohl ErrorMsg
    echo 'bkmr command: not found ✗'
    echohl None
  endif

  echo 'Auto-completion: ' . (g:bkmr_lsp_auto_complete ? 'enabled' : 'disabled')
  echo 'Trigger characters: ' . join(g:bkmr_lsp_trigger_chars, ', ')
endfunction

" Show plugin information
function! bkmr_lsp#show_info() abort
  echo '=== vim-bkmr-lsp Plugin Information ==='
  echo 'Version: 1.0.0'
  echo 'Author: Your Name'
  echo 'License: MIT'
  echo ''
  echo 'Commands:'
  echo '  :BkmrLspStatus    - Show plugin status'
  echo '  :BkmrLspRefresh   - Refresh snippet cache'
  echo '  :BkmrLspOpen [id] - Open snippet by ID'
  echo '  :BkmrLspSearch    - Search snippets interactively'
  echo '  :BkmrLspComplete  - Trigger completion'
  echo '  :BkmrLspInfo      - Show this information'
  echo ''
  echo 'Key mappings:'
  for [action, mapping] in items(g:bkmr_lsp_mappings)
    if !empty(mapping)
      echo '  ' . mapping . ' - ' . action
    endif
  endfor
  echo ''
  echo 'Configuration variables:'
  echo '  g:bkmr_lsp_binary         = ' . g:bkmr_lsp_binary
  echo '  g:bkmr_lsp_enabled        = ' . g:bkmr_lsp_enabled
  echo '  g:bkmr_lsp_auto_complete  = ' . g:bkmr_lsp_auto_complete
  echo '  g:bkmr_lsp_max_completions = ' . g:bkmr_lsp_max_completions
endfunction

" Get cached snippets (for external use)
function! bkmr_lsp#get_cached_snippets() abort
  return s:cached_snippets
endfunction

" Insert snippet by ID
function! bkmr_lsp#insert_snippet_by_id(id) abort
  " This would need completion context to work properly
  " For now, just open the snippet
  call bkmr_lsp#open_snippet(a:id)
endfunction

" Handle command response
function! s:handle_command_response(server, data) abort
  if has_key(a:data, 'error')
    echohl ErrorMsg
    echo 'Command failed: ' . get(a:data.error, 'message', 'Unknown error')
    echohl None
  else
    echo 'Command executed successfully'
  endif
endfunction

" Handle refresh response
function! s:handle_refresh_response(server, data) abort
  if has_key(a:data, 'error')
    echohl ErrorMsg
    echo 'Refresh failed: ' . get(a:data.error, 'message', 'Unknown error')
    echohl None
  else
    echo 'Snippets refreshed successfully'
    let s:cached_snippets = []  " Clear cache
  endif
endfunction

" Utility function to check if LSP server is running
function! bkmr_lsp#is_server_running() abort
  let l:status = lsp#get_server_status('bkmr-lsp')
  return l:status ==# 'running'
endfunction

" Get server info
function! bkmr_lsp#get_server_info() abort
  return lsp#get_server_info('bkmr-lsp')
endfunction