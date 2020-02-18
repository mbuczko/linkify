mod db;
mod link;
mod user;
mod utils;

use clap::{App, Arg, ArgMatches, SubCommand};
use db::{init_vault, Vault};
use log::Level;
use semver::Version;
use crate::link::Link;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

fn main() {
    simple_logger::init_with_level(Level::Debug).unwrap();

    let matches = App::new("Świnka skarbonka na linki")
        .version(VERSION)
        .about("Saves your precious links into local vault")
        .arg(
            Arg::with_name("database")
                .help("database to use")
                .short("d")
                .long("db")
                .takes_value(true),
        )
        .subcommand(
            SubCommand::with_name("add")
                .about("Adds a new link")
                .arg(
                    Arg::with_name("url")
                        .help("link to store in database")
                        .required(true),
                )
                .arg(
                    Arg::with_name("description")
                        .help("optional description")
                        .short("d")
                        .long("desc")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("tags")
                        .help("optional tags assigned to the link")
                        .short("t")
                        .long("tags")
                        .use_delimiter(true)
                        .takes_value(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("ls")
                .about("List matching links")
                .arg(
                    Arg::with_name("url")
                        .help("link or its part to match")
                )
                .arg(
                    Arg::with_name("description")
                        .help("optional part of description to match")
                        .short("d")
                        .long("desc")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("tags")
                        .help("optional comma-separated tags to match")
                        .short("t")
                        .long("tags")
                        .use_delimiter(true)
                        .takes_value(true),
                ),
        )
        .get_matches();

    let db = matches.value_of("database").unwrap_or("links.db");
    match init_vault(db, Version::parse(VERSION).unwrap()) {
        Ok(v) => process_command(v, matches),
        _ => panic!("cannot initialize database"),
    }
}

fn process_command(mut vault: Vault, matches: ArgMatches) {
    match matches.subcommand() {
        ("add", Some(sub_m)) => vault.add_link(&Link::from(sub_m)),
        ("ls", Some(sub_m)) => {
            let _links = vault.match_links(&Link::from(sub_m));
        },
         _ => {}
    }
}
