let s:plugin_root = expand('<sfile>:p:h:h:h:h')

if exists('g:markdown_composer_refresh_rate')
  let s:refresh_rate = g:markdown_composer_refresh_rate
elseif exists('g:markdown_composer_external_renderer')
  let s:refresh_rate = 500
else
  let s:refresh_rate = 0
endif

function! s:startServer()
  if exists('s:job')
    return
  endif

  let l:args = ['cargo', 'run', '--release']
  if !has('nvim')
    call extend(l:args, ['--no-default-features', '--features', 'json-rpc'])
  endif

  call extend(l:args, ['--'])

  if exists('g:markdown_composer_browser')
    call extend(l:args, ['--browser', g:markdown_composer_browser])
  endif

  if exists('g:markdown_composer_open_browser')
    if !g:markdown_composer_open_browser
      call add(l:args, '--no-auto-open')
    endif
  endif

  if exists('g:markdown_composer_syntax_theme')
    call extend(l:args, ['--highlight-theme', g:markdown_composer_syntax_theme])
  endif

  if exists('g:markdown_composer_external_renderer')
    call extend(l:args, ['--external-renderer', g:markdown_composer_external_renderer])
  endif

  call extend(l:args, ['--working-directory', getcwd()])

  let s:file = expand('%:p')
  if filereadable(s:file)
    call add(l:args, s:file)
  endif

  function! s:onServerExit(id, exit_status, event) abort
    unlet s:job
  endfunction

  if has('nvim')
    let s:job = jobstart(l:args, {
          \ 'cwd': s:plugin_root,
          \ 'rpc': v:true,
          \ 'on_exit': function('s:onServerExit'),
          \ })
  else
    " vim doesn't have a way to set the working directory for a job, so we have
    " to change the directory manually. See vim#1024.
    let l:original_cwd = getcwd()
    execute 'lcd' s:plugin_root
    call job_start(l:args, {
          \ 'mode': 'nl',
          \ 'out_cb': function('s:startupCallback'),
          \ 'err_io': 'null',
          \ 'exit_cb': function('s:onServerExit'),
          \ })
    execute 'lcd' l:original_cwd
  endif

  if s:refresh_rate > 0 && !exists('s:timer')
    let s:timer = timer_start(s:refresh_rate, function('s:markdownHandler'), { 'repeat': -1 })
   endif
endfunction

function! s:startupCallback(channel, message) abort
  let l:addr = 'localhost:' . a:message
  let s:job = ch_open(l:addr, {
       \ 'mode': 'json',
       \ })
endfunction

function! s:sendBuffer()
  if exists('s:job')
    let l:data = join(getline(1, '$'), "\n")
    if has('nvim')
      call rpcnotify(s:job, 'send_data', l:data)
    else
      call ch_sendexpr(s:job, {
            \ 'method': 'send_data',
            \ 'params': [l:data],
            \ })
    endif
  endif
endfunction

function! s:openBrowser()
  if exists('s:job')
    if has('nvim')
      call rpcnotify(s:job, 'open_browser')
    else
      call ch_sendexpr(s:job, {
            \ 'method': 'open_browser',
            \ 'params': [],
            \ })
    endif
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
    if has('nvim')
      call rpcnotify(s:job, 'chdir', getcwd())
    else
      call ch_sendexpr(s:job, {
            \ 'method': 'open_browser',
            \ 'params': [getcwd()],
            \ })
    endif
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

  if s:refresh_rate == 0
    autocmd TextChanged,TextChangedI *.md,*.mkd,*.markdown call s:sendBuffer()
  endif
augroup END

function! s:markdownHandler(timer)
  if &filetype ==# 'markdown' || &filetype ==# 'pandoc'
    call s:sendBuffer()
  endif
endfunction
