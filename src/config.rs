use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::{env::current_dir, fs, path::PathBuf, sync::Mutex};

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub bind: BindConfig,
    pub server: ServerConfig,
    pub allowlist: Option<Vec<String>>,
    pub blocklist: Option<Vec<String>>,
    pub rate_limit: Option<RateLimitConfig>,
    pub logging: Option<LoggingConfig>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct BindConfig {
    pub addr: String,
    pub listen: i32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ServerConfig {
    pub info: String,
    pub root: PathBuf,
    pub auto_index: Option<bool>,
    pub index: Option<PathBuf>,
    pub error_page: Option<PathBuf>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RateLimitConfig {
    pub max_requests: usize,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LoggingConfig {
    pub access_log: Option<String>,
    pub error_log: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            bind: BindConfig {
                addr: "0.0.0.0".to_owned(),
                listen: 80,
            },
            server: ServerConfig {
                info: "Powered by Rust".to_owned(),
                root: current_dir().unwrap_or(".".into()),
                auto_index: Some(false),
                index: None,
                error_page: Some("404.html".to_owned().into()),
            },
            allowlist: None,
            blocklist: None,
            rate_limit: None,
            logging: None,
        }
    }
}

lazy_static! {
    pub static ref CONFIG_PATH: Mutex<String> = Mutex::new("".to_owned());
    pub static ref CONFIG: Config = init_config();
}

fn init_config() -> Config {
    let config_path = CONFIG_PATH.lock().unwrap();
    let default_config = Config::default();
    match fs::read_to_string(config_path.to_owned()) {
        Ok(conf) => serde_yml::from_str(&conf).unwrap_or(default_config),
        _ => default_config,
    }
}
