use figment::{providers::Env, Figment};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Default, Serialize)]
pub struct Config {
    pub matrix: MatrixConfiguration,
}

impl Config {
    pub fn new() -> Result<Self, figment::Error> {
        Figment::new()
            .merge(figment::providers::Serialized::defaults(Config::default()))
            .merge(Env::prefixed("NP_").split("_"))
            .extract()
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MatrixConfiguration {
    pub instance: url::Url,
    pub username: Option<String>,
    pub password: Option<String>,
}

impl Default for MatrixConfiguration {
    fn default() -> Self {
        Self {
            instance: url::Url::parse("https://matrix.org").unwrap(),
            username: None,
            password: None,
        }
    }
}
