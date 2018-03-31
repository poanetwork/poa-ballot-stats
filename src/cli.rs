use clap::{App, Arg, ArgMatches};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

/// Returns the matched command line arguments.
pub fn get_matches() -> ArgMatches<'static> {
    App::new("POA ballot statistics")
        .author(AUTHORS)
        .version(VERSION)
        .about(DESCRIPTION)
        .arg(
            Arg::with_name("url")
                .value_name("URL")
                .help("The JSON-RPC endpoint")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .help("More detailed output")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("contracts")
                .short("c")
                .long("contracts")
                .help("JSON file with the contract addresses")
                .takes_value(true),
        )
        .get_matches()
}
