use crate::dp_info::{self, DpInfo};
use crate::drm_info::ConnectorInfo;
use crate::edid;
use crate::gpu;
use std::collections::HashMap;
use std::path::PathBuf;

pub fn print_all(connectors: &[ConnectorInfo]) {
    if connectors.is_empty() {
        println!("No DRM connectors found in /sys/class/drm/");
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

        if let Some(gpu) = &gpu_info {
            println!(
                "GPU: {} [{}] (vendor={}, device={})",
                gpu.description, gpu.card_name, gpu.pci_vendor, gpu.pci_device
            );
        } else {
            let name = card_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string());
            println!("GPU: {name}");
        }

        let last_idx = conns.len() - 1;
        for (i, conn) in conns.iter().enumerate() {
            let is_last = i == last_idx;
            let prefix = if is_last { "└── " } else { "├── " };
            let child_prefix = if is_last { "    " } else { "│   " };

            println!(
                "{prefix}{} [{}{}]",
                conn.name,
                conn.status,
                if conn.enabled { ", enabled" } else { "" }
            );

            if conn.status == "connected" {
                let connector_path = std::path::Path::new("/sys/class/drm").join(&conn.name);
                let dp = dp_info::get_dp_info(&conn.connector_type, &connector_path);
                print_connector_details(conn, &dp, child_prefix);
            }
        }

        println!();
    }
}

fn print_connector_details(conn: &ConnectorInfo, dp: &DpInfo, prefix: &str) {
    // EDID info
    if let Some(raw) = &conn.edid_raw {
        if let Some(edid) = edid::parse_edid(raw) {
            if let Some(name) = &edid.display_name {
                println!("{prefix}├── Display: {} {name}", edid.manufacturer);
            } else {
                println!(
                    "{prefix}├── Display: {} (product {:#06x})",
                    edid.manufacturer, edid.product_code
                );
            }

            if let (Some(w), Some(h)) = (edid.max_width, edid.max_height) {
                println!("{prefix}├── Native: {w}x{h}");
            }

            if edid.digital {
                let depth = edid
                    .bit_depth
                    .map(|b| format!("{b}-bit"))
                    .unwrap_or_else(|| "unknown depth".to_string());
                println!("{prefix}├── Color: {depth}, digital");
            }

            if edid.width_cm > 0 && edid.height_cm > 0 {
                let diag_cm =
                    ((edid.width_cm as f64).powi(2) + (edid.height_cm as f64).powi(2)).sqrt();
                let diag_in = diag_cm / 2.54;
                println!(
                    "{prefix}├── Size: {}cm x {}cm ({diag_in:.1}\")",
                    edid.width_cm, edid.height_cm
                );
            }

            println!(
                "{prefix}├── EDID: v{}, year {}, week {}",
                edid.version, edid.year, edid.week
            );
        }
    }

    // Protocol
    println!("{prefix}├── Protocol: {}", conn.connector_type);

    // DP-specific info from DPCD, nested under protocol
    if dp.is_dp {
        let dp_prefix = format!("{prefix}│   ");

        if let Some(aux_name) = &dp.aux_name {
            println!("{dp_prefix}├── AUX Channel: {aux_name}");
        }

        if let Some(dpcd) = &dp.dpcd {
            println!("{dp_prefix}├── DP Version: {}", dpcd.dp_version);
            println!(
                "{dp_prefix}├── Max Link Rate: {}",
                dp_info::format_link_rate(dpcd.max_link_rate_raw, dpcd.max_link_rate_gbps)
            );
            println!("{dp_prefix}├── Max Lanes: {}", dpcd.max_lane_count);
            println!(
                "{dp_prefix}├── Max Bandwidth: {}",
                dp_info::format_bandwidth(dpcd.max_link_rate_gbps, dpcd.max_lane_count)
            );

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
                println!("{dp_prefix}├── Capabilities: {}", caps.join(", "));
            }
        }

        if let Some(lc) = &dp.link_config {
            println!(
                "{dp_prefix}├── Active Link: {} x {} lane{}",
                dp_info::format_link_rate(lc.current_link_rate_raw, lc.current_link_rate_gbps),
                lc.current_lane_count,
                if lc.current_lane_count != 1 { "s" } else { "" }
            );
            println!(
                "{dp_prefix}├── Active Bandwidth: {}",
                dp_info::format_bandwidth(lc.current_link_rate_gbps, lc.current_lane_count)
            );
        }

        if let Some(ls) = &dp.link_status {
            if let Some(sc) = ls.sink_count {
                println!(
                    "{dp_prefix}├── Sink Count: {sc}{}",
                    if ls.downstream_port_status_changed {
                        " (changed)"
                    } else {
                        ""
                    }
                );
            }

            let lane_count = dp
                .link_config
                .as_ref()
                .map(|lc| lc.current_lane_count)
                .or(dp.dpcd.as_ref().map(|d| d.max_lane_count))
                .unwrap_or(4) as usize;

            let has_more = dp.psr.is_some() || dp.psr_driver_status.is_some();
            let last_branch = if has_more { "├" } else { "└" };

            for (i, lane) in ls.lane_status[..lane_count].iter().enumerate() {
                println!(
                    "{dp_prefix}├── Lane {i}: CR={} EQ={} Lock={}",
                    if lane.cr_done { "ok" } else { "FAIL" },
                    if lane.channel_eq_done { "ok" } else { "FAIL" },
                    if lane.symbol_locked { "ok" } else { "FAIL" },
                );
            }
            println!(
                "{dp_prefix}{last_branch}── Interlane Align: {}",
                if ls.interlane_align_done {
                    "ok"
                } else {
                    "FAIL"
                }
            );
        }

        // PSR info
        print_psr_info(dp, &dp_prefix);

        if dp.dpcd.is_none() && dp.psr.is_none() {
            println!("{dp_prefix}└── DPCD: not readable (requires root for /dev/drm_dp_aux*)");
        }
    }

    // DPMS
    println!("{prefix}├── DPMS: {}", conn.dpms);

    // Modes
    if !conn.modes.is_empty() {
        let modes_str = conn.modes.join(", ");
        println!("{prefix}└── Modes: {modes_str}");
    } else {
        println!("{prefix}└── Modes: (none)");
    }
}

fn print_psr_info(dp: &DpInfo, dp_prefix: &str) {
    let has_driver_status = dp.psr_driver_status.is_some();

    if let Some(psr) = &dp.psr {
        let last = !has_driver_status;
        let branch = if last { "└" } else { "├" };
        println!("{dp_prefix}{branch}── PSR: {}", psr.psr_version.as_str());

        let psr_prefix = if last {
            format!("{dp_prefix}    ")
        } else {
            format!("{dp_prefix}│   ")
        };

        // Active state
        let state = if psr.psr2_enabled {
            "PSR2 enabled"
        } else if psr.psr_enabled {
            "PSR1 enabled"
        } else {
            "disabled"
        };
        println!("{psr_prefix}├── State: {state}");

        // Sink status
        if let Some(status) = &psr.sink_status {
            println!("{psr_prefix}├── Sink Status: {}", status.as_str());
        }

        // Setup time
        println!(
            "{psr_prefix}├── Setup Time: {} us{}",
            psr.setup_time_us,
            if psr.no_train_on_exit {
                " (no link training on exit)"
            } else {
                ""
            }
        );

        // PSR2 selective update granularity
        if let Some(x) = psr.su_x_granularity {
            if let Some(y) = psr.su_y_granularity {
                println!("{psr_prefix}├── SU Granularity: {x}x{y} pixels");
            }
        }

        // Features
        let mut features = Vec::new();
        if psr.y_coord_required {
            features.push("Y-coordinate required");
        }
        if psr.su_granularity_required {
            features.push("SU granularity required");
        }
        if psr.su_aux_frame_sync_not_needed {
            features.push("AUX frame sync not needed");
        }
        if !features.is_empty() {
            println!("{psr_prefix}├── Features: {}", features.join(", "));
        }

        // Errors
        if psr.errors.is_empty() {
            println!("{psr_prefix}└── Errors: none");
        } else {
            println!("{psr_prefix}└── Errors: {}", psr.errors.join(", "));
        }
    }

    if let Some(driver_status) = &dp.psr_driver_status {
        println!("{dp_prefix}└── PSR Driver Status:");
        let inner_prefix = format!("{dp_prefix}    ");
        for line in driver_status.lines() {
            println!("{inner_prefix}{line}");
        }
    }
}
