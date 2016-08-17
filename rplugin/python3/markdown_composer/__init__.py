# -*- coding: utf-8 -*-

"""
This remote plugin serves to connect the Rust client to Neovim.

This may be replaced in the future by a pure-Rust implementation.
"""

import os
from pathlib import Path
import logging
import socket
import subprocess
import threading

import neovim
import msgpack


@neovim.plugin
class MarkdownPlugin(object):

    """
    Neovim remote plugin that communicates over a socket with a Rust client.
    """

    def __init__(self, vim):
        self.vim = vim
        self.server = None
        self.client = None
        self.listening_port = None

    def send_current_buffer(self):
        """
        Sends the current buffer to the client as a string.

        The buffer will not be sent if the client is None.
        """
        if self.client is None:
            return

        self._send_request('send_data', '\n'.join(self.vim.current.buffer))

    def open_browser(self):
        "Send the client a message indicating it should open a browser."
        msg = msgpack.packb(['open_browser'])
        self.client.stdin.write(msg)

    def should_autostart(self):
        "Returns whether the server should start automatically."
        return self.vim.vars.get('markdown_composer_autostart', True)

    @neovim.command('ComposerOpen')
    def composer_open(self):
        "(Re)opens the browser."
        self.open_browser()

    @neovim.command('ComposerUpdate', sync=True)
    def send_current_buffer_command(self):
        "Send the current buffer to the client synchronously."
        self.send_current_buffer()

    @neovim.command('ComposerStart')
    def composer_start(self):
        "Start the client manually."
        self.start_client()

    @neovim.command('ComposerPort')
    def composer_port(self):
        "Echoes the port that the plugin is listening on."
        self.vim.command('echom "{}"'.format(self.listening_port))

    @neovim.autocmd('FileType', pattern='markdown', sync=True)
    def auto_start_client(self):
        if self.should_autostart():
            self.start_client()

    def start_client(self):
        """
        Starts the Rust client, which handles most of the heavy lifting.

        The server will begin listening for TCP connections on an arbitrary
        socket. The server will handle only one connection at a time.
        """
        if self.client is not None:
            return

        # Arguments for the client
        browser = self.vim.vars.get('markdown_composer_browser')
        open_browser = (
            self.vim.vars.get('markdown_composer_open_browser', 1) == 1)
        syntax_theme = self.vim.vars.get('markdown_composer_syntax_theme')
        current_buffer = '\n'.join(self.vim.current.buffer)

        plugin_root = Path(__file__).parents[3]
        args = ['cargo', 'run', '--release', '--']
        if browser:
            args.append('--browser=%s' % browser)

        if not open_browser:
            args.append('--no-browser')

        if syntax_theme:
            args.append('--highlight-theme=%s' % syntax_theme)

        args.append('--working-directory=%s' % os.getcwd())

        if os.path.isfile(self.vim.current.buffer.name):
            args.append(self.vim.current.buffer.name)

        self.client = subprocess.Popen(args,
            bufsize=0,
            cwd=str(plugin_root),
            stdout=subprocess.PIPE,
            stdin=subprocess.PIPE)

    def _send_request(self, method, *args):
        if self.client is None:
            return

        msg = msgpack.packb([method] + list(args))
        self.client.stdin.write(msg)

    @neovim.autocmd('BufEnter', pattern='*.md,*.mkd,*.markdown')
    def change_working_directory(self):
        "Serve static files from the current buffer's working directory."
        if self.client is None:
            return

        current_dir = os.path.dirname(self.vim.current.buffer.name)
        self._send_request('chdir', current_dir)

    @neovim.autocmd('TextChanged,TextChangedI', pattern='*.md,*.mkd,*.markdown')
    def send_current_buffer_autocmd(self):
        "Send the current buffer to the client asynchronously."
        self.send_current_buffer()
