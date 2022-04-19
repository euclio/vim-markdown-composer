//! A simple client that listens for RPC requests and renders them as markdown.
//!
//! The markdown is rendered on an arbitrary port on localhost, which is then automatically opened
//! in a browser. As new messages are received through stdin, the markdown is asynchronously
//! rendered in the browser (no refresh is required).

use std::default::Default;
use std::io::prelude::*;
use std::mem;

use anyhow::{anyhow, Result};
use clap::{crate_authors, crate_version};
use futures_util::TryStreamExt;
use log::*;
use tokio::io;
use tokio_serde::SymmetricallyFramed;
use tokio_util::codec::{BytesCodec, FramedRead};

use aurelius::Server;
use clap::{App, Arg};
use serde::Deserialize;
use shlex::Shlex;
use tokio::fs;
use tokio::process::Command;

use markdown_composer::Decoder;

static ABOUT: &str = r"
Creates a static server for serving markdown previews. Reads RPC requests from stdin.

Supported procedures:

    send_data(data: String)     Pushes a markdown string to the rendering server.
    open_browser()              Opens the user default browser, or the browser specified by
                                `--browser`.
    chdir(path: String)         Changes the directory that the server serves static files from.
";

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
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

    let host = tokio::net::lookup_host("localhost:0").await?
        .next()
        .ok_or_else(|| anyhow!("unable to lookup host"))?;
    let mut server = Server::bind(&host).await?;

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
        let contents = fs::read_to_string(file_name).await?;
        server.send(&contents).await?;
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

    let mut deserialized = FramedRead::new(stdin, Decoder::default());

    while let Some(rpc) = deserialized.try_next().await.unwrap() {
        let res = match &*rpc.method {
            "send_data" => {
                server.send(&rpc.params[0]).await;
            }
            "open_browser" => match browser {
                Some(browser) => {
                    server.open_specific_browser(Command::new(browser));
                }
                None => {
                    server.open_browser();
                }
            },
            "chdir" => {
                let cwd = &rpc.params[0];
                info!("changing working directory: {}", cwd);
                server.set_static_root(cwd);
            }
            method => panic!("Received unknown command: {}", method),
        };
    }

    Ok(())
}
