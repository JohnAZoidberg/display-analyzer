use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

pub struct DpInfo {
    pub is_dp: bool,
    pub aux_name: Option<String>,
    pub dpcd: Option<DpcdInfo>,
    pub link_config: Option<LinkConfig>,
    pub link_status: Option<LinkStatus>,
}

#[allow(dead_code)]
pub struct DpcdInfo {
    pub dp_version: String,
    pub max_link_rate_gbps: f64,
    pub max_link_rate_raw: u8,
    pub max_lane_count: u8,
    pub enhanced_framing: bool,
    pub tps3_supported: bool,
    pub downspread: bool,
    pub num_receiver_ports: u8,
    pub mst_capable: bool,
}

#[allow(dead_code)]
pub struct LinkConfig {
    pub current_link_rate_gbps: f64,
    pub current_link_rate_raw: u8,
    pub current_lane_count: u8,
    pub current_enhanced_framing: bool,
    pub current_downspread: bool,
}

#[allow(dead_code)]
pub struct LinkStatus {
    pub lane_status: [LaneStatus; 4],
    pub interlane_align_done: bool,
    pub downstream_port_status_changed: bool,
    pub link_status_updated: bool,
    pub sink_count: Option<u8>,
}

#[derive(Default, Clone)]
pub struct LaneStatus {
    pub cr_done: bool,
    pub channel_eq_done: bool,
    pub symbol_locked: bool,
}

fn link_rate_to_gbps(raw: u8) -> f64 {
    match raw {
        0x06 => 1.62,
        0x0a => 2.7,
        0x14 => 5.4,
        0x1e => 8.1,
        _ => raw as f64 * 0.27,
    }
}

fn link_rate_name(raw: u8) -> &'static str {
    match raw {
        0x06 => "RBR",
        0x0a => "HBR",
        0x14 => "HBR2",
        0x1e => "HBR3",
        _ => "unknown",
    }
}

fn dpcd_version_string(rev: u8) -> String {
    let major = rev >> 4;
    let minor = rev & 0x0F;
    format!("{major}.{minor}")
}

fn find_aux_device(connector_path: &Path) -> Option<String> {
    // Look for drm_dp_aux* directory inside the connector sysfs path
    let entries = fs::read_dir(connector_path).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("drm_dp_aux") {
            return Some(name);
        }
    }
    None
}

fn read_dpcd_bytes(aux_dev: &str, offset: u64, len: usize) -> Option<Vec<u8>> {
    let path = format!("/dev/{aux_dev}");
    let mut file = fs::File::open(&path).ok()?;
    file.seek(SeekFrom::Start(offset)).ok()?;
    let mut buf = vec![0u8; len];
    file.read_exact(&mut buf).ok()?;
    Some(buf)
}

fn parse_dpcd_caps(data: &[u8]) -> DpcdInfo {
    let rev = data[0];
    let max_rate_raw = data[1];
    let lane_byte = data[2];
    let max_lane_count = lane_byte & 0x1F;
    let enhanced_framing = lane_byte & 0x80 != 0;
    let tps3_supported = lane_byte & 0x40 != 0;
    let downspread = data[3] & 0x01 != 0;
    let num_receiver_ports = (data[4] & 0x01) + 1;
    let mst_capable = if data.len() > 0x21 {
        data[0x21] & 0x01 != 0
    } else {
        false
    };

    DpcdInfo {
        dp_version: dpcd_version_string(rev),
        max_link_rate_gbps: link_rate_to_gbps(max_rate_raw),
        max_link_rate_raw: max_rate_raw,
        max_lane_count,
        enhanced_framing,
        tps3_supported,
        downspread,
        num_receiver_ports,
        mst_capable,
    }
}

fn parse_link_config(data: &[u8]) -> LinkConfig {
    let rate_raw = data[0];
    let lane_byte = data[1];
    LinkConfig {
        current_link_rate_gbps: link_rate_to_gbps(rate_raw),
        current_link_rate_raw: rate_raw,
        current_lane_count: lane_byte & 0x1F,
        current_enhanced_framing: lane_byte & 0x80 != 0,
        current_downspread: data[2] & 0x10 != 0,
    }
}

fn parse_link_status(data: &[u8]) -> LinkStatus {
    // data[0] = 0x200 (SINK_COUNT)
    let sink_count = Some(data[0] & 0x3F);

    // data[2] = 0x202, data[3] = 0x203: per-lane status
    let mut lane_status = [
        LaneStatus::default(),
        LaneStatus::default(),
        LaneStatus::default(),
        LaneStatus::default(),
    ];

    for lane in 0..4u8 {
        let byte_offset = 2 + (lane / 2) as usize;
        let shift = (lane % 2) * 4;
        if byte_offset < data.len() {
            let nibble = (data[byte_offset] >> shift) & 0x0F;
            lane_status[lane as usize] = LaneStatus {
                cr_done: nibble & 0x01 != 0,
                channel_eq_done: nibble & 0x02 != 0,
                symbol_locked: nibble & 0x04 != 0,
            };
        }
    }

    // data[4] = 0x204: lane align status
    let interlane_align_done = data.len() > 4 && data[4] & 0x01 != 0;
    let downstream_port_status_changed = data.len() > 4 && data[4] & 0x40 != 0;
    let link_status_updated = data.len() > 4 && data[4] & 0x80 != 0;

    LinkStatus {
        lane_status,
        interlane_align_done,
        downstream_port_status_changed,
        link_status_updated,
        sink_count,
    }
}

pub fn get_dp_info(connector_type: &str, connector_path: &Path) -> DpInfo {
    let is_dp = matches!(connector_type, "DP" | "eDP");

    if !is_dp {
        return DpInfo {
            is_dp: false,
            aux_name: None,
            dpcd: None,
            link_config: None,
            link_status: None,
        };
    }

    let aux_dev = find_aux_device(connector_path);
    let aux_name = aux_dev.as_ref().and_then(|dev| {
        let name_path = connector_path.join(dev).join("name");
        fs::read_to_string(name_path)
            .ok()
            .map(|s| s.trim().to_string())
    });

    let dpcd = aux_dev
        .as_ref()
        .and_then(|dev| read_dpcd_bytes(dev, 0x0000, 0x22))
        .map(|data| parse_dpcd_caps(&data));

    let link_config = aux_dev
        .as_ref()
        .and_then(|dev| read_dpcd_bytes(dev, 0x0100, 4))
        .map(|data| parse_link_config(&data));

    let link_status = aux_dev
        .as_ref()
        .and_then(|dev| read_dpcd_bytes(dev, 0x0200, 8))
        .map(|data| parse_link_status(&data));

    DpInfo {
        is_dp: true,
        aux_name,
        dpcd,
        link_config,
        link_status,
    }
}

pub fn format_link_rate(raw: u8, gbps: f64) -> String {
    format!("{gbps} Gbps/lane ({}, {:#04x})", link_rate_name(raw), raw)
}

pub fn format_bandwidth(rate_gbps: f64, lanes: u8) -> String {
    let total = rate_gbps * lanes as f64;
    // DP uses 8b/10b encoding for DP 1.0-1.3, effective data rate is 80%
    let effective = total * 0.8;
    format!("{total:.1} Gbps total ({effective:.1} Gbps effective)")
}
