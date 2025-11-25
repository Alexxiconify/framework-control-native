use eframe::egui;
use std::sync::Arc;
use tokio::sync::RwLock;

mod cli;
mod config;
mod types;
mod utils;
mod fan_curve_methods;

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

    fn show_fan_control(&mut self, ui: &mut egui::Ui) {
        ui.heading("Fan Control");
        ui.add_space(10.0);

        ui.label("Manual fan speed control:");
        ui.add_space(5.0);

        ui.horizontal(|ui| {
            ui.label("Fan Duty:");
            ui.add(egui::Slider::new(&mut self.fan_duty, 0..=100).suffix("%"));
        });

        ui.add_space(10.0);

        if ui.button("🔄 Apply Fan Speed").clicked() {
            // TODO: Apply fan speed
            tracing::info!("Setting fan duty to {}%", self.fan_duty);
        }

        ui.add_space(20.0);
        ui.separator();
        ui.add_space(10.0);

        ui.label("ℹ️ Fan curve and advanced controls coming soon");
    }

    fn show_power_management(&mut self, ui: &mut egui::Ui) {
        ui.heading("Power Management");
        ui.add_space(10.0);

        ui.group(|ui| {
            ui.label("TDP Settings:");
            ui.add_space(5.0);

            ui.horizontal(|ui| {
                ui.label("TDP:");
                ui.add(egui::Slider::new(&mut self.tdp_watts, 5..=28).suffix("W"));
            });

            ui.horizontal(|ui| {
                ui.label("Thermal Limit:");
                ui.add(egui::Slider::new(&mut self.thermal_limit, 60..=100).suffix("°C"));
            });

            ui.add_space(10.0);

            if ui.button("⚡ Apply Power Settings").clicked() {
                // TODO: Apply power settings
                tracing::info!("Setting TDP to {}W, thermal limit to {}°C",
                    self.tdp_watts, self.thermal_limit);
            }
        });

        ui.add_space(20.0);

        ui.label("ℹ️ Requires RyzenAdj for AMD CPUs");
    }

    fn show_battery_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading("Battery Settings");
        ui.add_space(10.0);

        ui.group(|ui| {
            ui.label("Charge Limit:");
            ui.add_space(5.0);

            ui.horizontal(|ui| {
                ui.label("Maximum Charge:");
                ui.add(egui::Slider::new(&mut self.charge_limit, 50..=100).suffix("%"));
            });

            ui.add_space(10.0);

            if ui.button("🔋 Apply Charge Limit").clicked() {
                // TODO: Apply charge limit
                tracing::info!("Setting charge limit to {}%", self.charge_limit);
            }
        });

        ui.add_space(20.0);

        ui.label("ℹ️ Setting a lower charge limit can extend battery lifespan");
    }

    fn show_system(&mut self, ui: &mut egui::Ui) {
        ui.heading("System Information");
        ui.add_space(10.0);

        if let Some(versions) = &self.versions {
            ui.group(|ui| {
                ui.heading("Firmware Versions");
                ui.add_space(5.0);

                egui::Grid::new("versions_grid")
                    .num_columns(2)
                    .spacing([40.0, 4.0])
                    .show(ui, |ui| {
                        if let Some(uefi) = &versions.uefi_version {
                            ui.label("UEFI:");
                            ui.label(uefi);
                            ui.end_row();
                        }
                        if let Some(ec) = &versions.ec_build_version {
                            ui.label("EC:");
                            ui.label(ec);
                            ui.end_row();
                        }
                        if let Some(mb_type) = &versions.mainboard_type {
                            ui.label("Mainboard:");
                            ui.label(mb_type);
                            ui.end_row();
                        }
                    });
            });
        }

        ui.add_space(20.0);

        ui.group(|ui| {
            ui.heading("About");
            ui.add_space(5.0);
            ui.label(format!("Framework Control v{}", env!("CARGO_PKG_VERSION")));
            ui.label("Native Windows application for Framework laptops");
            ui.add_space(5.0);
            ui.hyperlink_to("GitHub Repository", "https://github.com/framework-laptop/framework-control");
        });
    }
}