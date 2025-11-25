use std::fs::{create_dir_all, File};
use std::io::{Read, Write};
use std::path::PathBuf;

use tracing::info;
use crate::types::Config;

pub fn config_path() -> PathBuf {
    if let Ok(p) = std::env::var("FRAMEWORK_CONTROL_CONFIG") {
        return PathBuf::from(p);
    }
    // Prefer ProgramData for system-wide service config
    let base = std::env::var("PROGRAMDATA").unwrap_or_else(|_| r"C:\ProgramData".into());
    PathBuf::from(base).join("FrameworkControl").join("config.json")
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
                tracing::warn!("Failed to parse config file {:?}: {}. Using defaults.", path, e);
            }
        }
    } else {
        tracing::debug!("No config file at {:?}, using defaults", path);
    }
    Config::default()
}

pub fn save(cfg: &Config) -> Result<(), String> {
    let path = config_path();
    if let Some(dir) = path.parent() { 
        create_dir_all(dir).map_err(|e| format!("Failed to create config directory: {}", e))?; 
    }
    
    // Write to temporary file first for atomic operation
    let tmp_path = path.with_extension("json.tmp");
    let s = serde_json::to_string_pretty(cfg).map_err(|e| format!("Failed to serialize config: {}", e))?;
    
    let mut f = File::create(&tmp_path).map_err(|e| format!("Failed to create temp config file: {}", e))?;
    f.write_all(s.as_bytes()).map_err(|e| format!("Failed to write temp config file: {}", e))?;
    f.sync_all().map_err(|e| format!("Failed to sync temp config file: {}", e))?;
    drop(f);
    
    // Atomic rename
    std::fs::rename(&tmp_path, &path).map_err(|e| format!("Failed to rename config file: {}", e))?;
    
    tracing::info!("Config saved successfully to {:?}", path);
    Ok(())
}


