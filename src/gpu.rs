use std::fs;
use std::path::Path;

#[allow(dead_code)]
pub struct GpuInfo {
    pub card_name: String,
    pub driver: String,
    pub pci_vendor: String,
    pub pci_device: String,
    pub description: String,
}

fn read_sysfs(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

pub fn get_gpu_info(card_path: &Path) -> Option<GpuInfo> {
    let device_path = card_path.join("device");
    if !device_path.exists() {
        return None;
    }

    let card_name = card_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    // Read driver name from the driver symlink
    let driver = fs::read_link(device_path.join("driver"))
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| "unknown".to_string());

    let pci_vendor = read_sysfs(&device_path.join("vendor")).unwrap_or_default();
    let pci_device = read_sysfs(&device_path.join("device")).unwrap_or_default();

    let description = describe_gpu(&pci_vendor, &driver);

    Some(GpuInfo {
        card_name,
        driver,
        pci_vendor,
        pci_device,
        description,
    })
}

fn describe_gpu(vendor_id: &str, driver: &str) -> String {
    let vendor = match vendor_id {
        "0x8086" => "Intel",
        "0x1002" => "AMD",
        "0x10de" => "NVIDIA",
        _ => "Unknown",
    };

    let driver_desc = match driver {
        "i915" => "Intel i915",
        "xe" => "Intel Xe",
        "amdgpu" => "AMD GPU",
        "radeon" => "AMD Radeon (legacy)",
        "nouveau" => "NVIDIA Nouveau",
        "nvidia" => "NVIDIA (proprietary)",
        other => other,
    };

    format!("{vendor} ({driver_desc})")
}
