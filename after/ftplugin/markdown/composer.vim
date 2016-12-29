let s:plugin_root = expand('<sfile>:p:h:h:h:h')

function! s:startServer()
  if exists('s:job')
    return
  endif

  let args = ['cargo', 'run', '--release', '--']

  let s:file = expand('%:p')
  if filereadable(s:file)
    call add(args, s:file)
  endif

  if exists('g:markdown_composer_browser')
    call extend(args, ['--browser', g:markdown_composer_browser])
  endif

  if exists('g:markdown_composer_open_browser')
    if !g:markdown_composer_open_browser
      call add(args, '--no-browser')
    endif
  endif

  if exists('g:markdown_composer_syntax_theme')
    call extend(args, ['--highlight-theme', g:markdown_composer_syntax_theme])
  endif

  call extend(args, ['--working-directory', getcwd()])

  let s:job = jobstart(args, {
        \ 'rpc': v:true,
        \ 'cwd': s:plugin_root
        \ }
  \ )
endfunction

function! s:sendBuffer()
  if exists('s:job')
    call rpcnotify(s:job, 'send_data', join(getline(1, '$'), "\n"))
  endif
endfunction

function! s:openBrowser()
  if exists('s:job')
    call rpcnotify(s:job, 'open_browser')
  endif
endfunction

function! s:echoJob()
  if exists('s:job')
    echo s:job
  else
    echo 'No job running'
  endif
endfunction

function! s:chdir()
  if exists('s:job')
    call rpcnotify(s:job, 'chdir', getcwd())
  endif
endfunction

command! ComposerUpdate call s:sendBuffer()
command! ComposerOpen call s:openBrowser() | call s:sendBuffer()
command! ComposerStart call s:startServer()
command! ComposerJob call s:echoJob()

augroup markdown-composer
  autocmd!
  autocmd BufEnter *.md,*.mkd,*.markdown
        \ if !(exists('g:markdown_composer_autostart') && !g:markdown_composer_autostart) |
        \   call s:startServer() |
        \ endif |
        \ call s:chdir() |
        \ call s:sendBuffer()
  autocmd TextChanged,TextChangedI *.md,*.mkd,*.markdown call s:sendBuffer()
augroup END
