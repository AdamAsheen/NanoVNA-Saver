use crate::{RunConfig, detect_nanovna_port_names, run};
use eframe::egui;
use polars::prelude::{CsvWriter, SerWriter};
use std::fs::{OpenOptions, create_dir_all};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
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

fn resolve_output_path(input: &str) -> PathBuf {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return std::env::current_dir()
            .map(|dir| dir.join("output.csv"))
            .unwrap_or_else(|_| PathBuf::from("output.csv"));
    }

    let path = PathBuf::from(trimmed);
    if path.is_dir() || trimmed.ends_with('\\') || trimmed.ends_with('/') {
        path.join("output.csv")
    } else {
        path
    }
}

pub struct NanoVNASaverApp {
    terminal: String,
    available_ports: Vec<String>,
    selected_ports: Vec<String>,
    start_freq: u64,
    end_freq: u64,
    num_points: usize,
    num_ports: usize,
    save_path: String,
    label: String,
    if_bandwidth: u32,
    time: u64,
    num_sweeps: usize,
    is_running: bool,
    terminal_panel_width: f32,
    log_rx: Option<Receiver<String>>,
    run_rx: Option<Receiver<Result<String, String>>>,
    stop_flag: Option<Arc<AtomicBool>>,
}

impl Default for NanoVNASaverApp {
    fn default() -> Self {
        let mut app = Self {
            terminal: String::new(),
            available_ports: Vec::new(),
            selected_ports: Vec::new(),
            start_freq: 50_000,
            end_freq: 900_000_000,
            num_points: 101,
            num_ports: 2,
            save_path: std::env::current_dir()
                .map(|dir| dir.join("output.csv").display().to_string())
                .unwrap_or_else(|_| "output.csv".to_string()),
            label: String::new(),
            if_bandwidth: 0,
            time: 0,
            num_sweeps: 1,
            is_running: false,
            terminal_panel_width: 0.0,
            log_rx: None,
            run_rx: None,
            stop_flag: None,
        };
        app.refresh_ports();
        app
    }
}

impl NanoVNASaverApp {
    fn refresh_ports(&mut self) {
        self.available_ports = detect_nanovna_port_names().unwrap_or_default();
        self.selected_ports
            .retain(|selected| self.available_ports.iter().any(|port| port == selected));
    }

    fn validation_messages(&self) -> Vec<String> {
        let mut errors = Vec::new();

        if self.start_freq >= self.end_freq {
            errors.push("Start frequency must be less than End frequency".to_string());
        }

        if self.start_freq > 900_000_000 {
            errors.push("Start frequency must be 900 MHz or less".to_string());
        }

        if self.end_freq > 900_000_000 {
            errors.push("End frequency must be 900 MHz or less".to_string());
        }

        if self.end_freq < 50_000 {
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
            self.stop_flag = None;
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
        let initial_terminal_width =
            (ctx.screen_rect().width() / 3.0).clamp(260.0, max_terminal_width);

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
                            && let Some(flag) = &self.stop_flag {
                                let was_stopped = flag.swap(true, Ordering::Relaxed);
                                if !was_stopped {
                                    if !self.terminal.is_empty() {
                                        self.terminal.push('\n');
                                    }
                                    self.terminal.push_str("Stop requested. Interrupting active sweep...");
                                }
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
                            let label = {
                                let trimmed = self.label.trim();
                                if trimmed.is_empty() {
                                    "default_label".to_string()
                                } else {
                                    trimmed.to_string()
                                }
                            };
                            let output_path = resolve_output_path(&self.save_path);

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
                                stop_flag: {
                                    let flag = Arc::new(AtomicBool::new(false));
                                    self.stop_flag = Some(Arc::clone(&flag));
                                    flag
                                },
                            };

                            let (log_tx, log_rx) = mpsc::channel();
                            set_gui_row_sender(Some(log_tx));
                            self.log_rx = Some(log_rx);

                            let (tx, rx) = mpsc::channel();
                            self.run_rx = Some(rx);
                            self.is_running = true;

                            thread::spawn(move || {
                                let result = run(config).and_then(|sweep| {
                                    let mut df = sweep.dataframe;

                                    if let Some(parent) = output_path.parent()
                                        && !parent.as_os_str().is_empty()
                                    {
                                        create_dir_all(parent).map_err(|e| {
                                            format!(
                                                "Failed to create output directory '{}': {e}",
                                                parent.display()
                                            )
                                        })?;
                                    }

                                    let mut file = OpenOptions::new()
                                        .create(true)
                                        .write(true)
                                        .truncate(true)
                                        .open(&output_path)
                                        .map_err(|e| {
                                            if e.kind() == std::io::ErrorKind::PermissionDenied {
                                                format!(
                                                    "Failed to create CSV file '{}': {e}. This path may be a protected location, a folder, or the file may be locked by another program.",
                                                    output_path.display()
                                                )
                                            } else {
                                                format!(
                                                    "Failed to create CSV file '{}': {e}",
                                                    output_path.display()
                                                )
                                            }
                                        })?;

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

                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            let selected_text = if self.selected_ports.is_empty() {
                                "Select COM ports".to_string()
                            } else {
                                format!("{} selected", self.selected_ports.len())
                            };

                            egui::ComboBox::from_id_salt("serial_port_selector")
                                .selected_text(selected_text)
                                .width(120.0)
                                .show_ui(ui, |ui| {
                                    if self.available_ports.is_empty() {
                                        ui.label("No COM ports found");
                                    } else {
                                        for port in &self.available_ports {
                                            let mut is_selected = self.selected_ports.contains(port);
                                            if ui.checkbox(&mut is_selected, port).changed() {
                                                if is_selected {
                                                    self.selected_ports.push(port.clone());
                                                    self.selected_ports.sort();
                                                    self.selected_ports.dedup();
                                                } else {
                                                    self.selected_ports.retain(|selected| selected != port);
                                                }
                                            }
                                        }
                                    }
                                });
                        });

                        ui.add_space(8.0);

                        ui.allocate_ui_with_layout(
                            egui::vec2(100.0, 77.0),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                ui.label("Selected");

                                egui::ScrollArea::vertical().max_height(34.0).show(ui, |ui| {
                                    if self.selected_ports.is_empty() {
                                        ui.label("None");
                                    } else {
                                        for port in &self.selected_ports {
                                            ui.label(port);
                                        }
                                    }
                                });

                                ui.add_space(4.0);
                                ui.with_layout(egui::Layout::bottom_up(egui::Align::Max), |ui| {
                                    if ui.button("Refresh").clicked() {
                                        self.refresh_ports();
                                    }
                                });
                            },
                        );
                    });
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

                            egui::ComboBox::from_id_salt("num_ports_selector")
                                .selected_text(self.num_ports.to_string())
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut self.num_ports, 1, "1");
                                    ui.selectable_value(&mut self.num_ports, 2, "2");
                                });
                            ui.label("Num Ports");
                        });

                        ui.add_space(8.0);

                        ui.group(|ui| {
                            ui.set_min_height(77.0);
                            ui.vertical(|ui| {
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.save_path)
                                        .hint_text("Save Path")
                                        .desired_width(260.0),
                                );
                                ui.add_space(4.0);
                                ui.label("Save Path");
                            });
                        });
                    });
                });

                ui.add_space(8.0);

                // Label field
                ui.group(|ui| {
                    ui.set_min_height(77.0);
                    ui.vertical(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.label)
                                .hint_text("Label")
                                .desired_width(150.0),
                        );

                        ui.add_space(12.0);

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
