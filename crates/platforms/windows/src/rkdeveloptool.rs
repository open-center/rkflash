//! rkdeveloptool 后端适配器：把类型化操作映射为 rkflash-core 的命令与工作流（Windows 版使用）。

use rkflash_application::domain::{
    normalize_sector, CommandSpec, DeviceControl, DeviceInfo, DeviceQuery, DeviceSnapshot,
    ExecutionPlan, FlashOperation, StorageType,
};
use rkflash_application::{FlashPlanner, ToolProtocol};
use rkflash_config::ImageEntry;
use rkflash_core::firmware;
use rkflash_core::output::extract_progress;
use rkflash_core::partition::is_standard_partition;
use rkflash_core::rkdeveloptool::{self as protocol, Storage};
use rkflash_core::workflow::{
    DownloadItem, DownloadTarget, DownloadWorkflow, EraseFlashWorkflow, FirmwareUpgradeWorkflow,
};
use std::ffi::OsString;
use std::path::PathBuf;

const UNSUPPORTED: &str = "rkdeveloptool 不支持该操作";

#[derive(Default, Clone)]
pub struct RkDevelopToolAdapter;

fn sector_arg(value: &str, label: &str) -> Result<OsString, String> {
    normalize_sector(value)
        .map(OsString::from)
        .map_err(|error| format!("{label}{error}"))
}

/// 界面的存储选项映射为 rkdeveloptool 支持的存储；SPI NAND 没有对应的 `cs` 编号。
fn target_storage(storage: StorageType) -> Result<Storage, String> {
    match storage {
        StorageType::Auto => Ok(Storage::Auto),
        StorageType::Emmc => Ok(Storage::Emmc),
        StorageType::SpiNor => Ok(Storage::SpiNor),
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
                storage: target,
            } => {
                let target = target_storage(*target)?;
                Ok(ExecutionPlan::workflow(
                    name,
                    Box::new(FirmwareUpgradeWorkflow::new(
                        firmware::inspect(firmware)?,
                        target,
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
                if let Some(code) = target_storage(*storage)?.code() {
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
            | FlashOperation::UpgradeFirmware { storage, .. } => target_storage(*storage).is_ok(),
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

impl RkDevelopToolAdapter {
    fn download_items(&self, images: &[ImageEntry]) -> Result<Vec<DownloadItem>, String> {
        let mut items = Vec::new();
        for image in images.iter().filter(|image| image.enabled) {
            if image.path.trim().is_empty() {
                return Err(format!("分区“{}”未选择镜像文件", image.name));
            }
            let target = if is_standard_partition(&image.name) {
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

    pub fn runtime_log_relative_path() -> PathBuf {
        PathBuf::from("log")
    }
}

impl ToolProtocol for RkDevelopToolAdapter {
    fn device_scan_command(&self) -> CommandSpec {
        CommandSpec::new("扫描 Rockusb 设备", ["ld"])
    }

    fn parse_device_list(&self, text: &str) -> DeviceSnapshot {
        DeviceSnapshot {
            devices: protocol::parse_device_list(text)
                .into_iter()
                .map(|device| DeviceInfo {
                    number: device.number,
                    vid: device.vid,
                    pid: device.pid,
                    location: device.location,
                    mode: device.mode,
                    serial: String::new(),
                })
                .collect(),
            error: None,
        }
    }

    fn parse_progress(&self, line: &str) -> Option<f32> {
        extract_progress(line)
    }

    fn parse_failure(&self, line: &str) -> Option<String> {
        protocol::parse_failure(line)
    }

    fn is_transient(&self, line: &str) -> bool {
        protocol::is_transient(line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_queries_and_rejects_unsupported_operations() {
        let adapter = RkDevelopToolAdapter;
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
        let adapter = RkDevelopToolAdapter;
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
        let adapter = RkDevelopToolAdapter;
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
    fn protocol_delegates_device_and_progress_parsing_to_core() {
        let adapter = RkDevelopToolAdapter;
        let snapshot =
            adapter.parse_device_list("DevNo=1\tVid=0x2207,Pid=0x350b,LocationID=104\tMaskrom");
        assert_eq!(snapshot.devices[0].mode, "Maskrom");
        assert!(adapter.is_transient("Write LBA from file (45%)"));
        assert_eq!(
            adapter.parse_progress("Write LBA from file (45%)"),
            Some(0.45)
        );
    }
}
