use crate::db::DBError::{BadPassword, Unauthenticated, UnknownUser};
use crate::db::DBResult;
use crate::utils::password;
use crate::vault::user::User;
use crate::vault::vault::Vault;

use bcrypt::verify;
use clap::ArgMatches;
use rusqlite::params;

#[derive(Debug)]
pub struct Authentication {
    pub login: String,
    pub password: String,
}

impl Authentication {
    pub fn from(matches: &ArgMatches) -> Option<Self> {
        matches.value_of("user").map_or(None, |login| {
            Some(Authentication {
                login: login.to_string(),
                password: password(matches.value_of("password"), None),
            })
        })
    }
}

impl Vault {
    pub fn authenticate_user(&self, auth: &Option<Authentication>) -> DBResult<User> {
        auth.as_ref().map_or(Err(Unauthenticated), |a| {
            self.connection
                .query_row(
                    "SELECT id, login, password FROM users WHERE login = ?1",
                    params![a.login],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .map_or(Err(UnknownUser), |user: (i64, String, String)| {
                    if verify(&a.password, &user.2).unwrap_or(false) {
                        Ok(User::new(user.0, &user.1))
                    } else {
                        Err(BadPassword)
                    }
                })
        })
    }
}
