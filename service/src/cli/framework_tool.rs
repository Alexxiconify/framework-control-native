use serde::{Deserialize, Serialize};

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

#[derive(Clone)]
pub struct FrameworkTool;

impl FrameworkTool {
    pub async fn new() -> Self {
        Self
    }

    pub async fn read_versions(&self) -> Result<Versions, String> {
        Ok(Versions {
            ec_version: "Unknown".to_string(),
            bios_version: "Unknown".to_string(),
        })
    }

    pub async fn read_power_info(&self) -> Result<PowerBatteryInfo, String> {
        Ok(PowerBatteryInfo {
            charge_percent: 0.0,
            status: "Unknown".to_string(),
            capacity_current: 0,
            capacity_design: 0,
            voltage: 0.0,
            current: 0.0,
        })
    }

    pub async fn read_thermal(&self) -> Result<ThermalParsed, String> {
        tokio::task::spawn_blocking(|| {
            let temps = crate::ec::read_temps();
            let fans = crate::ec::read_fans();

            // Framework 16 sensor names
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
            if crate::ec::set_fan_duty(percent) {
                Ok(())
            } else {
                Err("Failed to set fan duty".to_string())
            }
        })
        .await
        .map_err(|e| format!("Task error: {:?}", e))?
    }

    pub async fn set_fan_control_auto(&self, _fan_index: Option<u8>) -> Result<(), String> {
        tokio::task::spawn_blocking(|| {
            if crate::ec::set_fan_auto() {
                Ok(())
            } else {
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

    pub async fn charge_limit_get(&self) -> Result<(u8, u8), String> {
        Ok((0, 100))
    }

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
}
