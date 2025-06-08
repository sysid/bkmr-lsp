" settings/bkmr_lsp.vim
" vim-lsp-settings configuration for bkmr-lsp
" This file should be placed in vim-lsp-settings plugin directory

function! s:bkmr_lsp_settings() abort
  return {
        \ 'cmd': ['bkmr-lsp'],
        \ 'allowlist': ['*'],
        \ 'capabilities': {
        \   'completionProvider': {
        \     'triggerCharacters': ['.', ':'],
        \     'resolveProvider': v:false,
        \   },
        \   'executeCommandProvider': {
        \     'commands': ['bkmr.open', 'bkmr.refresh']
        \   }
        \ },
        \ 'initialization_options': {},
        \ 'workspace_config': {},
        \ 'semantic_tokens': v:false,
        \ 'config': {
        \   'bkmr': {
        \     'maxCompletions': 50,
        \     'binaryPath': 'bkmr'
        \   }
        \ }
        \ }
endfunction

" Register with vim-lsp-settings if available
if exists('*lsp_settings#register')
  call lsp_settings#register('bkmr-lsp', function('s:bkmr_lsp_settings'))
endif