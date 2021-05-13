mod config;
mod db;
mod server;
mod utils;
mod vault;

use config::{Config, Env};
use utils::{password, read_file, truncate};
use vault::auth::Authentication;
use vault::link::{Link, Version};
use vault::Vault;

use clap::{load_yaml, App, ArgMatches};
use colored::Colorize;
use db::DBLookupType;
use log::Level;
use miniserde::json;
use std::process::exit;
use terminal_size::{terminal_size as ts, Width};

const LOG_LEVEL: Level = Level::Warn;
const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    let yaml = load_yaml!("cli.yml");
    let config = Config::default();
    let matches = App::from(yaml).get_matches();
    let db = matches
        .value_of("database")
        .or_else(|| config.get(Env::Database))
        .expect("Cannot find a database. Use --db parameter or LINKIFY_DB_PATH env variable.");

    simple_logger::init_with_level(config.get(Env::LogLevel).map_or(LOG_LEVEL, |l| match l {
        "info" => Level::Info,
        "debug" => Level::Debug,
        "error" => Level::Error,
        _ => LOG_LEVEL,
    }))
    .unwrap();

    match vault::init_vault(Some(db), semver::Version::parse(VERSION).unwrap()) {
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
                &Authentication::from_matches(config, sub_m),
                Link::from_matches(sub_m),
            ) {
                Ok(version) => {
                    println!("Added (version={})", version)
                }
                Err(e) => {
                    eprintln!("Error while adding a link ({:?})", e);
                    exit(1);
                }
            }
        }
        ("del", Some(sub_m)) => {
            match vault.del_link(
                &Authentication::from_matches(config, sub_m),
                matches.value_of("url").unwrap_or("<unknown>"),
            ) {
                Ok(Some(link)) => println!("Deleted (id={})", link.id.unwrap()),
                Ok(None) => {
                    eprintln!("No such a link found");
                    exit(1);
                }
                Err(e) => {
                    eprintln!("Error while deleting a link ({:?})", e);
                    exit(1);
                }
            }
        }
        ("ls", Some(sub_m)) => {
            let auth = Authentication::from_matches(config, sub_m);
            let query = sub_m.value_of("query").unwrap_or_default().to_string();
            let links = if query.starts_with('@') {
                let chunks: Vec<&str> = query.splitn(2, '/').collect();
                let stored_query = chunks.first().unwrap().strip_prefix('@');
                match vault.find_queries(&auth, stored_query, DBLookupType::Exact) {
                    Ok(queries) => {
                        if queries.len() != 1 {
                            eprintln!("No stored query found ({})", stored_query.unwrap());
                            exit(1);
                        } else {
                            let stored = queries.get(0).map(|q| q.query.clone()).unwrap();
                            let query = chunks.get(1).unwrap_or(&"");
                            vault.query_links(
                                &auth,
                                format!("{} {}", stored, query),
                                Version::unknown(),
                                None,
                            )
                        }
                    }
                    Err(e) => {
                        eprintln!("Error while fetching stored query ({:?}).", e);
                        exit(1);
                    }
                }
            } else {
                vault.query_links(&auth, query, Version::unknown(), None)
            };

            match links {
                Ok((links, _)) => {
                    let size = ts();
                    let tw = if let Some((Width(w), _)) = size {
                        w as i16
                    } else {
                        i16::max_value()
                    };
                    for link in links {
                        let href_len = link.href.chars().count() as i16;
                        let desc_len = tw - href_len - 3;
                        println!("{} | {}", link.href, truncate(&link.name, desc_len).blue())
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
            match vault.import_links(&Authentication::from_matches(config, sub_m), links) {
                Ok(n) => println!("Imported {} links.", n),
                Err(e) => {
                    eprintln!("Error while importing links ({:?}).", e);
                    exit(1);
                }
            }
        }
        ("users", Some(sub_m)) => match sub_m.subcommand() {
            ("add", Some(sub_m)) => {
                let pass = password(None, Some("Initial password"));
                match vault.add_user(sub_m.value_of("login").unwrap(), &pass) {
                    Ok(u) => println!("Added ({}).", u.login),
                    Err(_) => {
                        eprintln!("Error while adding new user. User might already exist.");
                        exit(1);
                    }
                }
            }
            ("del", Some(sub_m)) => match vault.del_user(sub_m.value_of("login").unwrap()) {
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
            ("passwd", Some(sub_m)) => match vault.passwd_user(sub_m.value_of("login").unwrap()) {
                Ok(u) => println!("Changed ({}).", u.login),
                Err(e) => {
                    eprintln!("Error while changing password ({:?}).", e);
                    exit(1);
                }
            },
            ("ls", Some(sub_m)) => {
                match vault.match_users(sub_m.value_of("login").unwrap_or_default()) {
                    Ok(users) => {
                        for (user, count) in users {
                            println!("{} ({})", user, count);
                        }
                    }
                    Err(_) => {
                        eprintln!("Error while fetching users.");
                        exit(1);
                    }
                }
            }
            ("token", Some(sub_m)) => match vault.generate_key(sub_m.value_of("login").unwrap()) {
                Ok((_u, k)) => println!(
                    "Generated API key: {}\nSample cURL:\n\n  \
                    curl -H 'Authorization: Bearer {}' \
                    \'http://localhost:8001/links?q=tags:rust\'\n",
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
