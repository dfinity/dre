use std::env;

/// Return the configuration value from the environment.
pub fn env_cfg(key: &str) -> String {
    match env::var(key) {
        Ok(value) => value,
        Err(err) => panic!(
            "Environment variable `{}` is not set. Please update the .env file. {}",
            key, err
        ),
    }
}
