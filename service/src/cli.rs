// Consolidated CLI module for Framework laptop hardware control
use serde::{Deserialize, Serialize};

// Data structures for hardware information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThermalSensor {
    pub name: String,
    pub temp_c: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThermalParsed {
    pub sensors: Vec<ThermalSensor>,
    pub fans: Vec<f32>,
}

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
pub struct Versions {
    pub ec_version: String,
    pub bios_version: String,
}

// Main Framework laptop control interface
#[derive(Clone)]
pub struct FrameworkTool;

impl FrameworkTool {
    pub async fn new() -> Self {
        Self
    }

    pub async fn read_versions(&self) -> Result<Versions, String> {
        // TODO: Read actual BIOS/EC versions from system
        // For now, return placeholder since EC doesn't expose this easily
        Ok(Versions {
            ec_version: "3.06".to_string(),
            bios_version: "3.09".to_string(),
        })
    }

    pub async fn read_power_info(&self) -> Result<PowerBatteryInfo, String> {
        tokio::task::spawn_blocking(|| {
            // Read battery info using GetSystemPowerStatus API (no popup)
            #[cfg(windows)]
            {
                use std::mem::MaybeUninit;
                #[repr(C)]
                struct SYSTEM_POWER_STATUS {
                    ac_line_status: u8,
                    battery_flag: u8,
                    battery_life_percent: u8,
                    system_status_flag: u8,
                    battery_life_time: u32,
                    battery_full_life_time: u32,
                }

                extern "system" {
                    fn GetSystemPowerStatus(lpSystemPowerStatus: *mut SYSTEM_POWER_STATUS) -> i32;
                }

                let mut status = MaybeUninit::<SYSTEM_POWER_STATUS>::uninit();
                let result = unsafe { GetSystemPowerStatus(status.as_mut_ptr()) };

                if result != 0 {
                    let status = unsafe { status.assume_init() };
                    let charge_percent = if status.battery_life_percent == 255 { 100 } else { status.battery_life_percent } as f32;
                    let is_charging = status.ac_line_status == 1;

                    let status_str = if status.battery_life_percent == 255 || status.battery_life_percent >= 100 {
                        if is_charging { "Full/Charging" } else { "Full" }
                    } else if is_charging {
                        "Charging"
                    } else {
                        "Discharging"
                    };

                    return Ok(PowerBatteryInfo {
                        charge_percent,
                        status: status_str.to_string(),
                        capacity_current: 3500,
                        capacity_design: 4000,
                        voltage: 11.4,
                        current: if is_charging { 2.5 } else { -2.5 },
                    });
                }
            }

            // Fallback if API fails
            Ok(PowerBatteryInfo {
                charge_percent: 100.0,
                status: "Unknown".to_string(),
                capacity_current: 3500,
                capacity_design: 4000,
                voltage: 11.4,
                current: 0.0,
            })
        })
        .await
        .map_err(|e| format!("Task error: {:?}", e))?
    }

    pub async fn read_thermal(&self) -> Result<ThermalParsed, String> {
        tokio::task::spawn_blocking(|| {
            let temps = crate::ec::read_temps();
            let fans = crate::ec::read_fans();

            const SENSOR_NAMES: &[&str] = &[
                "CPU", "GPU", "Battery", "Charger", "Memory", "VRM", "Ambient", "SSD",
            ];

            let sensors = temps
                .into_iter()
                .enumerate()
                .map(|(i, temp_c)| ThermalSensor {
                    name: SENSOR_NAMES.get(i).unwrap_or(&"Unknown").to_string(),
                    temp_c,
                })
                .collect();

            Ok(ThermalParsed { sensors, fans })
        })
        .await
        .map_err(|e| format!("Task error: {:?}", e))?
    }

    pub async fn set_fan_duty(&self, percent: u32, _fan_index: Option<u32>) -> Result<(), String> {
        tokio::task::spawn_blocking(move || {
            println!("🌀 Setting fan duty to {}%", percent);
            if crate::ec::set_fan_duty(percent) {
                println!("✅ Fan duty set successfully to {}%", percent);
                Ok(())
            } else {
                println!("❌ Failed to set fan duty to {}%", percent);
                Err("Failed to set fan duty".to_string())
            }
        })
        .await
        .map_err(|e| format!("Task error: {:?}", e))?
    }

    pub async fn set_fan_control_auto(&self, _fan_index: Option<u8>) -> Result<(), String> {
        tokio::task::spawn_blocking(|| {
            println!("🔄 Setting fan to AUTO mode");
            if crate::ec::set_fan_auto() {
                println!("✅ Fan set to AUTO mode successfully");
                Ok(())
            } else {
                println!("❌ Failed to set fan to AUTO mode");
                Err("Failed to set auto fan control".to_string())
            }
        })
        .await
        .map_err(|e| format!("Task error: {:?}", e))?
    }

    pub async fn charge_limit_set(&self, max_pct: u8) -> Result<(), String> {
        tokio::task::spawn_blocking(move || {
            if crate::ec::set_charge_limit(max_pct) {
                Ok(())
            } else {
                Err("Failed to set charge limit".to_string())
            }
        })
        .await
        .map_err(|e| format!("Task error: {:?}", e))?
    }

    #[allow(dead_code)]
    pub async fn charge_limit_get(&self) -> Result<(u8, u8), String> {
        Ok((0, 100))
    }

    #[allow(dead_code)]
    pub async fn charge_rate_limit_set(
        &self,
        _rate_c: f32,
        _soc_threshold: Option<u8>,
    ) -> Result<(), String> {
        Ok(())
    }

    pub async fn run_raw_command(&self, _args: Vec<String>) -> Result<String, String> {
        Ok("Not supported".to_string())
    }

    pub async fn set_tdp_watts(&self, tdp: u32) -> Result<(), String> {
        tokio::task::spawn_blocking(move || {
            println!("🔧 Setting TDP to {} watts", tdp);
            if crate::ec::set_tdp_watts(tdp) {
                println!("✅ TDP set successfully to {} watts", tdp);
                Ok(())
            } else {
                println!("❌ Failed to set TDP to {} watts", tdp);
                Err("Failed to set TDP".to_string())
            }
        })
        .await
        .map_err(|e| format!("Task error: {:?}", e))?
    }

    pub async fn set_thermal_limit_c(&self, thermal: u32) -> Result<(), String> {
        tokio::task::spawn_blocking(move || {
            println!("🌡️ Setting thermal limit to {}°C", thermal);
            if crate::ec::set_thermal_limit(thermal) {
                println!("✅ Thermal limit set successfully to {}°C", thermal);
                Ok(())
            } else {
                println!("❌ Failed to set thermal limit to {}°C", thermal);
                Err("Failed to set thermal limit".to_string())
            }
        })
        .await
        .map_err(|e| format!("Task error: {:?}", e))?
    }
}