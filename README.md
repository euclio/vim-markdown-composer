# vim-markdown-composer

vim-markdown-composer is a plugin that adds asynchronous markdown preview to
Neovim.

This plugin should be considered alpha-quality software.

![](http://i.imgur.com/TVJ0wCn.gif)

## Requirements

vim-markdown-composer requires [Python 3], [Rust], [cargo], and [Neovim].

If you haven't already installed the Python 3 plugin host, install it with `pip3
install neovim`.

Unfortunately, since Neovim only supports Unixes, the plugin will only work on
OS X and Linux at this time. However, Windows support should come for free once
Neovim supports it.

The Python 3 dependency may be dropped once Rust has a usable [msgpack-rpc]
library or Neovim has a Rust [plugin host][Neovim remote plugin]. Similarly, the
plugin may gain support for vim in the future.

## Installation

Use whatever plugin manager you like. If you aren't familiar with plugin
managers, I recommend [vim-plug].

Here's an an example of managing installation with vim-plug:

```vim
function! BuildComposer(info)
  if a:info.status != 'unchanged' || a:info.force
    !cargo build --release
    UpdateRemotePlugins
  endif
endfunction

Plug 'euclio/vim-markdown-composer', { 'do': function('BuildComposer') }
```

You should run `cargo build --release` in the plugin directory after
installation.

Also, don't forget to `:UpdateRemotePlugins` after installing or updating!

If you use the above snippet, everything should be taken care of automatically.

## Documentation

`:help markdown-composer`, or check out the `doc` directory.

# Acknowledgments

This plugin is inspired by suan's [vim-instant-markdown].

This plugin was built with [aurelius], a Rust library for live-updating markdown
previews.

[Python 3]: https://www.python.org/downloads/
[Rust]: http://www.rust-lang.org/
[cargo]: https://crates.io/
[Neovim]: http://neovim.io/
[vim-instant-markdown]: https://github.com/suan/vim-instant-markdown
[Neovim remote plugin]: http://neovim.io/doc/user/remote_plugin.html
[vim-plug]: https://github.com/junegunn/vim-plug
[msgpack-rpc]: https://github.com/msgpack-rpc/msgpack-rpc
[aurelius]: https://github.com/euclio/aurelius

