use serde::{Deserialize, Serialize};

// Core config types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub fan: FanControlConfig,
    #[serde(default)]
    pub power: PowerConfig,
    #[serde(default)]
    pub battery: BatteryConfig,
    #[serde(default)]
    pub ui: UiConfig,
    #[serde(default)]
    pub start_on_boot: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            fan: FanControlConfig::default(),
            power: PowerConfig::default(),
            battery: BatteryConfig::default(),
            ui: UiConfig::default(),
            start_on_boot: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FanControlMode {
    Disabled,
    Manual,
    Curve,
}

impl Default for FanControlMode {
    fn default() -> Self {
        Self::Curve
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FanControlConfig {
    #[serde(default)]
    pub mode: Option<FanControlMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manual: Option<ManualConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub curve: Option<CurveConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calibration: Option<FanCalibration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualConfig {
    pub duty_pct: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurveConfig {
    #[serde(default = "default_points")]
    pub points: Vec<[u32; 2]>,
    #[serde(default = "default_poll_ms")]
    pub poll_ms: u64,
    #[serde(default = "default_hysteresis_c")]
    pub hysteresis_c: u32,
    #[serde(default = "default_rate_limit_pct_per_step")]
    pub rate_limit_pct_per_step: u32,
}

fn default_points() -> Vec<[u32; 2]> {
    vec![[50, 0], [60, 30], [70, 50], [80, 80], [90, 100]]
}
fn default_poll_ms() -> u64 {
    2000
}
fn default_hysteresis_c() -> u32 {
    2
}
fn default_rate_limit_pct_per_step() -> u32 {
    100
}

impl Default for CurveConfig {
    fn default() -> Self {
        Self {
            points: default_points(),
            poll_ms: default_poll_ms(),
            hysteresis_c: default_hysteresis_c(),
            rate_limit_pct_per_step: default_rate_limit_pct_per_step(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UiConfig {
    /// Preferred UI theme (matches DaisyUI theme names)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
}


// Fan calibration types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FanCalibration {
    pub points: Vec<[u32; 2]>,
    pub updated_at: i64,
}

// Power config stored in Config and applied at boot (and on set)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SettingU32 {
    pub enabled: bool,
    pub value: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PowerProfile {
    pub tdp_watts: Option<SettingU32>,
    pub thermal_limit_c: Option<SettingU32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PowerConfig {
    /// Profile used when AC power is present (plugged in / charging)
    pub ac: Option<PowerProfile>,
    /// Profile used when running on battery (not charging)
    pub battery: Option<PowerProfile>,
}

// Battery config stored in Config and applied at boot (and on set)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SettingU8 {
    /// Whether this setting should be applied
    pub enabled: bool,
    /// The last chosen value (kept even when disabled)
    pub value: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SettingF32 {
    /// Whether this setting should be applied
    pub enabled: bool,
    /// The last chosen value (kept even when disabled)
    pub value: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BatteryConfig {
    /// EC charge limit maximum percent (25-100)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub charge_limit_max_pct: Option<SettingU8>,
    /// Charge rate in C (0.0 - 1.0). When disabled, use 1.0C to approximate no limit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub charge_rate_c: Option<SettingF32>,
    /// Optional SoC threshold (%) for rate limiting
    #[serde(skip_serializing_if = "Option::is_none")]
    pub charge_rate_soc_threshold_pct: Option<u8>,
}
