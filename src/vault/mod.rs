pub mod auth;
pub mod link;

mod migrations;
mod stored_query;
mod tags;
mod user;

use super::db::conn_manager;

use log::debug;
use std::path::Path;
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Result as SqliteResult;
use semver::Version;

pub struct Vault {
    pool: Pool<SqliteConnectionManager>,
}

impl Vault {
    pub fn get_connection(&self) -> PooledConnection<SqliteConnectionManager> {
        self.pool.get().unwrap()
    }
    pub fn new<P: AsRef<Path>>(db: P) -> Self {
        let manager = conn_manager(db);
        match r2d2::Pool::new(manager) {
            Ok(pool) => Vault { pool },
            _ => panic!("Cannot open connection to database"),
        }
    }
}

pub fn init_vault<P: AsRef<Path>>(db: P, app_semver: Version) -> SqliteResult<Vault> {
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

#[cfg(test)]
pub mod test_db {
    use super::*;
    use crate::vault::auth::Authentication;
    use crate::utils::random_string;
    use rstest::*;
    use lazy_static::lazy_static;
    use tempfile::NamedTempFile;

    lazy_static! {
        static ref VAULT: Vault = {
            let appver = semver::Version::parse(env!("CARGO_PKG_VERSION"));
            if let Ok(tmpfile) = NamedTempFile::new() {
                init_vault(tmpfile, appver.unwrap()).unwrap()
            } else {
                panic!("Cannot create temporary database file.");
            }
        };
    }

    #[fixture]
    pub fn vault() -> &'static Vault {
       &*VAULT
    }

    #[fixture]
    pub fn auth<S: AsRef<str>>(#[default(random_string(8))] login: S) -> Option<Authentication> {
        let pass = "secret";
        let name = login.as_ref();

        vault().add_user(name, pass).unwrap();
        vault().generate_key(name).unwrap();

        Authentication::from_credentials(name.to_string(), pass.to_owned())
    }
}
