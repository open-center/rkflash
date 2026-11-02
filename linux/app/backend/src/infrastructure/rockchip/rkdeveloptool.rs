//! 开源 rkdeveloptool（内置 libusb）的命令规划与输出协议；Windows 版使用该后端。

use super::firmware;
use super::parameter::parse_parameter;
use super::parser::{extract_failure, extract_progress};
use super::profile::RockchipProfile;
use super::workflow::{DownloadWorkflow, EraseFlashWorkflow, FirmwareUpgradeWorkflow};
use crate::application::{FlashPlanner, FlashProfile, ToolProtocol};
use crate::domain::{
    normalize_sector, CommandSpec, DeviceControl, DeviceInfo, DeviceQuery, DeviceSnapshot,
    ExecutionPlan, FlashOperation, StorageType,
};
use rkflash_config::ImageEntry;
use std::ffi::OsString;
use std::path::PathBuf;

const UNSUPPORTED: &str = "rkdeveloptool 不支持该操作";

#[derive(Default)]
pub(crate) struct RkDevelopToolAdapter {
    profile: RockchipProfile,
}

fn sector_arg(value: &str, label: &str) -> Result<OsString, String> {
    normalize_sector(value)
        .map(OsString::from)
        .map_err(|error| format!("{label}{error}"))
}

/// `cs` 命令的存储编号：1=eMMC，2=SD，9=SPI NOR。
pub(super) fn storage_code(storage: StorageType) -> Result<Option<&'static str>, String> {
    match storage {
        StorageType::Auto => Ok(None),
        StorageType::Emmc => Ok(Some("1")),
        StorageType::SpiNor => Ok(Some("9")),
        StorageType::SpiNand => Err(format!("{UNSUPPORTED}：SPI NAND 存储")),
    }
}

impl FlashPlanner for RkDevelopToolAdapter {
    fn plan(&self, operation: &FlashOperation) -> Result<ExecutionPlan, String> {
        let name = operation.name();
        match operation {
            FlashOperation::DownloadImages {
                images,
                reset_after,
            } => Ok(ExecutionPlan::workflow(
                name,
                Box::new(DownloadWorkflow::new(
                    self.download_items(images)?,
                    *reset_after,
                )),
            )),
            FlashOperation::UpgradeFirmware {
                firmware,
                no_reset,
                storage,
            } => {
                storage_code(*storage)?;
                Ok(ExecutionPlan::workflow(
                    name,
                    Box::new(FirmwareUpgradeWorkflow::new(
                        firmware::inspect(firmware)?,
                        *storage,
                        !*no_reset,
                    )),
                ))
            }
            FlashOperation::EraseFlash { firmware } => Ok(ExecutionPlan::workflow(
                name,
                Box::new(EraseFlashWorkflow::new(firmware::inspect(firmware)?)),
            )),
            FlashOperation::Query { query } => {
                let code = query_code(*query).ok_or_else(|| format!("{UNSUPPORTED}：{name}"))?;
                Ok(ExecutionPlan::single(name, CommandSpec::new(name, [code])))
            }
            FlashOperation::Control(DeviceControl::Reboot) => {
                Ok(ExecutionPlan::single(name, CommandSpec::new(name, ["rd"])))
            }
            FlashOperation::Control(_) | FlashOperation::ExportSerialLog { .. } => {
                Err(format!("{UNSUPPORTED}：{name}"))
            }
            FlashOperation::UpgradeLoader { loader, storage } => {
                let mut commands = Vec::new();
                if let Some(code) = storage_code(*storage)? {
                    commands.push(CommandSpec::new("切换存储", ["cs", code]));
                }
                commands.push(CommandSpec::new(
                    name,
                    [OsString::from("ul"), loader.as_os_str().to_owned()],
                ));
                Ok(ExecutionPlan::new(name, commands))
            }
            FlashOperation::ReadLba {
                start,
                count,
                output,
            } => Ok(ExecutionPlan::single(
                name,
                CommandSpec::new(
                    "导出扇区",
                    [
                        OsString::from("rl"),
                        sector_arg(start, "起始扇区")?,
                        sector_arg(count, "扇区数量")?,
                        output.as_os_str().to_owned(),
                    ],
                ),
            )),
            FlashOperation::WriteLba { start, image } => Ok(ExecutionPlan::single(
                name,
                CommandSpec::new(
                    "写入扇区",
                    [
                        OsString::from("wl"),
                        sector_arg(start, "起始扇区")?,
                        image.as_os_str().to_owned(),
                    ],
                ),
            )),
            FlashOperation::EraseLba { start, count } => Ok(ExecutionPlan::single(
                name,
                CommandSpec::new(
                    "擦除扇区",
                    [
                        OsString::from("el"),
                        sector_arg(start, "起始扇区")?,
                        sector_arg(count, "扇区数量")?,
                    ],
                ),
            )),
        }
    }

    fn supports(&self, operation: &FlashOperation) -> bool {
        match operation {
            FlashOperation::Query { query } => query_code(*query).is_some(),
            FlashOperation::Control(control) => *control == DeviceControl::Reboot,
            FlashOperation::ExportSerialLog { .. } => false,
            FlashOperation::UpgradeLoader { storage, .. }
            | FlashOperation::UpgradeFirmware { storage, .. } => storage_code(*storage).is_ok(),
            _ => true,
        }
    }

    fn tool_name(&self) -> &'static str {
        "rkdeveloptool"
    }
}

fn query_code(query: DeviceQuery) -> Option<&'static str> {
    match query {
        DeviceQuery::FlashId => Some("rid"),
        DeviceQuery::FlashInfo => Some("rfi"),
        DeviceQuery::ChipInfo => Some("rci"),
        DeviceQuery::Capability => Some("rcb"),
        DeviceQuery::PartitionTable => Some("ppt"),
        DeviceQuery::TestDevice => Some("td"),
        DeviceQuery::UsbSerialNumber | DeviceQuery::SecureMode => None,
    }
}

/// 下载列表中的一项：已知的 Rockchip 分区按名称写入，其余按起始扇区写入。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum DownloadTarget {
    Partition(String),
    Sector(OsString),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DownloadItem {
    pub name: String,
    pub target: DownloadTarget,
    pub path: PathBuf,
}

impl RkDevelopToolAdapter {
    fn download_items(&self, images: &[ImageEntry]) -> Result<Vec<DownloadItem>, String> {
        let mut items = Vec::new();
        for image in images.iter().filter(|image| image.enabled) {
            if image.path.trim().is_empty() {
                return Err(format!("分区“{}”未选择镜像文件", image.name));
            }
            let lower = image.name.trim().to_ascii_lowercase();
            let target = if self.profile.partition_flag(&lower).is_some() {
                DownloadTarget::Partition(image.name.trim().to_owned())
            } else {
                if image.address.trim().is_empty() {
                    return Err(format!("分区“{}”需要填写起始扇区", image.name));
                }
                DownloadTarget::Sector(sector_arg(
                    &image.address,
                    &format!("分区“{}”起始扇区", image.name),
                )?)
            };
            items.push(DownloadItem {
                name: image.name.clone(),
                target,
                path: PathBuf::from(&image.path),
            });
        }
        if items.is_empty() {
            return Err("没有启用的镜像".to_owned());
        }
        Ok(items)
    }

    pub(crate) fn runtime_log_relative_path() -> PathBuf {
        PathBuf::from("log")
    }
}

impl FlashProfile for RkDevelopToolAdapter {
    fn default_images(&self) -> Vec<ImageEntry> {
        self.profile.default_images()
    }

    fn parse_parameter(&self, text: &str) -> Result<Vec<ImageEntry>, String> {
        parse_parameter(text)
    }
}

impl ToolProtocol for RkDevelopToolAdapter {
    fn device_scan_command(&self) -> CommandSpec {
        CommandSpec::new("扫描 Rockusb 设备", ["ld"])
    }

    fn parse_device_list(&self, text: &str) -> DeviceSnapshot {
        DeviceSnapshot {
            devices: parse_device_list(text),
            error: None,
        }
    }

    fn parse_progress(&self, line: &str) -> Option<f32> {
        extract_progress(line)
    }

    fn parse_failure(&self, line: &str) -> Option<String> {
        let lower = line.to_ascii_lowercase();
        if lower.contains("does not support") || lower.contains("command is invalid") {
            return Some(line.trim().to_owned());
        }
        extract_failure(line)
    }

    fn is_transient(&self, line: &str) -> bool {
        (line.starts_with("Write LBA from file (") || line.starts_with("Read LBA to file ("))
            && !line.contains("(100%)")
    }
}

/// 解析 `DevNo=1\tVid=0x2207,Pid=0x350b,LocationID=104\tMaskrom`。
pub(super) fn parse_device_list(text: &str) -> Vec<DeviceInfo> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            let rest = line.strip_prefix("DevNo=")?;
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
            Some(DeviceInfo {
                number: fields.first()?.to_string(),
                vid: value("Vid="),
                pid: value("Pid="),
                location: value("LocationID="),
                mode: fields
                    .last()
                    .filter(|field| !field.contains('='))?
                    .to_string(),
                serial: String::new(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rkdeveloptool_device_list() {
        let devices = parse_device_list(
            "DevNo=1\tVid=0x2207,Pid=0x350b,LocationID=104\tMaskrom\r\nDevNo=2\tVid=0x2207,Pid=0x330c,LocationID=203\tLoader\r\n",
        );
        assert_eq!(devices.len(), 2);
        assert_eq!(devices[0].number, "1");
        assert_eq!(devices[0].pid, "0x350b");
        assert_eq!(devices[0].mode, "Maskrom");
        assert_eq!(devices[1].location, "203");
        assert_eq!(devices[1].mode, "Loader");
        assert!(parse_device_list("Not found any devices!\r\n").is_empty());
    }

    #[test]
    fn maps_queries_and_rejects_unsupported_operations() {
        let adapter = RkDevelopToolAdapter::default();
        let plan = adapter
            .plan(&FlashOperation::Query {
                query: DeviceQuery::PartitionTable,
            })
            .unwrap();
        assert_eq!(plan.commands[0].args[0], "ppt");
        assert!(!adapter.supports(&FlashOperation::Query {
            query: DeviceQuery::SecureMode
        }));
        assert!(adapter
            .plan(&FlashOperation::Control(DeviceControl::ClearSerial))
            .is_err());
        assert!(!adapter.supports(&FlashOperation::UpgradeLoader {
            loader: PathBuf::new(),
            storage: StorageType::SpiNand
        }));
    }

    #[test]
    fn loader_upgrade_selects_storage_first() {
        let adapter = RkDevelopToolAdapter::default();
        let plan = adapter
            .plan(&FlashOperation::UpgradeLoader {
                loader: PathBuf::from("MiniLoaderAll.bin"),
                storage: StorageType::SpiNor,
            })
            .unwrap();
        let args: Vec<_> = plan
            .commands
            .iter()
            .map(|command| command.args[0].to_string_lossy().into_owned())
            .collect();
        assert_eq!(args, ["cs", "ul"]);
        assert_eq!(plan.commands[0].args[1], "9");
    }

    #[test]
    fn download_uses_partition_names_for_known_images_and_sectors_otherwise() {
        let adapter = RkDevelopToolAdapter::default();
        let mut boot = ImageEntry::new(true, "0x20000", "boot");
        boot.path = "boot.img".to_owned();
        let mut custom = ImageEntry::new(true, "0x50000", "vendor");
        custom.path = "vendor.img".to_owned();
        let items = adapter.download_items(&[boot, custom]).unwrap();
        assert_eq!(
            items[0].target,
            DownloadTarget::Partition("boot".to_owned())
        );
        assert_eq!(items[1].target, DownloadTarget::Sector("0x50000".into()));
    }

    #[test]
    fn progress_refresh_lines_stay_out_of_the_log() {
        let adapter = RkDevelopToolAdapter::default();
        assert!(adapter.is_transient("Write LBA from file (45%)"));
        assert!(!adapter.is_transient("Write LBA from file (100%)"));
        assert_eq!(
            adapter.parse_progress("Write LBA from file (45%)"),
            Some(0.45)
        );
    }
}
