use eframe::egui;
use tokio_serial::available_ports;

pub struct NanoVNASaverApp {
    terminal: String,
    available_ports: Vec<String>,
    selected_port: Option<String>,
    start_freq: u64,
    end_freq: u64,
    num_points: usize,
    num_ports: usize,
    label: String,
    if_bandwidth: u32,
    time: u64,
    num_sweeps: usize,
    is_running: bool,
}

impl Default for NanoVNASaverApp {
    fn default() -> Self {
        let mut app = Self {
            terminal: String::new(),
            available_ports: Vec::new(),
            selected_port: None,
            start_freq: 50_000,
            end_freq: 900_000_000,
            num_points: 101,
            num_ports: 2,
            label: String::new(),
            if_bandwidth: 0,
            time: 0,
            num_sweeps: 1,
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

        if let Some(selected) = &self.selected_port
            && !self.available_ports.iter().any(|port| port == selected)
        {
            self.selected_port = None;
        }

        if self.selected_port.is_none() {
            self.selected_port = self.available_ports.first().cloned();
        }
    }

    fn validation_messages(&self) -> Vec<String> {
        let mut errors = Vec::new();

        if self.start_freq >= self.end_freq {
            errors.push("Start frequency must be less than End frequency".to_string());
        }

        if self.num_points > 101 {
            errors.push("Points must be 101 or less".to_string());
        }

        if (self.time == 0 && self.num_sweeps == 0) || (self.time > 0 && self.num_sweeps > 0) {
            errors.push("Either Time or Num Sweeps must be set, but not both".to_string());
        }

        errors
    }
}

impl eframe::App for NanoVNASaverApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let validation_errors = self.validation_messages();

        // Calculate 1/3 of window width for results
        let window_width = ctx.screen_rect().width();
        let terminal_width = window_width / 3.0;

        // Right side - results
        egui::SidePanel::right("Terminal_panel")
            .exact_width(terminal_width)
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(12, 12, 12))
                    .inner_margin(egui::Margin::symmetric(8.0, 8.0)),
            )
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new("Terminal")
                        .color(egui::Color32::WHITE)
                        .strong(),
                );
                ui.separator();

                egui::ScrollArea::vertical()
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(&self.terminal)
                                .monospace()
                                .color(egui::Color32::WHITE),
                        );
                    });
            });

        // Left side - main content
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("NanoVNA-Saver");

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let button_text = if self.is_running { "Stop" } else { "Start" };
                    let button_color = if self.is_running {
                        egui::Color32::from_rgb(200, 40, 40)
                    } else {
                        egui::Color32::from_rgb(40, 160, 40)
                    };

                    let strong_text = egui::RichText::new(button_text).strong();

                    if self.is_running {
                        if ui
                            .add(egui::Button::new(strong_text).fill(button_color))
                            .clicked()
                        {
                            self.is_running = false;
                        }
                    } else if ui
                        .add(egui::Button::new(strong_text).fill(button_color))
                        .clicked()
                    {
                        if validation_errors.is_empty() {
                            self.is_running = true;
                        } else {
                            for error in &validation_errors {
                                if !self.terminal.is_empty() {
                                    self.terminal.push('\n');
                                }
                                self.terminal.push_str("Error: ");
                                self.terminal.push_str(error);
                            }
                        }
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
                                ui.selectable_value(
                                    &mut self.selected_port,
                                    Some(port.clone()),
                                    port,
                                );
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
                            ui.add_sized(
                                [120.0, 0.0],
                                egui::DragValue::new(&mut self.start_freq)
                                    .range(0..=u64::MAX)
                                    .speed(1.0),
                            );
                            ui.label("Start Freq (Hz)");

                            ui.add_space(4.0);

                            ui.add_sized(
                                [120.0, 0.0],
                                egui::DragValue::new(&mut self.end_freq)
                                    .range(0..=u64::MAX)
                                    .speed(1.0),
                            );
                            ui.label("End Freq (Hz)");
                        });

                        ui.add_space(8.0);

                        // Points (to the right)
                        ui.vertical(|ui| {
                            ui.add_sized(
                                [80.0, 0.0],
                                egui::DragValue::new(&mut self.num_points)
                                    .range(0..=101)
                                    .speed(1.0),
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

                        ui.add_sized(
                            [150.0, 0.0],
                            egui::DragValue::new(&mut self.if_bandwidth)
                                .range(0..=100)
                                .speed(1.0),
                        );
                        ui.label("IF Bandwidth");
                    });
                });

                ui.add_space(8.0);

                // Time field
                ui.group(|ui| {
                    ui.vertical(|ui| {
                        ui.add_sized(
                            [80.0, 0.0],
                            egui::DragValue::new(&mut self.num_sweeps)
                                .range(0..=2147483647) //max for i32 since that's what the NanoVNA accepts
                                .speed(1.0),
                        );
                        ui.label("Num Sweeps");
                        ui.add_space(4.0);

                        ui.add_sized(
                            [80.0, 0.0],
                            egui::DragValue::new(&mut self.time)
                                .range(0..=2147483647)
                                .speed(1.0),
                        );
                        ui.label("Time (s)");
                    });
                });
            });

            ui.add_space(8.0);
            ui.label("Main controls will go here");
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_messages_normal() {
        let mock = NanoVNASaverApp::default();

        let expected: Vec::<String> = Vec::new();

        assert_eq!(mock.validation_messages(), expected);
    }

    #[test]
    fn test_validation_messages_all() {
        let mut mock = NanoVNASaverApp::default();

        mock.end_freq = 10_000;
        mock.num_points = 10_000;
        mock.time = 0;
        mock.num_sweeps = 0;

        let mut expected: Vec::<String> = Vec::new();
        expected.push("Start frequency must be less than End frequency".to_string());
        expected.push("Points must be 101 or less".to_string());
        expected.push("Either Time or Num Sweeps must be set, but not both".to_string());

        assert_eq!(mock.validation_messages(), expected);
    }
}