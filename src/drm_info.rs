use std::fs;
use std::path::{Path, PathBuf};

pub struct ConnectorInfo {
    pub name: String,
    pub connector_type: String,
    pub status: String,
    pub enabled: bool,
    pub dpms: String,
    pub modes: Vec<String>,
    pub edid_raw: Option<Vec<u8>>,
    pub card_path: PathBuf,
}

fn read_sysfs(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

fn parse_connector_type(name: &str) -> String {
    // Connector names look like "card1-eDP-1", "card1-DP-2", "card1-HDMI-A-1"
    // Strip the "cardN-" prefix to get the type + index
    if let Some(rest) = name.split_once('-') {
        let rest = rest.1;
        // Strip trailing "-N" index to get the type
        if let Some(pos) = rest.rfind('-') {
            let maybe_type = &rest[..pos];
            let maybe_idx = &rest[pos + 1..];
            if maybe_idx.chars().all(|c| c.is_ascii_digit()) {
                return maybe_type.to_string();
            }
        }
        rest.to_string()
    } else {
        name.to_string()
    }
}

pub fn enumerate_connectors() -> Vec<ConnectorInfo> {
    let drm_dir = Path::new("/sys/class/drm");
    let mut connectors = Vec::new();

    let entries = match fs::read_dir(drm_dir) {
        Ok(e) => e,
        Err(_) => return connectors,
    };

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();

        // Skip entries that aren't connectors (e.g. "card1", "renderD128", "version")
        if !name.contains('-') {
            continue;
        }

        let path = entry.path();

        // Must have a "status" file to be a connector
        let status = match read_sysfs(&path.join("status")) {
            Some(s) => s,
            None => continue,
        };

        let enabled = read_sysfs(&path.join("enabled"))
            .map(|s| s == "enabled")
            .unwrap_or(false);

        let dpms = read_sysfs(&path.join("dpms")).unwrap_or_default();

        let modes: Vec<String> = read_sysfs(&path.join("modes"))
            .map(|s| {
                s.lines()
                    .map(|l| l.trim().to_string())
                    .filter(|l| !l.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        let edid_raw = fs::read(path.join("edid")).ok().filter(|v| !v.is_empty());

        // Derive the card path (e.g. /sys/class/drm/card1)
        let card_name = name.split('-').next().unwrap_or(&name);
        let card_path = drm_dir.join(card_name);

        let connector_type = parse_connector_type(&name);

        connectors.push(ConnectorInfo {
            name,
            connector_type,
            status,
            enabled,
            dpms,
            modes,
            edid_raw,
            card_path,
        });
    }

    connectors.sort_by(|a, b| a.name.cmp(&b.name));
    connectors
}
