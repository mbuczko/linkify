extern crate rusqlite;
extern crate semver;

use log::{debug, info};
use rusqlite::{Connection, Result, NO_PARAMS};
use semver::Version;
use std::cmp::Ordering;
use std::fmt;
use std::str;
use clap::Values;

#[derive(RustEmbed)]
#[folder = "resources/db/migrations/"]
struct Asset;

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Migration {
    file: String,
    version: String,
    description: String,
}

#[derive(Debug)]
pub struct Vault {
    connection: Connection,
}

impl Migration {
    pub fn new(file: String, version: String, description: String) -> Self {
        Migration {
            file: file,
            version: version,
            description: description,
        }
    }
}

impl fmt::Display for Migration {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} ({}) => {}",
            self.version, self.description, self.file
        )
    }
}

impl Vault {
    fn build_migration(&self, base_version: String, app_semver: Version) -> Option<String> {
        let mut migrations: Vec<Migration> = Asset::iter()
            .map(|file| {
                let v: Vec<&str> = file.split("__").collect();
                Migration::new(
                    file.to_string(),
                    v[0].to_string(),
                    v[1].trim_end_matches(".sql").replace("_", " "),
                )
            })
            .collect();

        // sort migrations by versions first
        migrations.sort_by(|m1, m2| m1.version.cmp(&m2.version));

        // ...and keep only those which haven't been applied yet
        migrations.retain(|m| base_version.is_empty() || m.version.gt(&base_version));

        // compose final transaction
        let final_txn = migrations.iter().fold(String::default(), |mut txn, m| {
            let buf = Asset::get(m.file.as_ref()).unwrap();
            match str::from_utf8(&buf) {
                Ok(s) => {
                    txn.push_str(s);
                    txn.push_str(
                        format!(
                            "\n\n\
                    INSERT INTO migrations(version, description, script, app_semver) \
                    VALUES ('{version}', '{description}', '{script}', '{semver}');\n\n",
                            version = m.version,
                            description = m.description,
                            script = m.file,
                            semver = app_semver
                        )
                        .as_str(),
                    );
                    txn
                }
                _ => panic!("Non UTF8 format of migration file!"),
            }
        });
        return if final_txn.is_empty() {
            None
        } else {
            Some(format!("BEGIN TRANSACTION;\n\n{}\n\nCOMMIT;", final_txn))
        };
    }
    pub fn version(&self) -> Result<(String, Version)> {
        self.connection.query_row(
            "SELECT version, app_semver FROM migrations ORDER BY version DESC LIMIT 1",
            NO_PARAMS,
            |row| {
                let ver: String = row.get(1)?;
                Ok((row.get(0)?, Version::parse(&ver).unwrap()))
            },
        )
    }
    pub fn upgrade(&self, base_script_version: String, app_semver: Version) {
        match self.build_migration(base_script_version, app_semver) {
            Some(m) => {
                self.connection.execute_batch(m.as_str());
                debug!("Upgraded database to {}", self.version().unwrap().0)
            }
            _ => debug!("Database up to date."),
        }
    }
    pub fn add_link(&self, url: &str, desc: Option<&str>, tags: Option<Values>) {
        println!("adding {} => {}", url, desc.unwrap_or(""));
        if let Some(t) = tags {
            for tag in t.collect::<Vec<&str>>() {
                println!("-> {}", tag)
            }
        }
    }
    pub fn new(db: &str) -> Self {
        match Connection::open(db) {
            Ok(conn) => Vault { connection: conn },
            _ => panic!("Cannot open connection to database"),
        }
    }
}

pub fn init_vault(db: &str, app_semver: Version) -> Result<Vault> {
    let vault = Vault::new(db);
    let (last_script_version, last_app_version) = match vault.version() {
        Ok((lsv, lav)) => (lsv, lav),
        Err(_) => (String::default(), Version::parse("0.0.0").unwrap()),
    };

    if last_app_version > app_semver {
        panic!(
            "Your app version {} is too old, minimal required version is: {}",
            app_semver, last_app_version
        )
    } else if last_app_version < app_semver {
        debug!("Upgrading data version to {}", app_semver);
        vault.upgrade(last_script_version, app_semver);
    }
    Ok(vault)
}
