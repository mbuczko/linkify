use crate::link::Link;
use log::debug;
use rusqlite::types::Value;
use rusqlite::{params, vtab::array, Connection, Result, NO_PARAMS};
use rust_embed::RustEmbed;
use semver::Version;
use std::fmt;
use std::rc::Rc;
use std::str;

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
            file,
            version,
            description,
        }
    }
}

impl fmt::Display for Migration {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} => {} ({})",
            self.file, self.version, self.description
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
    fn version(&self) -> Result<(String, Version)> {
        self.connection.query_row(
            "SELECT version, app_semver FROM migrations ORDER BY version DESC LIMIT 1",
            NO_PARAMS,
            |row| {
                let ver: String = row.get(1)?;
                Ok((row.get(0)?, Version::parse(&ver).unwrap()))
            },
        )
    }
    fn upgrade(&self, base_script_version: String, app_semver: Version) {
        if let Some(m) = self.build_migration(base_script_version, app_semver) {
            match self.connection.execute_batch(m.as_str()) {
                Ok(_) => debug!("Upgraded to {}", self.version().unwrap().0),
                _ => panic!("Couldn't update the database. Bailing out."),
            }
        }
    }
    pub fn add_link(&mut self, link: &Link) {
        // insert a link first
        let txn = self.connection.transaction().unwrap();
        txn.execute(
            "INSERT INTO links(url, description, hash) VALUES(?1, ?2, ?3) \
            ON CONFLICT(url) \
            DO UPDATE SET description = ?2, hash = ?3",
            params![link.url, link.description, link.hash],
        )
        .expect("Couldn't add a link");

        // note that last_insert_rowid returns 0 for already existing URLs
        let id = match txn.last_insert_rowid() {
            0 => txn
                .query_row(
                    "SELECT id FROM links WHERE url = ?1 AND user_id IS NULL",
                    params![link.url],
                    |row| row.get(0),
                )
                .unwrap(),
            n => n,
        };

        // remove tags associated so far
        txn.execute("DELETE FROM links_tags WHERE link_id = ?1", params![id])
            .expect("Couldn't update tags");

        // join link with its tags (if provided)
        if let Some(tv) = &link.tags {
            let mut values: Vec<Value> = Vec::new();
            for tag in tv {
                txn.execute(
                    "INSERT INTO tags(tag, user_id) VALUES(?1, NULL) \
            ON CONFLICT(tag, user_id) \
            DO UPDATE SET used_at = CURRENT_TIMESTAMP",
                    params![tag],
                )
                .expect("Couldn't add tags");
                values.push(Value::from(tag.to_string()));
            }
            txn.execute(
                "INSERT INTO links_tags(link_id, tag_id) \
        SELECT ?1, id FROM tags WHERE tag IN rarray(?2) AND user_id IS NULL",
                params![id, Rc::new(values)],
            )
            .expect("Could not connect tags with link");
        }
        match txn.commit() {
            Ok(_) => println!("{}", link),
            _ => panic!("Couldn't add link"),
        }
    }
    pub fn new(db: &str) -> Self {
        match Connection::open(db) {
            Ok(conn) => {
                array::load_module(&conn).unwrap();
                Vault { connection: conn }
            }
            _ => panic!("Cannot open connection to database or load required modules (array)"),
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
