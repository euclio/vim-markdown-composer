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
extern crate rmp_serialize;
extern crate rustc_serialize;

use std::default::Default;
use std::io::BufReader;
use std::net::TcpStream;
use std::path::PathBuf;

use aurelius::Server;
use aurelius::browser;
use docopt::Docopt;
use msgpack::decode::ReadError::UnexpectedEOF;
use rmp_serialize::Decoder;
use rmp_serialize::decode::Error;
use rustc_serialize::Decodable;

static USAGE: &'static str = r"
Usage: markdown_composer [options] <nvim-port> [<initial-markdown>]
       markdown_composer --help

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
    arg_nvim_port: u16,
    arg_initial_markdown: Option<String>,
    flag_no_browser: bool,
    flag_browser: Option<String>,
    flag_highlight_theme: Option<String>,
    flag_working_directory: Option<String>,
    flag_custom_css: Option<String>,
}

fn open_browser(server: &Server, browser: Option<String>) {
    let url = format!("http://{}", server.http_addr().unwrap());

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

    if let Some(markdown) = args.arg_initial_markdown {
        config.initial_markdown = markdown;
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
    let sender = server.start();

    if !args.flag_no_browser {
        open_browser(&server, args.flag_browser.clone());
    }

    let nvim_port = args.arg_nvim_port;
    let stream = TcpStream::connect(("localhost", nvim_port))
        .ok()
        .expect(&format!("no listener on port {}", nvim_port));

    let mut decoder = Decoder::new(BufReader::new(stream));
    loop {
        let msg = <Vec<String> as Decodable>::decode(&mut decoder);
        match msg {
            Ok(msg) => {
                let cmd = &msg.first().unwrap()[..];
                let params = &msg[1..];
                match cmd {
                    "send_data" => sender.send(params[0].to_owned()).unwrap(),
                    "open_browser" => open_browser(&server, args.flag_browser.clone()),
                    "chdir" => server.change_working_directory(params[0].to_owned()),
                    _ => panic!("Received unknown command: {}", cmd),
                }
            }
            Err(Error::InvalidMarkerRead(UnexpectedEOF)) => {
                // In this case, the remote client probably just hung up.
                break;
            }
            Err(err) => panic!(err),
        }
    }
}
