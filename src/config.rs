use std::collections::HashMap;
use std::env;

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum Env {
    Database,
    ApiKey,
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
        if let Ok(apikey) = env::var("LINKIFY_API_KEY") {
            config.values.insert(Env::ApiKey, apikey);
        }
        config
    }
    pub fn get(&self, key: Env) -> Option<&str> {
        self.values.get(&key).map(String::as_str)
    }
}
