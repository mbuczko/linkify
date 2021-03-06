use crate::config::{Config, Env};
use crate::db::DBError::{BadPassword, Unauthenticated, UnknownUser};
use crate::db::DBResult;
use crate::vault::user::User;
use crate::vault::Vault;

use bcrypt::verify;
use clap::ArgMatches;
use log::debug;
use miniserde::Serialize;
use rusqlite::params;

#[derive(Debug)]
pub struct ApiKey(String);

#[derive(Clone, Serialize, Debug)]
pub struct UserInfo {
    pub login: String,
    pub token: String,
}

pub enum Authentication {
    Credentials(String, String),
    Token(ApiKey),
}

impl Authentication {
    pub fn from_token(token: Option<&str>) -> Option<Self> {
        token.map(|t| Authentication::Token(ApiKey(t.to_string())))
    }
    pub fn from_credentials(login: String, password: String) -> Option<Self> {
        Some(Authentication::Credentials(login, password))
    }
    pub fn from_matches(config: Config, matches: &ArgMatches) -> Option<Self> {
        matches
            .value_of("apikey")
            .or_else(|| config.get(Env::ApiKey))
            .map(|apikey| Self::from_token(Some(apikey)))
            .unwrap()
    }
}

impl Vault {
    pub fn authenticate_user(&self, auth: &Option<Authentication>) -> DBResult<User> {
        auth.as_ref().map_or(Err(Unauthenticated), |a| match a {
            Authentication::Credentials(login, password) => {
                debug!("Authenticating with credentials ({}).", login);
                self.get_connection()
                    .query_row(
                        "SELECT id, login, password FROM users WHERE login = ?1",
                        params![login],
                        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                    )
                    .map_or(Err(UnknownUser), |user: (i64, String, String)| {
                        if verify(&password, &user.2).unwrap_or(false) {
                            Ok(User::new(user.0, &user.1))
                        } else {
                            Err(BadPassword)
                        }
                    })
            }
            Authentication::Token(token) => {
                debug!("Authenticating with token.");
                self.get_connection()
                    .query_row(
                        "SELECT id, login FROM users WHERE api_key = ?1",
                        params![token.0],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .map_or(Err(UnknownUser), |user: (i64, String)| {
                        Ok(User::new(user.0, &user.1))
                    })
            }
        })
    }

    pub fn user_info(&self, auth: &Option<Authentication>) -> DBResult<UserInfo> {
        let user = self.authenticate_user(auth)?;
        self.get_connection()
            .query_row(
                "SELECT api_key FROM users WHERE id = ?1",
                params![user.id],
                |row| row.get(0),
            )
            .map_or(Err(UnknownUser), |token| {
                Ok(UserInfo {
                    login: user.login,
                    token,
                })
            })
    }
}
