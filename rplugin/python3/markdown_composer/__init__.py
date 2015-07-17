# -*- coding: utf-8 -*-

"""
This remote plugin serves to connect the Rust client to Neovim.

This may be replaced in the future by a pure-Rust implementation.
"""

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
        self.client_process = None

    def send_current_buffer(self):
        """
        Sends the current buffer to the client as a string.

        The buffer will not be sent if the client is None.
        """
        if self.client is None:
            return

        msg = msgpack.packb('\n'.join(self.vim.current.buffer))
        self.client.send(msg)

    @neovim.command('ComposerUpdate', sync=True)
    def send_current_buffer_command(self):
        "Send the current buffer to the client synchronously."
        self.send_current_buffer()

    @neovim.command('ComposerStart')
    def composer_start(self):
        "Start the client manually."
        self.start_client()

    @neovim.autocmd('FileType', pattern='markdown', sync=True)
    def start_client(self):
        """
        Starts the Rust client, which handles most of the heavy lifting.

        The server will begin listening for TCP connections on an arbitrary
        socket. The server will handle only one connection at a time.
        """

        if self.server is not None:
            return

        self.server = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.server.bind(('localhost', 0))
        port = self.server.getsockname()[1]
        self.server.listen(1)

        logging.info('starting markdown client on port %d', port)

        # Arguments for the client
        browser = self.vim.vars.get('markdown_composer_browser')
        open_browser = (
            self.vim.vars.get('markdown_composer_open_browser', 1) == 1)
        syntax_theme = self.vim.vars.get('markdown_composer_syntax_theme')
        current_buffer = '\n'.join(self.vim.current.buffer)

        def launch_client_process():
            """
            Start the client and listen for connection requests.

            This function does not return.
            """
            plugin_root = Path(__file__).parents[3]
            args = ['cargo', 'run', '--release', '--']
            if browser:
                args.append('--browser=%s' % browser)

            if not open_browser:
                args.append('--no-browser')

            if syntax_theme:
                args.append('--highlight-theme=%s' % syntax_theme)

            self.client_process = subprocess.Popen(
                args + [str(port), current_buffer],
                cwd=str(plugin_root),
                stdout=subprocess.PIPE)

            while True:
                self.client, _ = self.server.accept()

        threading.Thread(target=launch_client_process).start()

    @neovim.autocmd('CursorHold,CursorHoldI,CursorMoved,CursorMovedI',
                    pattern='*.md,*.mkd,*.markdown')
    def send_current_buffer_autocmd(self):
        "Send the current buffer to the client asynchronously."
        self.send_current_buffer()
