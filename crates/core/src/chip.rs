//! Rockchip 芯片编码的读取、显示名称与固件/设备兼容性判断。

/// 从 `rci` 输出的 “Chip Info:” 后读取 4 字节芯片编码。
pub fn chip_code_from_chip_info(output: &str) -> Option<u32> {
    let lower = output.to_ascii_lowercase();
    let start = lower.find("chip info:")? + "chip info:".len();
    let bytes: Vec<u8> = output[start..]
        .split_whitespace()
        .filter_map(|token| u8::from_str_radix(token, 16).ok())
        .take(4)
        .collect();
    (bytes.len() == 4).then(|| u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn modern_chip_number(code: u32) -> Option<String> {
    let bytes = code.to_le_bytes();
    bytes
        .iter()
        .all(u8::is_ascii_digit)
        .then(|| bytes.iter().rev().map(|byte| char::from(*byte)).collect())
}

pub fn chip_display_name(code: u32) -> String {
    if let Some(number) = modern_chip_number(code) {
        return format!("RK{number}");
    }
    match code {
        0x35 => "RK3588 / RK3588S".to_owned(),
        0x38 => "RK3566 / RK3568".to_owned(),
        0x39 => "RK3528".to_owned(),
        0x33 => "RK3399 / RK3399Pro".to_owned(),
        0x30 => "PX30 / RK3326".to_owned(),
        _ => format!("未知 (0x{code:08X})"),
    }
}

/// `Some(false)` 表示确定不匹配；`None` 表示两种编码格式不同、无法自动严格比对。
pub fn chip_compatibility(firmware: u32, device: u32) -> Option<bool> {
    if firmware == device {
        return Some(true);
    }
    Some(modern_chip_number(firmware)? == modern_chip_number(device)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_chip_code_and_compares_formats() {
        let output = "Chip Info:  38 38 35 33 0 0 0 0\r\n";
        let code = chip_code_from_chip_info(output).unwrap();
        assert_eq!(chip_display_name(code), "RK3588");
        assert_eq!(
            chip_compatibility(code, u32::from_le_bytes(*b"8853")),
            Some(true)
        );
        assert_eq!(
            chip_compatibility(code, u32::from_le_bytes(*b"6653")),
            Some(false)
        );
        assert_eq!(chip_compatibility(code, 0x35), None);
    }
}
