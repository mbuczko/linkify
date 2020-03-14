use crate::config::{Config, Env};
use crate::db::DBError::{BadPassword, Unauthenticated, UnknownUser};
use crate::db::DBResult;
use crate::utils::password;
use crate::vault::user::User;
use crate::vault::vault::Vault;

use bcrypt::verify;
use clap::ArgMatches;
use log::debug;
use rand::Rng;
use rusqlite::params;

#[derive(Debug)]
pub struct UserPass {
    login: String,
    password: String,
}

pub enum Authentication {
    User(UserPass),
    Token(String),
}

const KEY_LEN: usize = 32;
const KEY_CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                             abcdefghijklmnopqrstuvwxyz\
                             0123456789";

impl Authentication {
    pub fn from(config: Config, matches: &ArgMatches) -> Option<Self> {
        if let Some(key) = matches.value_of("apikey").or(config.get(Env::ApiKey)) {
            Some(Authentication::Token(key.to_string()))
        } else {
            matches
                .value_of("user")
                .or(config.get(Env::User))
                .map_or(None, |login| {
                    debug!("Authenticating ({}).", login);
                    Some(Authentication::User(UserPass {
                        login: login.to_string().to_ascii_lowercase(),
                        password: password(
                            matches.value_of("password").or(config.get(Env::Password)),
                            None,
                        ),
                    }))
                })
        }
    }
}

impl Vault {
    pub fn authenticate_user(&self, auth: &Option<Authentication>) -> DBResult<User> {
        auth.as_ref().map_or(Err(Unauthenticated), |a| match a {
            Authentication::User(user_login) => self
                .connection
                .query_row(
                    "SELECT id, login, password FROM users WHERE login = ?1",
                    params![user_login.login],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .map_or(Err(UnknownUser), |user: (i64, String, String)| {
                    if verify(&user_login.password, &user.2).unwrap_or(false) {
                        Ok(User::new(user.0, &user.1))
                    } else {
                        Err(BadPassword)
                    }
                }),
            Authentication::Token(token) => self
                .connection
                .query_row(
                    "SELECT id, login FROM users WHERE api_key = ?1",
                    params![token],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .map_or(Err(UnknownUser), |user: (i64, String)| {
                    Ok(User::new(user.0, &user.1))
                }),
        })
    }
    pub fn generate_api_key(&self) -> String {
        let mut rng = rand::thread_rng();
        (0..KEY_LEN)
            .map(|_| {
                let idx = rng.gen_range(0, KEY_CHARSET.len());
                KEY_CHARSET[idx] as char
            })
            .collect()
    }
}
