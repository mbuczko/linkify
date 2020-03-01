use crate::db::{DBError, DBResult};

use log::debug;
use rusqlite::{vtab::array, Connection, Error as SqliteError, Result as SqliteResult, NO_PARAMS};
use rust_embed::RustEmbed;
use semver::Version;
use std::fmt;
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
    pub connection: Connection,
}

impl From<SqliteError> for DBError {
    fn from(err: SqliteError) -> Self {
        DBError::Sqlite(err)
    }
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

        // keep only those which haven't been applied yet
        migrations.retain(|m| base_version.is_empty() || m.version.gt(&base_version));

        // ...and compose final transaction
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
    fn version(&self) -> DBResult<(String, Version)> {
        self.connection
            .query_row(
                "SELECT version, app_semver FROM migrations ORDER BY version DESC LIMIT 1",
                NO_PARAMS,
                |row| {
                    Ok((
                        row.get(0)?,
                        Version::parse(&row.get::<_, String>(1)?).unwrap(),
                    ))
                },
            )
            .map_err(Into::into)
    }
    fn upgrade(&self, base_script_version: String, app_semver: Version) {
        if let Some(m) = self.build_migration(base_script_version, app_semver) {
            match self.connection.execute_batch(m.as_str()) {
                Ok(_) => debug!("Upgraded to {}", self.version().unwrap().0),
                _ => panic!("Couldn't update the database. Bailing out."),
            }
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

pub fn init_vault(db: &str, app_semver: Version) -> SqliteResult<Vault> {
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

    // Foreign key support is not enabled in SQLite by default
    vault.connection.execute("PRAGMA foreign_keys = ON", NO_PARAMS)?;
    Ok(vault)
}
