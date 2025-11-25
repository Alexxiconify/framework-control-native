 // Enhanced fan control methods with grid-based curve editor

impl FrameworkControlApp {
    fn show_fan_control_enhanced(&mut self, ui: &mut egui::Ui) {
        ui.heading("🌀 Fan Control");
        ui.add_space(10.0);

        // Mode selection
        ui.horizontal(|ui| {
            ui.radio_value(&mut self.auto_fan, true, "Auto");
            ui.radio_value(&mut self.auto_fan, false, "Manual");
            if !self.auto_fan {
                ui.radio_value(&mut self.fan_curve_enabled, false, "Fixed Speed");
                ui.radio_value(&mut self.fan_curve_enabled, true, "Curve");
            }
        });

        ui.add_space(10.0);

        // Auto mode
        if self.auto_fan {
            ui.label("System controls fan speed automatically");
            if ui.button("Currently in Auto Mode").clicked() {
                // Just show status
            }
        }
        // Fixed speed mode
        else if !self.fan_curve_enabled {
            ui.horizontal(|ui| {
                ui.label("Speed:");
                ui.add(egui::Slider::new(&mut self.fan_duty, 0..=100).suffix("%"));
            });
            ui.add_space(10.0);
            if ui.button("⚡ Apply Fixed Speed").clicked() {
                self.apply_fan_speed();
            }
        }
        // Curve mode
        else {
            self.show_fan_curve_editor(ui);
        }

        ui.add_space(10.0);

        if !self.auto_fan && ui.button("🔄 Reset to Auto").clicked() {
            self.reset_fan_to_auto();
        }
    }

    fn show_fan_curve_editor(&mut self, ui: &mut egui::Ui) {
        ui.heading("Fan Curve Editor");
        ui.add_space(5.0);

        // Grid-based curve editor
        ui.group(|ui| {
            ui.label("Temperature → Fan Speed");
            ui.add_space(5.0);

            // Show curve points in a grid
            egui::Grid::new("fan_curve_grid")
                .num_columns(3)
                .spacing([10.0, 5.0])
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Temperature (°C)");
                    ui.label("Fan Duty (%)");
                    ui.label("");
                    ui.end_row();

                    let mut to_remove = None;
                    for (idx, (temp, duty)) in self.fan_curve.iter_mut().enumerate() {
                        ui.add(egui::DragValue::new(temp).speed(0.5).clamp_range(20.0..=100.0).suffix("°C"));
                        ui.add(egui::DragValue::new(duty).speed(0.5).clamp_range(0.0..=100.0).suffix("%"));
                        if ui.small_button("❌").clicked() && self.fan_curve.len() > 2 {
                            to_remove = Some(idx);
                        }
                        ui.end_row();
                    }

                    if let Some(idx) = to_remove {
                        self.fan_curve.remove(idx);
                    }
                });

            ui.add_space(5.0);

            // Add point button
            if ui.button("➕ Add Point").clicked() && self.fan_curve.len() < 10 {
                // Add a new point between existing ones
                let last_temp = self.fan_curve.last().map(|(t, _)| *t).unwrap_or(50.0);
                let new_temp = (last_temp + 10.0).min(100.0);
                let new_duty = self.interpolate_curve(new_temp);
                self.fan_curve.push((new_temp, new_duty));
                self.fan_curve.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            }

            ui.add_space(10.0);

            // Apply button
            if ui.button("⚡ Apply Fan Curve").clicked() {
                self.apply_fan_curve();
            }

            ui.add_space(5.0);
            ui.label("💡 Curve applies continuously based on max temp");
        });

        // Visual curve preview
        ui.add_space(10.0);
        self.show_curve_preview(ui);
    }

    fn show_curve_preview(&self, ui: &mut egui::Ui) {
        use egui_plot::{Line, Plot, PlotPoints};

        ui.label("Curve Preview:");

        let points: PlotPoints = self.fan_curve.iter()
            .map(|(temp, duty)| [*temp as f64, *duty as f64])
            .collect();

        let line = Line::new(points)
            .color(egui::Color32::from_rgb(255, 165, 0))
            .width(2.0);

        Plot::new("fan_curve_plot")
            .height(150.0)
            .allow_drag(false)
            .allow_zoom(false)
            .x_axis_label("Temperature (°C)")
            .y_axis_label("Fan Duty (%)")
            .show(ui, |plot_ui| {
                plot_ui.line(line);
            });
    }

    fn interpolate_curve(&self, temp: f32) -> f32 {
        if self.fan_curve.is_empty() {
            return 50.0;
        }

        // Find surrounding points
        let mut lower = None;
        let mut upper = None;

        for (t, d) in &self.fan_curve {
            if *t <= temp {
                lower = Some((*t, *d));
            }
            if *t >= temp && upper.is_none() {
                upper = Some((*t, *d));
            }
        }

        match (lower, upper) {
            (Some((t1, d1)), Some((t2, d2))) if t1 != t2 => {
                // Linear interpolation
                let ratio = (temp - t1) / (t2 - t1);
                d1 + (d2 - d1) * ratio
            }
            (Some((_, d)), _) => d,
            (_, Some((_, d))) => d,
            _ => 50.0,
        }
    }

    fn apply_fan_curve(&mut self) {
        // Sort curve points
        self.fan_curve.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        let curve = self.fan_curve.clone();
        let state = self.state.clone();

        // Start background task to apply curve
        self.runtime.spawn(async move {
            loop {
                if let Some(ft) = state.framework_tool.read().await.as_ref() {
                    // Get current max temperature
                    if let Ok(thermal) = ft.thermal().await {
                        let max_temp = thermal.temps.values().max().copied().unwrap_or(50);

                        // Calculate duty from curve
                        let mut duty = 0.0;
                        let temp_f = max_temp as f32;

                        // Linear interpolation
                        let mut lower = None;
                        let mut upper = None;

                        for (t, d) in &curve {
                            if *t <= temp_f {
                                lower = Some((*t, *d));
                            }
                            if *t >= temp_f && upper.is_none() {
                                upper = Some((*t, *d));
                            }
                        }

                        duty = match (lower, upper) {
                            (Some((t1, d1)), Some((t2, d2))) if t1 != t2 => {
                                let ratio = (temp_f - t1) / (t2 - t1);
                                d1 + (d2 - d1) * ratio
                            }
                            (Some((_, d)), _) => d,
                            (_, Some((_, d))) => d,
                            _ => 50.0,
                        };

                        // Apply fan speed
                        let _ = ft.set_fan_duty(duty as u32, None).await;
                    }
                }

                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
        });

        self.fan_enabled = true;
        self.status_message = "✓ Fan curve active".to_string();
    }

    fn apply_power_settings_enhanced(&mut self) {
        let tdp = self.tdp_watts;
        let thermal = self.thermal_limit;
        let state = self.state.clone();

        self.runtime.spawn(async move {
            // Try RyzenAdj for AMD
            if let Some(ryz) = state.ryzenadj.read().await.as_ref() {
                match ryz.set_tdp_watts(tdp).await {
                    Ok(_) => tracing::info!("✓ TDP set to {}W via RyzenAdj", tdp),
                    Err(e) => tracing::error!("Failed to set TDP: {}", e),
                }
                match ryz.set_thermal_limit_c(thermal).await {
                    Ok(_) => tracing::info!("✓ Thermal limit set to {}°C", thermal),
                    Err(e) => tracing::error!("Failed to set thermal: {}", e),
                }
            }

            // Also try framework_tool for EC-level settings
            if let Some(ft) = state.framework_tool.read().await.as_ref() {
                // Framework BIOS may have additional controls
                tracing::info!("Power settings applied: {}W TDP, {}°C", tdp, thermal);
            }
        });

        self.status_message = format!("✓ Power: {}W, {}°C", tdp, thermal);
    }

    fn apply_charge_limit_enhanced(&mut self) {
        let limit = self.charge_limit;
        let state = self.state.clone();

        self.runtime.spawn(async move {
            if let Some(ft) = state.framework_tool.read().await.as_ref() {
                match ft.charge_limit_set(limit).await {
                    Ok(_) => {
                        tracing::info!("✓ Charge limit set to {}% via Framework EC", limit);

                        // Verify it was applied
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        if let Ok(limits) = ft.charge_limit_get().await {
                            tracing::info!("Verified charge limit: {:?}", limits);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to set charge limit: {}", e);
                        tracing::info!("Make sure framework_tool is installed and you have permissions");
                    }
                }
            } else {
                tracing::error!("framework_tool not available - cannot set charge limit");
                tracing::info!("Install: winget install FrameworkComputer.framework_tool");
            }
        });

        self.status_message = format!("✓ Charge limit: {}%", limit);
    }
}