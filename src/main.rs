mod config;
mod db;
mod utils;
mod vault;
mod server;

use crate::config::{Config, Env};
use crate::utils::{read_file, truncate};
use crate::vault::auth::Authentication;
use crate::vault::link::Link;
use crate::vault::{init_vault, Vault};

use clap::{load_yaml, App, ArgMatches};
use colored::Colorize;
use log::Level;
use miniserde::json;
use semver::Version;
use std::process::exit;
use terminal_size::{terminal_size as ts, Width};

const LOG_LEVEL: Level = Level::Warn;
const VERSION: &'static str = env!("CARGO_PKG_VERSION");

fn main() {
    let yaml = load_yaml!("cli.yml");
    let config = Config::default();
    let matches = App::from(yaml).get_matches();
    let db = matches
        .value_of("database")
        .or(config.get(Env::Database))
        .expect("Cannot find a database. Use --db parameter or LINKIFY_DB_PATH env variable.");

    simple_logger::init_with_level(config.get(Env::LogLevel).map_or(LOG_LEVEL, |l| match l {
        "info" => Level::Info,
        "debug" => Level::Debug,
        "error" => Level::Error,
        _ => LOG_LEVEL,
    }))
    .unwrap();

    match init_vault(db, Version::parse(VERSION).unwrap()) {
        Ok(v) => {
            if matches.is_present("server") {
                server::start(v);
            } else {
                process_command(config, v, matches)
            }
        }
        _ => panic!("Cannot initialize database"),
    }
}

fn process_command(config: Config, vault: Vault, matches: ArgMatches) {
    match matches.subcommand() {
        ("add", Some(sub_m)) => {
            match vault.add_link(
                &Link::from_matches(sub_m),
                &Authentication::from_matches(config, sub_m),
            ) {
                Ok(id) => println!("Added (id={})", id),
                Err(e) => {
                    eprintln!("Error while adding a link ({:?})", e);
                    exit(1);
                }
            }
        }
        ("del", Some(sub_m)) => {
            match vault.del_link(
                &Link::from_matches(sub_m),
                &Authentication::from_matches(config, sub_m),
            ) {
                Ok(id) => println!("Deleted (id={})", id),
                Err(e) => {
                    eprintln!("Error while deleting a link ({:?})", e);
                    exit(1);
                }
            }
        }
        ("ls", Some(sub_m)) => {
            match vault.match_links(
                &Link::from_matches(sub_m),
                &Authentication::from_matches(config, sub_m),
                None,
                false
            ) {
                Ok(links) => {
                    let size = ts();
                    let tw = if let Some((Width(w), _)) = size {
                        w as i16
                    } else {
                        i16::max_value()
                    };
                    for link in links {
                        let href_len = link.href.chars().count() as i16;
                        let desc_len = tw - href_len - 3;
                        println!(
                            "{} » {}",
                            link.href,
                            truncate(&link.title, desc_len).blue()
                        )
                    }
                }
                Err(e) => {
                    eprintln!("Error while fetching links ({:?}).", e);
                    exit(1);
                }
            }
        }
        ("import", Some(sub_m)) => {
            let contents = read_file(sub_m.value_of("file").expect("Cannot read file."));
            let links: Vec<Link> = json::from_str(&contents).expect("Invalid JSON.");
            match vault.import_links(links, &Authentication::from_matches(config, sub_m)) {
                Ok(n) => println!("Imported {} links.", n),
                Err(e) => {
                    eprintln!("Error while importing links ({:?}).", e);
                    exit(1);
                }
            }
        }
        ("users", Some(sub_m)) => match sub_m.subcommand() {
            ("add", Some(sub_m)) => match vault.add_user(sub_m.value_of("login")) {
                Ok(u) => println!("Added ({}).", u.login),
                Err(_) => {
                    eprintln!("Error while adding new user. User might already exist.");
                    exit(1);
                }
            },
            ("del", Some(sub_m)) => match vault.del_user(sub_m.value_of("login")) {
                Ok((u, is_deleted)) => {
                    if is_deleted {
                        println!("Removed ({}).", u.login)
                    } else {
                        println!("Abandoned.")
                    }
                }
                Err(e) => {
                    eprintln!("Error while removing user ({:?}).", e);
                    exit(1);
                }
            },
            ("passwd", Some(sub_m)) => match vault.passwd_user(sub_m.value_of("login")) {
                Ok(u) => println!("Changed ({}).", u.login),
                Err(e) => {
                    eprintln!("Error while changing password ({:?}).", e);
                    exit(1);
                }
            },
            ("ls", Some(sub_m)) => match vault.match_users(sub_m.value_of("login")) {
                Ok(users) => {
                    for (user, count) in users {
                        println!("{} ({})", user, count);
                    }
                }
                Err(_) => {
                    eprintln!("Error while fetching users.");
                    exit(1);
                }
            },
            ("gen", Some(sub_m)) => match vault.generate_key(sub_m.value_of("login")) {
                Ok((_u, k)) => println!(
                    "Generated API key: {}\nSample cURL:\n\n  \
                    curl -H 'Authorization: Bearer {}' \
                    \'http://localhost:8001?tags=rust&notes=programming\'\n",
                    k, k
                ),
                Err(e) => {
                    eprintln!("Error while generating API key ({:?})", e);
                    exit(1);
                }
            },
            _ => (),
        },
        _ => {}
    }
}
