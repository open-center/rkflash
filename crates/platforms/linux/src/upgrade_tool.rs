//! Rockchip 官方 upgrade_tool 的命令规划与输出协议（Linux 版使用）。

use crate::parser::parse_device_list;
use rkflash_application::domain::{
    normalize_sector, CommandSpec, DeviceControl, DeviceQuery, DeviceSnapshot, ExecutionPlan,
    FlashOperation, StorageType,
};
use rkflash_application::{FlashPlanner, ToolProtocol};
use rkflash_config::ImageEntry;
use rkflash_core::output::{extract_failure, extract_progress};
use std::ffi::OsString;
use std::path::PathBuf;

#[derive(Default, Clone)]
pub struct UpgradeToolAdapter;

/// upgrade_tool `DI` 命令按分区名写入时使用的参数。
const PARTITION_FLAGS: &[(&str, &str)] = &[
    ("parameter", "-p"),
    ("uboot", "-u"),
    ("trust", "-t"),
    ("misc", "-m"),
    ("resource", "-re"),
    ("kernel", "-k"),
    ("boot", "-b"),
    ("recovery", "-r"),
    ("system", "-s"),
];

fn partition_flag(name: &str) -> Option<&'static str> {
    PARTITION_FLAGS
        .iter()
        .find_map(|(partition, flag)| (*partition == name).then_some(*flag))
}

/// `UL` 命令的目标存储参数；自动时不传，由工具识别。
fn storage_token(storage: StorageType) -> Option<&'static str> {
    match storage {
        StorageType::Auto => None,
        StorageType::Emmc => Some("EMMC"),
        StorageType::SpiNor => Some("SPINOR"),
        StorageType::SpiNand => Some("SPINAND"),
    }
}

/// 扇区参数统一走域层校验后下发，杜绝把原始输入直接交给破坏性命令。
fn sector_arg(value: &str, label: &str) -> Result<OsString, String> {
    normalize_sector(value)
        .map(OsString::from)
        .map_err(|error| format!("{label}{error}"))
}

impl FlashPlanner for UpgradeToolAdapter {
    fn plan(&self, operation: &FlashOperation) -> Result<ExecutionPlan, String> {
        match operation {
            FlashOperation::DownloadImages {
                images,
                reset_after,
            } => self.download_plan(images, *reset_after),
            FlashOperation::UpgradeFirmware {
                firmware, no_reset, ..
            } => {
                let mut args = vec![OsString::from("UF"), firmware.as_os_str().to_owned()];
                if *no_reset {
                    args.push(OsString::from("-noreset"));
                }
                Ok(ExecutionPlan::single(
                    operation.name(),
                    CommandSpec::new("升级统一固件", args),
                ))
            }
            FlashOperation::EraseFlash { firmware } => Ok(ExecutionPlan::single(
                operation.name(),
                CommandSpec::new(
                    "擦除 Flash",
                    [OsString::from("EF"), firmware.as_os_str().to_owned()],
                ),
            )),
            FlashOperation::Query { query } => Ok(self.query_plan(*query)),
            FlashOperation::Control(control) => Ok(self.control_plan(*control)),
            FlashOperation::UpgradeLoader { loader, storage } => {
                let mut args = vec![OsString::from("UL"), loader.as_os_str().to_owned()];
                if let Some(token) = storage_token(*storage) {
                    args.push(OsString::from(token));
                }
                Ok(ExecutionPlan::single(
                    operation.name(),
                    CommandSpec::new(operation.name(), args),
                ))
            }
            FlashOperation::ReadLba {
                start,
                count,
                output,
            } => Ok(ExecutionPlan::single(
                operation.name(),
                CommandSpec::new(
                    "导出扇区",
                    [
                        OsString::from("RL"),
                        sector_arg(start, "起始扇区")?,
                        sector_arg(count, "扇区数量")?,
                        output.as_os_str().to_owned(),
                    ],
                ),
            )),
            FlashOperation::WriteLba { start, image } => Ok(ExecutionPlan::single(
                operation.name(),
                CommandSpec::new(
                    "写入扇区",
                    [
                        OsString::from("WL"),
                        sector_arg(start, "起始扇区")?,
                        image.as_os_str().to_owned(),
                    ],
                ),
            )),
            FlashOperation::EraseLba { start, count } => Ok(ExecutionPlan::single(
                operation.name(),
                CommandSpec::new(
                    "擦除扇区",
                    [
                        OsString::from("EL"),
                        sector_arg(start, "起始扇区")?,
                        sector_arg(count, "扇区数量")?,
                    ],
                ),
            )),
            FlashOperation::ExportSerialLog { output } => Ok(ExecutionPlan::single(
                operation.name(),
                CommandSpec::new(
                    operation.name(),
                    [OsString::from("RCL"), output.as_os_str().to_owned()],
                ),
            )),
        }
    }

    fn tool_name(&self) -> &'static str {
        "upgrade_tool"
    }
}

impl ToolProtocol for UpgradeToolAdapter {
    fn device_scan_command(&self) -> CommandSpec {
        CommandSpec::new("扫描 Rockusb 设备", ["LD"])
    }

    fn parse_device_list(&self, text: &str) -> DeviceSnapshot {
        parse_device_list(text)
    }

    fn parse_progress(&self, line: &str) -> Option<f32> {
        extract_progress(line)
    }

    fn parse_failure(&self, line: &str) -> Option<String> {
        extract_failure(line)
    }
}

impl UpgradeToolAdapter {
    pub fn runtime_log_relative_path() -> PathBuf {
        PathBuf::from("upgrade_tool/log")
    }

    fn download_plan(
        &self,
        images: &[ImageEntry],
        reset_after: bool,
    ) -> Result<ExecutionPlan, String> {
        let mut commands = Vec::new();
        for image in images.iter().filter(|image| image.enabled) {
            let name = image.name.to_ascii_lowercase();
            if let Some(flag) = partition_flag(&name) {
                commands.push(CommandSpec::new(
                    format!("写入 {}", image.name),
                    [
                        OsString::from("DI"),
                        OsString::from(flag),
                        OsString::from(&image.path),
                    ],
                ));
            } else {
                if image.address.trim().is_empty() {
                    return Err(format!("分区“{}”需要填写起始扇区", image.name));
                }
                // 破坏性的 WL 命令只接收经过解析校验、去除首尾空白的扇区值。
                let address = sector_arg(&image.address, &format!("分区“{}”起始扇区", image.name))?;
                commands.push(CommandSpec::new(
                    format!("按地址写入 {}", image.name),
                    [OsString::from("WL"), address, OsString::from(&image.path)],
                ));
            }
        }
        if reset_after {
            commands.push(CommandSpec::new("重启设备", ["RD"]));
        }
        Ok(ExecutionPlan::new("下载镜像", commands))
    }

    fn query_plan(&self, query: DeviceQuery) -> ExecutionPlan {
        let code = match query {
            DeviceQuery::FlashId => "RID",
            DeviceQuery::FlashInfo => "RFI",
            DeviceQuery::ChipInfo => "RCI",
            DeviceQuery::Capability => "RCB",
            DeviceQuery::PartitionTable => "PL",
            DeviceQuery::UsbSerialNumber => "LD",
            DeviceQuery::SecureMode => "RSM",
            DeviceQuery::TestDevice => "TD",
        };
        ExecutionPlan::single(query.name(), CommandSpec::new(query.name(), [code]))
    }

    fn control_plan(&self, control: DeviceControl) -> ExecutionPlan {
        let args: Vec<OsString> = match control {
            DeviceControl::Reboot => vec![OsString::from("RD")],
            DeviceControl::SwitchStorage => vec![OsString::from("SSD")],
            DeviceControl::ClearSerial => vec![OsString::from("SN"), OsString::new()],
            DeviceControl::ResetUsbPipe => vec![OsString::from("RP")],
        };
        ExecutionPlan::single(control.name(), CommandSpec::new(control.name(), args))
    }
}

#[cfg(test)]
mod tests {
    use super::UpgradeToolAdapter;
    use rkflash_application::domain::{DeviceQuery, FlashOperation, StorageType};
    use rkflash_application::FlashPlanner;
    use rkflash_config::ImageEntry;
    use std::path::PathBuf;

    #[test]
    fn download_mapping_is_driven_by_rockchip_profile() {
        let adapter = UpgradeToolAdapter;
        let mut boot = ImageEntry::new(true, "0x20000", "boot");
        boot.path = "boot.img".to_owned();
        let mut vendor = ImageEntry::new(true, "0x50000", "vendor");
        vendor.path = "vendor.img".to_owned();

        let plan = adapter
            .plan(&FlashOperation::DownloadImages {
                images: vec![boot, vendor],
                reset_after: true,
            })
            .unwrap();

        let opcodes: Vec<_> = plan
            .commands
            .iter()
            .map(|command| command.args[0].to_string_lossy().into_owned())
            .collect();
        assert_eq!(opcodes, ["DI", "WL", "RD"]);
        assert_eq!(plan.commands[0].args[1], "-b");
        assert_eq!(plan.commands[1].args[1], "0x50000");
    }

    #[test]
    fn rejects_invalid_custom_partition_address_before_planning() {
        let adapter = UpgradeToolAdapter;
        let mut entry = ImageEntry::new(true, "0xZZZZ", "custom");
        entry.path = "custom.img".to_owned();

        let error = adapter
            .plan(&FlashOperation::DownloadImages {
                images: vec![entry],
                reset_after: false,
            })
            .unwrap_err();
        assert!(error.contains("custom"));
    }

    #[test]
    fn storage_specialization_stays_in_adapter() {
        let adapter = UpgradeToolAdapter;
        let plan = adapter
            .plan(&FlashOperation::UpgradeLoader {
                loader: PathBuf::from("MiniLoaderAll.bin"),
                storage: StorageType::SpiNor,
            })
            .unwrap();
        assert_eq!(plan.commands[0].args[0], "UL");
        assert_eq!(plan.commands[0].args[2], "SPINOR");
    }

    #[test]
    fn query_does_not_change_device_mode() {
        let adapter = UpgradeToolAdapter;
        let plan = adapter
            .plan(&FlashOperation::Query {
                query: DeviceQuery::FlashId,
            })
            .unwrap();

        assert_eq!(plan.commands.len(), 1);
        assert_eq!(plan.commands[0].args[0], "RID");
    }

    #[test]
    fn serial_number_query_uses_usb_enumeration_value() {
        let adapter = UpgradeToolAdapter;
        let plan = adapter
            .plan(&FlashOperation::Query {
                query: DeviceQuery::UsbSerialNumber,
            })
            .unwrap();

        assert_eq!(plan.commands.len(), 1);
        assert_eq!(plan.commands[0].args[0], "LD");
    }
}
