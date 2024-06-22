use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::fs;
use std::net::IpAddr;

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub bind: BindConfig,
    pub server: ServerConfig,
    pub allowlist: Option<IpListConfig>,
    pub blacklist: Option<IpListConfig>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct BindConfig {
    pub host: String,
    pub port: i32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ServerConfig {
    pub info: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct IpListConfig {
    pub ips: Vec<IpAddr>,
}

lazy_static! {
    pub static ref CONFIG: Config = init_config();
}

fn init_config() -> Config {
    match fs::read_to_string("config.yaml") {
        Ok(conf) => serde_yaml::from_str(&conf).unwrap(),
        _ => Config {
            bind: BindConfig {
                host: "0.0.0.0".to_owned(),
                port: 8080,
            },
            server: ServerConfig {
                info: "Powered by Rust".to_owned(),
            },
            allowlist: None,
            blacklist: None,
        },
    }
}
