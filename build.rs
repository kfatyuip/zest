use std::{env, fs, path::Path};
use toml::Value;

fn main() {
    let root_path = env::var("CARGO_MANIFEST_DIR").unwrap();

    let config_path = Path::new(&root_path).join("config.yaml");
    if config_path.exists() {
        let content = fs::read_to_string(config_path).unwrap();
        let config: Value = serde_yml::from_str(&content).unwrap();

        if config.get("allowip").is_some() {
            println!("cargo:rustc-cfg=feature=\"allow_ip\"");
        }
        if config.get("blockip").is_some() {
            println!("cargo:rustc-cfg=feature=\"block_ip\"");
        }
    }
}
