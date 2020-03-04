use std::collections::HashMap;
use std::env;

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum Env {
    Database,
    LogLevel,
    User,
    Password,
}

pub struct Config {
    pub values: HashMap<Env, String>,
}

impl Config {
    pub fn default() -> Self {
        let mut config = Config {
            values: HashMap::<Env, String>::with_capacity(3),
        };
        if let Ok(path) = env::var("LINKIFY_DB_PATH") {
            config.values.insert(Env::Database, path);
        }
        if let Ok(user) = env::var("LINKIFY_USER") {
            config.values.insert(Env::User, user);
        }
        if let Ok(password) = env::var("LINKIFY_PASSWORD") {
            config.values.insert(Env::Password, password);
        }
        if let Ok(password) = env::var("LOG_LEVEL") {
            config.values.insert(Env::LogLevel, password);
        }
        config
    }
    pub fn get(&self, key: Env) -> Option<&str> {
        self.values.get(&key).map(String::as_str)
    }
}
