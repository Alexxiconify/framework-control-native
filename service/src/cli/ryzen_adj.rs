use crate::cli::ryzen_adj_parser::{self, RyzenAdjInfo};
use crate::utils::global_cache;
use std::time::Duration;
use tokio::process::Command;
use tracing::info;

/// Simple function to find executable in PATH (Windows-specific)
fn find_in_path(name: &str) -> Option<String> {
    let path_var = std::env::var("PATH").ok()?;
    let extensions = std::env::var("PATHEXT").unwrap_or_else(|_| ".EXE".to_string());

    for dir in path_var.split(';') {
        for ext in extensions.split(';') {
            let mut candidate = std::path::PathBuf::from(dir);
            if ext.is_empty() {
                candidate.push(name);
            } else {
                candidate.push(format!("{}{}", name, ext.to_lowercase()));
            }
            if candidate.exists() {
                return Some(candidate.to_string_lossy().to_string());
            }
        }
    }
    None
}

/// Thin wrapper around the `ryzenadj` CLI.
/// Resolves the binary path once and provides async helpers to run commands.
#[derive(Clone)]
pub struct RyzenAdj {
    pub(crate) path: String,
}

impl RyzenAdj {
    pub async fn new() -> Result<Self, String> {
        let path = resolve_ryzenadj().await?;
        info!("ryzenadj resolved at: {}", path);
        let cli = Self { path };
        if let Err(e) = cli.info_with_error_cache(false).await {
            return Err(format!("ryzenadj not runnable: {}", e));
        }
        Ok(cli)
    }

    /// Set TDP by applying stapm/fast/slow limits equally (expects watts)
    pub async fn set_tdp_watts(&self, watts: u32) -> Result<(), String> {
        let mw = watts.saturating_mul(1000).to_string();
        let _ = self
            .run(&[
                "--stapm-limit",
                &mw,
                "--fast-limit",
                &mw,
                "--slow-limit",
                &mw,
            ])
            .await?;
        Ok(())
    }

    /// Set thermal limit (Tctl) in degrees Celsius
    pub async fn set_thermal_limit_c(&self, celsius: u32) -> Result<(), String> {
        let _ = self.run(&["--tctl-temp", &celsius.to_string()]).await?;
        Ok(())
    }

    /// Get parsed info from ryzenadj `--info` output
    pub async fn info(&self) -> Result<RyzenAdjInfo, String> {
        self.info_with_error_cache(true).await
    }

    /// Variant of `info` that allows callers to opt out of error caching.
    /// Useful for validation flows after install where we want fresh attempts
    async fn info_with_error_cache(&self, cache_errors: bool) -> Result<RyzenAdjInfo, String> {
        const INFO_TTL: Duration = Duration::from_millis(2000);
        global_cache::cache_get_or_update("ryzenadj.info", INFO_TTL, cache_errors, || async {
            let raw = self.run(&["--info"]).await?;
            Ok(ryzen_adj_parser::parse_info(&raw))
        })
        .await
    }

    async fn run(&self, args: &[&str]) -> Result<String, String> {
        use tokio::time::{timeout, Duration};
        let args: Vec<&str> = {
            let mut v: Vec<&str> = args.to_vec();
            let has_dump = v.iter().any(|a| a.eq_ignore_ascii_case("--dump-table"));
            if !has_dump {
                v.push("--dump-table");
            }
            v
        };
        let child = Command::new(&self.path)
            .args(&args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("spawn failed: {e}"))?;
        let output = timeout(Duration::from_secs(60), child.wait_with_output())
            .await
            .map_err(|_| "ryzenadj timed out".to_string())
            .and_then(|res| res.map_err(|e| format!("wait failed: {e}")))?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(format!(
                "exit {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }
}

async fn resolve_ryzenadj() -> Result<String, String> {
    // Prefer alongside the running service binary
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = if cfg!(windows) {
                dir.join("ryzenadj/ryzenadj.exe")
            } else {
                dir.join("ryzenadj/ryzenadj")
            };
            if candidate.exists() {
                if let Some(s) = candidate.to_str() {
                    return Ok(s.to_string());
                }
            }
            // Also allow a root-level binary next to the service
            let root_candidate = if cfg!(windows) {
                dir.join("ryzenadj.exe")
            } else {
                dir.join("ryzenadj")
            };
            if root_candidate.exists() {
                if let Some(s) = root_candidate.to_str() {
                    return Ok(s.to_string());
                }
            }
        }
    }

    if let Some(p) = find_in_path("ryzenadj") {
        return Ok(p);
    }
    if let Some(p) = find_in_path("ryzenadj.exe") {
        return Ok(p);
    }

    Err("ryzenadj not found".into())
}