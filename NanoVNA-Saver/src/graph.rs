use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints};

const vna_colors: &[egui::Color32] = &[
    egui::Color32::from_rgb(255, 165, 0),
    egui::Color32::from_rgb(70, 130, 255),
    egui::Color32::from_rgb(220, 50, 50),
    egui::Color32::from_rgb(50, 200, 50),
    egui::Color32::from_rgb(180, 0, 255),
    egui::Color32::from_rgb(0, 200, 200),
    egui::Color32::from_rgb(255, 100, 180),
    egui::Color32::from_rgb(255, 220, 0),
    egui::Color32::from_rgb(150, 100, 50),
    egui::Color32::from_rgb(200, 200, 200),
];

pub fn s11_log_mag(ui: &mut egui::Ui, data: &[Vec<[f64; 3]>], height: f32, width: f32) {
    ui.label("S11 Log Magnitude (dB)");
    Plot::new("s11_log_mag")
        .height(height)
        .width(width)
        .x_axis_label("Frequency (Hz)")
        .y_axis_label("dB")
        .show(ui, |plot_ui| {
        for (i, vna_data) in data.iter().enumerate() {
            let color = VNA_COLORS[i % VNA_COLORS.len()];
            let points: PlotPoints = vna_data
                .iter()
                .map(|p| {
                    let mag_db = 20.0 * (p[1] * p[1] + p[2] * p[2]).sqrt().max(1e-30).log10();
                    [p[0], mag_db]
                })
                .collect();
            plot_ui.line(Line::new(points).color(color).name(format!("VNA {}", i + 1)));
        }
        });
}

pub fn s21_log_mag(ui: &mut egui::Ui, data: &[Vec<[f64; 3]>], height: f32, width: f32) {
    ui.label("S21 Log Magnitude (dB)");
    Plot::new("s21_log_mag")
        .height(height)
        .width(width)
        .x_axis_label("Frequency (Hz)")
        .y_axis_label("dB")
        .show(ui, |plot_ui| {
            let points: PlotPoints = data
                .iter()
                .map(|p| {
                    let mag_db = 20.0 * (p[1] * p[1] + p[2] * p[2]).sqrt().max(1e-30).log10();
                    [p[0], mag_db]
                })
                .collect();
            plot_ui.line(Line::new(points).name("S21"));
        });
}

pub fn s21_phase(ui: &mut egui::Ui, data: &[[f64; 3]], height: f32, width: f32) {
    ui.label("S21 Phase (°)");
    Plot::new("s21_phase")
        .height(height)
        .width(width)
        .x_axis_label("Frequency (Hz)")
        .y_axis_label("degrees")
        .show(ui, |plot_ui| {
            let points: PlotPoints = data
                .iter()
                .map(|p| {
                    let phase_deg = p[2].atan2(p[1]).to_degrees();
                    [p[0], phase_deg]
                })
                .collect();
            plot_ui.line(Line::new(points).name("S21 Phase"));
        });
}

pub fn s11_smith(ui: &mut egui::Ui, data: &[[f64; 3]], height: f32, width: f32) {
    ui.label("S11 Smith Chart");
    Plot::new("s11_smith")
        .height(height)
        .width(width)
        .x_axis_label("Real")
        .y_axis_label("Imaginary")
        .data_aspect(1.0)
        .show(ui, |plot_ui| {
            let circle: PlotPoints = (0..=360)
                .map(|deg| {
                    let rad = (deg as f64).to_radians();
                    [rad.cos(), rad.sin()]
                })
                .collect();
            plot_ui.line(
                Line::new(circle)
                    .color(egui::Color32::from_rgb(100, 100, 100))
                    .name("boundary"),
            );
            for r in [0.2, 0.5, 1.0, 2.0, 5.0] {
                let center_x = r / (r + 1.0);
                let radius = 1.0 / (r + 1.0);
                let pts: PlotPoints = (0..=360)
                    .map(|deg| {
                        let rad = (deg as f64).to_radians();
                        [center_x + radius * rad.cos(), radius * rad.sin()]
                    })
                    .collect();
                plot_ui.line(
                    Line::new(pts)
                        .color(egui::Color32::from_rgb(60, 60, 60))
                        .name(format!("R={r}")),
                );
            }

            for x in [0.2, 0.5, 1.0, 2.0, 5.0] {
                for sign in [1.0_f64, -1.0_f64] {
                    let xv = sign * x;
                    let center_y = 1.0 / xv;
                    let radius = (1.0 / xv).abs();
                    let pts: PlotPoints = (0..=360)
                        .map(|deg| {
                            let rad = (deg as f64).to_radians();
                            [1.0 + radius * rad.cos(), center_y + radius * rad.sin()]
                        })
                        .filter(|p| p[0] * p[0] + p[1] * p[1] <= 1.01)
                        .collect();
                    plot_ui.line(
                        Line::new(pts)
                            .color(egui::Color32::from_rgb(60, 60, 60))
                            .name(format!("X={xv}")),
                    );
                }
            }

            if !data.is_empty() {
                let pts: PlotPoints = data.iter().map(|p| [p[1], p[2]]).collect();
                plot_ui.line(
                    Line::new(pts)
                        .color(egui::Color32::from_rgb(255, 165, 0))
                        .name("S11"),
                );
            }
        });
}
