# vim-markdown-composer

[![Build Status](https://travis-ci.org/euclio/vim-markdown-composer.svg)](https://travis-ci.org/euclio/vim-markdown-composer)

vim-markdown-composer is a plugin that adds asynchronous markdown preview to
Neovim.

![](http://i.imgur.com/TVJ0wCn.gif)

## Requirements

vim-markdown-composer requires [Rust], [cargo], and [Neovim].

vim-markdown-composer officially targets the latest version of [stable Rust]. If
you'd like to install multiple versions of Rust on the same machine, check out
[rustup.rs](https://www.rustup.rs/).

Unfortunately, since Neovim only supports Unixes, the plugin will only work on
OS X and Linux at this time. However, Windows support should come for free once
Neovim supports it.

## Installation

Use whatever plugin manager you like. If you aren't familiar with plugin
managers, I recommend [vim-plug].

Here's an an example of managing installation with vim-plug:

```vim
function! BuildComposer(info)
  if a:info.status != 'unchanged' || a:info.force
    !cargo build --release
  endif
endfunction

Plug 'euclio/vim-markdown-composer', { 'do': function('BuildComposer') }
```

You should run `cargo build --release` in the plugin directory after
installation.

If you use the above snippet, everything should be taken care of automatically.

## Documentation

`:help markdown-composer`, or check out the `doc` directory.

# Acknowledgments

This plugin is inspired by suan's [vim-instant-markdown].

This plugin was built with [aurelius], a Rust library for live-updating markdown
previews.

[Rust]: http://www.rust-lang.org/
[cargo]: https://crates.io/
[Neovim]: http://neovim.io/
[vim-instant-markdown]: https://github.com/suan/vim-instant-markdown
[Neovim remote plugin]: http://neovim.io/doc/user/remote_plugin.html
[vim-plug]: https://github.com/junegunn/vim-plug
[msgpack-rpc]: https://github.com/msgpack-rpc/msgpack-rpc
[aurelius]: https://github.com/euclio/aurelius
[stable Rust]: https://www.rust-lang.org/downloads.html
