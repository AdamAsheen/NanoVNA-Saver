use eframe::egui;
use tokio_serial::available_ports;

pub struct NanoVNASaverApp {
    terminal: String,
    available_ports: Vec<String>,
    selected_port: Option<String>,
}

impl Default for NanoVNASaverApp {
    fn default() -> Self {
        let mut app = Self {
            terminal: String::new(),
            available_ports: Vec::new(),
            selected_port: None,
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
            ui.heading("NanoVNA-Saver");
            ui.separator();

            ui.group(|ui| {
                ui.set_width(120.0);
                ui.heading("Serial Port");
                ui.add_space(4.0);

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

                if ui.button("Refresh Ports").clicked() {
                    self.refresh_ports();
                }
            });

            ui.add_space(8.0);
            ui.label("Main controls will go here");
        });
    }
}
