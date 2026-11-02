//! 分区表文件（GPT 镜像、Rockchip parameter）的内容识别与 parameter 分区解析。

use std::fs::File;
use std::io::Read;
use std::path::Path;

const METADATA_PROBE_LIMIT: u64 = 512 * 1024 + 12;

/// 分区元数据文件的内容格式；决定使用 `gpt` 还是 `prm` 写入。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartitionMetadata {
    GptImage,
    GptParameter,
    LegacyParameter,
    UnknownParameter,
    Other,
}

impl PartitionMetadata {
    pub fn inspect(path: &Path) -> Result<Self, String> {
        let file =
            File::open(path).map_err(|error| format!("无法读取 {}：{error}", path.display()))?;
        let mut data = Vec::new();
        file.take(METADATA_PROBE_LIMIT)
            .read_to_end(&mut data)
            .map_err(|error| format!("无法读取 {}：{error}", path.display()))?;
        Ok(Self::inspect_bytes(&data))
    }

    pub fn inspect_bytes(data: &[u8]) -> Self {
        if data.len() >= 520 && &data[512..520] == b"EFI PART" {
            return Self::GptImage;
        }
        let mut body = data;
        let mut wrapped = false;
        if data.len() >= 8 && &data[..4] == b"PARM" {
            wrapped = true;
            let length = le32(data, 4) as usize;
            if 8 + length > data.len() {
                return Self::UnknownParameter;
            }
            body = &data[8..8 + length];
        }
        let Ok(text) = std::str::from_utf8(body) else {
            return if wrapped {
                Self::UnknownParameter
            } else {
                Self::Other
            };
        };
        match parameter_layout(text) {
            Some(true) => Self::GptParameter,
            Some(false) => Self::LegacyParameter,
            None if wrapped || looks_like_parameter(text) => Self::UnknownParameter,
            None => Self::Other,
        }
    }

    pub fn is_partition_table(self) -> bool {
        self != Self::Other
    }
}

/// `Some(true)` 为 GPT 布局，`Some(false)` 为传统 mtdparts 布局，`None` 为无法判断。
fn parameter_layout(text: &str) -> Option<bool> {
    let mut has_mtdparts = false;
    for line in text.lines().map(str::trim) {
        if line.starts_with('#') {
            continue;
        }
        let Some((key, raw_value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim();
        let raw_value = raw_value.trim();
        if key.eq_ignore_ascii_case("TYPE") {
            let value = raw_value.split_whitespace().next().unwrap_or_default();
            return if value.eq_ignore_ascii_case("GPT") {
                Some(true)
            } else if value.eq_ignore_ascii_case("MTD") {
                Some(false)
            } else {
                None
            };
        }
        if key.eq_ignore_ascii_case("CMDLINE")
            && raw_value.to_ascii_lowercase().contains("mtdparts=")
        {
            has_mtdparts = true;
        }
    }
    has_mtdparts.then_some(false)
}

fn looks_like_parameter(text: &str) -> bool {
    const KEYS: &[&str] = &[
        "FIRMWARE_VER",
        "MACHINE_MODEL",
        "MACHINE_ID",
        "MANUFACTURER",
        "MAGIC",
        "ATAG",
        "MACHINE",
        "CHECK_MASK",
        "PWR_HLD",
        "TYPE",
        "CMDLINE",
    ];
    text.lines().map(str::trim).any(|line| {
        !line.starts_with('#')
            && line
                .split_once(':')
                .is_some_and(|(key, _)| KEYS.contains(&key.trim().to_ascii_uppercase().as_str()))
    })
}

/// Rockchip 传统固件布局中的标准分区（名称, 起始扇区）；下载列表默认展示这些条目，
/// 烧录工具也按这些名称识别可以直接按分区名写入的镜像。
pub const STANDARD_PARTITIONS: &[(&str, &str)] = &[
    ("parameter", "0x00000000"),
    ("uboot", "0x00004000"),
    ("trust", "0x00006000"),
    ("misc", "0x00008000"),
    ("resource", "0x0000a000"),
    ("kernel", "0x00010000"),
    ("boot", "0x00020000"),
    ("recovery", "0x00040000"),
    ("system", "0x00080000"),
];

/// 是否为标准分区名（不区分大小写，忽略首尾空白）。
pub fn is_standard_partition(name: &str) -> bool {
    let name = name.trim();
    STANDARD_PARTITIONS
        .iter()
        .any(|(standard, _)| standard.eq_ignore_ascii_case(name))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterPartition {
    pub name: String,
    pub address: String,
}

/// 从 parameter 的 `mtdparts=` 定义解析分区名与起始扇区（保留原始文本格式）。
pub fn parse_parameter_partitions(text: &str) -> Result<Vec<ParameterPartition>, String> {
    let marker = "mtdparts=";
    let start = text
        .find(marker)
        .ok_or_else(|| "未找到 mtdparts 分区定义".to_owned())?;
    let definition = &text[start + marker.len()..];
    let definition = definition
        .split_whitespace()
        .next()
        .unwrap_or(definition)
        .trim_matches(|character| character == '\'' || character == '"');
    let partitions = definition
        .split_once(':')
        .map(|(_, value)| value)
        .unwrap_or(definition);

    let mut entries = Vec::new();
    for item in partitions.split(',') {
        let item = item.trim();
        let open = item
            .find('(')
            .ok_or_else(|| format!("无法解析分区项：{item}"))?;
        let close = item[open + 1..]
            .find(')')
            .map(|index| index + open + 1)
            .ok_or_else(|| format!("分区名缺少右括号：{item}"))?;
        let name = item[open + 1..close].trim();
        let before_name = &item[..open];
        let address = before_name
            .split_once('@')
            .map(|(_, value)| value.trim())
            .ok_or_else(|| format!("分区缺少起始地址：{item}"))?;
        if name.is_empty() || address.is_empty() {
            return Err(format!("分区名或地址为空：{item}"));
        }
        entries.push(ParameterPartition {
            name: name.to_owned(),
            address: address.to_owned(),
        });
    }

    if entries.is_empty() {
        Err("分区表为空".to_owned())
    } else {
        Ok(entries)
    }
}

fn le32(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_partition_metadata_by_content() {
        let mut gpt = vec![0u8; 1024];
        gpt[512..520].copy_from_slice(b"EFI PART");
        assert_eq!(
            PartitionMetadata::inspect_bytes(&gpt),
            PartitionMetadata::GptImage
        );
        assert_eq!(
            PartitionMetadata::inspect_bytes(
                b"CMDLINE: console=ttyS0 mtdparts=rk29xxnand:0x2000@0x4000(uboot)"
            ),
            PartitionMetadata::LegacyParameter
        );
        assert_eq!(
            PartitionMetadata::inspect_bytes(b"MACHINE_MODEL: test\n"),
            PartitionMetadata::UnknownParameter
        );
        assert_eq!(
            PartitionMetadata::inspect_bytes(b"\x7fELF binary"),
            PartitionMetadata::Other
        );
    }

    #[test]
    fn parses_rockchip_parameter_layout() {
        let text =
            "CMDLINE:mtdparts=rk29xxnand:0x00002000@0x00002000(uboot),-@0x00004000(rootfs:grow)";
        let result = parse_parameter_partitions(text).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "uboot");
        assert_eq!(result[0].address, "0x00002000");
        assert_eq!(result[1].name, "rootfs:grow");
    }
}
