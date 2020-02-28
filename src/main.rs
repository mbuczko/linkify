mod db;
mod link;
mod query;
mod user;
mod utils;

use crate::link::Link;
use crate::user::Authentication;
use clap::{App, Arg, ArgMatches, SubCommand};
use db::{init_vault, Vault};
use log::Level;
use semver::Version;
use std::process::exit;
use miniserde::json;
use crate::utils::read_file;

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
                    Arg::with_name("user")
                        .help("an owner of stored link")
                        .short("u")
                        .long("user")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("password")
                        .help("owner's password")
                        .short("p")
                        .long("pass")
                        .takes_value(true),
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
            SubCommand::with_name("import")
                .about("Import links from JSON file")
                .arg(
                    Arg::with_name("file")
                        .help("JSON file to import")
                        .required(true),
                )
                .arg(
                    Arg::with_name("user")
                        .help("an owner of stored link")
                        .short("u")
                        .long("user")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("password")
                        .help("owner's password")
                        .short("p")
                        .long("pass")
                        .takes_value(true),
                )
        )
        .subcommand(
            SubCommand::with_name("ls")
                .about("Lists matching links")
                .arg(Arg::with_name("url").help("link or its part to match"))
                .arg(
                    Arg::with_name("description")
                        .help("optional part of description to match")
                        .short("d")
                        .long("desc")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("user")
                        .help("an owner of stored link")
                        .short("u")
                        .long("user")
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
        .subcommand(
            SubCommand::with_name("users")
                .about("Manage with users")
                .subcommand(
                    SubCommand::with_name("add")
                        .about("Adds a new user")
                        .arg(
                            Arg::with_name("user")
                                .help("User's identifier (login)")
                                .required(true),
                        )
                        .arg(
                            Arg::with_name("password")
                                .help("user's password")
                                .short("p")
                                .long("pass")
                                .takes_value(true),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("passwd")
                        .about("Changes user's password")
                        .arg(
                            Arg::with_name("user")
                                .help("User's identifier (login)")
                                .required(true),
                        )
                        .arg(
                            Arg::with_name("password")
                                .help("user's new password")
                                .short("p")
                                .long("pass")
                                .takes_value(true),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("ls")
                        .about("Lists matching users")
                        .arg(Arg::with_name("login").help("User's identifier pattern to list")),
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
        },
        ("users", Some(sub_m)) => match sub_m.subcommand() {
            ("add", Some(sub_m)) => match vault.add_user(&Authentication::from(sub_m)) {
                Ok(_) => println!("Added."),
                Err(_) => {
                    eprintln!("Error while adding new user. User might already exist.");
                    exit(1);
                }
            },
            ("passwd", Some(sub_m)) => match vault.passwd_user(&Authentication::from(sub_m)) {
                Ok(_) => println!("Changed."),
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
