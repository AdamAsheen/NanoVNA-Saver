use eframe::egui;

#[derive(Default)]
pub struct NanoVNASaverApp;

impl eframe::App for NanoVNASaverApp {
	fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
		egui::CentralPanel::default().show(ctx, |ui| {
			ui.label("NanoVNA-Saver GUI message");
		});
	}
}


