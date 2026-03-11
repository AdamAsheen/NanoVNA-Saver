use crate::{RunConfig, detect_nanovna_port_names, run};
use eframe::egui;
use polars::prelude::{CsvWriter, SerWriter};
use std::fs::File;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Mutex, OnceLock};
use std::thread;

static GUI_ROW_TX: OnceLock<Mutex<Option<Sender<String>>>> = OnceLock::new();

fn gui_row_callback(row: &str) {
    if let Some(lock) = GUI_ROW_TX.get()
        && let Ok(guard) = lock.lock()
        && let Some(tx) = guard.as_ref()
    {
        let _ = tx.send(row.to_string());
    }
}

fn set_gui_row_sender(sender: Option<Sender<String>>) {
    let lock = GUI_ROW_TX.get_or_init(|| Mutex::new(None));
    if let Ok(mut guard) = lock.lock() {
        *guard = sender;
    }
}

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
    terminal_panel_width: f32,
    log_rx: Option<Receiver<String>>,
    run_rx: Option<Receiver<Result<String, String>>>,
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
            terminal_panel_width: 0.0,
            log_rx: None,
            run_rx: None,
        };
        app.refresh_ports();
        app
    }
}

impl NanoVNASaverApp {
    fn refresh_ports(&mut self) {
        self.available_ports = detect_nanovna_port_names().unwrap_or_default();

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

        if self.start_freq > 900_000_000 {
            errors.push("Start frequency must be 900 MHz or less".to_string());
        }

        if self.end_freq >  900_000_000 {
            errors.push("End frequency must be 900 MHz or less".to_string());
        }

        if self.end_freq <  50_000 {
            errors.push("End frequency must be 50 kHz or more".to_string());
        }

        if self.start_freq < 50_000 {
            errors.push("Start frequency must be 50 kHz or more".to_string());
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
        if let Some(rx) = &self.log_rx {
            while let Ok(row) = rx.try_recv() {
                if !self.terminal.is_empty() {
                    self.terminal.push('\n');
                }
                self.terminal.push_str(&row);
            }
        }

        if let Some(rx) = &self.run_rx
            && let Ok(result) = rx.try_recv()
        {
            self.is_running = false;
            self.run_rx = None;
            self.log_rx = None;
            set_gui_row_sender(None);

            if !self.terminal.is_empty() {
                self.terminal.push('\n');
            }

            match result {
                Ok(message) => self.terminal.push_str(&message),
                Err(err) => {
                    self.terminal.push_str("Error: ");
                    self.terminal.push_str(&err);
                }
            }
        }

        let validation_errors = self.validation_messages();

        let max_terminal_width = (ctx.screen_rect().width() * 0.8).max(260.0);

        // Right side - results
        let initial_terminal_width = (ctx.screen_rect().width() / 3.0).clamp(260.0, max_terminal_width);

        let default_terminal_width = if self.terminal_panel_width > 0.0 {
            self.terminal_panel_width
        } else {
            initial_terminal_width
        };

        let terminal_response = egui::SidePanel::right("Terminal_panel")
            .resizable(true)
            .default_width(default_terminal_width)
            .min_width(260.0)
            .max_width(max_terminal_width)
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

                    self.terminal_panel_width = terminal_response.response.rect.width();

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
                            let (num_sweeps, time) = if self.time > 0 {
                                (0, Some(self.time))
                            } else {
                                (self.num_sweeps, None)
                            };
                            let if_bandwidth = if self.if_bandwidth > 0 {
                                Some(self.if_bandwidth)
                            } else {
                                None
                            };
                            let label = self.label.trim().to_string();

                            let num_ports = if self.num_ports == 2 { 2 } else { 1 };

                            let config = RunConfig {
                                num_sweeps,
                                vna_number: 1,
                                start_freq: self.start_freq,
                                end_freq: self.end_freq,
                                num_points: self.num_points,
                                num_ports,
                                if_bandwidth,
                                time,
                                label,
                                row_callback: Some(gui_row_callback),
                            };

                            let (log_tx, log_rx) = mpsc::channel();
                            set_gui_row_sender(Some(log_tx));
                            self.log_rx = Some(log_rx);

                            let (tx, rx) = mpsc::channel();
                            self.run_rx = Some(rx);
                            self.is_running = true;

                            thread::spawn(move || {
                                let result = run(config).and_then(|sweep| {
                                    let output_path = std::env::current_dir()
                                        .map_err(|e| format!("Failed to get current directory: {e}"))?
                                        .join("output.csv");

                                    let mut df = sweep.dataframe;
                                    let mut file = File::create(&output_path)
                                        .map_err(|e| format!("Failed to create CSV file: {e}"))?;

                                    CsvWriter::new(&mut file)
                                        .include_header(true)
                                        .finish(&mut df)
                                        .map_err(|e| format!("Failed to write CSV: {e}"))?;

                                    Ok(format!(
                                        "Sweep complete. Bytes: {}, Elapsed: {:.2}s\nResults file: {}",
                                        sweep.total_bytes,
                                        sweep.elapsed_seconds,
                                        output_path.display()
                                    ))
                                });
                                let _ = tx.send(result);
                            });
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
                    ui.set_width(240.0);
                    ui.set_min_height(77.0);

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
                    if ui.button("Refresh").clicked() {
                        self.refresh_ports();
                    }
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
                                    .range(50_000..=900_000_000)
                                    .speed(1.0),
                            );
                            ui.label("Start Freq (Hz)");

                            ui.add_space(4.0);

                            ui.add_sized(
                                [120.0, 0.0],
                                egui::DragValue::new(&mut self.end_freq)
                                    .range(50_000..=900_000_000)
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
