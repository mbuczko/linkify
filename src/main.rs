mod db;

#[macro_use]
extern crate rust_embed;
extern crate clap;

#[macro_use]
extern crate log;
extern crate simple_logger;

use clap::{App, Arg};
use db::init_vault;
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
                .short("d")
                .long("db")
                .takes_value(true)
                .help("database to use"),
        )
        .arg(
            Arg::with_name("url")
                .short("u")
                .long("url")
                .required(true)
                .takes_value(true)
                .help("URL to save or search for"),
        )
        .arg(
            Arg::with_name("store")
                .short("s")
                .long("store")
                .help("Store given URL in DB database"),
        )
        .get_matches();

    let db = matches.value_of("db").unwrap_or("links.db");
    let url = matches.value_of("url").unwrap();

    init_vault(db, Version::parse(VERSION).unwrap());
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
