use eframe::egui;
use std::sync::Arc;
use tokio::sync::RwLock;

mod cli;
mod config;
mod types;
mod utils;

// Re-export for convenience
use types::*;

fn main() -> Result<(), eframe::Error> {
    // Simple .env file loading
    if let Ok(content) = std::fs::read_to_string(".env") {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some(pos) = line.find('=') {
                let key = line[..pos].trim();
                let value = line[pos + 1..].trim().trim_matches('"').trim_matches('\'');
                std::env::set_var(key, value);
            }
        }
    }

    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        )
        .without_time()
        .init();

    // Create app state
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let state = runtime.block_on(async { AppState::initialize().await });

    // Start background tasks
    let state_clone = state.clone();
    runtime.spawn(async move {
        tasks::boot(&state_clone).await;
    });

    // Launch GUI
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 700.0])
            .with_min_inner_size([800.0, 600.0])
            .with_icon(load_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "Framework Control",
        options,
        Box::new(|cc| Ok(Box::new(FrameworkControlApp::new(cc, state, runtime)))),
    )
}

fn load_icon() -> egui::IconData {
    // Simple 32x32 icon data (Framework logo colors)
    let icon_size = 32;
    let mut rgba = vec![0u8; icon_size * icon_size * 4];

    // Create a simple orange square with dark border (Framework colors)
    for y in 0..icon_size {
        for x in 0..icon_size {
            let idx = (y * icon_size + x) * 4;
            if x == 0 || x == icon_size - 1 || y == 0 || y == icon_size - 1 {
                // Dark border
                rgba[idx] = 40;
                rgba[idx + 1] = 40;
                rgba[idx + 2] = 40;
                rgba[idx + 3] = 255;
            } else {
                // Orange fill (Framework brand color)
                rgba[idx] = 255;
                rgba[idx + 1] = 102;
                rgba[idx + 2] = 0;
                rgba[idx + 3] = 255;
            }
        }
    }

    egui::IconData {
        rgba,
        width: icon_size as u32,
        height: icon_size as u32,
    }
}

// Application state
#[derive(Clone)]
struct AppState {
    framework_tool: Arc<RwLock<Option<cli::FrameworkTool>>>,
    ryzenadj: Arc<RwLock<Option<cli::RyzenAdj>>>,
    config: Arc<RwLock<Config>>,
    telemetry_samples: Arc<RwLock<std::collections::VecDeque<TelemetrySample>>>,
}

impl AppState {
    async fn initialize() -> Self {
        let config = Arc::new(RwLock::new(config::load()));

        let ryzenadj = Arc::new(RwLock::new(cli::RyzenAdj::new().await.ok()));
        Self::spawn_ryzenadj_resolver(ryzenadj.clone());

        let framework_tool = Arc::new(RwLock::new(
            cli::framework_tool::resolve_or_install().await.ok()
        ));
        Self::spawn_framework_tool_resolver(framework_tool.clone());

        Self {
            framework_tool,
            ryzenadj,
            config,
            telemetry_samples: Arc::new(RwLock::new(Default::default())),
        }
    }

    fn spawn_ryzenadj_resolver(ryz_lock: Arc<RwLock<Option<cli::RyzenAdj>>>) {
        tokio::spawn(async move {
            use tokio::time::{sleep, Duration};
            let mut consecutive_errors = 0u32;
            loop {
                let is_missing = { ryz_lock.read().await.is_none() };
                if is_missing {
                    match cli::RyzenAdj::new().await {
                        Ok(new_ryz) => {
                            *ryz_lock.write().await = Some(new_ryz);
                            tracing::info!("RyzenAdj is now available");
                            consecutive_errors = 0;
                        }
                        Err(e) => {
                            consecutive_errors += 1;
                            if consecutive_errors <= 3 || consecutive_errors % 10 == 0 {
                                tracing::debug!("RyzenAdj not available: {}", e);
                            }
                        }
                    }
                }
                sleep(Duration::from_secs(5)).await;
            }
        });
    }

    fn spawn_framework_tool_resolver(ft_lock: Arc<RwLock<Option<cli::FrameworkTool>>>) {
        tokio::spawn(async move {
            use tokio::time::{sleep, Duration};
            let mut consecutive_errors = 0u32;
            loop {
                let current = { ft_lock.read().await.clone() };
                match current {
                    Some(cli) => {
                        if let Err(e) = cli.versions().await {
                            *ft_lock.write().await = None;
                            tracing::warn!("framework_tool unavailable ({})", e);
                            consecutive_errors = 0;
                        }
                    }
                    None => {
                        match cli::FrameworkTool::new().await {
                            Ok(cli) => {
                                *ft_lock.write().await = Some(cli);
                                tracing::info!("framework_tool is now available");
                                consecutive_errors = 0;
                            }
                            Err(e) => {
                                consecutive_errors += 1;
                                if consecutive_errors <= 3 || consecutive_errors % 10 == 0 {
                                    tracing::debug!("framework_tool not available: {}", e);
                                }
                            }
                        }
                    }
                }
                sleep(Duration::from_secs(5)).await;
            }
        });
    }
}

// Background tasks module
mod tasks {
    use super::*;

    pub async fn boot(state: &AppState) {
        // Fan curve task
        {
            let ft_clone = state.framework_tool.clone();
            let cfg_clone = state.config.clone();
            tokio::spawn(async move {
                fan_curve::run(ft_clone, cfg_clone).await;
            });
        }

        // Power settings task
        {
            let ryz_clone = state.ryzenadj.clone();
            let cfg_clone = state.config.clone();
            let ft_clone = state.framework_tool.clone();
            tokio::spawn(async move {
                power::run(ryz_clone, cfg_clone, ft_clone).await;
            });
        }

        // Battery settings task
        {
            let ft_clone = state.framework_tool.clone();
            let cfg_clone = state.config.clone();
            tokio::spawn(async move {
                battery::run(ft_clone, cfg_clone).await;
            });
        }

        // Telemetry task
        {
            let ft_clone = state.framework_tool.clone();
            let cfg_clone = state.config.clone();
            let samples_clone = state.telemetry_samples.clone();
            tokio::spawn(async move {
                telemetry::run(ft_clone, cfg_clone, samples_clone).await;
            });
        }
    }

    mod fan_curve {
        use super::*;
        pub async fn run(_ft: Arc<RwLock<Option<cli::FrameworkTool>>>, _cfg: Arc<RwLock<Config>>) {
            // TODO: Implement fan curve logic
            tokio::time::sleep(tokio::time::Duration::from_secs(u64::MAX)).await;
        }
    }

    mod power {
        use super::*;
        pub async fn run(
            _ryz: Arc<RwLock<Option<cli::RyzenAdj>>>,
            _cfg: Arc<RwLock<Config>>,
            _ft: Arc<RwLock<Option<cli::FrameworkTool>>>,
        ) {
            // TODO: Implement power management
            tokio::time::sleep(tokio::time::Duration::from_secs(u64::MAX)).await;
        }
    }

    mod battery {
        use super::*;
        pub async fn run(_ft: Arc<RwLock<Option<cli::FrameworkTool>>>, _cfg: Arc<RwLock<Config>>) {
            // TODO: Implement battery management
            tokio::time::sleep(tokio::time::Duration::from_secs(u64::MAX)).await;
        }
    }

    mod telemetry {
        use super::*;
        pub async fn run(
            _ft: Arc<RwLock<Option<cli::FrameworkTool>>>,
            _cfg: Arc<RwLock<Config>>,
            _samples: Arc<RwLock<std::collections::VecDeque<TelemetrySample>>>,
        ) {
            // TODO: Implement telemetry collection
            tokio::time::sleep(tokio::time::Duration::from_secs(u64::MAX)).await;
        }
    }
}

// Main application GUI
struct FrameworkControlApp {
    state: AppState,
    runtime: tokio::runtime::Runtime,

    // Cached data
    thermal_data: Option<cli::framework_tool_parser::ThermalParsed>,
    power_data: Option<cli::framework_tool_parser::PowerBatteryInfo>,
    versions: Option<cli::framework_tool_parser::VersionsParsed>,

    // Fan control settings
    fan_duty: u32,
    fan_enabled: bool,
    auto_fan: bool,
    fan_curve_enabled: bool,
    fan_curve: Vec<(f32, f32)>, // (temp_celsius, duty_percent)
    editing_curve: bool,

    // Power settings
    tdp_watts: u32,
    thermal_limit: u32,
    power_enabled: bool,

    // Battery settings
    charge_limit: u8,
    charge_limit_enabled: bool,

    // Status messages
    status_message: String,
    last_update: std::time::Instant,
}

impl FrameworkControlApp {
    fn new(_cc: &eframe::CreationContext<'_>, state: AppState, runtime: tokio::runtime::Runtime) -> Self {
        Self {
            state,
            runtime,
            thermal_data: None,
            power_data: None,
            versions: None,
            fan_duty: 50,
            fan_enabled: false,
            auto_fan: true,
            fan_curve_enabled: false,
            fan_curve: vec![
                (40.0, 20.0),  // 40°C -> 20% duty
                (50.0, 30.0),  // 50°C -> 30% duty
                (60.0, 40.0),  // 60°C -> 40% duty
                (70.0, 60.0),  // 70°C -> 60% duty
                (80.0, 80.0),  // 80°C -> 80% duty
                (90.0, 100.0), // 90°C -> 100% duty
            ],
            editing_curve: false,
            tdp_watts: 15,
            thermal_limit: 80,
            power_enabled: false,
            charge_limit: 80,
            charge_limit_enabled: false,
            status_message: String::new(),
            last_update: std::time::Instant::now(),
        }
    }

    fn update_data(&mut self, ctx: &egui::Context) {
        // Update thermal data
        if let Some(ft) = self.runtime.block_on(async {
            self.state.framework_tool.read().await.clone()
        }) {
            if let Ok(thermal) = self.runtime.block_on(ft.thermal()) {
                self.thermal_data = Some(thermal);
            }
            if let Ok(power) = self.runtime.block_on(ft.power()) {
                self.power_data = Some(power);
            }
            if self.versions.is_none() {
                if let Ok(v) = self.runtime.block_on(ft.versions()) {
                    self.versions = Some(v);
                }
            }
        }
        ctx.request_repaint_after(std::time::Duration::from_secs(2));
    }
}

impl eframe::App for FrameworkControlApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Update data from background
        self.update_data(ctx);

        // Top panel - title and status
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("⚡ Framework Control");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(versions) = &self.versions {
                        ui.label(format!("EC: {} | UEFI: {}",
                            versions.ec_build_version.as_deref().unwrap_or("?"),
                            versions.uefi_version.as_deref().unwrap_or("?")));
                    }
                });
            });

            if !self.status_message.is_empty() {
                ui.separator();
                ui.colored_label(egui::Color32::from_rgb(255, 165, 0), &self.status_message);
            }
        });

        // Central panel - all features in one view
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(10.0);

                // System Status Section
                ui.horizontal(|ui| {
                    // Left column - Temperatures
                    ui.vertical(|ui| {
                        self.show_temperature_panel(ui);
                        ui.add_space(10.0);
                        self.show_power_panel(ui);
                    });

                    ui.add_space(20.0);

                    // Right column - Fan speeds
                    ui.vertical(|ui| {
                        self.show_fans_panel(ui);
                    });
                });

                ui.add_space(20.0);
                ui.separator();
                ui.add_space(10.0);

                // Control Section (full width)
                ui.columns(2, |columns| {
                    // Fan Control with Curve Editor
                    columns[0].group(|ui| {
                        self.show_fan_control_enhanced(ui);
                    });

                    // Power & Battery Control
                    columns[1].group(|ui| {
                        self.show_power_battery_control(ui);
                    });
                });

                ui.add_space(20.0);
                ui.separator();
                ui.add_space(10.0);

                // System Info at bottom
                self.show_system(ui);
            });
        });
    }
}

// UI Panels
impl FrameworkControlApp {
    fn show_dashboard(&mut self, ui: &mut egui::Ui) {
        ui.heading("System Overview");
        ui.add_space(10.0);

        // Thermal info
        if let Some(thermal) = &self.thermal_data {
            ui.group(|ui| {
                ui.heading("🌡️ Temperatures");
                ui.add_space(5.0);

                egui::Grid::new("temps_grid")
                    .num_columns(2)
                    .spacing([40.0, 4.0])
                    .show(ui, |ui| {
                        for (name, temp) in &thermal.temps {
                            ui.label(name);
                            let color = if *temp > 80 {
                                egui::Color32::RED
                            } else if *temp > 70 {
                                egui::Color32::YELLOW
                            } else {
                                egui::Color32::GREEN
                            };
                            ui.colored_label(color, format!("{}°C", temp));
                            ui.end_row();
                        }
                    });
            });
        }

        ui.add_space(10.0);

        // Fan info
        if let Some(thermal) = &self.thermal_data {
            ui.group(|ui| {
                ui.heading("🌀 Fans");
                ui.add_space(5.0);

                egui::Grid::new("fans_grid")
                    .num_columns(2)
                    .spacing([40.0, 4.0])
                    .show(ui, |ui| {
                        for (idx, rpm) in thermal.rpms.iter().enumerate() {
                            ui.label(format!("Fan {}", idx + 1));
                            ui.label(format!("{} RPM", rpm));
                            ui.end_row();
                        }
                    });
            });
        }

        ui.add_space(10.0);

        // Power info
        if let Some(power) = &self.power_data {
            ui.group(|ui| {
                ui.heading("🔋 Power");
                ui.add_space(5.0);

                egui::Grid::new("power_grid")
                    .num_columns(2)
                    .spacing([40.0, 4.0])
                    .show(ui, |ui| {
                        ui.label("Status");
                        ui.label(if power.charging.unwrap_or(false) { "⚡ Charging" } else { "🔋 Battery" });
                        ui.end_row();

                        if let Some(pct) = power.percentage {
                            ui.label("Battery");
                            ui.label(format!("{}%", pct));
                            ui.end_row();
                        }

                        if let Some(voltage) = power.present_voltage_mv {
                            ui.label("Voltage");
                            ui.label(format!("{:.2}V", voltage as f32 / 1000.0));
                            ui.end_row();
                        }
                    });
            });
        }
    }

    fn show_temperature_panel(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.heading("🌡️ Temperatures");
            if let Some(thermal) = &self.thermal_data {
                egui::Grid::new("temps").num_columns(2).spacing([40.0, 4.0]).show(ui, |ui| {
                    for (name, temp) in &thermal.temps {
                        ui.label(name);
                        let color = if *temp > 85 { egui::Color32::RED }
                                   else if *temp > 75 { egui::Color32::from_rgb(255, 165, 0) }
                                   else { egui::Color32::from_rgb(0, 200, 0) };
                        ui.colored_label(color, format!("{}°C", temp));
                        ui.end_row();
                    }
                });
            } else {
                ui.label("Install framework_tool");
            }
        });
    }

    fn show_fans_panel(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.heading("🌀 Fans");
            if let Some(thermal) = &self.thermal_data {
                egui::Grid::new("fans").num_columns(2).spacing([40.0, 4.0]).show(ui, |ui| {
                    for (idx, rpm) in thermal.rpms.iter().enumerate() {
                        ui.label(format!("Fan {}", idx + 1));
                        ui.colored_label(if *rpm > 4000 { egui::Color32::from_rgb(255, 165, 0) }
                                       else { egui::Color32::from_rgb(100, 200, 255) },
                                       format!("{} RPM", rpm));
                        ui.end_row();
                    }
                });
            }
        });
    }

    fn show_power_panel(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.heading("🔋 Power");
            if let Some(power) = &self.power_data {
                egui::Grid::new("power").num_columns(2).spacing([40.0, 4.0]).show(ui, |ui| {
                    ui.label("Status");
                    ui.colored_label(
                        if power.charging.unwrap_or(false) { egui::Color32::from_rgb(0, 200, 0) }
                        else { egui::Color32::from_rgb(100, 150, 255) },
                        if power.charging.unwrap_or(false) { "⚡ Charging" } else { "🔋 Battery" }
                    );
                    ui.end_row();

                    if let Some(pct) = power.percentage {
                        ui.label("Level");
                        ui.colored_label(
                            if pct < 20 { egui::Color32::RED }
                            else if pct < 50 { egui::Color32::from_rgb(255, 165, 0) }
                            else { egui::Color32::from_rgb(0, 200, 0) },
                            format!("{}%", pct)
                        );
                        ui.end_row();
                    }
                });
            }
        });
    }

    // Enhanced fan control with grid-based curve editor
    fn show_fan_control_enhanced(&mut self, ui: &mut egui::Ui) {
        ui.heading("🌀 Fan Control");
        ui.add_space(5.0);

        ui.horizontal(|ui| {
            if ui.radio(self.auto_fan && !self.fan_curve_enabled, "Auto").clicked() {
                self.auto_fan = true;
                self.fan_curve_enabled = false;
            }
            if ui.radio(!self.auto_fan && !self.fan_curve_enabled, "Manual").clicked() {
                self.auto_fan = false;
                self.fan_curve_enabled = false;
            }
            if ui.radio(!self.auto_fan && self.fan_curve_enabled, "Curve").clicked() {
                self.auto_fan = false;
                self.fan_curve_enabled = true;
            }
        });

        ui.add_space(10.0);

        if self.auto_fan {
            ui.label("✓ System controlled");
        } else if !self.fan_curve_enabled {
            ui.horizontal(|ui| {
                ui.label("Speed:");
                ui.add(egui::Slider::new(&mut self.fan_duty, 0..=100).suffix("%"));
            });
            if ui.button("⚡ Apply").clicked() {
                self.apply_fan_speed();
            }
        } else {
            ui.label("Grid-based Fan Curve:");
            ui.add_space(5.0);

            egui::Grid::new("curve").num_columns(3).spacing([10.0, 5.0]).striped(true).show(ui, |ui| {
                ui.label("Temp (°C)");
                ui.label("Fan (%)");
                ui.label("");
                ui.end_row();

                let mut to_remove = None;
                let curve_len = self.fan_curve.len();
                for (idx, (temp, duty)) in self.fan_curve.iter_mut().enumerate() {
                    ui.add(egui::DragValue::new(temp).speed(1.0).clamp_range(20.0..=100.0));
                    ui.add(egui::DragValue::new(duty).speed(1.0).clamp_range(0.0..=100.0));
                    if ui.small_button("✖").clicked() && curve_len > 2 {
                        to_remove = Some(idx);
                    }
                    ui.end_row();
                }

                if let Some(idx) = to_remove {
                    self.fan_curve.remove(idx);
                }
            });

            ui.add_space(5.0);
            ui.horizontal(|ui| {
                if ui.button("➕ Add Point").clicked() && self.fan_curve.len() < 10 {
                    let last = self.fan_curve.last().map(|(t, _)| *t).unwrap_or(50.0);
                    self.fan_curve.push(((last + 10.0).min(100.0), 50.0));
                    self.fan_curve.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
                }
                if ui.button("⚡ Apply Curve").clicked() {
                    self.apply_fan_curve();
                }
            });
        }

        ui.add_space(5.0);
        if !self.auto_fan && ui.button("🔄 Reset Auto").clicked() {
            self.reset_fan_to_auto();
        }
    }

    fn show_power_battery_control(&mut self, ui: &mut egui::Ui) {
        ui.heading("⚡ Power");
        ui.checkbox(&mut self.power_enabled, "Custom Limits");
        ui.add_enabled_ui(self.power_enabled, |ui| {
            ui.horizontal(|ui| {
                ui.label("TDP:");
                ui.add(egui::Slider::new(&mut self.tdp_watts, 5..=28).suffix("W"));
            });
            ui.horizontal(|ui| {
                ui.label("Thermal:");
                ui.add(egui::Slider::new(&mut self.thermal_limit, 60..=100).suffix("°C"));
            });
            if ui.button("⚡ Apply").clicked() {
                self.apply_power_settings();
            }
        });
        ui.separator();
        ui.heading("🔋 Battery");
        ui.checkbox(&mut self.charge_limit_enabled, "Charge Limit");
        ui.add_enabled_ui(self.charge_limit_enabled, |ui| {
            ui.horizontal(|ui| {
                ui.label("Max:");
                ui.add(egui::Slider::new(&mut self.charge_limit, 50..=100).suffix("%"));
            });
            if ui.button("🔋 Apply").clicked() {
                self.apply_charge_limit();
            }
        });
    }

    // Action methods
    fn apply_fan_speed(&mut self) {
        let duty = self.fan_duty;
        let state = self.state.clone();
        self.runtime.spawn(async move {
            if let Some(ft) = state.framework_tool.read().await.as_ref() {
                match ft.set_fan_duty(duty, None).await {
                    Ok(_) => tracing::info!("✓ Fan duty set to {}%", duty),
                    Err(e) => tracing::error!("Failed to set fan: {}", e),
                }
            }
        });
        self.fan_enabled = true;
        self.status_message = format!("✓ Fan: {}%", duty);
    }

    fn reset_fan_to_auto(&mut self) {
        let state = self.state.clone();
        self.runtime.spawn(async move {
            if let Some(ft) = state.framework_tool.read().await.as_ref() {
                match ft.autofanctrl().await {
                    Ok(_) => tracing::info!("✓ Fan reset to auto"),
                    Err(e) => tracing::error!("Failed to reset fan: {}", e),
                }
            }
        });
        self.fan_enabled = false;
        self.auto_fan = true;
        self.status_message = "✓ Fan: Auto".to_string();
    }

    fn apply_fan_curve(&mut self) {
        self.fan_curve.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let curve = self.fan_curve.clone();
        let state = self.state.clone();

        self.runtime.spawn(async move {
            loop {
                if let Some(ft) = state.framework_tool.read().await.as_ref() {
                    if let Ok(thermal) = ft.thermal().await {
                        let max_temp = thermal.temps.values().max().copied().unwrap_or(50) as f32;

                        let mut duty = 50.0;
                        for i in 0..curve.len() {
                            if i == 0 && max_temp <= curve[i].0 {
                                duty = curve[i].1;
                                break;
                            }
                            if i == curve.len() - 1 && max_temp >= curve[i].0 {
                                duty = curve[i].1;
                                break;
                            }
                            if i < curve.len() - 1 && max_temp >= curve[i].0 && max_temp <= curve[i+1].0 {
                                let t1 = curve[i].0;
                                let t2 = curve[i+1].0;
                                let d1 = curve[i].1;
                                let d2 = curve[i+1].1;
                                let ratio = (max_temp - t1) / (t2 - t1);
                                duty = d1 + (d2 - d1) * ratio;
                                break;
                            }
                        }

                        let _ = ft.set_fan_duty(duty as u32, None).await;
                        tracing::debug!("Fan curve: {}°C -> {}%", max_temp, duty as u32);
                    }
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
        });

        self.status_message = "✓ Curve active".to_string();
    }

    fn apply_power_settings(&mut self) {
        let (tdp, thermal, state) = (self.tdp_watts, self.thermal_limit, self.state.clone());
        self.runtime.spawn(async move {
            if let Some(r) = state.ryzenadj.read().await.as_ref() {
                let _ = r.set_tdp_watts(tdp).await;
                let _ = r.set_thermal_limit_c(thermal).await;
                tracing::info!("✓ Power: {}W, {}°C", tdp, thermal);
            }
        });
        self.status_message = format!("✓ Power: {}W/{}°C", tdp, thermal);
    }

    fn apply_charge_limit(&mut self) {
        let (limit, state) = (self.charge_limit, self.state.clone());
        self.runtime.spawn(async move {
            if let Some(ft) = state.framework_tool.read().await.as_ref() {
                match ft.charge_limit_set(limit).await {
                    Ok(_) => tracing::info!("✓ Charge limit: {}%", limit),
                    Err(e) => tracing::error!("Failed to set charge limit: {}", e),
                }
            }
        });
        self.status_message = format!("✓ Charge: {}%", limit);
    }

    fn show_system(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.heading("ℹ️ System");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("v{}", env!("CARGO_PKG_VERSION")));
                });
            });

            if let Some(v) = &self.versions {
                ui.horizontal(|ui| {
                    if let Some(u) = &v.uefi_version {
                        ui.label(format!("UEFI: {}", u));
                    }
                    if let Some(e) = &v.ec_build_version {
                        ui.separator();
                        ui.label(format!("EC: {}", e));
                    }
                });
            }
        });
    }
}