use eframe::egui;
use tokio_serial::available_ports;

pub struct NanoVNASaverApp {
    terminal: String,
    available_ports: Vec<String>,
    selected_port: Option<String>,
    start_freq: String,
    end_freq: String,
    num_points: String,
    num_ports: usize,
    label: String,
    if_bandwidth: String,
    is_running: bool,
}

impl Default for NanoVNASaverApp {
    fn default() -> Self {
        let mut app = Self {
            terminal: String::new(),
            available_ports: Vec::new(),
            selected_port: None,
            start_freq: "50000".to_string(),
            end_freq: "900000000".to_string(),
            num_points: "101".to_string(),
            num_ports: 2,
            label: String::new(),
            if_bandwidth: String::new(),
            is_running: false,
        };
        app.refresh_ports();
        app
    }
}

impl NanoVNASaverApp {
    fn refresh_ports(&mut self) {
        self.available_ports = available_ports()
            .map(|ports| ports.into_iter().map(|p| p.port_name).collect())
            .unwrap_or_default();

        if let Some(selected) = &self.selected_port {
            if !self.available_ports.iter().any(|port| port == selected) {
                self.selected_port = None;
            }
        }

        if self.selected_port.is_none() {
            self.selected_port = self.available_ports.first().cloned();
        }
    }
}

impl eframe::App for NanoVNASaverApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Calculate 1/3 of window width for results
        let window_width = ctx.screen_rect().width();
        let terminal_width = window_width / 3.0;

        // Right side - results
        egui::SidePanel::right("Terminal_panel")
            .exact_width(terminal_width)
            .show(ctx, |ui| {
                ui.heading("Terminal");
                ui.separator();

                egui::ScrollArea::vertical()
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        ui.label(&self.terminal);
                    });
            });

        // Left side - main content
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("NanoVNA-Saver");

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let button_text = if self.is_running { "Stop" } else { "Start" };
                    if ui.button(button_text).clicked() {
                        self.is_running = !self.is_running;
                    }
                });
            });
            ui.separator();

            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                // Serial Port Configuration
                ui.group(|ui| {
                    ui.set_width(180.0);

                    egui::ComboBox::from_label("")
                        .selected_text(
                            self.selected_port
                                .as_deref()
                                .unwrap_or("No COM ports found"),
                        )
                        .show_ui(ui, |ui| {
                            for port in &self.available_ports {
                                ui.selectable_value(&mut self.selected_port, Some(port.clone()), port);
                            }
                        });

                    ui.add_space(4.0);
                    let detection_status = if self.available_ports.is_empty() {
                        "NanoVNA not detected"
                    } else {
                        "NanoVNA detected"
                    };
                    ui.label(detection_status);
                });

                ui.add_space(8.0);

                // Sweep Configuration Panel
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        // Start and End (vertical)
                        ui.vertical(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.start_freq)
                                    .hint_text("Start Freq (Hz)")
                                    .desired_width(120.0),
                            );
                            ui.label("Start Freq (Hz)");

                            ui.add_space(4.0);

                            ui.add(
                                egui::TextEdit::singleline(&mut self.end_freq)
                                    .hint_text("End Freq (Hz)")
                                    .desired_width(120.0),
                            );
                            ui.label("End Freq (Hz)");
                        });

                        ui.add_space(8.0);

                        // Points (to the right)
                        ui.vertical(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.num_points)
                                    .hint_text("Points")
                                    .desired_width(80.0),
                            );
                            ui.label("Points");

                            ui.add_space(4.0);

                            egui::ComboBox::from_label("")
                                .selected_text(self.num_ports.to_string())
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut self.num_ports, 1, "1");
                                    ui.selectable_value(&mut self.num_ports, 2, "2");
                                    
                                });
                            ui.label("Num Ports");
                        });
                    });
                });

                ui.add_space(8.0);

                // Label field
                ui.group(|ui| {
                    ui.vertical(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.label)
                                .hint_text("Label")
                                .desired_width(150.0),
                        );

                        ui.add_space(4.0);

                        ui.add(
                            egui::TextEdit::singleline(&mut self.if_bandwidth)
                                .hint_text("IF Bandwidth")
                                .desired_width(150.0),
                        );
                    });
                });
            });

            ui.add_space(8.0);
            ui.label("Main controls will go here");
        });
    }
}
