use crate::dp_info;
use crate::drm_info::ConnectorInfo;
use crate::edid;
use crate::gpu;
use std::collections::HashMap;
use std::path::PathBuf;

pub fn draw_display_info(ui: &mut egui::Ui, connectors: &[ConnectorInfo]) {
    if connectors.is_empty() {
        ui.label("No DRM connectors found in /sys/class/drm/");
        return;
    }

    // Group connectors by GPU card
    let mut by_card: HashMap<PathBuf, Vec<&ConnectorInfo>> = HashMap::new();
    for conn in connectors {
        by_card
            .entry(conn.card_path.clone())
            .or_default()
            .push(conn);
    }

    let mut cards: Vec<_> = by_card.into_iter().collect();
    cards.sort_by(|a, b| a.0.cmp(&b.0));

    for (card_path, conns) in &cards {
        let gpu_info = gpu::get_gpu_info(card_path);
        let gpu_label = if let Some(gpu) = &gpu_info {
            format!(
                "GPU: {} [{}] (vendor={}, device={})",
                gpu.description, gpu.card_name, gpu.pci_vendor, gpu.pci_device
            )
        } else {
            let name = card_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string());
            format!("GPU: {name}")
        };

        ui.heading(&gpu_label);
        ui.separator();

        for conn in conns {
            let header = format!(
                "{} [{}{}]",
                conn.name,
                conn.status,
                if conn.enabled { ", enabled" } else { "" }
            );

            egui::CollapsingHeader::new(&header)
                .default_open(conn.status == "connected")
                .show(ui, |ui| {
                    draw_connector_details(ui, conn);
                });
        }

        ui.add_space(10.0);
    }
}

fn draw_connector_details(ui: &mut egui::Ui, conn: &ConnectorInfo) {
    // EDID info
    if let Some(raw) = &conn.edid_raw {
        if let Some(edid) = edid::parse_edid(raw) {
            egui::Grid::new(format!("edid_{}", conn.name))
                .num_columns(2)
                .spacing([10.0, 4.0])
                .show(ui, |ui| {
                    if let Some(name) = &edid.display_name {
                        ui.label("Display:");
                        ui.label(format!("{} {name}", edid.manufacturer));
                        ui.end_row();
                    }

                    if let (Some(w), Some(h)) = (edid.max_width, edid.max_height) {
                        ui.label("Native:");
                        ui.label(format!("{w}x{h}"));
                        ui.end_row();
                    }

                    if edid.digital {
                        ui.label("Color:");
                        let depth = edid
                            .bit_depth
                            .map(|b| format!("{b}-bit"))
                            .unwrap_or_else(|| "unknown depth".to_string());
                        ui.label(format!("{depth}, digital"));
                        ui.end_row();
                    }

                    if edid.width_cm > 0 && edid.height_cm > 0 {
                        let diag_cm = ((edid.width_cm as f64).powi(2)
                            + (edid.height_cm as f64).powi(2))
                        .sqrt();
                        let diag_in = diag_cm / 2.54;
                        ui.label("Size:");
                        ui.label(format!(
                            "{}cm x {}cm ({diag_in:.1}\")",
                            edid.width_cm, edid.height_cm
                        ));
                        ui.end_row();
                    }

                    ui.label("EDID:");
                    ui.label(format!(
                        "v{}, year {}, week {}",
                        edid.version, edid.year, edid.week
                    ));
                    ui.end_row();
                });
        }
    }

    ui.add_space(4.0);

    egui::Grid::new(format!("conn_{}", conn.name))
        .num_columns(2)
        .spacing([10.0, 4.0])
        .show(ui, |ui| {
            ui.label("Protocol:");
            ui.label(&conn.connector_type);
            ui.end_row();

            // DP-specific info
            let connector_path = std::path::Path::new("/sys/class/drm").join(&conn.name);
            let dp = dp_info::get_dp_info(&conn.connector_type, &connector_path);
            if dp.is_dp {
                if let Some(rate) = &dp.link_rate {
                    ui.label("DP Link Rate:");
                    ui.label(rate);
                    ui.end_row();
                }
                if let Some(lanes) = &dp.lane_count {
                    ui.label("DP Lanes:");
                    ui.label(lanes);
                    ui.end_row();
                }
            }

            ui.label("DPMS:");
            ui.label(&conn.dpms);
            ui.end_row();

            ui.label("Modes:");
            ui.label(conn.modes.join(", "));
            ui.end_row();
        });
}
