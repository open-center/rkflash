//! Rockchip RKFW/RKAF 统一固件（update.img）解析与安全解包。
//!
//! 与 macOS 版 RKFirmwareImage.swift 保持同一套格式规则：只依据文件头 magic、
//! 偏移表和 payload 内容判断格式，所有偏移和长度先做越界检查再读取。

use super::md5::Md5;
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

const RKFW_HEADER_SIZE: u64 = 0x66;
const RKAF_TABLE_OFFSET: u64 = 0x8c;
const RKAF_ENTRY_SIZE: u64 = 0x70;
const MAXIMUM_PARTS: u32 = 16;
const COPY_CHUNK: usize = 4 * 1024 * 1024;
const METADATA_PROBE_LIMIT: u64 = 512 * 1024 + 12;

/// 读取大文件时的进度回调（已处理字节数, 总字节数）与取消检查。
pub(crate) struct Progress<'a> {
    pub report: &'a mut dyn FnMut(u64, u64),
    pub cancelled: &'a dyn Fn() -> bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Container {
    Rkfw,
    Rkaf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Md5Status {
    Valid,
    Invalid,
    NotPresent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FirmwarePartition {
    pub name: String,
    pub filename: String,
    pub data_offset: u64,
    pub size: u64,
    pub nand_address: u64,
}

impl FirmwarePartition {
    fn is_reserved(&self) -> bool {
        self.filename.eq_ignore_ascii_case("RESERVED")
    }

    fn is_package_file(&self) -> bool {
        self.name.eq_ignore_ascii_case("package-file")
    }

    fn is_loader_payload(&self) -> bool {
        let name = self.name.trim().to_ascii_lowercase();
        let file = self
            .filename
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        ["loader", "bootloader", "miniloader", "idbloader"].contains(&name.as_str())
            || file.contains("miniloader")
            || file == "loader.bin"
    }
}

#[derive(Debug, Clone)]
pub(crate) struct FirmwareInfo {
    pub source: PathBuf,
    pub container: Container,
    pub firmware_version: Option<String>,
    pub loader_version: Option<String>,
    pub chip_code: Option<u32>,
    pub model: String,
    loader_extent: Option<(u64, u64)>,
    rkaf_offset: u64,
    rkaf_size: u64,
    file_size: u64,
    pub partitions: Vec<FirmwarePartition>,
}

impl FirmwareInfo {
    pub(crate) fn chip_name(&self) -> String {
        self.chip_code
            .map(chip_display_name)
            .unwrap_or_else(|| "未知".to_owned())
    }
}

/// 解包后待烧录的分区：按固件清单中的地址写入。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FlashItem {
    pub partition_name: String,
    pub address: u64,
    pub image_path: PathBuf,
}

#[derive(Debug, Clone)]
pub(crate) struct Extraction {
    pub loader_path: Option<PathBuf>,
    pub items: Vec<FlashItem>,
}

pub(crate) fn inspect(path: &Path) -> Result<FirmwareInfo, String> {
    let mut file = File::open(path).map_err(|error| format!("无法打开固件文件：{error}"))?;
    let file_size = file
        .metadata()
        .map_err(|error| format!("无法读取固件大小：{error}"))?
        .len();
    let magic = read_exact_at(&mut file, 0, 4, "文件头")?;

    let mut firmware_version = None;
    let mut loader_version = None;
    let mut chip_code = None;
    let mut loader_extent = None;
    let (container, rkaf_offset, rkaf_size) = if magic == b"RKFW" {
        let header = read_exact_at(&mut file, 0, RKFW_HEADER_SIZE as usize, "RKFW 头")?;
        if u64::from(le16(&header, 0x04)) < RKFW_HEADER_SIZE {
            return Err("固件数据不完整：RKFW 头长度。".to_owned());
        }
        firmware_version = Some(version_string(le32(&header, 0x06)));
        chip_code = Some(le32(&header, 0x15));
        let loader_offset = u64::from(le32(&header, 0x19));
        let loader_size = u64::from(le32(&header, 0x1d));
        let rkaf_offset = u64::from(le32(&header, 0x21));
        let low_image_size = u64::from(le32(&header, 0x25));
        let rkaf_size = if header[0x37] == 0x48 && header[0x38] == 0x49 {
            (u64::from(le32(&header, 0x39)) << 32) | low_image_size
        } else {
            low_image_size
        };
        if !extent_is_valid(loader_offset, loader_size, file_size) {
            return Err("固件条目 Loader 的偏移或长度越界。".to_owned());
        }
        if !extent_is_valid(rkaf_offset, rkaf_size, file_size) {
            return Err("固件条目 RKAF 的偏移或长度越界。".to_owned());
        }
        loader_version = inspect_loader_version(&mut file, loader_offset, loader_size)?;
        loader_extent = Some((loader_offset, loader_size));
        (Container::Rkfw, rkaf_offset, rkaf_size)
    } else if magic == b"RKAF" {
        (Container::Rkaf, 0, file_size)
    } else {
        return Err("不是受支持的 Rockchip RKFW/RKAF 固件。".to_owned());
    };

    let prefix = read_exact_at(
        &mut file,
        rkaf_offset,
        RKAF_TABLE_OFFSET as usize,
        "RKAF 头",
    )?;
    if &prefix[..4] != b"RKAF" {
        return Err("不是受支持的 Rockchip RKFW/RKAF 固件。".to_owned());
    }
    let part_count = le32(&prefix, 0x88);
    if part_count > MAXIMUM_PARTS {
        return Err(format!(
            "RKAF 分区数量 {part_count} 超出当前格式上限 {MAXIMUM_PARTS}。"
        ));
    }
    let table_size = u64::from(part_count) * RKAF_ENTRY_SIZE;
    if !extent_is_valid(RKAF_TABLE_OFFSET, table_size, rkaf_size) {
        return Err("固件数据不完整：RKAF 分区表。".to_owned());
    }
    let table = read_exact_at(
        &mut file,
        rkaf_offset + RKAF_TABLE_OFFSET,
        table_size as usize,
        "RKAF 分区表",
    )?;

    let mut partitions = Vec::with_capacity(part_count as usize);
    for index in 0..part_count as usize {
        let base = index * RKAF_ENTRY_SIZE as usize;
        let name = c_string(&table, base, 32);
        let filename = c_string(&table, base + 0x20, 60);
        let position = u64::from(le32(&table, base + 0x60));
        let nand_address = u64::from(le32(&table, base + 0x64));
        let size = u64::from(le32(&table, base + 0x6c));
        let no_payload =
            position == u64::from(u32::MAX) || size == u64::from(u32::MAX) || size == 0;
        if !no_payload && !extent_is_valid(position, size, rkaf_size) {
            let label = if name.is_empty() {
                format!("#{}", index + 1)
            } else {
                name.clone()
            };
            return Err(format!("固件条目 {label} 的偏移或长度越界。"));
        }
        partitions.push(FirmwarePartition {
            name: if name.is_empty() {
                format!("part-{}", index + 1)
            } else {
                name
            },
            filename,
            data_offset: if no_payload {
                0
            } else {
                rkaf_offset + position
            },
            size: if no_payload { 0 } else { size },
            nand_address,
        });
    }

    Ok(FirmwareInfo {
        source: path.to_path_buf(),
        container,
        firmware_version,
        loader_version,
        chip_code,
        model: c_string(&prefix, 0x08, 0x22),
        loader_extent,
        rkaf_offset,
        rkaf_size,
        file_size,
        partitions,
    })
}

/// 校验 RKFW 尾部附带的 MD5；需要读取整个固件，只能在后台线程调用。
pub(crate) fn verify_md5(info: &FirmwareInfo, progress: Progress<'_>) -> Result<Md5Status, String> {
    if info.container != Container::Rkfw {
        return Ok(Md5Status::NotPresent);
    }
    let mut file =
        File::open(&info.source).map_err(|error| format!("无法打开固件文件：{error}"))?;
    let mut candidates = Vec::new();
    let preferred = info.rkaf_offset + info.rkaf_size;
    if extent_is_valid(preferred, 32, info.file_size) {
        candidates.push(preferred);
    }
    if info.file_size >= 32 && !candidates.contains(&(info.file_size - 32)) {
        candidates.push(info.file_size - 32);
    }

    let Progress { report, cancelled } = progress;
    let mut found_tail = false;
    for tail_offset in candidates {
        let tail = read_exact_at(&mut file, tail_offset, 32, "MD5")?;
        let Ok(expected) = std::str::from_utf8(&tail) else {
            continue;
        };
        if !expected.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            continue;
        }
        found_tail = true;
        file.seek(SeekFrom::Start(0))
            .map_err(|error| format!("读取固件失败：{error}"))?;
        let mut hasher = Md5::new();
        let mut buffer = vec![0u8; COPY_CHUNK];
        let mut remaining = tail_offset;
        while remaining > 0 {
            if cancelled() {
                return Err("操作已由用户中止".to_owned());
            }
            let count = remaining.min(COPY_CHUNK as u64) as usize;
            file.read_exact(&mut buffer[..count])
                .map_err(|_| "固件数据不完整：MD5 数据。".to_owned())?;
            hasher.update(&buffer[..count]);
            remaining -= count as u64;
            report(tail_offset - remaining, tail_offset);
        }
        if hasher.hex_digest().eq_ignore_ascii_case(expected) {
            return Ok(Md5Status::Valid);
        }
    }
    Ok(if found_tail {
        Md5Status::Invalid
    } else {
        Md5Status::NotPresent
    })
}

/// 把包内 Loader 与各分区解包到 `directory`；重复引用同一 payload 的分区只写一次。
pub(crate) fn extract(
    info: &FirmwareInfo,
    directory: &Path,
    progress: Progress<'_>,
) -> Result<Extraction, String> {
    fs::create_dir_all(directory)
        .map_err(|error| format!("无法创建解包目录 {}：{error}", directory.display()))?;
    let mut source =
        File::open(&info.source).map_err(|error| format!("无法打开固件文件：{error}"))?;
    let Progress { report, cancelled } = progress;

    let total: u64 = info.loader_extent.map_or(0, |(_, size)| size)
        + info
            .partitions
            .iter()
            .filter(|partition| !partition.is_reserved() && !partition.is_package_file())
            .map(|partition| partition.size)
            .sum::<u64>();
    let mut done = 0u64;
    let mut copy = |source: &mut File, offset: u64, size: u64, destination: &Path| {
        copy_range(
            source,
            offset,
            size,
            destination,
            &mut |copied| {
                report(done + copied, total);
            },
            cancelled,
        )?;
        done += size;
        Ok::<(), String>(())
    };

    let mut loader_path = None;
    if let Some((offset, size)) = info.loader_extent.filter(|(_, size)| *size > 0) {
        let destination = directory.join("MiniLoaderAll.bin");
        copy(&mut source, offset, size, &destination)?;
        loader_path = Some(destination);
    }

    let mut used_names = HashSet::new();
    let mut extracted: HashMap<(u64, u64), PathBuf> = HashMap::new();
    let mut items = Vec::new();
    for partition in &info.partitions {
        if partition.is_reserved() || partition.size == 0 || partition.is_package_file() {
            continue;
        }
        if partition.is_loader_payload() && loader_path.is_some() {
            continue;
        }
        let key = (partition.data_offset, partition.size);
        let destination = match extracted.get(&key) {
            Some(existing) => existing.clone(),
            None => {
                let name = unique_filename(safe_output_name(partition)?, &mut used_names);
                let destination = directory.join(name);
                copy(
                    &mut source,
                    partition.data_offset,
                    partition.size,
                    &destination,
                )?;
                unwrap_parameter_if_needed(&destination)?;
                extracted.insert(key, destination.clone());
                destination
            }
        };
        if partition.is_loader_payload() {
            loader_path.get_or_insert(destination);
        } else {
            items.push(FlashItem {
                partition_name: partition.name.clone(),
                address: partition.nand_address,
                image_path: destination,
            });
        }
    }
    Ok(Extraction { loader_path, items })
}

/// 只解包固件内置的 Loader（擦除 Flash 前初始化 Maskrom 所需）。
pub(crate) fn extract_loader(info: &FirmwareInfo, directory: &Path) -> Result<PathBuf, String> {
    let (offset, size) = info
        .loader_extent
        .filter(|(_, size)| *size > 0)
        .or_else(|| {
            info.partitions
                .iter()
                .find(|partition| partition.is_loader_payload() && partition.size > 0)
                .map(|partition| (partition.data_offset, partition.size))
        })
        .ok_or("固件包内没有可用的 Loader")?;
    fs::create_dir_all(directory)
        .map_err(|error| format!("无法创建解包目录 {}：{error}", directory.display()))?;
    let mut source =
        File::open(&info.source).map_err(|error| format!("无法打开固件文件：{error}"))?;
    let destination = directory.join("MiniLoaderAll.bin");
    copy_range(
        &mut source,
        offset,
        size,
        &destination,
        &mut |_| {},
        &|| false,
    )?;
    Ok(destination)
}

/// 分区元数据文件的内容格式；决定使用 `gpt` 还是 `prm` 写入。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PartitionMetadata {
    GptImage,
    GptParameter,
    LegacyParameter,
    UnknownParameter,
    Other,
}

impl PartitionMetadata {
    pub(crate) fn inspect(path: &Path) -> Result<Self, String> {
        let file =
            File::open(path).map_err(|error| format!("无法读取 {}：{error}", path.display()))?;
        let mut data = Vec::new();
        file.take(METADATA_PROBE_LIMIT)
            .read_to_end(&mut data)
            .map_err(|error| format!("无法读取 {}：{error}", path.display()))?;
        Ok(Self::inspect_bytes(&data))
    }

    pub(crate) fn inspect_bytes(data: &[u8]) -> Self {
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

    pub(crate) fn is_partition_table(self) -> bool {
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

/// 从 `rci` 输出的 “Chip Info:” 后读取 4 字节芯片编码。
pub(crate) fn chip_code_from_chip_info(output: &str) -> Option<u32> {
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

pub(crate) fn chip_display_name(code: u32) -> String {
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
pub(crate) fn chip_compatibility(firmware: u32, device: u32) -> Option<bool> {
    if firmware == device {
        return Some(true);
    }
    Some(modern_chip_number(firmware)? == modern_chip_number(device)?)
}

fn inspect_loader_version(
    file: &mut File,
    offset: u64,
    size: u64,
) -> Result<Option<String>, String> {
    if size < 14 {
        return Ok(None);
    }
    let header = read_exact_at(file, offset, 14, "Loader 头")?;
    if &header[..4] != b"BOOT" && &header[..4] != b"LDR " {
        return Ok(None);
    }
    let raw = le32(&header, 0x06);
    Ok(Some(if raw <= u32::from(u16::MAX) {
        format!("{}.{}", (raw >> 8) & 0xff, raw & 0xff)
    } else {
        version_string(raw)
    }))
}

fn version_string(raw: u32) -> String {
    format!(
        "{}.{}.{}",
        (raw >> 24) & 0xff,
        (raw >> 16) & 0xff,
        raw & 0xffff
    )
}

fn safe_output_name(partition: &FirmwarePartition) -> Result<String, String> {
    let raw = if partition.filename.is_empty() {
        format!("{}.img", partition.name)
    } else {
        partition.filename.clone()
    };
    let normalized = raw.replace('\\', "/");
    let components: Vec<&str> = normalized
        .split('/')
        .filter(|part| !part.is_empty())
        .collect();
    let unsafe_name = || format!("固件条目包含不安全的文件名：{raw}");
    if normalized.starts_with('/') || components.contains(&"..") || normalized.contains(':') {
        return Err(unsafe_name());
    }
    match components.last() {
        Some(&name) if name != "." && name != ".." => Ok(name.to_owned()),
        _ => Err(unsafe_name()),
    }
}

fn unique_filename(filename: String, used: &mut HashSet<String>) -> String {
    if used.insert(filename.to_ascii_lowercase()) {
        return filename;
    }
    let (base, extension) = match filename.rsplit_once('.') {
        Some((base, extension)) if !base.is_empty() => (base.to_owned(), format!(".{extension}")),
        _ => (filename.clone(), String::new()),
    };
    (2..)
        .map(|counter| format!("{base}-{counter}{extension}"))
        .find(|candidate| used.insert(candidate.to_ascii_lowercase()))
        .expect("unbounded counter always finds a free name")
}

/// 包内 parameter 带 `PARM` + 长度 + CRC 包装时，解包后只保留文本内容。
fn unwrap_parameter_if_needed(path: &Path) -> Result<(), String> {
    let data = fs::read(path).map_err(|error| format!("解包固件条目失败：{error}"))?;
    if data.len() < 8 || &data[..4] != b"PARM" {
        return Ok(());
    }
    let length = le32(&data, 4) as usize;
    if 8 + length + 4 > data.len() {
        return Err("固件条目 parameter 的偏移或长度越界。".to_owned());
    }
    fs::write(path, &data[8..8 + length]).map_err(|error| format!("解包 parameter 失败：{error}"))
}

fn copy_range(
    source: &mut File,
    offset: u64,
    size: u64,
    destination: &Path,
    report: &mut dyn FnMut(u64),
    cancelled: &dyn Fn() -> bool,
) -> Result<(), String> {
    let name = destination
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let failed = |error: std::io::Error| format!("解包固件条目 {name} 失败：{error}");
    source.seek(SeekFrom::Start(offset)).map_err(failed)?;
    let mut output = File::create(destination).map_err(failed)?;
    let mut buffer = vec![0u8; COPY_CHUNK.min(size as usize).max(1)];
    let mut copied = 0u64;
    while copied < size {
        if cancelled() {
            return Err("操作已由用户中止".to_owned());
        }
        let count = (size - copied).min(buffer.len() as u64) as usize;
        source.read_exact(&mut buffer[..count]).map_err(failed)?;
        output.write_all(&buffer[..count]).map_err(failed)?;
        copied += count as u64;
        report(copied);
    }
    output.flush().map_err(failed)
}

fn read_exact_at(
    file: &mut File,
    offset: u64,
    count: usize,
    field: &str,
) -> Result<Vec<u8>, String> {
    let truncated = |_| format!("固件数据不完整：{field}。");
    file.seek(SeekFrom::Start(offset)).map_err(truncated)?;
    let mut data = vec![0u8; count];
    file.read_exact(&mut data).map_err(truncated)?;
    Ok(data)
}

fn extent_is_valid(offset: u64, size: u64, limit: u64) -> bool {
    offset <= limit && size <= limit - offset
}

fn le16(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([data[offset], data[offset + 1]])
}

fn le32(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

fn c_string(data: &[u8], offset: usize, count: usize) -> String {
    let field = &data[offset..offset + count];
    let end = field
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(field.len());
    String::from_utf8_lossy(&field[..end]).trim().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Part<'a> {
        name: &'a str,
        filename: &'a str,
        address: u32,
        payload: &'a [u8],
    }

    fn put32(data: &mut [u8], offset: usize, value: u32) {
        data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn put_str(data: &mut [u8], offset: usize, text: &str) {
        data[offset..offset + text.len()].copy_from_slice(text.as_bytes());
    }

    fn build_rkaf(parts: &[Part<'_>]) -> Vec<u8> {
        let table_end = RKAF_TABLE_OFFSET as usize + parts.len() * RKAF_ENTRY_SIZE as usize;
        let mut image = vec![0u8; table_end];
        put_str(&mut image, 0, "RKAF");
        put_str(&mut image, 0x08, "TEST-BOARD");
        put32(&mut image, 0x88, parts.len() as u32);
        for (index, part) in parts.iter().enumerate() {
            let base = RKAF_TABLE_OFFSET as usize + index * RKAF_ENTRY_SIZE as usize;
            let position = image.len() as u32;
            put_str(&mut image, base, part.name);
            put_str(&mut image, base + 0x20, part.filename);
            put32(&mut image, base + 0x60, position);
            put32(&mut image, base + 0x64, part.address);
            put32(&mut image, base + 0x6c, part.payload.len() as u32);
            image.extend_from_slice(part.payload);
        }
        image
    }

    fn build_rkfw(loader: &[u8], rkaf: &[u8], chip: u32) -> Vec<u8> {
        let mut image = vec![0u8; RKFW_HEADER_SIZE as usize];
        put_str(&mut image, 0, "RKFW");
        image[0x04..0x06].copy_from_slice(&(RKFW_HEADER_SIZE as u16).to_le_bytes());
        put32(&mut image, 0x06, 0x0102_0003);
        put32(&mut image, 0x15, chip);
        put32(&mut image, 0x19, RKFW_HEADER_SIZE as u32);
        put32(&mut image, 0x1d, loader.len() as u32);
        put32(
            &mut image,
            0x21,
            (RKFW_HEADER_SIZE as usize + loader.len()) as u32,
        );
        put32(&mut image, 0x25, rkaf.len() as u32);
        image.extend_from_slice(loader);
        image.extend_from_slice(rkaf);
        let mut hasher = Md5::new();
        hasher.update(&image);
        image.extend_from_slice(hasher.hex_digest().as_bytes());
        image
    }

    fn temp_dir(label: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "rkflash-firmware-{label}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn no_progress() -> (impl FnMut(u64, u64), impl Fn() -> bool) {
        (|_: u64, _: u64| {}, || false)
    }

    #[test]
    fn inspects_verifies_and_extracts_rkfw_update_image() {
        let mut loader = b"BOOT".to_vec();
        loader.extend_from_slice(&[0u8; 2]);
        loader.extend_from_slice(&0x0000_0102u32.to_le_bytes());
        loader.extend_from_slice(&[0xAA; 20]);
        let mut parameter = b"PARM".to_vec();
        let text = b"FIRMWARE_VER: 1.0\nTYPE: GPT\nCMDLINE: mtdparts=rk29xxnand:-@0x4000(rootfs)\n";
        parameter.extend_from_slice(&(text.len() as u32).to_le_bytes());
        parameter.extend_from_slice(text);
        parameter.extend_from_slice(&[0u8; 4]);
        let rkaf = build_rkaf(&[
            Part {
                name: "parameter",
                filename: "parameter.txt",
                address: 0,
                payload: &parameter,
            },
            Part {
                name: "boot",
                filename: "Image/boot.img",
                address: 0x8000,
                payload: b"BOOTDATA",
            },
            Part {
                name: "backup",
                filename: "RESERVED",
                address: 0,
                payload: b"",
            },
        ]);
        let chip = u32::from_le_bytes(*b"8853");
        let image = build_rkfw(&loader, &rkaf, chip);
        let directory = temp_dir("rkfw");
        let path = directory.join("update.img");
        fs::write(&path, &image).unwrap();

        let info = inspect(&path).unwrap();
        assert_eq!(info.container, Container::Rkfw);
        assert_eq!(info.chip_name(), "RK3588");
        assert_eq!(info.firmware_version.as_deref(), Some("1.2.3"));
        assert_eq!(info.loader_version.as_deref(), Some("1.2"));
        assert_eq!(info.model, "TEST-BOARD");
        assert_eq!(info.partitions.len(), 3);

        let (mut report, cancelled) = no_progress();
        let status = verify_md5(
            &info,
            Progress {
                report: &mut report,
                cancelled: &cancelled,
            },
        )
        .unwrap();
        assert_eq!(status, Md5Status::Valid);

        let output = directory.join("out");
        let extraction = extract(
            &info,
            &output,
            Progress {
                report: &mut report,
                cancelled: &cancelled,
            },
        )
        .unwrap();
        assert_eq!(fs::read(extraction.loader_path.unwrap()).unwrap(), loader);
        assert_eq!(extraction.items.len(), 2);
        assert_eq!(extraction.items[0].partition_name, "parameter");
        assert_eq!(fs::read(&extraction.items[0].image_path).unwrap(), text);
        assert_eq!(
            PartitionMetadata::inspect(&extraction.items[0].image_path).unwrap(),
            PartitionMetadata::GptParameter
        );
        assert_eq!(extraction.items[1].address, 0x8000);
        assert_eq!(
            extraction.items[1].image_path.file_name().unwrap(),
            "boot.img"
        );
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn detects_tampered_md5() {
        let rkaf = build_rkaf(&[Part {
            name: "boot",
            filename: "boot.img",
            address: 0,
            payload: b"X",
        }]);
        let mut image = build_rkfw(b"", &rkaf, 0x35);
        let last = image.len() - 40;
        image[last] ^= 0xff;
        let directory = temp_dir("md5");
        let path = directory.join("bad.img");
        fs::write(&path, &image).unwrap();
        let info = inspect(&path).unwrap();
        let (mut report, cancelled) = no_progress();
        let status = verify_md5(
            &info,
            Progress {
                report: &mut report,
                cancelled: &cancelled,
            },
        )
        .unwrap();
        assert_eq!(status, Md5Status::Invalid);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn rejects_out_of_bounds_partition_and_unsafe_names() {
        let mut rkaf = build_rkaf(&[Part {
            name: "boot",
            filename: "boot.img",
            address: 0,
            payload: b"DATA",
        }]);
        put32(&mut rkaf, RKAF_TABLE_OFFSET as usize + 0x6c, 0x7fff_ffff);
        let directory = temp_dir("bounds");
        let path = directory.join("bad.img");
        fs::write(&path, &rkaf).unwrap();
        assert!(inspect(&path).unwrap_err().contains("越界"));

        let partition = FirmwarePartition {
            name: "evil".to_owned(),
            filename: "..\\..\\evil.img".to_owned(),
            data_offset: 0,
            size: 1,
            nand_address: 0,
        };
        assert!(safe_output_name(&partition).is_err());
        fs::remove_dir_all(directory).unwrap();
    }

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
