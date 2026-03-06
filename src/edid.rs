/// Parsed EDID information extracted from raw EDID bytes.
#[allow(dead_code)]
pub struct EdidInfo {
    pub manufacturer: String,
    pub product_code: u16,
    pub display_name: Option<String>,
    pub serial_string: Option<String>,
    pub serial_number: u32,
    pub year: u16,
    pub week: u8,
    pub version: String,
    pub width_cm: u8,
    pub height_cm: u8,
    pub max_width: Option<u16>,
    pub max_height: Option<u16>,
    pub bit_depth: Option<u8>,
    pub digital: bool,
}

fn decode_manufacturer(bytes: &[u8]) -> String {
    let raw = ((bytes[8] as u16) << 8) | bytes[9] as u16;
    let c1 = ((raw >> 10) & 0x1f) as u8 + b'A' - 1;
    let c2 = ((raw >> 5) & 0x1f) as u8 + b'A' - 1;
    let c3 = (raw & 0x1f) as u8 + b'A' - 1;
    String::from_utf8_lossy(&[c1, c2, c3]).to_string()
}

fn find_descriptor_string(bytes: &[u8], tag: u8) -> Option<String> {
    // EDID has 4 descriptor blocks starting at offset 54, each 18 bytes
    for i in 0..4 {
        let base = 54 + i * 18;
        if base + 18 > bytes.len() {
            break;
        }
        // Descriptor header: bytes 0-1 = 0x0000, byte 3 = tag
        if bytes[base] == 0 && bytes[base + 1] == 0 && bytes[base + 3] == tag {
            let text = &bytes[base + 5..base + 18];
            let s: String = text
                .iter()
                .map(|&b| b as char)
                .take_while(|&c| c != '\n' && c != '\0')
                .collect();
            return Some(s.trim().to_string());
        }
    }
    None
}

pub fn parse_edid(raw: &[u8]) -> Option<EdidInfo> {
    // Minimum EDID size is 128 bytes
    if raw.len() < 128 {
        return None;
    }

    // Check EDID header magic
    let header = [0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00];
    if raw[..8] != header {
        return None;
    }

    let manufacturer = decode_manufacturer(raw);
    let product_code = u16::from_le_bytes([raw[10], raw[11]]);
    let serial_number = u32::from_le_bytes([raw[12], raw[13], raw[14], raw[15]]);
    let week = raw[16];
    let year = raw[17] as u16 + 1990;
    let version = format!("{}.{}", raw[18], raw[19]);

    // Video input definition (byte 20)
    let digital = raw[20] & 0x80 != 0;
    let bit_depth = if digital {
        match (raw[20] >> 4) & 0x07 {
            1 => Some(6),
            2 => Some(8),
            3 => Some(10),
            4 => Some(12),
            5 => Some(14),
            6 => Some(16),
            _ => None,
        }
    } else {
        None
    };

    let width_cm = raw[21];
    let height_cm = raw[22];

    let display_name = find_descriptor_string(raw, 0xFC);
    let serial_string = find_descriptor_string(raw, 0xFF);

    // Try to find preferred resolution from the first detailed timing descriptor
    let (max_width, max_height) = parse_first_detailed_timing(raw);

    Some(EdidInfo {
        manufacturer,
        product_code,
        display_name,
        serial_string,
        serial_number,
        year,
        week,
        version,
        width_cm,
        height_cm,
        max_width,
        max_height,
        bit_depth,
        digital,
    })
}

fn parse_first_detailed_timing(raw: &[u8]) -> (Option<u16>, Option<u16>) {
    // First detailed timing descriptor is at offset 54
    if raw.len() < 72 {
        return (None, None);
    }

    let base = 54;
    // If first two bytes are zero, this is a descriptor, not timing
    if raw[base] == 0 && raw[base + 1] == 0 {
        return (None, None);
    }

    let h_active = raw[base + 2] as u16 | ((raw[base + 4] as u16 & 0xF0) << 4);
    let v_active = raw[base + 5] as u16 | ((raw[base + 7] as u16 & 0xF0) << 4);

    (Some(h_active), Some(v_active))
}
