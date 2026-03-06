use crate::drm_info::{self, ConnectorInfo};
use crate::render;

pub struct DisplayAnalyzerApp {
    connectors: Vec<ConnectorInfo>,
}

impl DisplayAnalyzerApp {
    pub fn new() -> Self {
        Self {
            connectors: drm_info::enumerate_connectors(),
        }
    }
}

impl eframe::App for DisplayAnalyzerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Display Analyzer");
                if ui.button("Rescan").clicked() {
                    self.connectors = drm_info::enumerate_connectors();
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                render::draw_display_info(ui, &self.connectors);
            });
        });
    }
}
