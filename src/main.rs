mod db;
mod utils;
mod vault;

use crate::utils::read_file;
use crate::vault::auth::Authentication;
use crate::vault::link::Link;
use crate::vault::vault::{init_vault, Vault};

use clap::{App, ArgMatches, load_yaml};
use log::Level;
use miniserde::json;
use semver::Version;
use std::process::exit;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

fn main() {
    simple_logger::init_with_level(Level::Debug).unwrap();

    let yaml = load_yaml!("cli.yml");
    let matches = App::from(yaml).get_matches();
    let db = matches.value_of("database").unwrap_or("links.db");
    match init_vault(db, Version::parse(VERSION).unwrap()) {
        Ok(v) => process_command(v, matches),
        _ => panic!("Cannot initialize database"),
    }
}

fn process_command(mut vault: Vault, matches: ArgMatches) {
    match matches.subcommand() {
        ("add", Some(sub_m)) => {
            match vault.add_link(&mut Link::from(sub_m), &Authentication::from(sub_m)) {
                Ok(link) => println!("{}", link),
                Err(e) => {
                    eprintln!("Error while adding a link ({:?})", e);
                    exit(1);
                }
            }
        }
        ("ls", Some(sub_m)) => {
            match vault.match_links(&Link::from(sub_m), &Authentication::from(sub_m)) {
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
            match vault.import_links(links, &Authentication::from(sub_m)) {
                Ok(n) => println!("Imported {} links.", n),
                Err(e) => {
                    eprintln!("Error while importing links ({:?}).", e);
                    exit(1);
                }
            }
        }
        ("users", Some(sub_m)) => match sub_m.subcommand() {
            ("add", Some(sub_m)) => match vault.add_user(&Authentication::from(sub_m)) {
                Ok(u) => println!("Added ({}).", u.login),
                Err(_) => {
                    eprintln!("Error while adding new user. User might already exist.");
                    exit(1);
                }
            },
            ("passwd", Some(sub_m)) => match vault.passwd_user(&Authentication::from(sub_m)) {
                Ok(u) => println!("Changed ({}).", u.login),
                Err(e) => {
                    eprintln!("Error while changing password ({:?}).", e);
                    exit(1);
                }
            },
            ("ls", Some(sub_m)) => match vault.match_users(sub_m.value_of("user")) {
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
