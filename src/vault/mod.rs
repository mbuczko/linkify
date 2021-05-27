pub mod auth;
pub mod link;

mod migrations;
mod stored_query;
mod tags;
mod user;

use super::db::conn_manager;

use log::debug;
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
    pub fn new(db: Option<&str>) -> Self {
        let manager = conn_manager(db);
        match r2d2::Pool::new(manager) {
            Ok(pool) => Vault { pool },
            _ => panic!("Cannot open connection to database"),
        }
    }
}

pub fn init_vault(db: Option<&str>, app_semver: Version) -> SqliteResult<Vault> {
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
    use lazy_static::lazy_static;
    use rstest::*;

    const USER: &str = "foo";
    const PASS: &str = "secret";

    lazy_static! {
        static ref AUTH: Option<Authentication> = Some(Authentication::from_credentials(USER.to_owned(), PASS.to_owned()).unwrap());
        static ref VAULT: Vault = {
            let appver = semver::Version::parse(env!("CARGO_PKG_VERSION"));
            let vault = init_vault(None, appver.unwrap()).unwrap();

            // initialize testing vault with sample user
            vault.add_user(USER, PASS).unwrap();
            vault.generate_key(USER).unwrap();
            vault
        };
    }

    #[fixture]
    pub fn vault() -> &'static Vault {
        &*VAULT
    }

    #[fixture]
    pub fn auth() -> &'static Option<Authentication> {
        &*AUTH
    }
}
