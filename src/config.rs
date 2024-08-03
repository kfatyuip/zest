use async_rwlock::RwLock;
use clap::{command, Parser};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use serde_yml::Value;
use std::{collections::HashMap, env::current_dir, fs, path::PathBuf, sync::Mutex};

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub bind: BindConfig,
    pub server: ServerConfig,
    pub allowlist: Option<Vec<String>>,
    pub blocklist: Option<Vec<String>>,
    pub rate_limit: Option<RateLimitConfig>,
    pub locations: Option<HashMap<String, Value>>,
    pub logging: Option<LoggingConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            bind: BindConfig {
                addr: "0.0.0.0".to_owned(),
                listen: 8080,
            },
            server: ServerConfig {
                info: "Powered by Rust".to_owned(),
                root: current_dir().unwrap_or(".".into()),
                error_page: Some("404.html".to_owned().into()),
                cache: None,
            },
            allowlist: None,
            blocklist: None,
            rate_limit: None,
            locations: None,
            logging: None,
        }
    }
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
    pub error_page: Option<PathBuf>,
    pub cache: Option<CacheConfig>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CacheConfig {
    pub index_capacity: Option<usize>,
    pub file_capacity: Option<usize>,
}

impl Default for CacheConfig {
    fn default() -> Self {
        CacheConfig {
            index_capacity: Some(16),
            file_capacity: Some(32),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RateLimitConfig {
    pub max_requests: usize,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct LocationConfig {
    pub auto_index: Option<bool>,
    pub index: Option<PathBuf>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct LoggingConfig {
    pub access_log: Option<String>,
    pub error_log: Option<String>,
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(short, long, default_value = None, help = "set config file path")]
    pub config: Option<String>,

    #[arg(short, long, default_value = None, help = "set the root directory")]
    pub root: Option<PathBuf>,

    #[arg(short, long, default_value = None, help = "set the listening port")]
    pub port: Option<i32>,
}

lazy_static! {
    pub static ref CONFIG_PATH: Mutex<String> = Mutex::new("".to_owned());
    pub static ref DEFAULT_CONFIG: Config = init_config();
    pub static ref CONFIG: RwLock<Config> = RwLock::new((*DEFAULT_CONFIG).clone());
    pub static ref ARGS: Args = Args::parse();
}

pub fn init_config() -> Config {
    let config_path = CONFIG_PATH.lock().unwrap();
    let default_config = Config::default();
    let mut config = match fs::read_to_string(config_path.to_owned()) {
        Ok(conf) => serde_yml::from_str(&conf).unwrap_or(default_config),
        _ => default_config,
    };

    if let Some(root) = &ARGS.root {
        config.server.root = root.to_path_buf();
    }

    if let Some(port) = &ARGS.port {
        config.bind.listen = *port;
    }

    config
}
