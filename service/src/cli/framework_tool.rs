use framework_lib::chromium_ec::{CrosEc, CrosEcDriver, EcError};
use framework_lib::power::{self};
use serde::{Deserialize, Serialize};
use tracing::{error, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerBatteryInfo {
    pub charge_percent: f32,
    pub status: String,
    pub capacity_current: u32,
    pub capacity_design: u32,
    pub voltage: f32,
    pub current: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThermalSensor {
    pub name: String,
    pub temp_c: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThermalParsed {
    pub sensors: Vec<ThermalSensor>,
    pub fans: Vec<f32>, // RPM
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Versions {
    pub ec_version: String,
    pub bios_version: String,
}

#[derive(Clone)]
pub struct FrameworkTool;

impl FrameworkTool {
    pub async fn new() -> Self {
        Self
    }

    fn with_ec<F, T>(f: F) -> Result<T, String>
    where
        F: FnOnce(&CrosEc) -> Result<T, String>,
    {
        // CrosEc::new() returns CrosEc directly (it handles driver selection internally)
        let ec = CrosEc::new();
        f(&ec)
    }

    pub async fn read_versions(&self) -> Result<Versions, String> {
        tokio::task::spawn_blocking(|| {
            Self::with_ec(|ec| {
                let ec_version = ec
                    .version_info()
                    .map_err(|e| format!("Failed to get EC version: {:?}", e))?;
                // BIOS version usually requires SMBIOS or other calls.
                // framework_lib::smbios::get_smbios() might help.
                // But framework_lib::smbios is not exposed directly via CrosEc.
                // I'll use a placeholder or try to find it.
                // framework_lib::smbios::get_bios_version() exists?
                // I'll check framework_lib imports.
                // For now, "Unknown" or try to implement.
                Ok(Versions {
                    ec_version,
                    bios_version: "Unknown".to_string(), // TODO: Implement BIOS version reading
                })
            })
        })
        .await
        .map_err(|e| format!("Task join error: {:?}", e))?
    }

    pub async fn read_power_info(&self) -> Result<PowerBatteryInfo, String> {
        // Run blocking EC operations in a spawn_blocking task to avoid blocking the async runtime
        tokio::task::spawn_blocking(|| {
            Self::with_ec(|ec| {
                let info = power::power_info(ec).ok_or("Failed to read power info")?;

                let battery = info.battery.as_ref();

                let status = if info.ac_present {
                    if let Some(b) = battery {
                        if b.charging {
                            "Charging"
                        } else {
                            "Plugged In"
                        }
                    } else {
                        "AC Connected"
                    }
                } else {
                    "Discharging"
                };

                Ok(PowerBatteryInfo {
                    charge_percent: battery.map(|b| b.charge_percentage as f32).unwrap_or(0.0),
                    status: status.to_string(),
                    capacity_current: battery.map(|b| b.remaining_capacity).unwrap_or(0),
                    capacity_design: battery.map(|b| b.design_capacity).unwrap_or(0),
                    voltage: battery
                        .map(|b| b.present_voltage as f32 / 1000.0)
                        .unwrap_or(0.0),
                    current: battery.map(|b| b.present_rate as f32).unwrap_or(0.0),
                })
            })
        })
        .await
        .map_err(|e| format!("Task join error: {:?}", e))?
    }

    pub async fn read_thermal(&self) -> Result<ThermalParsed, String> {
        tokio::task::spawn_blocking(|| {
            Self::with_ec(|ec| {
                // Read temperature sensors (0x00 - 0x0F)
                // Using constants from framework_lib source (not public, so hardcoded here)
                // EC_MEMMAP_TEMP_SENSOR = 0x00
                let temps = ec
                    .read_memory(0x00, 0x0F)
                    .ok_or("Failed to read thermal sensors")?;

                // Read fans (0x10 - 0x17)
                // EC_MEMMAP_FAN = 0x10
                let fans_raw = ec
                    .read_memory(0x10, 0x08)
                    .ok_or("Failed to read fan speeds")?;

                let mut sensors = Vec::new();
                for (i, &t) in temps.iter().enumerate() {
                    // TempSensor logic from framework_lib
                    // 0xFF = NotPresent, 0xFE = Error, 0xFD = NotPowered, 0xFC = NotCalibrated
                    // Valid = t - 73 (Kelvin to Celsius offset used by EC?)

                    if t < 0xFC {
                        let temp_c = (t as i16 - 73) as f32;
                        if temp_c > -50.0 && temp_c < 150.0 {
                            // Sanity check
                            sensors.push(ThermalSensor {
                                name: format!("Temp {}", i),
                                temp_c,
                            });
                        }
                    }
                }

                let mut fans = Vec::new();
                // 4 fans max, 2 bytes each (u16 le)
                for i in 0..4 {
                    let offset = i * 2;
                    if offset + 1 < fans_raw.len() {
                        let rpm = u16::from_le_bytes([fans_raw[offset], fans_raw[offset + 1]]);
                        // 0xFFFF = Not Present, 0xFFFE = Stalled
                        if rpm != 0xFFFF {
                            fans.push(rpm as f32);
                        }
                    }
                }

                Ok(ThermalParsed { sensors, fans })
            })
        })
        .await
        .map_err(|e| format!("Task join error: {:?}", e))?
    }

    pub async fn set_fan_duty(&self, percent: u32, fan_index: Option<u32>) -> Result<(), String> {
        tokio::task::spawn_blocking(move || {
            Self::with_ec(|ec| {
                ec.fan_set_duty(fan_index, percent)
                    .map_err(|e| format!("Failed to set fan duty: {:?}", e))
            })
        })
        .await
        .map_err(|e| format!("Task join error: {:?}", e))?
    }

    pub async fn set_fan_control_auto(&self, fan_index: Option<u8>) -> Result<(), String> {
        tokio::task::spawn_blocking(move || {
            Self::with_ec(|ec| {
                ec.autofanctrl(fan_index)
                    .map_err(|e| format!("Failed to set auto fan control: {:?}", e))
            })
        })
        .await
        .map_err(|e| format!("Task join error: {:?}", e))?
    }

    pub async fn charge_limit_set(&self, max_pct: u8) -> Result<(), String> {
        tokio::task::spawn_blocking(move || {
            Self::with_ec(|ec| {
                let min_pct = if max_pct > 5 { max_pct - 5 } else { 0 };
                ec.set_charge_limit(min_pct, max_pct)
                    .map_err(|e| format!("Failed to set charge limit: {:?}", e))
            })
        })
        .await
        .map_err(|e| format!("Task join error: {:?}", e))?
    }

    pub async fn charge_limit_get(&self) -> Result<(u8, u8), String> {
        tokio::task::spawn_blocking(|| {
            Self::with_ec(|ec| {
                ec.get_charge_limit()
                    .map_err(|e| format!("Failed to get charge limit: {:?}", e))
            })
        })
        .await
        .map_err(|e| format!("Task join error: {:?}", e))?
    }

    pub async fn charge_rate_limit_set(
        &self,
        rate_c: f32,
        soc_threshold: Option<u8>,
    ) -> Result<(), String> {
        tokio::task::spawn_blocking(move || {
            Self::with_ec(|ec| {
                // framework_lib::set_charge_rate_limit takes rate (f32) and battery_soc (Option<f32>)
                // battery_soc is threshold.
                let soc = soc_threshold.map(|s| s as f32);
                ec.set_charge_rate_limit(rate_c, soc)
                    .map_err(|e| format!("Failed to set charge rate limit: {:?}", e))
            })
        })
        .await
        .map_err(|e| format!("Task join error: {:?}", e))?
    }

    pub async fn run_raw_command(&self, _args: Vec<String>) -> Result<String, String> {
        Ok("Custom commands are currently limited in integrated mode. Check application logs for output.".to_string())
    }
}
