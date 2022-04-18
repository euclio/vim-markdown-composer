//! A simple client that listens for RPC requests and renders them as markdown.
//!
//! The markdown is rendered on an arbitrary port on localhost, which is then automatically opened
//! in a browser. As new messages are received through stdin, the markdown is asynchronously
//! rendered in the browser (no refresh is required).

use std::default::Default;
use std::fs;
use std::io;
use std::io::prelude::*;
use std::mem;
use std::process::Command;

use anyhow::Result;
use clap::{crate_authors, crate_version};
use log::*;

use aurelius::Server;
use clap::{App, Arg};
use serde::Deserialize;
use shlex::Shlex;

static ABOUT: &str = r"
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

#[cfg(feature = "msgpack")]
impl<'de> Deserialize<'de> for Rpc {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{Error, Unexpected};

        const NOTIFICATION_MESSAGE_TYPE: u64 = 2;

        let (msg_type, method, params) = <(u64, String, Vec<String>)>::deserialize(deserializer)?;

        debug!("<- [{}, {}, {:?}]", msg_type, method, params);

        if msg_type != NOTIFICATION_MESSAGE_TYPE {
            return Err(Error::invalid_value(
                Unexpected::Unsigned(msg_type),
                &format!("notification message type ({})", NOTIFICATION_MESSAGE_TYPE).as_str(),
            ));
        }

        Ok(Rpc {
            msg_type,
            method,
            params,
        })
    }
}

// FIXME: Workaround for rust-lang/rust#55779. Move back to the impl when fixed.
#[derive(Debug, Deserialize)]
#[allow(unused)]
struct InnerRpc {
    method: String,
    params: Vec<String>,
}

#[cfg(feature = "json-rpc")]
impl<'de> Deserialize<'de> for Rpc {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (id, rpc): (u64, InnerRpc) = Deserialize::deserialize(deserializer)?;

        debug!("<- [{}, {:?}]", id, rpc);

        Ok(Rpc {
            id: id,
            method: rpc.method,
            params: rpc.params,
        })
    }
}

fn read_rpc(reader: impl Read, mut server: Server, browser: Option<&str>) -> Result<()> {
    #[cfg(feature = "msgpack")]
    let mut deserializer = rmp_serde::Deserializer::new(std::io::BufReader::new(reader));

    #[cfg(feature = "json-rpc")]
    let mut deserializer = serde_json::Deserializer::new(serde_json::de::IoRead::new(reader));

    loop {
        let mut rpc = match Rpc::deserialize(&mut deserializer) {
            Ok(rpc) => rpc,
            #[cfg(feature = "msgpack")]
            Err(rmp_serde::decode::Error::InvalidMarkerRead(_)) => {
                // In this case, the remote client probably just hung up.
                break;
            }
            #[cfg(feature = "json-rpc")]
            Err(err) if err.is_eof() => {
                break;
            }
            Err(err) => panic!("{}", err),
        };

        let res = match &rpc.method[..] {
            "send_data" => {
                let markdown = mem::replace(&mut rpc.params[0], String::new());
                server.send(markdown)
            }
            "open_browser" => match browser {
                Some(browser) => server.open_specific_browser(Command::new(browser)),
                None => server.open_browser(),
            },
            "chdir" => {
                let cwd = &rpc.params[0];
                info!("changing working directory: {}", cwd);
                server.set_static_root(cwd);
                Ok(())
            }
            method => panic!("Received unknown command: {}", method),
        };

        // TODO: Return error to the client instead of exiting the process.
        res?;
    }

    Ok(())
}

fn main() -> Result<()> {
    log_panics::init();
    log4rs::init_file("config/log.yaml", Default::default()).unwrap();

    let matches = App::new("markdown_composer")
        .author(crate_authors!())
        .version(crate_version!())
        .about(ABOUT)
        .arg(
            Arg::with_name("no-auto-open")
                .long("no-auto-open")
                .help("Don't open the web browser automatically."),
        )
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
                .takes_value(true)
                .multiple(true),
        )
        .arg(
            Arg::with_name("external-renderer")
                .long("external-renderer")
                .help("An external process that should be used for rendering markdown.")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("markdown-file")
                .help("A markdown file that should be rendered by the server on startup."),
        )
        .get_matches();

    let mut server = Server::bind("localhost:0")?;

    if let Some(external_renderer) = matches.value_of("external-renderer") {
        let words = Shlex::new(external_renderer).collect::<Vec<_>>();
        let (command, args) = words.split_first().expect("command was empty");
        let mut command = Command::new(command);
        command.args(args);
        server.set_external_renderer(command);
    }

    if let Some(highlight_theme) = matches.value_of("theme") {
        server.set_highlight_theme(highlight_theme.to_string());
    }

    if let Some(working_directory) = matches.value_of("working-directory") {
        server.set_static_root(working_directory);
    }

    if let Some(custom_css) = matches.values_of("css") {
        server.set_custom_css(custom_css.map(String::from).collect())?;
    }

    if let Some(file_name) = matches.value_of("markdown-file") {
        server.send(fs::read_to_string(file_name)?)?;
    }

    let browser = matches.value_of("browser");

    if !matches.is_present("no-auto-open") {
        let res = match browser {
            Some(browser) => server.open_specific_browser(Command::new(browser)),
            None => server.open_browser(),
        };

        res?;
    }

    let stdin = io::stdin();
    let stdin_lock = stdin.lock();

    read_rpc(stdin_lock, server, browser)?;

    Ok(())
}
