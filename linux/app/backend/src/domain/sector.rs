//! 扇区地址与数量的解析校验；由应用校验层与计划构建层共用，
//! 确保下发到烧录工具的值都经过同一套解析。

/// 校验十进制或 `0x` 十六进制扇区值，返回去除首尾空白后的原文。
///
/// 破坏性读写命令的扇区参数必须先经过这里：既拒绝非法输入，
/// 也保证工具收到的是校验过的规范文本而不是用户输入的原始串。
pub fn normalize_sector(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    let parsed = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
        .map(|digits| u64::from_str_radix(digits, 16))
        .unwrap_or_else(|| trimmed.parse::<u64>());
    parsed
        .map(|_| trimmed.to_owned())
        .map_err(|_| format!("必须是十进制数或 0x 开头的十六进制数：{trimmed}"))
}

#[cfg(test)]
mod tests {
    use super::normalize_sector;

    #[test]
    fn accepts_decimal_and_hex_and_trims_whitespace() {
        assert_eq!(normalize_sector("8192").as_deref(), Ok("8192"));
        assert_eq!(
            normalize_sector(" 0x00002000 ").as_deref(),
            Ok("0x00002000")
        );
    }

    #[test]
    fn rejects_garbage_input() {
        assert!(normalize_sector("0xGG").is_err());
        assert!(normalize_sector("-1").is_err());
        assert!(normalize_sector("").is_err());
        assert!(normalize_sector("8192;rm -rf /").is_err());
    }
}
