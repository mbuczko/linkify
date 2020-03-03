mod config;
mod db;
mod utils;
mod vault;

use crate::config::{Config, Env};
use crate::utils::read_file;
use crate::vault::auth::Authentication;
use crate::vault::link::Link;
use crate::vault::vault::{init_vault, Vault};

use clap::{load_yaml, App, ArgMatches};
use log::Level;
use miniserde::json;
use semver::Version;
use std::process::exit;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

fn main() {
    simple_logger::init_with_level(Level::Debug).unwrap();

    let yaml = load_yaml!("cli.yml");
    let config = Config::default();
    let matches = App::from(yaml).get_matches();
    let db = matches
        .value_of("database")
        .or(config.get(Env::Database))
        .expect(
            "Cannot find a database. Use --db parameter or LINKIFY_DB_PATH env variable.",
        );
    match init_vault(db, Version::parse(VERSION).unwrap()) {
        Ok(v) => process_command(config, v, matches),
        _ => panic!("Cannot initialize database"),
    }
}

fn process_command(config: Config, mut vault: Vault, matches: ArgMatches) {
    match matches.subcommand() {
        ("add", Some(sub_m)) => {
            match vault.add_link(&Link::from(sub_m), &Authentication::from(config, sub_m)) {
                Ok(id) => println!("Added (id={})", id),
                Err(e) => {
                    eprintln!("Error while adding a link ({:?})", e);
                    exit(1);
                }
            }
        }
        ("ls", Some(sub_m)) => {
            match vault.match_links(&Link::from(sub_m), &Authentication::from(config, sub_m)) {
                Ok(links) => {
                    for link in links {
                        println!("{}", link)
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
            match vault.import_links(links, &Authentication::from(config, sub_m)) {
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
            ("rm", Some(sub_m)) => match vault.del_user(sub_m.value_of("login")) {
                Ok(u) => {
                    if u.is_some() {
                        println!("Removed ({}).", u.unwrap().login)
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
            _ => (),
        },
        _ => {}
    }
}
