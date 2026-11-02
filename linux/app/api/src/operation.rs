//! 与具体烧录工具无关的类型化操作契约。

use rkflash_config::ImageEntry;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceQuery {
    FlashId,
    FlashInfo,
    ChipInfo,
    Capability,
    PartitionTable,
    /// 复用设备枚举命令：输出包含每个 Rockchip 设备的 `SerialNo=` 字段。
    UsbSerialNumber,
    SecureMode,
    TestDevice,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceControl {
    Reboot,
    SwitchStorage,
    ClearSerial,
    ResetUsbPipe,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageType {
    Auto,
    Emmc,
    SpiNor,
    SpiNand,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlashOperation {
    DownloadImages {
        images: Vec<ImageEntry>,
        reset_after: bool,
    },
    UpgradeFirmware {
        firmware: PathBuf,
        no_reset: bool,
    },
    EraseFlash {
        firmware: PathBuf,
    },
    Query {
        query: DeviceQuery,
    },
    Control(DeviceControl),
    UpgradeLoader {
        loader: PathBuf,
        storage: StorageType,
    },
    ReadLba {
        start: String,
        count: String,
        output: PathBuf,
    },
    WriteLba {
        start: String,
        image: PathBuf,
    },
    EraseLba {
        start: String,
        count: String,
    },
    ExportSerialLog {
        output: PathBuf,
    },
}

impl FlashOperation {
    pub fn name(&self) -> &'static str {
        match self {
            Self::DownloadImages { .. } => "下载镜像",
            Self::UpgradeFirmware { .. } => "升级固件",
            Self::EraseFlash { .. } => "擦除 Flash",
            Self::Query { query, .. } => query.name(),
            Self::Control(control) => control.name(),
            Self::UpgradeLoader { .. } => "升级 Loader",
            Self::ReadLba { .. } => "读取 LBA",
            Self::WriteLba { .. } => "写入 LBA",
            Self::EraseLba { .. } => "擦除 LBA",
            Self::ExportSerialLog { .. } => "导出串口日志",
        }
    }
}

impl DeviceQuery {
    pub fn name(self) -> &'static str {
        match self {
            Self::FlashId => "读取 Flash ID",
            Self::FlashInfo => "读取 Flash 信息",
            Self::ChipInfo => "读取 Chip 信息",
            Self::Capability => "读取 Capability",
            Self::PartitionTable => "读取设备分区表",
            Self::UsbSerialNumber => "读取 USB 序列号",
            Self::SecureMode => "检测安全模式",
            Self::TestDevice => "测试设备",
        }
    }
}

impl DeviceControl {
    pub fn name(self) -> &'static str {
        match self {
            Self::Reboot => "重启设备",
            Self::SwitchStorage => "切换存储",
            Self::ClearSerial => "清空序列号",
            Self::ResetUsbPipe => "重置 USB 管道",
        }
    }
}
