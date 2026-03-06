use crate::dp_info::{self, DpInfo};
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
                    let connector_path = std::path::Path::new("/sys/class/drm").join(&conn.name);
                    let dp = dp_info::get_dp_info(&conn.connector_type, &connector_path);
                    draw_connector_details(ui, conn, &dp);
                });
        }

        ui.add_space(10.0);
    }
}

fn draw_connector_details(ui: &mut egui::Ui, conn: &ConnectorInfo, dp: &DpInfo) {
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

            if dp.is_dp {
                if let Some(aux_name) = &dp.aux_name {
                    ui.label("AUX Channel:");
                    ui.label(aux_name);
                    ui.end_row();
                }

                if let Some(dpcd) = &dp.dpcd {
                    ui.label("DP Version:");
                    ui.label(&dpcd.dp_version);
                    ui.end_row();

                    ui.label("Max Link Rate:");
                    ui.label(dp_info::format_link_rate(
                        dpcd.max_link_rate_raw,
                        dpcd.max_link_rate_gbps,
                    ));
                    ui.end_row();

                    ui.label("Max Lanes:");
                    ui.label(dpcd.max_lane_count.to_string());
                    ui.end_row();

                    ui.label("Max Bandwidth:");
                    ui.label(dp_info::format_bandwidth(
                        dpcd.max_link_rate_gbps,
                        dpcd.max_lane_count,
                    ));
                    ui.end_row();

                    let mut caps = Vec::new();
                    if dpcd.enhanced_framing {
                        caps.push("enhanced framing");
                    }
                    if dpcd.tps3_supported {
                        caps.push("TPS3");
                    }
                    if dpcd.downspread {
                        caps.push("0.5% downspread");
                    }
                    if dpcd.mst_capable {
                        caps.push("MST");
                    }
                    if !caps.is_empty() {
                        ui.label("Capabilities:");
                        ui.label(caps.join(", "));
                        ui.end_row();
                    }
                }

                if let Some(lc) = &dp.link_config {
                    ui.label("Active Link:");
                    ui.label(format!(
                        "{} x {} lane{}",
                        dp_info::format_link_rate(
                            lc.current_link_rate_raw,
                            lc.current_link_rate_gbps
                        ),
                        lc.current_lane_count,
                        if lc.current_lane_count != 1 { "s" } else { "" }
                    ));
                    ui.end_row();

                    ui.label("Active Bandwidth:");
                    ui.label(dp_info::format_bandwidth(
                        lc.current_link_rate_gbps,
                        lc.current_lane_count,
                    ));
                    ui.end_row();
                }

                if let Some(ls) = &dp.link_status {
                    if let Some(sc) = ls.sink_count {
                        ui.label("Sink Count:");
                        ui.label(format!(
                            "{sc}{}",
                            if ls.downstream_port_status_changed {
                                " (changed)"
                            } else {
                                ""
                            }
                        ));
                        ui.end_row();
                    }

                    let lane_count = dp
                        .link_config
                        .as_ref()
                        .map(|lc| lc.current_lane_count)
                        .or(dp.dpcd.as_ref().map(|d| d.max_lane_count))
                        .unwrap_or(4) as usize;

                    let all_ok = ls.lane_status[..lane_count]
                        .iter()
                        .all(|l| l.cr_done && l.channel_eq_done && l.symbol_locked);

                    ui.label("Link Training:");
                    if all_ok && ls.interlane_align_done {
                        ui.label(format!("OK (all {lane_count} lanes locked, aligned)"));
                    } else {
                        ui.label("ISSUES (see details)");
                    }
                    ui.end_row();

                    if !(all_ok && ls.interlane_align_done) {
                        for (i, lane) in ls.lane_status[..lane_count].iter().enumerate() {
                            ui.label(format!("  Lane {i}:"));
                            ui.label(format!(
                                "CR={} EQ={} Lock={}",
                                if lane.cr_done { "ok" } else { "FAIL" },
                                if lane.channel_eq_done { "ok" } else { "FAIL" },
                                if lane.symbol_locked { "ok" } else { "FAIL" },
                            ));
                            ui.end_row();
                        }
                    }
                }

                if dp.dpcd.is_none() {
                    ui.label("DPCD:");
                    ui.label("not readable (requires root for /dev/drm_dp_aux*)");
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
