use std::fs::{create_dir_all, File};
use std::io::{Read, Write};
use std::path::PathBuf;

use crate::types::Config;

pub fn config_path() -> PathBuf {
    if let Ok(p) = std::env::var("FRAMEWORK_CONTROL_CONFIG") {
        return PathBuf::from(p);
    }
    // Prefer APPDATA for user-mode config
    let base = std::env::var("APPDATA").unwrap_or_else(|_| r"C:\ProgramData".into());
    PathBuf::from(base)
        .join("FrameworkControl")
        .join("config.json")
}

pub fn load() -> Config {
    let path = config_path();
    if let Ok(mut f) = File::open(&path) {
        let mut buf = String::new();
        if let Err(e) = f.read_to_string(&mut buf) {
            // Failed to read config file
            return Config::default();
        }
        match serde_json::from_str::<Config>(&buf) {
            Ok(cfg) => {
                return cfg;
            }
            Err(e) => {
                // Failed to parse config file. Using defaults.
            }
        }
    } else {
        // No config file, using defaults
    }
    Config::default()
}

pub fn save(cfg: &Config) {
    let path = config_path();
    if let Some(parent) = path.parent() {
        if let Err(e) = create_dir_all(parent) {
            // Failed to create config directory
            return;
        }
    }

    match serde_json::to_string_pretty(cfg) {
        Ok(json) => {
            if let Ok(mut f) = File::create(&path) {
                if let Err(e) = f.write_all(json.as_bytes()) {
                    // Failed to write config file
                } else {
                    // Saved config
                }
            } else {
                // Failed to create config file
            }
        }
        Err(e) => {
            tracing::error!("Failed to serialize config: {}", e);
        }
    }
}
