use nanovna_saver::gui::NanoVNASaverApp;

fn main() {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default().with_inner_size([1900.0, 1000.0]),
        ..Default::default()
    };

    let _ = eframe::run_native(
        "NanoVNA-Saver",
        options,
        Box::new(|_cc| Ok(Box::new(NanoVNASaverApp::default()))),
    );
}
