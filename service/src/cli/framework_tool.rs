use super::framework_tool_parser::{
    parse_power, parse_thermal, parse_versions, PowerBatteryInfo, ThermalParsed, VersionsParsed,
};
use crate::utils::global_cache;
use std::time::Duration;
use tokio::process::Command;
use tracing::{error, info};

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

/// Thin wrapper around the `framework_tool` CLI.
/// Resolves the binary path once and provides async helpers to run commands.
#[derive(Clone)]
pub struct FrameworkTool {
    pub(crate) path: String,
}

impl FrameworkTool {
    pub async fn new() -> Result<Self, String> {
        let path = resolve_framework_tool().await?;
        info!("framework_tool resolved at: {}", path);
        let cli = Self { path };
        // Validate the binary is runnable with a lightweight call.
        if let Err(e) = cli.versions().await {
            return Err(format!("framework_tool not runnable: {}", e));
        }
        Ok(cli)
    }

    pub async fn power(&self) -> Result<PowerBatteryInfo, String> {
        const TTL: Duration = Duration::from_millis(2000);
        global_cache::cache_get_or_update("framework_tool.power", TTL, true, || async {
            let out = self.run(&["--power", "-vv"]).await?;
            Ok(parse_power(&out))
        })
        .await
    }

    pub async fn thermal(&self) -> Result<ThermalParsed, String> {
        const TTL: Duration = Duration::from_millis(1000);
        global_cache::cache_get_or_update("framework_tool.thermal", TTL, true, || async {
            let out = self.run(&["--thermal"]).await?;
            Ok(parse_thermal(&out))
        })
        .await
    }

    pub async fn versions(&self) -> Result<VersionsParsed, String> {
        let out = self.run(&["--versions"]).await?;
        Ok(parse_versions(&out))
    }

    pub async fn set_fan_duty(&self, percent: u32, fan_index: Option<u32>) -> Result<(), String> {
        let percent_s = percent.to_string();
        let fan_idx_s = fan_index.map(|idx| idx.to_string());
        let mut args: Vec<&str> = vec!["--fansetduty"];
        if let Some(ref idxs) = fan_idx_s {
            args.push(idxs.as_str());
        }
        args.push(percent_s.as_str());
        let _ = self.run(&args).await?;
        Ok(())
    }

    pub async fn autofanctrl(&self) -> Result<(), String> {
        let _ = self.run(&["--autofanctrl"]).await?;
        Ok(())
    }

    /// Get charge limit min/max percentage as reported by EC
    pub async fn charge_limit_get(
        &self,
    ) -> Result<super::framework_tool_parser::BatteryChargeLimitInfo, String> {
        use super::framework_tool_parser::parse_charge_limit;
        const TTL: Duration = Duration::from_millis(2000);
        global_cache::cache_get_or_update("framework_tool.charge_limit", TTL, true, || async {
            let out = self.run(&["--charge-limit"]).await?;
            let info = parse_charge_limit(&out);
            if info.charge_limit_min_pct.is_some() || info.charge_limit_max_pct.is_some() {
                Ok(info)
            } else {
                Err("failed to parse charge limit".to_string())
            }
        })
        .await
    }

    /// Set max charge limit percentage
    pub async fn charge_limit_set(&self, max_pct: u8) -> Result<(), String> {
        let arg = max_pct.to_string();
        let _ = self.run(&["--charge-limit", &arg]).await?;
        Ok(())
    }

    /// Set charge rate limit in C; optional SoC threshold in percent
    pub async fn charge_rate_limit_set(
        &self,
        rate_c: f32,
        soc_threshold_pct: Option<u8>,
    ) -> Result<(), String> {
        let rate = format!("{:.3}", rate_c);
        match soc_threshold_pct {
            Some(soc) => {
                let s = soc.to_string();
                let _ = self.run(&["--charge-rate-limit", &rate, &s]).await?;
            }
            None => {
                let _ = self.run(&["--charge-rate-limit", &rate]).await?;
            }
        }
        Ok(())
    }

    pub async fn run_raw(&self, args: &[&str]) -> Result<String, String> {
        self.run(args).await
    }

    async fn run(&self, args: &[&str]) -> Result<String, String> {
        use tokio::time::{timeout, Duration};
        let child = Command::new(&self.path)
            .args(args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("spawn failed: {e}"))?;
        let output = timeout(Duration::from_secs(60), child.wait_with_output())
            .await
            .map_err(|_| "framework_tool timed out".to_string())
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

/// Embed the framework_tool binary
const FRAMEWORK_TOOL_BYTES: &[u8] = include_bytes!("../bin_data/framework_tool.exe");

async fn resolve_framework_tool() -> Result<String, String> {
    // 1. Determine path next to executable
    let exe = std::env::current_exe().map_err(|e| format!("Failed to get current exe: {}", e))?;
    let dir = exe.parent().ok_or("Failed to get parent directory")?;
    let tool_path = dir.join("framework_tool.exe");

    // 2. Check if it exists, if not (or if we want to enforce version), write it
    // For now, we only write if missing to avoid overwriting if user has a custom version
    if !tool_path.exists() {
        info!("Extracting embedded framework_tool.exe...");
        if let Err(e) = std::fs::write(&tool_path, FRAMEWORK_TOOL_BYTES) {
            return Err(format!("Failed to extract framework_tool.exe: {}", e));
        }
    }

    Ok(tool_path.to_string_lossy().to_string())
}

/// Resolve framework_tool, extracting if needed.
pub async fn resolve_or_install() -> Result<FrameworkTool, String> {
    // Try to resolve/extract
    match FrameworkTool::new().await {
        Ok(cli) => Ok(cli),
        Err(e) => {
            error!("Failed to resolve framework_tool: {}", e);
            Err(e)
        }
    }
}

/// Fallback: cross-platform direct download (Legacy/Unused now)
pub async fn attempt_install_via_direct_download() -> Result<(), String> {
    Ok(())
}
