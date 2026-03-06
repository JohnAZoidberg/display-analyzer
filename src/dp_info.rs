use std::fs;
use std::path::Path;

pub struct DpInfo {
    pub is_dp: bool,
    pub link_rate: Option<String>,
    pub lane_count: Option<String>,
}

pub fn get_dp_info(connector_type: &str, connector_path: &Path) -> DpInfo {
    let is_dp = matches!(connector_type, "DP" | "eDP");

    if !is_dp {
        return DpInfo {
            is_dp: false,
            link_rate: None,
            lane_count: None,
        };
    }

    // Try to read DP AUX info from i915 debugfs or sysfs
    // The exact path varies by driver, try common patterns
    let link_rate = try_read_dp_attr(connector_path, "link_rate");
    let lane_count = try_read_dp_attr(connector_path, "lane_count");

    DpInfo {
        is_dp: true,
        link_rate,
        lane_count,
    }
}

fn try_read_dp_attr(connector_path: &Path, attr: &str) -> Option<String> {
    // Check for DP attributes in the connector sysfs directory
    let path = connector_path.join(attr);
    fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}
