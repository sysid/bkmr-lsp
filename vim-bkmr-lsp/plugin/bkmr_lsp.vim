" plugin/bkmr_lsp.vim
" vim-bkmr-lsp: Vim plugin for bkmr snippet manager via LSP
" Author: sysid
" Version: 1.0.0
" License: MIT

if exists('g:loaded_bkmr_lsp') || &compatible
  finish
endif
let g:loaded_bkmr_lsp = 1

" Check if required dependencies are available
if !exists('*lsp#register_server')
  echoerr 'vim-bkmr-lsp requires vim-lsp plugin. Please install prabirshrestha/vim-lsp'
  finish
endif

" Default configuration
let g:bkmr_lsp_binary = get(g:, 'bkmr_lsp_binary', 'bkmr-lsp')
let g:bkmr_lsp_enabled = get(g:, 'bkmr_lsp_enabled', 1)
let g:bkmr_lsp_auto_complete = get(g:, 'bkmr_lsp_auto_complete', 1)
let g:bkmr_lsp_trigger_chars = get(g:, 'bkmr_lsp_trigger_chars', ['.', ':'])
let g:bkmr_lsp_max_completions = get(g:, 'bkmr_lsp_max_completions', 50)

" Key mappings configuration
let g:bkmr_lsp_mappings = get(g:, 'bkmr_lsp_mappings', {
      \ 'complete': '<leader>bs',
      \ 'open': '<leader>bo',
      \ 'refresh': '<leader>br',
      \ 'search': '<leader>bf'
      \ })

" Register the LSP server if enabled and binary exists
if g:bkmr_lsp_enabled && executable(g:bkmr_lsp_binary)
  augroup bkmr_lsp_setup
    autocmd!
    autocmd User lsp_setup call bkmr_lsp#register_server()
  augroup END
endif

" Set up buffer-local mappings when LSP is enabled
augroup bkmr_lsp_mappings
  autocmd!
  autocmd User lsp_buffer_enabled call bkmr_lsp#setup_buffer_mappings()
augroup END

" Auto-completion setup
if g:bkmr_lsp_auto_complete
  augroup bkmr_lsp_completion
    autocmd!
    autocmd TextChangedI * call bkmr_lsp#auto_trigger_completion()
  augroup END
endif

" Commands
command! BkmrLspStatus call bkmr_lsp#show_status()
command! BkmrLspRefresh call bkmr_lsp#refresh_snippets()
command! -nargs=? BkmrLspOpen call bkmr_lsp#open_snippet(<q-args>)
command! BkmrLspSearch call bkmr_lsp#search_snippets()
command! BkmrLspComplete call bkmr_lsp#trigger_completion()
command! BkmrLspInfo call bkmr_lsp#show_info()

" Public API functions (for other plugins to use)
function! BkmrLspGetSnippets()
  return bkmr_lsp#get_cached_snippets()
endfunction

function! BkmrLspInsertSnippet(id)
  return bkmr_lsp#insert_snippet_by_id(a:id)
endfunction