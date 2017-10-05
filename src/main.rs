//! A simple client that listens for RPC requests and renders them as markdown.
//!
//! The markdown is rendered on an arbitrary port on localhost, which is then automatically opened
//! in a browser. As new messages are received through stdin, the markdown is asynchronously
//! rendered in the browser (no refresh is required).

#[macro_use]
extern crate clap;

#[macro_use]
extern crate log;

#[macro_use]
extern crate serde_derive;

extern crate aurelius;
extern crate log4rs;
extern crate log_panics;
extern crate serde;

#[cfg(feature = "msgpack")]
extern crate rmp_serde as rmps;

#[cfg(feature = "json-rpc")]
extern crate serde_json;

use std::default::Default;
use std::fs::File;
use std::io::prelude::*;
use std::io;
use std::net::SocketAddr;
use std::process::Command;

use aurelius::{browser, Listening, Server};
use clap::{App, Arg};
use serde::Deserialize;

static ABOUT: &'static str = r"
Creates a static server for serving markdown previews. Reads RPC requests from stdin.

Supported procedures:

    send_data(data: String)     Pushes a markdown string to the rendering server.
    open_browser()              Opens the user default browser, or the browser specified by
                                `--browser`.
    chdir(path: String)         Changes the directory that the server serves static files from.
";

/// Represents an RPC request.
///
/// Assumes that the request's parameters are always `String`s.
#[derive(Debug)]
#[cfg_attr(feature = "msgpack", derive(Deserialize))]
pub struct Rpc {
    /// The type of msgpack request. Should always be notification.
    #[cfg(feature = "msgpack")]
    msg_type: u64,

    /// The ID of the JSON rpc request.
    #[cfg(feature = "json-rpc")]
    id: u64,

    pub method: String,
    pub params: Vec<String>,
}

#[cfg(feature = "json-rpc")]
impl<'de> Deserialize<'de> for Rpc {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct InnerRpc {
            method: String,
            params: Vec<String>,
        }

        let (id, rpc): (u64, InnerRpc) = Deserialize::deserialize(deserializer)?;

        Ok(Rpc {
            id: id,
            method: rpc.method,
            params: rpc.params,
        })
    }
}

fn open_browser(http_addr: &SocketAddr, browser: &Option<String>) {
    let url = format!("http://{}", http_addr);

    if let &Some(ref browser) = browser {
        let split_cmd = browser.split_whitespace().collect::<Vec<_>>();
        let (cmd, args) = split_cmd.split_first().unwrap();
        let mut command = Command::new(cmd);
        command.args(args);
        browser::open_specific(&url, command).unwrap();
    } else {
        browser::open(&url).unwrap();
    }
}

fn read_rpc<R>(reader: R, browser: &Option<String>, handle: &mut Listening)
where
    R: Read,
{
    #[cfg(feature = "msgpack")]
    let mut deserializer = rmps::Deserializer::new(std::io::BufReader::new(reader));

    #[cfg(feature = "json-rpc")]
    let mut deserializer = serde_json::Deserializer::new(serde_json::de::IoRead::new(reader));

    loop {
        let rpc = match Rpc::deserialize(&mut deserializer) {
            Ok(rpc) => rpc,
            #[cfg(feature = "msgpack")]
            Err(rmps::decode::Error::InvalidMarkerRead(_)) => {
                // In this case, the remote client probably just hung up.
                break;
            }
            Err(err) => panic!("{}", err),
        };

        match &rpc.method[..] {
            "send_data" => {
                handle.send(&rpc.params[0]).unwrap();
            }
            "open_browser" => {
                open_browser(&handle.http_addr().unwrap(), &browser);
            }
            "chdir" => {
                let cwd = &rpc.params[0];
                info!("changing working directory: {}", cwd);
                handle.change_working_directory(cwd);
            }
            method => panic!("Received unknown command: {}", method),
        }
    }
}

fn main() {
    log_panics::init();
    log4rs::init_file("config/log.yaml", Default::default()).unwrap();

    let matches = App::new("markdown_composer")
        .author(crate_authors!())
        .version(crate_version!())
        .about(ABOUT)
        .arg(Arg::with_name("no-auto-open").long("no-auto-open").help(
            "Don't open the web browser automatically.",
        ))
        .arg(
            Arg::with_name("browser")
                .long("browser")
                .value_name("executable")
                .help(
                    "Specify a browser that the program should open. If not supplied, the program \
                   will determine the user's default browser.",
                )
                .takes_value(true),
        )
        .arg(
            Arg::with_name("theme")
                .long("highlight-theme")
                .help(
                    "The theme to use for syntax highlighting. All highlight.js themes are \
                   supported.",
                )
                .default_value("github"),
        )
        .arg(
            Arg::with_name("working-directory")
                .long("working-directory")
                .value_name("dir")
                .help(
                    "The directory that static files should be served out of. All relative links \
                   in the markdown will be served relative to this directory.",
                )
                .takes_value(true),
        )
        .arg(
            Arg::with_name("css")
                .long("custom-css")
                .value_name("url/path")
                .help(
                    "CSS that should be used to style the markdown output. Defaults to \
                   GitHub-like CSS.",
                )
                .takes_value(true),
        )
        .arg(
            Arg::with_name("external-renderer")
                .long("external-renderer")
                .help(
                    "An external process that should be used for rendering markdown.",
                )
                .takes_value(true),
        )
        .arg(Arg::with_name("markdown-file").help(
            "A markdown file that should be rendered by the server on startup.",
        ))
        .get_matches();

    let mut server = Server::new();

    if let Some(markdown_file) = matches.value_of("markdown-file") {
        debug!("Reading initial markdown file: {:?}", markdown_file);
        let mut initial_markdown = String::new();
        let mut file = File::open(markdown_file).unwrap();
        file.read_to_string(&mut initial_markdown).unwrap();
        server.initial_markdown(initial_markdown);
    }

    if let Some(highlight_theme) = matches.value_of("theme") {
        server.highlight_theme(highlight_theme);
    }

    if let Some(working_directory) = matches.value_of("working-directory") {
        server.working_directory(working_directory);
    }

    if let Some(custom_css) = matches.value_of("css") {
        server.css(custom_css);
    }

    if let Some(external_renderer) = matches.value_of("external-renderer") {
        server.external_renderer(external_renderer);
    }

    let mut listening = server.start().unwrap();

    if !matches.is_present("no-auto-open") {
        let browser = matches.value_of("browser").map(|s| s.to_owned());
        debug!(
            "opening {} with {:?}",
            listening.http_addr().unwrap(),
            &browser
        );
        open_browser(&listening.http_addr().unwrap(), &browser);
    }

    let browser = matches.value_of("browser").map(|s| s.to_string());

    read_rpc(io::stdin(), &browser, &mut listening);
}
