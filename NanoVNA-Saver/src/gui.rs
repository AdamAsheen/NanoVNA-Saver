use eframe::egui;

pub struct NanoVNASaverApp {
	terminal_output: String,
}

impl Default for NanoVNASaverApp {
	fn default() -> Self {
		Self {
			terminal_output: "Terminal ready...\n".to_string(),
		}
	}
}

impl eframe::App for NanoVNASaverApp {
	fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
		// Calculate 1/3 of window width for terminal
		let window_width = ctx.screen_rect().width();
		let terminal_width = window_width / 3.0;
		
		// Right side - terminal
		egui::SidePanel::right("terminal_panel")
			.exact_width(terminal_width)
			.show(ctx, |ui| {
				ui.heading("Terminal");
				ui.separator();
				
				egui::ScrollArea::vertical()
					.stick_to_bottom(true)
					.show(ui, |ui| {
						ui.add(
							egui::TextEdit::multiline(&mut self.terminal_output)
								.desired_width(f32::INFINITY)
								.font(egui::TextStyle::Monospace)
								.interactive(false)
						);
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


