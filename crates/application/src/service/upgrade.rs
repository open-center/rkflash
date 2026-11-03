use super::FlashApplication;
use crate::domain::FlashOperation;
use rkflash_api::{EraseFlashRequest, FirmwareUpgradeRequest, UpgradeApi};
use std::path::Path;

impl UpgradeApi for FlashApplication {
    fn firmware_size(&self, path: &Path) -> Result<u64, String> {
        self.file_system.file_size(path)
    }

    fn check_upgrade_firmware(&self, request: &FirmwareUpgradeRequest) -> Result<(), String> {
        self.check(&FlashOperation::from(request.clone()))
    }

    fn upgrade_firmware(&self, request: FirmwareUpgradeRequest) -> Result<(), String> {
        self.run(request.into())
    }

    fn check_erase_flash(&self, request: &EraseFlashRequest) -> Result<(), String> {
        self.check(&FlashOperation::from(request.clone()))
    }

    fn erase_flash(&self, request: EraseFlashRequest) -> Result<(), String> {
        self.run(request.into())
    }
}
