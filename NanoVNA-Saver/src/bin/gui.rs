use nanovna_saver::gui::NanoVNASaverApp;

fn main() {
    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../../assets/icon.png")).unwrap();

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1900.0, 1000.0])
            .with_icon(icon),
        ..Default::default()
    };

    let _ = eframe::run_native(
        "NanoVNA-Saver",
        options,
        Box::new(|_cc| Ok(Box::new(NanoVNASaverApp::default()))),
    );
}
