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
        .author("Haksior MB")
        .about("Saves your precious links")
        .arg(
            Arg::with_name("database")
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

    let db = matches.value_of("database").unwrap_or("links.db");

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

fn process_command(mut vault: Vault, matches: ArgMatches) {
    match matches.subcommand() {
        ("add", Some(sub_m)) => {
            let tags = sub_m
                .values_of("tags")
                .and_then(|t| Some(t.map(String::from).collect::<Vec<String>>()));
            let link = Link::new(
                sub_m.value_of("url").unwrap(),
                sub_m.value_of("description"),
                tags
            );
            vault.add_link(&link);
        }
        _ => {}
    }
}
