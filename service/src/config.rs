use std::fs::{create_dir_all, File};
use std::io::{Read, Write};
use std::path::PathBuf;

use crate::types::Config;
use tracing::info;

pub fn config_path() -> PathBuf {
    if let Ok(p) = std::env::var("FRAMEWORK_CONTROL_CONFIG") {
        return PathBuf::from(p);
    }
    // Prefer ProgramData for system-wide service config
    let base = std::env::var("PROGRAMDATA").unwrap_or_else(|_| r"C:\ProgramData".into());
    PathBuf::from(base)
        .join("FrameworkControl")
        .join("config.json")
}

pub fn load() -> Config {
    let path = config_path();
    if let Ok(mut f) = File::open(&path) {
        let mut buf = String::new();
        if let Err(e) = f.read_to_string(&mut buf) {
            tracing::warn!("Failed to read config file {:?}: {}", path, e);
            return Config::default();
        }
        match serde_json::from_str::<Config>(&buf) {
            Ok(cfg) => {
                info!("Loaded config from {:?}", path);
                return cfg;
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to parse config file {:?}: {}. Using defaults.",
                    path,
                    e
                );
            }
        }
    } else {
        tracing::debug!("No config file at {:?}, using defaults", path);
    }
    Config::default()
}
