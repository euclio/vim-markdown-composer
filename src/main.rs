//! A simple client that listens for msgpack-serialized strings on a port and renders them as
//! markdown.
//!
//! The markdown is rendered on an arbitrary port on localhost, which is then automatically opened
//! in a browser. As new messages are received on the input port, the markdown is asynchonously
//! rendered in the browser (no refresh is required).

#[macro_use]
extern crate log;

extern crate aurelius;
extern crate docopt;
extern crate log4rs;
extern crate rmp as msgpack;
extern crate rmp_serde;
extern crate rustc_serialize;
extern crate serde;

use std::default::Default;
use std::error::Error;
use std::fs::File;
use std::io::{self, BufReader};
use std::io::prelude::*;
use std::net::SocketAddr;
use std::path::PathBuf;

use aurelius::Server;
use aurelius::browser;
use docopt::Docopt;
use msgpack::decode::ReadError::UnexpectedEOF;
use rmp_serde::{Deserializer, decode};
use serde::Deserialize;

static USAGE: &'static str = r"
Usage: markdown_composer [options] [<markdown-file>]

Creates a static server for serving markdown previews. Reads msgpack-rpc requests from stdin.

Supported procedures:

    send_data(data: String)     Pushes a markdown string to the rendering server.
    open_browser()              Opens the user default browser, or the browser specified by
                                `--browser`.
    chdir(path: String)         Changes the directory that the server serves static files from.

You may provide an optional file argument. The contents of this file will be rendered and displayed
by the server on startup.

Options:
    -h, --help                  Show this message.

    --no-browser                Don't open the web browser automatically.

    --browser=<executable>      Specify a browser that the program should open. If not supplied,
                                the program will determine the user's default browser.

    --highlight-theme=<theme>   The theme to use for syntax highlighting. All highlight.js themes
                                are supported. If no theme is supplied, the 'github' theme is used.

    --working-directory=<dir>   The directory that static files should be served out of. Useful for
                                static content linked in the markdown. Can be changed at runtime
                                with the 'chdir' command.

    --custom-css=<url/path>     CSS that should be used to style the markdown output. Defaults to
                                github-like CSS.
";

#[derive(RustcDecodable, Debug)]
struct Args {
    arg_markdown_file: Option<String>,
    flag_no_browser: bool,
    flag_browser: Option<String>,
    flag_highlight_theme: Option<String>,
    flag_working_directory: Option<String>,
    flag_custom_css: Option<String>,
}

fn open_browser(http_addr: &SocketAddr, browser: Option<String>) {
    let url = format!("http://{}", http_addr);

    if let Some(ref browser) = browser {
        let split_cmd = browser.split_whitespace().collect::<Vec<_>>();
        let (cmd, args) = split_cmd.split_first().unwrap();
        browser::open_specific(&url, cmd, args).unwrap();
    } else {
        browser::open(&url).unwrap();
    }
}

fn main() {
    log4rs::init_file("config/log.yaml", Default::default()).unwrap();

    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.decode())
        .unwrap_or_else(|e| e.exit());

    let mut config = aurelius::Config::default();

    if let Some(markdown_file) = args.arg_markdown_file {
        debug!("Reading initial markdown file: {:?}", markdown_file);
        let mut file = File::open(markdown_file).unwrap();
        file.read_to_string(&mut config.initial_markdown).unwrap();
    }

    if let Some(highlight_theme) = args.flag_highlight_theme {
        config.highlight_theme = highlight_theme;
    }

    if let Some(working_directory) = args.flag_working_directory {
        config.working_directory = PathBuf::from(working_directory);
    }

    if let Some(custom_css) = args.flag_custom_css {
        config.custom_css = custom_css;
    }

    let mut server = Server::new_with_config(config);
    let mut handle = server.start();

    if !args.flag_no_browser {
        open_browser(&handle.http_addr().unwrap(), args.flag_browser.clone());
    }

    let mut decoder = Deserializer::new(BufReader::new(io::stdin()));
    loop {
        let msg =
            <rmp_serde::Value as Deserialize>::deserialize(&mut decoder);

        match msg {
            Ok(msg) => {
                let msg = msg.0;

                // Assume we received a notification.
                assert_eq!(msg[0].as_u64().unwrap(), 2);
                let cmd = msg[1].as_str().unwrap();
                let params = msg[2].as_array().unwrap();

                match cmd {
                    "send_data" => handle.send(params[0].as_str().unwrap().to_owned()),
                    "open_browser" => {
                        open_browser(&handle.http_addr().unwrap(), args.flag_browser.clone())
                    }
                    "chdir" => {
                        handle.change_working_directory(params[0].as_str().unwrap().to_owned())
                    }
                    _ => panic!("Received unknown command: {}", cmd),
                }
            }
            Err(decode::Error::InvalidMarkerRead(UnexpectedEOF)) => {
                // In this case, the remote client probably just hung up.
                break;
            }
            Err(err) => panic!("{}", err.description()),
        }
    }
}
