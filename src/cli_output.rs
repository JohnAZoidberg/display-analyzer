use crate::dp_info;
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
                print_connector_details(conn, child_prefix);
            }
        }

        println!();
    }
}

fn print_connector_details(conn: &ConnectorInfo, prefix: &str) {
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

    // DP-specific info
    let connector_path = std::path::Path::new("/sys/class/drm").join(&conn.name);
    let dp = dp_info::get_dp_info(&conn.connector_type, &connector_path);
    if dp.is_dp {
        if let Some(rate) = &dp.link_rate {
            println!("{prefix}├── DP Link Rate: {rate}");
        }
        if let Some(lanes) = &dp.lane_count {
            println!("{prefix}├── DP Lanes: {lanes}");
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
