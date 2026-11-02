//! 开源 rkdeveloptool 的命令约定与输出解析（macOS 与 Windows 版共用的烧录核心）。

use crate::output::extract_failure;

/// `ld` 列出的一台 Rockchip USB 设备。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsbDevice {
    pub number: String,
    pub vid: String,
    pub pid: String,
    pub location: String,
    /// `Maskrom`、`Loader` 或 `MSC`。
    pub mode: String,
}

impl UsbDevice {
    pub fn is_mode(&self, mode: &str) -> bool {
        self.mode.eq_ignore_ascii_case(mode)
    }
}

/// 写入前用 `cs` 选择的目标存储。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Storage {
    /// 不切换，使用 Loader 默认存储。
    Auto,
    Emmc,
    SdCard,
    SpiNor,
}

impl Storage {
    /// `cs` 命令的存储编号：1=eMMC，2=SD，9=SPI NOR。
    pub fn code(self) -> Option<&'static str> {
        match self {
            Self::Auto => None,
            Self::Emmc => Some("1"),
            Self::SdCard => Some("2"),
            Self::SpiNor => Some("9"),
        }
    }
}

/// 解析 `DevNo=1\tVid=0x2207,Pid=0x350b,LocationID=104\tMaskrom`。
pub fn parse_device_list(text: &str) -> Vec<UsbDevice> {
    text.lines()
        .filter_map(|line| {
            let rest = line.trim().strip_prefix("DevNo=")?;
            let fields: Vec<&str> = rest
                .split(|character: char| character.is_whitespace() || character == ',')
                .filter(|field| !field.is_empty())
                .collect();
            let value = |key: &str| {
                fields
                    .iter()
                    .find_map(|field| field.strip_prefix(key))
                    .unwrap_or_default()
                    .to_owned()
            };
            Some(UsbDevice {
                number: fields.first()?.to_string(),
                vid: value("Vid="),
                pid: value("Pid="),
                location: value("LocationID="),
                mode: fields
                    .last()
                    .filter(|field| !field.contains('='))?
                    .to_string(),
            })
        })
        .collect()
}

/// rkdeveloptool 的失败输出：通用规则之外，还包括不支持的操作与无效命令。
pub fn parse_failure(line: &str) -> Option<String> {
    let lower = line.to_ascii_lowercase();
    if lower.contains("does not support") || lower.contains("command is invalid") {
        return Some(line.trim().to_owned());
    }
    extract_failure(line)
}

/// 用回车原地刷新的读写进度行；用于更新进度条，不写入任务日志（100% 那一行保留）。
pub fn is_transient(line: &str) -> bool {
    (line.starts_with("Write LBA from file (") || line.starts_with("Read LBA to file ("))
        && !line.contains("(100%)")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_device_list() {
        let devices = parse_device_list(
            "DevNo=1\tVid=0x2207,Pid=0x350b,LocationID=104\tMaskrom\r\nDevNo=2\tVid=0x2207,Pid=0x330c,LocationID=203\tLoader\r\n",
        );
        assert_eq!(devices.len(), 2);
        assert_eq!(devices[0].number, "1");
        assert_eq!(devices[0].pid, "0x350b");
        assert!(devices[0].is_mode("maskrom"));
        assert_eq!(devices[1].location, "203");
        assert!(devices[1].is_mode("Loader"));
        assert!(parse_device_list("Not found any devices!\r\n").is_empty());
    }

    #[test]
    fn progress_refresh_lines_stay_out_of_the_log() {
        assert!(is_transient("Write LBA from file (45%)"));
        assert!(!is_transient("Write LBA from file (100%)"));
        assert!(parse_failure("The device does not support this operation!").is_some());
    }
}
