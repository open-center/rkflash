//! 高级功能页：设备查询、设备控制、Loader 升级、LBA 读写擦与串口日志导出。

use crate::StorageType;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceQuery {
    FlashId,
    FlashInfo,
    ChipInfo,
    Capability,
    PartitionTable,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoaderUpgradeRequest {
    pub loader: PathBuf,
    pub storage: StorageType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LbaReadRequest {
    pub start: String,
    pub count: String,
    pub output: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LbaWriteRequest {
    pub start: String,
    pub image: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LbaEraseRequest {
    pub start: String,
    pub count: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerialLogRequest {
    pub output: PathBuf,
}

pub trait AdvancedApi {
    fn query_device(&self, query: DeviceQuery) -> Result<(), String>;
    fn check_control_device(&self, control: DeviceControl) -> Result<(), String>;
    fn control_device(&self, control: DeviceControl) -> Result<(), String>;
    fn check_upgrade_loader(&self, request: &LoaderUpgradeRequest) -> Result<(), String>;
    fn upgrade_loader(&self, request: LoaderUpgradeRequest) -> Result<(), String>;
    fn read_lba(&self, request: LbaReadRequest) -> Result<(), String>;
    fn check_write_lba(&self, request: &LbaWriteRequest) -> Result<(), String>;
    fn write_lba(&self, request: LbaWriteRequest) -> Result<(), String>;
    fn check_erase_lba(&self, request: &LbaEraseRequest) -> Result<(), String>;
    fn erase_lba(&self, request: LbaEraseRequest) -> Result<(), String>;
    fn export_serial_log(&self, request: SerialLogRequest) -> Result<(), String>;
}

impl DeviceQuery {
    pub const ALL: [Self; 8] = [
        Self::FlashId,
        Self::FlashInfo,
        Self::ChipInfo,
        Self::Capability,
        Self::PartitionTable,
        Self::UsbSerialNumber,
        Self::SecureMode,
        Self::TestDevice,
    ];

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
    pub const ALL: [Self; 4] = [
        Self::Reboot,
        Self::SwitchStorage,
        Self::ClearSerial,
        Self::ResetUsbPipe,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Reboot => "重启设备",
            Self::SwitchStorage => "切换存储",
            Self::ClearSerial => "清空序列号",
            Self::ResetUsbPipe => "重置 USB 管道",
        }
    }
}
