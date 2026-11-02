//! `upgrade_tool` 设备列表解析；进度与失败识别位于 rkflash-core。

use rkflash_application::domain::{DeviceInfo, DeviceSnapshot};

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

#[cfg(test)]
mod tests {
    use super::parse_device_list;

    #[test]
    fn parses_device_output() {
        let snapshot = parse_device_list(
            "List of rockusb connected(1)\nDevNo=1 Vid=0x2207,Pid=0x330c,LocationID=7143 Mode=Maskrom SerialNo=abc",
        );
        assert_eq!(snapshot.devices.len(), 1);
        assert_eq!(snapshot.devices[0].mode, "Maskrom");
        assert_eq!(snapshot.devices[0].location, "7143");
    }
}
