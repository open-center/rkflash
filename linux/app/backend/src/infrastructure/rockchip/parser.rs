//! `upgrade_tool` 文本输出的统一解析入口。

use crate::domain::{DeviceInfo, DeviceSnapshot};

pub(super) fn parse_device_list(text: &str) -> DeviceSnapshot {
    let mut devices = Vec::new();
    for line in text.lines().filter(|line| line.contains("DevNo=")) {
        let value = |key: &str| -> String {
            line.split(|character: char| character.is_whitespace() || character == ',')
                .find_map(|part| part.strip_prefix(key).map(str::to_owned))
                .unwrap_or_default()
        };
        devices.push(DeviceInfo {
            number: value("DevNo="),
            vid: value("Vid="),
            pid: value("Pid="),
            location: value("LocationID="),
            mode: value("Mode="),
            serial: value("SerialNo="),
        });
    }
    DeviceSnapshot {
        devices,
        error: None,
    }
}

pub(super) fn extract_progress(line: &str) -> Option<f32> {
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

pub(super) fn extract_failure(line: &str) -> Option<String> {
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
    use super::{extract_failure, extract_progress, parse_device_list};

    #[test]
    fn parses_device_output() {
        let snapshot = parse_device_list(
            "List of rockusb connected(1)\nDevNo=1 Vid=0x2207,Pid=0x330c,LocationID=7143 Mode=Maskrom SerialNo=abc",
        );
        assert_eq!(snapshot.devices.len(), 1);
        assert_eq!(snapshot.devices[0].mode, "Maskrom");
        assert_eq!(snapshot.devices[0].location, "7143");
    }

    #[test]
    fn parses_percentage() {
        assert_eq!(extract_progress("Download Image... (12%)"), Some(0.12));
        assert_eq!(extract_progress("no progress"), None);
    }

    #[test]
    fn recognizes_upgrade_tool_failures_even_when_exit_status_is_zero() {
        assert_eq!(
            extract_failure("Read flash ID Fail!"),
            Some("Read flash ID Fail!".to_owned())
        );
        assert!(extract_failure("ERROR:ReadCapability failed,err=-2").is_some());
        assert!(extract_failure("Read Flash Info Success").is_none());
    }
}
