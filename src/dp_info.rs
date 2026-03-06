use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

pub struct DpInfo {
    pub is_dp: bool,
    pub aux_name: Option<String>,
    pub dpcd: Option<DpcdInfo>,
    pub link_config: Option<LinkConfig>,
    pub link_status: Option<LinkStatus>,
    pub psr: Option<PsrInfo>,
    pub psr_driver_status: Option<String>,
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

pub struct PsrInfo {
    pub psr_version: PsrVersion,
    pub no_train_on_exit: bool,
    pub setup_time_us: u16,
    pub y_coord_required: bool,
    pub su_granularity_required: bool,
    pub su_aux_frame_sync_not_needed: bool,
    pub su_x_granularity: Option<u8>,
    pub su_y_granularity: Option<u8>,
    pub psr_enabled: bool,
    pub psr2_enabled: bool,
    pub sink_status: Option<PsrSinkStatus>,
    pub errors: Vec<String>,
}

#[derive(Clone, Copy, PartialEq)]
pub enum PsrVersion {
    None,
    Psr1,
    Psr2,
    Psr2YCoord,
    Psr2EarlyTransport,
}

impl PsrVersion {
    pub fn as_str(&self) -> &'static str {
        match self {
            PsrVersion::None => "Not supported",
            PsrVersion::Psr1 => "PSR1",
            PsrVersion::Psr2 => "PSR2",
            PsrVersion::Psr2YCoord => "PSR2 (Y-coordinate)",
            PsrVersion::Psr2EarlyTransport => "PSR2 (Early Transport)",
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Copy)]
pub enum PsrSinkStatus {
    Inactive,
    ActiveSrcSynced,
    ActiveRfb,
    ActiveSinkSynced,
    Resync,
    InternalError,
    Unknown(u8),
}

impl PsrSinkStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            PsrSinkStatus::Inactive => "inactive",
            PsrSinkStatus::ActiveSrcSynced => "active (source synced)",
            PsrSinkStatus::ActiveRfb => "active (RFB)",
            PsrSinkStatus::ActiveSinkSynced => "active (sink synced)",
            PsrSinkStatus::Resync => "resync",
            PsrSinkStatus::InternalError => "internal error",
            PsrSinkStatus::Unknown(_) => "unknown",
        }
    }
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

fn parse_psr_info(aux_dev: &str) -> Option<PsrInfo> {
    // DPCD 0x070: PSR_SUPPORT, 0x071: PSR_CAPS
    let psr_caps = read_dpcd_bytes(aux_dev, 0x070, 2)?;

    let psr_version = match psr_caps[0] {
        0 => PsrVersion::None,
        1 => PsrVersion::Psr1,
        2 => PsrVersion::Psr2,
        3 => PsrVersion::Psr2YCoord,
        4 => PsrVersion::Psr2EarlyTransport,
        _ => PsrVersion::None,
    };

    if psr_version == PsrVersion::None {
        return None;
    }

    let caps_byte = psr_caps[1];
    let no_train_on_exit = caps_byte & 0x01 != 0;
    let setup_time_us = match (caps_byte >> 1) & 0x07 {
        0 => 330,
        1 => 275,
        2 => 220,
        3 => 165,
        4 => 110,
        5 => 55,
        6 => 0,
        _ => 330,
    };
    let y_coord_required = caps_byte & 0x10 != 0;
    let su_granularity_required = caps_byte & 0x20 != 0;
    let su_aux_frame_sync_not_needed = caps_byte & 0x40 != 0;

    // PSR2 selective update granularity (0x072, 0x074)
    let su_x_granularity = if matches!(
        psr_version,
        PsrVersion::Psr2 | PsrVersion::Psr2YCoord | PsrVersion::Psr2EarlyTransport
    ) {
        read_dpcd_bytes(aux_dev, 0x072, 1).map(|d| d[0])
    } else {
        None
    };
    let su_y_granularity = if matches!(
        psr_version,
        PsrVersion::Psr2 | PsrVersion::Psr2YCoord | PsrVersion::Psr2EarlyTransport
    ) {
        read_dpcd_bytes(aux_dev, 0x074, 1).map(|d| d[0])
    } else {
        None
    };

    // DPCD 0x170: PSR_EN_CFG (current enable state)
    let psr_en = read_dpcd_bytes(aux_dev, 0x170, 1)
        .map(|d| d[0])
        .unwrap_or(0);
    let psr_enabled = psr_en & 0x01 != 0;
    let psr2_enabled = psr_en & 0x40 != 0;

    // DPCD 0x2008: PSR sink status
    let sink_status = read_dpcd_bytes(aux_dev, 0x2008, 1).map(|d| match d[0] & 0x07 {
        0 => PsrSinkStatus::Inactive,
        1 => PsrSinkStatus::ActiveSrcSynced,
        2 => PsrSinkStatus::ActiveRfb,
        3 => PsrSinkStatus::ActiveSinkSynced,
        4 => PsrSinkStatus::Resync,
        7 => PsrSinkStatus::InternalError,
        v => PsrSinkStatus::Unknown(v),
    });

    // DPCD 0x2006: PSR error status
    let mut errors = Vec::new();
    if let Some(err) = read_dpcd_bytes(aux_dev, 0x2006, 1).map(|d| d[0]) {
        if err & 0x01 != 0 {
            errors.push("Link CRC error".to_string());
        }
        if err & 0x02 != 0 {
            errors.push("RFB storage error".to_string());
        }
        if err & 0x04 != 0 {
            errors.push("VSC SDP uncorrectable error".to_string());
        }
    }

    Some(PsrInfo {
        psr_version,
        no_train_on_exit,
        setup_time_us,
        y_coord_required,
        su_granularity_required,
        su_aux_frame_sync_not_needed,
        su_x_granularity,
        su_y_granularity,
        psr_enabled,
        psr2_enabled,
        sink_status,
        errors,
    })
}

fn read_psr_driver_status(connector_name: &str) -> Option<String> {
    // Try i915 per-connector debugfs: /sys/kernel/debug/dri/<N>/<connector>/i915_psr_status
    // The card number in debugfs doesn't always match sysfs, so try all
    let debug_dir = Path::new("/sys/kernel/debug/dri");
    let entries = fs::read_dir(debug_dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path().join(connector_name).join("i915_psr_status");
        if let Ok(content) = fs::read_to_string(&path) {
            return Some(content.trim().to_string());
        }
    }

    // Try global i915_edp_psr_status
    let entries = fs::read_dir(debug_dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path().join("i915_edp_psr_status");
        if let Ok(content) = fs::read_to_string(&path) {
            return Some(content.trim().to_string());
        }
    }

    None
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
            psr: None,
            psr_driver_status: None,
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

    let psr = aux_dev.as_ref().and_then(|dev| parse_psr_info(dev));

    // Extract connector name for debugfs lookup
    let connector_name = connector_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let psr_driver_status = read_psr_driver_status(&connector_name);

    DpInfo {
        is_dp: true,
        aux_name,
        dpcd,
        link_config,
        link_status,
        psr,
        psr_driver_status,
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
