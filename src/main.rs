mod db;

#[macro_use]
extern crate rust_embed;
extern crate clap;

#[macro_use]
extern crate log;
extern crate simple_logger;

use clap::{App, Arg, ArgMatches, SubCommand};
use db::{init_vault, Vault};
use log::Level;
use semver::Version;
use std::fmt;

#[derive(Debug)]
struct Link {
    url: String,
}

#[derive(PartialEq)]
enum Action {
    Store,
    Find,
}

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

impl Action {
    fn from_opt(is_storing: bool) -> Action {
        match is_storing {
            true => Self::Store,
            _ => Self::Find,
        }
    }
}

impl fmt::Display for Link {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "url: {}", self.url)
    }
}

fn main() {
    simple_logger::init_with_level(Level::Debug).unwrap();

    let matches = App::new("Świnka skarbonka na linki")
        .version(VERSION)
        .author("Haksior MB")
        .about("Saves your precious links")
        .arg(
            Arg::with_name("db")
                .help("database to use")
                .short("d")
                .long("db")
                .takes_value(true),
        )
        .subcommand(
            SubCommand::with_name("add")
                .about("adds a new link")
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
        .get_matches();

    let db = matches.value_of("db").unwrap_or("links.db");
    //let url = matches.value_of("url").unwrap();

    match init_vault(db, Version::parse(VERSION).unwrap()) {
        Ok(v) => process_command(v, matches),
        _ => panic!("cannot initialize database"),
    }
    //    if is_storing {
    //        conn.execute("INSERT INTO links(url) VALUES (?1)", params![url])?;
    //    } else {
    //        let mut stmt = conn.prepare("SELECT url FROM links WHERE url LIKE ?1")?;
    //        let links = stmt.query_map(params![format!("%{}%", url)], |row| {
    //            Ok(Link { url: row.get(0)? })
    //        })?;
    //        for link in links {
    //            println!("{}", link.unwrap());
    //        }
    //    }
}

fn process_command(vault: Vault, matches: ArgMatches) {
    match matches.subcommand() {
        ("add", Some(sub_m)) => vault.add_link(
            sub_m.value_of("url").unwrap(),
            sub_m.value_of("description"),
            sub_m.values_of("tags")
        ),
        _ => {}
    }
}
