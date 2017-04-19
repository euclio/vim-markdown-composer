//! A simple client that listens for msgpack-serialized strings on a port and renders them as
//! markdown.
//!
//! The markdown is rendered on an arbitrary port on localhost, which is then automatically opened
//! in a browser. As new messages are received on the input port, the markdown is asynchonously
//! rendered in the browser (no refresh is required).

#[macro_use]
extern crate clap;

#[macro_use]
extern crate log;

#[macro_use]
extern crate serde_derive;

extern crate aurelius;
extern crate log4rs;
extern crate rmp_serde;
extern crate rmpv as msgpack;
extern crate serde;

use std::default::Default;
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::io::{self, BufReader};
use std::mem;
use std::net::SocketAddr;
use std::path::PathBuf;

use aurelius::Server;
use aurelius::browser;
use clap::{App, Arg};
use rmp_serde::{Deserializer, decode};
use serde::Deserialize;

static ABOUT: &'static str = r"
Creates a static server for serving markdown previews. Reads msgpack-rpc requests from stdin.

Supported procedures:

    send_data(data: String)     Pushes a markdown string to the rendering server.
    open_browser()              Opens the user default browser, or the browser specified by
                                `--browser`.
    chdir(path: String)         Changes the directory that the server serves static files from.
";

/// Represents an RPC request.
///
/// Assumes that the request's parameters are always `String`s.
#[derive(Debug, Deserialize)]
pub struct Rpc {
    /// This field will be an ID for a msgpack request, or an ID for a JSON-RPC request.
    ///
    /// We include it because we know that it will be sent over the wire, but it's not actually
    /// required for anything, so we keep it private.
    _id_or_type: u64,
    pub method: String,
    pub params: Vec<String>,
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

    let matches = App::new("markdown_composer")
        .author(crate_authors!())
        .version(crate_version!())
        .about(ABOUT)
        .arg(Arg::with_name("no-auto-open")
            .long("no-auto-open")
            .help("Don't open the web browser automatically."))
        .arg(Arg::with_name("browser")
            .long("browser")
            .value_name("executable")
            .help("Specify a browser that the program should open. If not supplied, the program \
                   will determine the user's default browser.")
            .takes_value(true))
        .arg(Arg::with_name("theme")
            .long("highlight-theme")
            .help("The theme to use for syntax highlighting. All highlight.js themes are \
                   supported.")
            .default_value("github"))
        .arg(Arg::with_name("working-directory")
            .long("working-directory")
            .value_name("dir")
            .help("The directory that static files should be served out of. All relative links \
                   in the markdown will be served relative to this directory.")
            .takes_value(true))
        .arg(Arg::with_name("css")
            .long("custom-css")
            .value_name("url/path")
            .help("CSS that should be used to style the markdown output. Defaults to \
                   GitHub-like CSS.")
            .takes_value(true))
        .arg(Arg::with_name("markdown-file")
            .help("A markdown file that should be rendered by the server on startup."))
        .get_matches();

    let mut config = aurelius::Config::default();

    if let Some(markdown_file) = matches.value_of("markdown-file") {
        debug!("Reading initial markdown file: {:?}", markdown_file);
        let mut file = File::open(markdown_file).unwrap();
        file.read_to_string(&mut config.initial_markdown).unwrap();
    }

    if let Some(highlight_theme) = matches.value_of("theme") {
        config.highlight_theme = highlight_theme.to_owned();
    }

    if let Some(working_directory) = matches.value_of("working-directory") {
        config.working_directory = PathBuf::from(working_directory);
    }

    if let Some(custom_css) = matches.value_of("css") {
        config.custom_css = custom_css.to_owned();
    }

    let mut server = Server::new_with_config(config);
    let mut handle = server.start();

    if !matches.is_present("no-auto-open") {
        let browser = matches.value_of("browser").map(|s| s.to_owned());
        debug!("opening {} with {:?}", handle.http_addr().unwrap(), &browser);
        open_browser(&handle.http_addr().unwrap(), browser);
    }

    let mut decoder = Deserializer::new(BufReader::new(io::stdin()));
    loop {
        let mut rpc = match Rpc::deserialize(&mut decoder) {
            Ok(rpc) => rpc,
            Err(decode::Error::InvalidMarkerRead(_)) => {
                // In this case, the remote client probably just hung up.
                break;
            }
            Err(err) => panic!("{}", err.description()),
        };

        match &rpc.method[..] {
            "send_data" => {
                // Avoid copy
                let data = mem::replace(&mut rpc.params[0], String::default());
                handle.send(data);
            },
            "open_browser" => {
                let browser = matches.value_of("browser").map(|s| s.to_owned());
                open_browser(&handle.http_addr().unwrap(), browser);
            },
            "chdir" => {
                handle.change_working_directory(rpc.params[0].clone());
            },
            method => panic!("Received unknown command: {}", method),
        }
    }
}
