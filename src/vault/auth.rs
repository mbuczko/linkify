use crate::config::{Config, Env};
use crate::db::DBError::{BadPassword, Unauthenticated, UnknownUser};
use crate::db::DBResult;
use crate::utils::password;
use crate::vault::user::User;
use crate::vault::Vault;

use bcrypt::verify;
use clap::ArgMatches;
use log::debug;
use rusqlite::params;

#[derive(Debug)]
pub struct UserPass {
    login: String,
    password: String,
}

#[derive(Debug)]
pub struct ApiKey(String);

pub enum Authentication {
    User(UserPass),
    Token(ApiKey),
}

impl Authentication {
    pub fn from_token(token: Option<&str>) -> Option<Self> {
        match token {
            Some(t) => Some(Authentication::Token(ApiKey(t.to_string()))),
            _ => None,
        }
    }
    pub fn from_matches(config: Config, matches: &ArgMatches) -> Option<Self> {
        let token = matches.value_of("apikey").or_else(|| config.get(Env::ApiKey));
        if token.is_some() {
            Self::from_token(token)
        } else {
            matches
                .value_of("user")
                .or_else(|| config.get(Env::User))
                .map(|login| {
                    debug!("Authenticating ({}).", login);
                    Authentication::User(UserPass {
                        login: login.to_string().to_ascii_lowercase(),
                        password: password(
                            matches.value_of("password").or_else(|| config.get(Env::Password)),
                            None,
                        ),
                    })
                })
        }
    }
}

impl Vault {
    pub fn authenticate_user(&self, auth: Option<Authentication>) -> DBResult<User> {
        auth.as_ref().map_or(Err(Unauthenticated), |a| match a {
            Authentication::User(user_login) => self
                .get_connection()
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
                .get_connection()
                .query_row(
                    "SELECT id, login FROM users WHERE api_key = ?1",
                    params![token.0],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .map_or(Err(UnknownUser), |user: (i64, String)| {
                    Ok(User::new(user.0, &user.1))
                }),
        })
    }
}
