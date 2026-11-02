//! Rockchip 烧录工具（upgrade_tool、rkdeveloptool）文本输出的通用识别规则。

/// 从包含 `NN%` 的输出行读取 0–1 的进度。
pub fn extract_progress(line: &str) -> Option<f32> {
    let percent = line.find('%')?;
    let before = &line[..percent];
    let digits_reversed: String = before
        .chars()
        .rev()
        .skip_while(|character| character.is_whitespace())
        .take_while(|character| character.is_ascii_digit() || *character == '.')
        .collect();
    if digits_reversed.is_empty() {
        return None;
    }
    let value: f32 = digits_reversed
        .chars()
        .rev()
        .collect::<String>()
        .parse()
        .ok()?;
    Some((value / 100.0).clamp(0.0, 1.0))
}

/// 识别 Rockchip 工具在输出中报告的失败（即使退出码为 0）。
pub fn extract_failure(line: &str) -> Option<String> {
    let message = line.trim();
    if message.is_empty() {
        return None;
    }

    let lower = message.to_ascii_lowercase();
    let reports_failure = lower.contains("fail")
        || lower.starts_with("error")
        || lower.contains(" error:")
        || lower.contains("not found any device")
        || lower.contains("no device found");
    reports_failure.then(|| message.to_owned())
}

#[cfg(test)]
mod tests {
    use super::{extract_failure, extract_progress};

    #[test]
    fn parses_percentage() {
        assert_eq!(extract_progress("Download Image... (12%)"), Some(0.12));
        assert_eq!(extract_progress("no progress"), None);
    }

    #[test]
    fn recognizes_tool_failures_even_when_exit_status_is_zero() {
        assert_eq!(
            extract_failure("Read flash ID Fail!"),
            Some("Read flash ID Fail!".to_owned())
        );
        assert!(extract_failure("ERROR:ReadCapability failed,err=-2").is_some());
        assert!(extract_failure("Read Flash Info Success").is_none());
    }
}
