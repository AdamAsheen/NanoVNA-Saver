use eframe::egui;

#[derive(Default)]
pub struct NanoVNASaverApp {
    terminal: String,
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
            ui.label("Main controls will go here");
        });
    }
}
