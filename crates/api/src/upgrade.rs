//! 升级固件页：update.img 整包升级与按固件擦除。

use crate::StorageType;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirmwareUpgradeRequest {
    pub firmware: PathBuf,
    pub no_reset: bool,
    /// 写入前选择的目标存储；只在 [`crate::ToolInfo::firmware_storages`] 非空时生效。
    pub storage: StorageType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EraseFlashRequest {
    pub firmware: PathBuf,
}

pub trait UpgradeApi {
    fn firmware_size(&self, path: &Path) -> Result<u64, String>;
    fn check_upgrade_firmware(&self, request: &FirmwareUpgradeRequest) -> Result<(), String>;
    fn upgrade_firmware(&self, request: FirmwareUpgradeRequest) -> Result<(), String>;
    fn check_erase_flash(&self, request: &EraseFlashRequest) -> Result<(), String>;
    fn erase_flash(&self, request: EraseFlashRequest) -> Result<(), String>;
}
