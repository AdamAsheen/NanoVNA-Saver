use crate::{RunConfig, detect_nanovna_port_names, run};
use eframe::egui;
use polars::frame::DataFrame;
use polars::prelude::{CsvWriter, SerWriter};
use std::fs::{OpenOptions, create_dir_all};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Mutex, OnceLock};
use std::thread;

static GUI_ROW_TX: OnceLock<Mutex<Option<Sender<String>>>> = OnceLock::new();
const MAX_TERMINAL_CHARS: usize = 200_000;
const MAX_LOG_ROWS_PER_FRAME: usize = 500;

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

fn trim_terminal_buffer(terminal: &mut String) {
    if terminal.len() <= MAX_TERMINAL_CHARS {
        return;
    }

    let mut cut_at = terminal.len() - MAX_TERMINAL_CHARS;
    while cut_at < terminal.len() && !terminal.is_char_boundary(cut_at) {
        cut_at += 1;
    }

    if cut_at < terminal.len()
        && let Some(newline_offset) = terminal[cut_at..].find('\n')
    {
        cut_at += newline_offset + 1;
    }

    if cut_at > 0 {
        terminal.drain(..cut_at);
    }
}

fn append_terminal_line(terminal: &mut String, line: &str) {
    if !terminal.is_empty() {
        terminal.push('\n');
    }
    terminal.push_str(line);
    trim_terminal_buffer(terminal);
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
    run_rx: Option<Receiver<Result<(DataFrame, String), String>>>,
    dataframe: Option<DataFrame>,
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
            dataframe: None,
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

        if self.selected_ports.is_empty() {
            errors.push("Select at least one COM port".to_string());
        }

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
            let mut processed = 0usize;
            while processed < MAX_LOG_ROWS_PER_FRAME {
                let Ok(row) = rx.try_recv() else {
                    break;
                };
                append_terminal_line(&mut self.terminal, &row);
                processed += 1;
            }

            if processed == MAX_LOG_ROWS_PER_FRAME {
                // Keep repainting while there is buffered log data to avoid UI stalls.
                ctx.request_repaint();
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

            match result {
                Ok((dataframe, message)) => {
                    append_terminal_line(&mut self.terminal, &message);
                    self.dataframe = Some(dataframe);
                }
                Err(err) => append_terminal_line(&mut self.terminal, &format!("Error: {err}")),
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
                    ui.add(
                        egui::TextEdit::singleline(&mut self.save_path)
                            .hint_text("Save Path")
                            .desired_width(320.0),
                    );
                    ui.add_space(8.0);

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
                                    append_terminal_line(
                                        &mut self.terminal,
                                        "Stop requested. Interrupting active sweep...",
                                    );
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
                                selected_port_names: Some(self.selected_ports.clone()),
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
                                    let df_clone = df.clone();

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

                                    Ok((df_clone, format!(
                                        "Sweep complete. Bytes: {}, Elapsed: {:.2}s\nResults file: {}",
                                        sweep.total_bytes,
                                        sweep.elapsed_seconds,
                                        output_path.display()
                                    )))
                                });
                                let _ = tx.send(result);
                            });
                        } else {
                            for error in &validation_errors {
                                append_terminal_line(
                                    &mut self.terminal,
                                    &format!("Error: {error}"),
                                );
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
            ui.separator();

            if let Some(df) = &self.dataframe {
                let available = ui.available_size();
                let plot_height = (available.y - 16.0) / 2.0;
                let plot_width = (available.x - 16.0) / 2.0;

                let channels = df.column("channel").unwrap().str().unwrap().into_iter().collect::<Vec<_>>();
                let freqs = df.column("frequency_hz").unwrap().f64().unwrap().into_iter().collect::<Vec<_>>();
                let reals = df.column("real").unwrap().f64().unwrap().into_iter().collect::<Vec<_>>();
                let imags = df.column("imag").unwrap().f64().unwrap().into_iter().collect::<Vec<_>>();

                let sweep_ids = df.column("sweep_id").unwrap().str().unwrap().into_iter().collect::<Vec<_>>();
                let vna_nums = df.column("vna_number").unwrap().i32().unwrap().into_iter().collect::<Vec<_>>();

                let mut last_sweep_per_vna: std::collections::HashMap<i32, &str> = std::collections::HashMap::new();
                for i in 0..sweep_ids.len() {
                    if let (Some(sid), Some(vna)) = (sweep_ids[i], vna_nums[i]) {
                        last_sweep_per_vna.insert(vna, sid);
                    }
                }

                let max_vna = vna_nums.iter().filter_map(|v| *v).max().unwrap_or(1) as usize;

                let mut s11: Vec<Vec<[f64; 3]>> = vec![Vec::new(); max_vna];
                let mut s21: Vec<Vec<[f64; 3]>> = vec![Vec::new(); max_vna];

                for i in 0..channels.len() {
                    let (Some(ch), Some(freq), Some(real), Some(imag), Some(vna), Some(sid)) =
                        (channels[i], freqs[i], reals[i], imags[i], vna_nums[i], sweep_ids[i]) else { continue; };
                    if last_sweep_per_vna.get(&vna).copied() != Some(sid) { continue; }
                    let vna_idx = (vna as usize) - 1;
                    match ch {
                        "S11" => { if vna_idx < max_vna { s11[vna_idx].push([freq, real, imag]); } }
                        "S21" => { if vna_idx < max_vna { s21[vna_idx].push([freq, real, imag]); } }
                        &_ => {}
                    }
                }

                ui.horizontal(|ui| {
                    ui.vertical(|ui| { crate::graph::s11_log_mag(ui, &s11, plot_height, plot_width); });
                    ui.add_space(8.0);
                    ui.vertical(|ui| { crate::graph::s11_smith(ui, &s11, plot_height, plot_width); });
                });

                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.vertical(|ui| { crate::graph::s21_log_mag(ui, &s21, plot_height, plot_width); });
                    ui.add_space(8.0);
                    ui.vertical(|ui| { crate::graph::s21_phase(ui, &s21, plot_height, plot_width); });
                });
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_values() {
        let default = NanoVNASaverApp::default();

        assert_eq!(default.terminal, String::new());
        assert_eq!(default.start_freq, 50_000);
        assert_eq!(default.end_freq, 900_000_000);
        assert_eq!(default.num_points, 101);
        assert_eq!(default.num_ports, 2);
        assert_eq!(default.label, String::new());
        assert_eq!(default.if_bandwidth, 0);
        assert_eq!(default.time, 0);
        assert_eq!(default.num_sweeps, 1);
        assert!(!default.is_running);
    }

    #[test]
    fn test_validation_messages_normal() {
        let mut mock = NanoVNASaverApp::default();

        mock.selected_ports.push("port_name".to_string());
        let expected: Vec<String> = Vec::new();

        assert_eq!(mock.validation_messages(), expected);
    }

    #[test]
    fn test_validation_messages_all() {
        let mock = NanoVNASaverApp {
            selected_ports: Vec::new(),
            end_freq: 5_000,
            start_freq: 10_000,
            num_points: 10_000,
            time: 0,
            num_sweeps: 0,
            ..Default::default()
        };

        let expected: Vec<String> = vec![
            "Select at least one COM port".to_string(),
            "Start frequency must be less than End frequency".to_string(),
            "End frequency must be 50 kHz or more".to_string(),
            "Start frequency must be 50 kHz or more".to_string(),
            "Points must be 101 or less".to_string(),
            "Either Time or Num Sweeps must be set, but not both".to_string(),
        ];

        assert_eq!(mock.validation_messages(), expected);
    }
}
